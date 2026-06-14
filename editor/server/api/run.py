"""Vite DevServer manager — /api/run endpoints (Phase 1 Runtime Bridge).

Manages a Vite dev server for the compiled React application:
- Creates .omni-run/ project in the user's project directory
- Generates Vite scaffolding (package.json, vite.config.ts, etc.)
- Starts/stops Vite as a subprocess
- Writes compiled React files → Vite HMR auto-reloads browser tab

Routes:
    POST /api/run/start     — Create Vite project + install deps + start server
    POST /api/run/stop      — Kill Vite subprocess
    GET  /api/run/status    — Return {running, url}
    POST /api/run/write-files — Write compiled files to .omni-run/src/
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

from fastapi import APIRouter, Request
from pydantic import BaseModel

router = APIRouter()

DEFAULT_VITE_PORT = 5173
MAX_PORT_RETRIES = 10


# ── Request models ──────────────────────────────────────

class StartRunRequest(BaseModel):
    files: dict[str, str]  # { "App.tsx": "source code", ... }
    debug: bool = False    # Enable runtime debug mode


class WriteFilesRequest(BaseModel):
    files: dict[str, str]  # { "App.tsx": "updated source", ... }


# ── Helpers ─────────────────────────────────────────────

def _detect_component_name(files: dict[str, str]) -> str:
    """Detect the primary component name from compiled files.

    Looks for `export function X()` or `export { X }` in .tsx/.js files.
    Falls back to using the first file's basename.
    """
    import re
    for filename, content in files.items():
        ext = Path(filename).suffix
        if ext not in (".tsx", ".js"):
            continue
        # Match: export function Counter() { ... } or export function Counter(mountEl) {
        m = re.search(r"export\s+function\s+(\w+)\s*\(", content)
        if m:
            return m.group(1)
    # Fallback: use first .tsx/.js filename without extension
    for filename in files:
        ext = Path(filename).suffix
        if ext in (".tsx", ".js"):
            return Path(filename).stem
    return "App"


def _detect_target(files: dict[str, str]) -> str:
    """Detect target from compiled file extensions."""
    for filename in files:
        if filename.endswith(".js"):
            return "omniview"
        if filename.endswith(".tsx"):
            return "react"
    return "react"


def _copy_runtime_file(run_dir: Path, target: str = "react") -> None:
    """Copy runtime file into .omni-run/src/ for debug mode."""
    import shutil as _shutil
    runtime_name = "omniview-runtime.ts" if target == "omniview" else "omni-runtime.ts"
    # Find the runtime source file relative to the editor server
    runtime_src = Path(__file__).parent.parent.parent / "runtime" / "src" / runtime_name
    if not runtime_src.exists():
        # Fallback: try relative to project root
        runtime_src = Path(__file__).parent.parent.parent.parent / "editor" / "runtime" / "src" / runtime_name
    if runtime_src.exists():
        src_dir = run_dir / "src"
        src_dir.mkdir(parents=True, exist_ok=True)
        _shutil.copy2(runtime_src, src_dir / "omni-runtime.ts")


def _find_available_port(start: int = DEFAULT_VITE_PORT) -> int:
    """Find the first available port starting from `start`."""
    import socket
    for port in range(start, start + MAX_PORT_RETRIES):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            try:
                s.bind(("127.0.0.1", port))
                return port
            except OSError:
                continue
    return start  # Fallback — let Vite report the error


def _ensure_run_dir(project_dir: Path) -> Path:
    """Create .omni-run/ directory if it doesn't exist."""
    run_dir = project_dir / ".omni-run"
    run_dir.mkdir(parents=True, exist_ok=True)
    return run_dir


def _scaffold_vite_project(run_dir: Path, project_name: str, component_name: str = "App",
                          debug_mode: bool = False, debug_ws_port: int = 8942,
                          target: str = "react") -> None:
    """Generate Vite project scaffolding files (reuses templates from omniview/cli.py)."""
    src_dir = run_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)

    is_omniview = (target == "omniview")

    # package.json
    if is_omniview:
        package_json = {
            "name": project_name,
            "private": True,
            "type": "module",
            "scripts": {
                "dev": "vite",
                "build": "vite build",
                "preview": "vite preview",
            },
            "dependencies": {},
            "devDependencies": {
                "typescript": "^5.5.0",
                "vite": "^5.4.0",
            },
        }
    else:
        package_json = {
            "name": project_name,
            "private": True,
            "type": "module",
            "scripts": {
                "dev": "vite",
                "build": "vite build",
                "preview": "vite preview",
            },
            "dependencies": {
                "react": "^18.3.1",
                "react-dom": "^18.3.1",
                "zustand": "^4.5.0",
            },
            "devDependencies": {
                "@types/react": "^18.3.0",
                "@types/react-dom": "^18.3.0",
                "@vitejs/plugin-react": "^4.3.0",
                "typescript": "^5.5.0",
                "vite": "^5.4.0",
            },
        }
    with open(run_dir / "package.json", "w", encoding="utf-8") as f:
        json.dump(package_json, f, indent=2)
        f.write("\n")

    # tsconfig.json
    if is_omniview:
        tsconfig = {
            "compilerOptions": {
                "target": "ES2020",
                "module": "ESNext",
                "moduleResolution": "bundler",
                "strict": True,
                "esModuleInterop": True,
                "skipLibCheck": True,
                "noEmit": True,
            },
            "include": ["src/**/*.ts", "src/**/*.js"],
            "exclude": ["node_modules"],
        }
    else:
        tsconfig = {
            "compilerOptions": {
                "target": "ES2020",
                "module": "ESNext",
                "moduleResolution": "bundler",
                "jsx": "react-jsx",
                "strict": True,
                "esModuleInterop": True,
                "skipLibCheck": True,
                "noEmit": True,
            },
            "include": ["src/**/*.ts", "src/**/*.tsx"],
            "exclude": ["node_modules"],
        }
    with open(run_dir / "tsconfig.json", "w", encoding="utf-8") as f:
        json.dump(tsconfig, f, indent=2)
        f.write("\n")

    # vite.config.ts
    with open(run_dir / "vite.config.ts", "w", encoding="utf-8") as f:
        if is_omniview:
            f.write('import { defineConfig } from "vite";\n')
            f.write("\n")
            f.write("export default defineConfig({});\n")
        else:
            f.write('import { defineConfig } from "vite";\n')
            f.write('import react from "@vitejs/plugin-react";\n')
            f.write("\n")
            f.write("export default defineConfig({\n")
            f.write("  plugins: [react()],\n")
            f.write("});\n")

    # index.html
    ext = "js" if is_omniview else "tsx"
    with open(run_dir / "index.html", "w", encoding="utf-8") as f:
        f.write("<!DOCTYPE html>\n")
        f.write('<html lang="en">\n')
        f.write("<head>\n")
        f.write('  <meta charset="UTF-8" />\n')
        f.write('  <meta name="viewport" content="width=device-width, initial-scale=1.0" />\n')
        f.write(f"  <title>{project_name}</title>\n")
        f.write("</head>\n")
        f.write("<body>\n")
        f.write('  <div id="root"></div>\n')
        f.write(f'  <script type="module" src="/src/main.{ext}"></script>\n')
        f.write("</body>\n")
        f.write("</html>\n")

    # src/vite-env.d.ts
    with open(src_dir / "vite-env.d.ts", "w", encoding="utf-8") as f:
        f.write('/// <reference types="vite/client" />\n')

    # src/main.{ext} (imports the primary compiled component)
    main_file = src_dir / f"main.{ext}"
    with open(main_file, "w", encoding="utf-8") as f:
        if debug_mode:
            f.write(f'(window as any).__OMNI_DEBUG_WS_URL__ = "ws://localhost:{debug_ws_port}/api/debug/ws";\n')
        if is_omniview:
            f.write(f'import {{ {component_name} }} from "./{component_name}.js";\n')
            f.write("\n")
            f.write(f'{component_name}(document.getElementById("root")!);\n')
        else:
            f.write('import React from "react";\n')
            f.write('import ReactDOM from "react-dom/client";\n')
            f.write(f'import {{ {component_name} }} from "./{component_name}";\n')
            f.write("\n")
            f.write('ReactDOM.createRoot(document.getElementById("root")!).render(\n')
            f.write("  <React.StrictMode>\n")
            f.write(f"    <{component_name} />\n")
            f.write("  </React.StrictMode>\n")
            f.write(");\n")

    # .gitignore
    with open(run_dir / ".gitignore", "w", encoding="utf-8") as f:
        f.write("node_modules\n")
        f.write("dist\n")
        f.write(".DS_Store\n")


def _spawn_vite(run_dir: Path, port: int) -> subprocess.Popen | None:
    """Spawn `npx vite` as a subprocess."""
    try:
        proc = subprocess.Popen(
            f"npx vite --port {port} --host",
            cwd=str(run_dir),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            shell=True,
        )
        return proc
    except Exception:
        return None


# ── Endpoints ───────────────────────────────────────────

@router.post("/start")
async def start_run(req: StartRunRequest, request: Request):
    """Create Vite project, install deps, start dev server.

    Receives compiled React files, scaffolds a Vite project in
    <project_dir>/.omni-run/, installs npm deps, and starts Vite.
    """
    project_dir_str = getattr(request.app.state, "project_dir", None)
    if not project_dir_str:
        return {"success": False, "error": "No project directory configured"}

    project_dir = Path(project_dir_str)

    # Kill any existing run server
    existing_proc = getattr(request.app.state, "vite_run_process", None)
    if existing_proc and existing_proc.poll() is None:
        existing_proc.terminate()
        try:
            existing_proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            existing_proc.kill()

    # Setup .omni-run directory
    run_dir = _ensure_run_dir(project_dir)
    project_name = project_dir.name

    # Detect component name and target from compiled files
    component_name = _detect_component_name(req.files)
    target = _detect_target(req.files)
    debug_mode = getattr(req, "debug", False)

    # Copy runtime file when debug mode is enabled
    if debug_mode:
        _copy_runtime_file(run_dir, target)

    # Scaffold Vite project (always re-create to match current project)
    _scaffold_vite_project(run_dir, project_name, component_name,
                          debug_mode=debug_mode, target=target)

    # Write compiled files to src/
    src_dir = run_dir / "src"
    for filename, content in req.files.items():
        file_path = src_dir / filename
        file_path.parent.mkdir(parents=True, exist_ok=True)
        file_path.write_text(content, encoding="utf-8")

    # Install dependencies (skip if node_modules exists)
    node_modules = run_dir / "node_modules"
    if not node_modules.exists():
        try:
            subprocess.run(
                "npm install",
                cwd=str(run_dir),
                shell=True,
                capture_output=True,
                timeout=120,  # 2 minute timeout for first install
            )
        except subprocess.TimeoutExpired:
            return {"success": False, "error": "npm install timed out after 2 minutes"}
        except Exception as e:
            return {"success": False, "error": f"npm install failed: {e}"}

    # Find available port and start Vite
    port = _find_available_port()
    proc = _spawn_vite(run_dir, port)

    if proc is None:
        return {"success": False, "error": "Failed to start Vite dev server"}

    # Wait briefly for Vite to start
    import time
    time.sleep(1.5)
    if proc.poll() is not None:
        return {"success": False, "error": f"Vite exited immediately (code {proc.returncode}). Check console for errors."}

    request.app.state.vite_run_process = proc
    request.app.state.vite_run_port = port
    request.app.state.vite_run_dir = str(run_dir)

    url = f"http://localhost:{port}"
    return {"success": True, "url": url, "port": port}


@router.post("/stop")
async def stop_run(request: Request):
    """Stop the Vite dev server."""
    proc = getattr(request.app.state, "vite_run_process", None)
    if proc and proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()

    request.app.state.vite_run_process = None
    request.app.state.vite_run_port = None
    return {"success": True}


@router.get("/status")
async def run_status(request: Request):
    """Return current Vite dev server status."""
    proc = getattr(request.app.state, "vite_run_process", None)
    port = getattr(request.app.state, "vite_run_port", None)
    running = proc is not None and proc.poll() is None

    if running and port:
        return {"running": True, "url": f"http://localhost:{port}"}
    return {"running": False, "url": None}


@router.post("/write-files")
async def write_files(req: WriteFilesRequest, request: Request):
    """Write compiled React files to .omni-run/src/.

    Vite HMR picks up changes automatically.
    """
    proc = getattr(request.app.state, "vite_run_process", None)
    if not proc or proc.poll() is not None:
        return {"success": False, "error": "Vite dev server is not running. Start a run first."}

    run_dir_str = getattr(request.app.state, "vite_run_dir", None)
    if not run_dir_str:
        return {"success": False, "error": "Run directory not found"}

    src_dir = Path(run_dir_str) / "src"
    count = 0
    for filename, content in req.files.items():
        file_path = src_dir / filename
        file_path.parent.mkdir(parents=True, exist_ok=True)
        file_path.write_text(content, encoding="utf-8")
        count += 1

    return {"success": True, "count": count}
