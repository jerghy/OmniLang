"""WebSocket /api/watch — 文件轮询 + 实时编译推送 + heartbeat。（M1a）

复用 omnic watch (omniview/cli.py:596) 的 os.path.getmtime 轮询策略。
不使用 watchdog — 避免双文件监听竞态 (F5)。
"""

import asyncio
import os
import threading
import time
from pathlib import Path

from fastapi import APIRouter, WebSocket, WebSocketDisconnect, Request

router = APIRouter()


class OmniFilePoller:
    """轮询式文件监听——复用 omnic watch 的 os.path.getmtime 策略 (F5)。

    omnic watch (omniview/cli.py:596) 每 500ms 轮询 os.path.getmtime。
    编辑器复用相同策略，检测 .omni 文件变化后触发编译推送。
    """

    def __init__(self, websocket: WebSocket, project_dir: Path, interval: float = 0.5):
        self._ws = websocket
        self._project_dir = project_dir
        self._interval = interval
        self._mtimes: dict[str, float] = {}
        self._running = False
        self._thread: threading.Thread | None = None
        # Initial snapshot of file timestamps
        self._snapshot_mtimes()

    def _snapshot_mtimes(self) -> None:
        """Capture initial timestamps so only subsequent changes fire."""
        for omni_file in self._project_dir.rglob("*.omni"):
            if omni_file.name.startswith("."):
                continue
            try:
                self._mtimes[str(omni_file)] = omni_file.stat().st_mtime
            except OSError:
                pass

    def start(self) -> None:
        self._running = True
        self._thread = threading.Thread(target=self._poll_loop, daemon=True)
        self._thread.start()

    def stop(self) -> None:
        self._running = False

    def _poll_loop(self) -> None:
        pending_compile: set[str] = set()
        debounce_lock = threading.Lock()

        while self._running:
            time.sleep(self._interval)
            for omni_file in self._project_dir.rglob("*.omni"):
                if omni_file.name.startswith("."):
                    continue
                try:
                    mtime = omni_file.stat().st_mtime
                except OSError:
                    continue
                key = str(omni_file)
                if self._mtimes.get(key) != mtime:
                    self._mtimes[key] = mtime
                    with debounce_lock:
                        pending_compile.add(key)

            # Debounce: fire only after 200ms of no new changes
            if pending_compile:
                time.sleep(0.2)
                with debounce_lock:
                    if pending_compile:
                        # Compile all changed files
                        for key in list(pending_compile):
                            try:
                                self._compile_and_send(Path(key))
                            except Exception:
                                pass  # Don't crash poller on compile errors
                        pending_compile.clear()

    def _compile_and_send(self, omni_file: Path) -> None:
        """Compile a .omni file and send result via WebSocket."""
        from omniview.api import compile_omni

        try:
            source = omni_file.read_text(encoding="utf-8")
        except Exception:
            return

        rel_path = str(omni_file.relative_to(self._project_dir)).replace("\\", "/")

        result = compile_omni(source, filepath=rel_path, target="react")

        message = {
            "type": "compiled",
            "files": result.files,
            "diff": {k: v for k, v in (result.diff or {}).items()},
            "errors": [e.to_dict() for e in result.errors],
        }

        # Schedule the async send on the event loop
        try:
            loop = asyncio.get_event_loop()
            asyncio.run_coroutine_threadsafe(self._ws.send_json(message), loop)
        except RuntimeError:
            pass  # Event loop not available


@router.websocket("/watch")
async def watch_endpoint(ws: WebSocket):
    await ws.accept()

    from editor.server.main import app
    project_dir_str = getattr(app.state, "project_dir", None)
    if not project_dir_str:
        await ws.send_json({"type": "error", "message": "No project directory configured"})
        await ws.close()
        return

    project_dir = Path(project_dir_str)
    poller = OmniFilePoller(ws, project_dir)
    poller.start()

    # Register heartbeat
    from editor.server.heartbeat import HeartbeatManager
    try:
        from editor.server.main import heartbeat_manager
    except (ImportError, AttributeError):
        heartbeat_manager = HeartbeatManager()
        heartbeat_manager.start()

    try:
        while True:
            data = await ws.receive_json()
            if data.get("type") == "ping":
                heartbeat_manager.record_ping()
    except (WebSocketDisconnect, RuntimeError):
        pass
    finally:
        poller.stop()
