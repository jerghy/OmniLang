"""WebSocket debug bridge — relays runtime state from running app to Editor.

Phase 2 Runtime Bridge: accepts WebSocket connections from compiled React apps
running in OmniRun mode. Relays state-change messages and execution traces to
the Editor frontend, and forwards Editor commands (breakpoints, step, resume)
to the running app.

Protocol (JSON over WebSocket):
    App → Server → Editor Frontend:
        { type: "state-change", signalId, name, value, timestamp }
        { type: "execution-trace", nodeId, edgeId?, timestamp }
        { type: "runtime-ready" }
        { type: "runtime-error", message }

    Editor Frontend → Server → App:
        { type: "set-breakpoint", signalId }
        { type: "remove-breakpoint", signalId }
        { type: "step" }
        { type: "resume" }
        { type: "get-state" }
"""

from __future__ import annotations

import json
from typing import Any

from fastapi import APIRouter, WebSocket, WebSocketDisconnect

router = APIRouter()

# Track connected app WebSocket (one per run session)
_app_connection: WebSocket | None = None
# Track connected Editor frontend WebSocket clients (multiple browser tabs possible)
_editor_connections: set[WebSocket] = set()


async def _broadcast_to_editors(message: dict[str, Any]) -> None:
    """Send a message to all connected Editor frontend WebSockets."""
    dead: set[WebSocket] = set()
    msg_str = json.dumps(message, default=str)
    for ws in _editor_connections:
        try:
            await ws.send_text(msg_str)
        except Exception:
            dead.add(ws)
    _editor_connections.difference_update(dead)


@router.websocket("/ws")
async def debug_websocket(ws: WebSocket):
    """WebSocket endpoint for debug bridge.

    Both the running app and the Editor frontend connect to this endpoint.
    The first connection is treated as the "app" side; subsequent connections
    are "editor" (frontend) clients that receive state updates.

    Messages from the app are broadcast to all editor clients.
    Messages from editor clients are forwarded to the app.
    """
    await ws.accept()

    # Determine role: first connection = app, rest = editor frontends
    global _app_connection
    if _app_connection is None:
        _app_connection = ws
        role = "app"
    else:
        _editor_connections.add(ws)
        role = "editor"

    try:
        while True:
            data = await ws.receive_text()
            try:
                message: dict[str, Any] = json.loads(data)
            except json.JSONDecodeError:
                continue

            if role == "app":
                # App → broadcast to all editor frontends
                await _broadcast_to_editors(message)

                # Handle runtime-ready: clear stale state
                if message.get("type") == "runtime-ready":
                    # Notify editors that runtime has (re)connected
                    await _broadcast_to_editors({
                        "type": "runtime-ready",
                        "timestamp": message.get("timestamp", 0),
                    })
            else:
                # Editor → forward to app (commands like set-breakpoint, step, etc.)
                if _app_connection is not None:
                    try:
                        await _app_connection.send_text(data)
                    except Exception:
                        _app_connection = None

    except WebSocketDisconnect:
        pass
    finally:
        if role == "app":
            _app_connection = None
            await _broadcast_to_editors({"type": "runtime-disconnected"})
        else:
            _editor_connections.discard(ws)


@router.get("/status")
async def debug_status():
    """Return debug connection status."""
    return {
        "appConnected": _app_connection is not None,
        "editorClients": len(_editor_connections),
    }
