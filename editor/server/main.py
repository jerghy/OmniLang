"""OmniLang Visual Editor — HTTP + WebSocket service.

Usage:
    omnic serve --dir ./my-project
    # -> http://localhost:5173

Architecture:
    GET  /              -> Editor SPA (production: client/dist, dev: minimal HTML)
    POST /api/compile   -> Compile .omni -> { files, diff, errors }
    WS   /api/watch     -> File polling + live compile push
    GET  /api/project   -> { dir, target }
"""

import sys
from pathlib import Path

from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse

# ── App ────────────────────────────────────────────────────

app = FastAPI(title="OmniLang Editor")
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

# State
app.state.project_dir: str | None = None
app.state.previous_compile: dict[str, str] | None = None

# Static mounts
_REACT_DIST = Path(__file__).parent.parent / "client" / "dist"
if (_REACT_DIST / "assets").exists():
    app.mount("/assets", StaticFiles(directory=str(_REACT_DIST / "assets")), name="assets")

_WASM_DIR = Path(__file__).parent.parent / "public" / "wasm"
if _WASM_DIR.exists():
    app.mount("/wasm", StaticFiles(directory=str(_WASM_DIR)), name="wasm")

# ── Static — Editor minimal HTML ───────────────────────────

_EDITOR_HTML = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OmniLang Editor (M1a)</title>
<style>
  *{box-sizing:border-box;margin:0;padding:0}
  body{font-family:system-ui,sans-serif;display:flex;height:100vh}
  .editor{flex:1;display:flex;flex-direction:column;border-right:1px solid #e5e7eb}
  .editor textarea{flex:1;padding:12px;font-family:monospace;font-size:13px;border:none;resize:none;outline:none;tab-size:4}
  .preview{flex:1;display:flex;flex-direction:column}
  .preview-header{padding:8px 12px;font-size:12px;color:#6b7280;border-bottom:1px solid #e5e7eb;display:flex;gap:16px}
  .preview-content{flex:1;padding:12px;overflow:auto;font-family:monospace;font-size:12px;white-space:pre-wrap}
  .diff-add{background:#dcfce7;color:#166534}
  .diff-rem{background:#fee2e2;color:#991b1b}
  .error{background:#fef2f2;color:#991b1b;padding:8px;margin:4px 0;border-radius:4px;font-size:12px}
  .status{padding:6px 12px;font-size:11px;color:#6b7280;border-top:1px solid #e5e7eb}
  .toolbar{padding:6px 12px;border-bottom:1px solid #e5e7eb;display:flex;gap:8px;align-items:center}
  .toolbar select,.toolbar button{padding:4px 8px;font-size:12px;border:1px solid #d1d5db;border-radius:4px;background:#fff}
</style>
</head>
<body>
<div class="editor">
  <div class="toolbar">
    <span style="font-weight:600;font-size:13px">.omni Source</span>
    <select id="target" style="margin-left:auto">
      <option value="react" selected>React</option>
      <option value="vue">Vue</option>
      <option value="dioxus">Dioxus</option>
      <option value="flutter">Flutter</option>
      <option value="python">Python</option>
      <option value="javascript">JavaScript</option>
      <option value="rust">Rust</option>
      <option value="dart">Dart</option>
    </select>
    <button onclick="compile()">Compile (Ctrl+Enter)</button>
  </div>
  <textarea id="source" placeholder="// Write .omni source here...
// Or: edit the .omni file on disk, and the watcher will auto-compile.
// Ctrl+Enter to compile manually." spellcheck="false"></textarea>
  <div class="status" id="status">Ready</div>
</div>
<div class="preview">
  <div class="preview-header">
    <span>Generated Output</span>
    <span id="diffInfo" style="margin-left:auto"></span>
  </div>
  <div class="preview-content" id="output"></div>
</div>

<script>
const wsProtocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
let ws = null;
let previousFiles = null;

function connectWS() {
  ws = new WebSocket(`${wsProtocol}//${location.host}/api/watch`);
  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.type === 'compiled') {
      previousFiles = data.files;
      renderOutput(data.files, data.diff, data.errors);
      setStatus('Live reload — ' + new Date().toLocaleTimeString());
    } else if (data.type === 'error') {
      setStatus('Error: ' + data.message, true);
    }
  };
  ws.onclose = () => { setStatus('Watch disconnected. Reconnecting...', true); setTimeout(connectWS, 3000); };
  ws.onerror = () => { /* will close */ };
}
connectWS();

async function compile() {
  const source = document.getElementById('source').value;
  const target = document.getElementById('target').value;
  setStatus('Compiling...');
  try {
    const res = await fetch('/api/compile', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({ source, target, previousFiles })
    });
    const data = await res.json();
    if (data.success) {
      previousFiles = data.files;
      renderOutput(data.files, data.diff, data.errors);
      setStatus('Compiled — ' + new Date().toLocaleTimeString());
    } else {
      renderOutput(null, null, data.errors);
      setStatus('Compile failed', true);
    }
  } catch(e) {
    setStatus('Network error: ' + e.message, true);
  }
}

function renderOutput(files, diff, errors) {
  const output = document.getElementById('output');
  const diffInfo = document.getElementById('diffInfo');

  let html = '';
  if (errors && errors.length > 0) {
    for (const e of errors) {
      html += `<div class="error">[${e.code}] ${e.file}:${e.line}: ${e.message}</div>`;
    }
  }
  if (files) {
    for (const [name, content] of Object.entries(files)) {
      html += `<div style="margin:8px 0"><strong>${escapeHTML(name)}</strong> (${content.length} chars)</div>`;
      if (diff && diff[name]) {
        html += `<pre style="margin:4px 0 12px 12px;font-size:11px">${renderDiff(diff[name])}</pre>`;
      } else {
        html += `<pre style="margin:4px 0 12px 12px;font-size:11px;background:#f9fafb;padding:8px;border-radius:4px;max-height:300px;overflow:auto">${escapeHTML(content.slice(0,2000))}</pre>`;
      }
    }
  }
  if (!files && (!errors || errors.length === 0)) {
    html = '<div style="color:#9ca3af;padding:24px">Compile to see output</div>';
  }
  output.innerHTML = html;
  diffInfo.textContent = diff && Object.keys(diff).length > 0 ? `${Object.keys(diff).length} file(s) changed` : '';
}

function renderDiff(text) {
  return text.split('\\n').map(line => {
    if (line.startsWith('+')) return `<span class="diff-add">${escapeHTML(line)}</span>`;
    if (line.startsWith('-')) return `<span class="diff-rem">${escapeHTML(line)}</span>`;
    return escapeHTML(line);
  }).join('\\n');
}

function escapeHTML(s) { return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
function setStatus(msg, isError) {
  const el = document.getElementById('status');
  el.textContent = msg;
  el.style.color = isError ? '#dc2626' : '#6b7280';
}

// keyboard shortcut
document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); compile(); }
});

// load initial source from server
(async () => {
  try {
    const res = await fetch('/api/project');
    const proj = await res.json();
    if (proj.mainFile) {
      const srcRes = await fetch('/api/fs/read', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({ filePath: proj.mainFile })
      });
      const src = await srcRes.json();
      if (src.content) document.getElementById('source').value = src.content;
    }
  } catch(e) { /* no project loaded, start with empty editor */ }
})();
</script>
</body>
</html>"""


@app.get("/", response_class=HTMLResponse)
async def index():
    REACT_DIST = Path(__file__).parent.parent / "client" / "dist"
    if (REACT_DIST / "index.html").exists():
        return HTMLResponse((REACT_DIST / "index.html").read_text(encoding="utf-8"))
    return _EDITOR_HTML


# ── Register routers ───────────────────────────────────────

from editor.server.routers.compile import router as compile_router
from editor.server.routers.files import router as files_router
from editor.server.routers.project import router as project_router
from editor.server.routers.annotations import router as annotations_router
from editor.server.routers.components import router as components_router
from editor.server.routers.connectors import router as connectors_router
from editor.server.routers.deploy import router as deploy_router
from editor.server.routers.watch import router as watch_router
from editor.server.routers.run import router as run_router
from editor.server.routers.debug import router as debug_router

app.include_router(compile_router)
app.include_router(files_router)
app.include_router(project_router)
app.include_router(annotations_router)
app.include_router(components_router)
app.include_router(connectors_router)
app.include_router(deploy_router, prefix="/api")
app.include_router(watch_router, prefix="/api")
app.include_router(run_router, prefix="/api/run")
app.include_router(debug_router, prefix="/api/debug")


# ── CLI — omnic serve command ──────────────────────────────

def cmd_serve(args):
    """omnic serve command entry."""
    import subprocess

    from editor.server.services.project_manager import normalize_path

    port = getattr(args, 'port', None) or 5173
    project_dir = getattr(args, 'dir', None)
    no_frontend = getattr(args, 'no_frontend', False)
    dev_mode = getattr(args, 'dev', False)

    if not project_dir:
        print("Error: --dir is required. Usage: omnic serve --dir ./my-project", file=sys.stderr)
        sys.exit(1)

    project_dir = normalize_path(project_dir)
    app.state.project_dir = project_dir

    REACT_DIST = Path(__file__).parent.parent / "client" / "dist"
    vite_port = getattr(args, 'vite_port', None) or 5174

    print(f"  OmniLang Editor")
    print(f"  API:     http://localhost:{port}")
    print(f"  Project: {project_dir}")

    has_prod_build = (REACT_DIST / "index.html").exists()
    frontend_proc = None

    if has_prod_build and not dev_mode:
        print(f"  Editor:  http://localhost:{port} (production build)")
    elif not no_frontend:
        import subprocess as _sp
        client_dir = Path(__file__).parent.parent / "client"
        tag = " — forced" if dev_mode and has_prod_build else ""
        print(f"  Frontend: http://localhost:{vite_port} (Vite dev{tag})")
        print(f"            (proxies /api -> localhost:{port})")
        try:
            frontend_proc = _sp.Popen(
                f'npx vite --port {vite_port} --host',
                cwd=str(client_dir),
                stdout=_sp.DEVNULL,
                stderr=_sp.DEVNULL,
                shell=True,
            )
        except Exception as e:
            print(f"  [WARN] Vite startup failed: {e}")
            print(f"  Manual: cd editor/client && npm run dev")
    else:
        print(f"  Editor:  cd editor/client && npm run dev  ->  http://localhost:{vite_port}")
        print(f"           (Vite proxies /api -> localhost:{port})")

    print(f"  Press Ctrl+C to stop.")
    print()

    import uvicorn
    try:
        uvicorn.run(app, host="127.0.0.1", port=port, log_level="info")
    finally:
        if frontend_proc:
            frontend_proc.terminate()
            try:
                frontend_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                frontend_proc.kill()


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser()
    p.add_argument("--dir", required=True, help="Project directory")
    p.add_argument("--port", type=int, default=5173, help="Port")
    a = p.parse_args()
    cmd_serve(a)
