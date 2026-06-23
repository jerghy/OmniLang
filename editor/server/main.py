"""OmniLang Visual Editor — HTTP + WebSocket 服务。（M1a-2）

用法:
    omnic serve --dir ./my-project
    # → http://localhost:5173

架构:
    GET  /              → 编辑器 SPA (生产: client/dist, 开发: 最小 HTML)
    POST /api/compile   → 编译 .omni → { files, diff, errors }
    WS   /api/watch     → 文件轮询 + 实时编译推送
    GET  /api/project   → { dir, target }
"""

import sys
import os
import threading
import time
from pathlib import Path

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, Request
from fastapi.staticfiles import StaticFiles
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse, JSONResponse
from pydantic import BaseModel
import uvicorn

from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.generator import generate_annotations
from editor.server.annotation.validator import validate_annotations
from editor.server.annotation.editor_toml import read_editor_toml, write_editor_toml, EditorTomlState
from editor.server.annotation.types import AnnotationError, AnnotationFile, CodeTabType, ViewRenderMode, Chain, NodeEntry, WireEntry, ViewMarker

# ═══════════════════════════════════════════════════════
# App
# ═══════════════════════════════════════════════════════

app = FastAPI(title="OmniLang Editor")
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

# ═══════════════════════════════════════════════════════
# State
# ═══════════════════════════════════════════════════════

app.state.project_dir: str | None = None
app.state.previous_compile: dict[str, str] | None = None

# Mount React build assets if available
_REACT_DIST = Path(__file__).parent.parent / "client" / "dist"
if (_REACT_DIST / "assets").exists():
    app.mount("/assets", StaticFiles(directory=str(_REACT_DIST / "assets")), name="assets")

# Mount WASM compiler files
_WASM_DIR = Path(__file__).parent.parent / "public" / "wasm"
if _WASM_DIR.exists():
    app.mount("/wasm", StaticFiles(directory=str(_WASM_DIR)), name="wasm")


# ═══════════════════════════════════════════════════════
# Static — 编辑器最小 HTML (M1a)
# ═══════════════════════════════════════════════════════

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
    # 生产模式：serve React build
    REACT_DIST = Path(__file__).parent.parent / "client" / "dist"
    if (REACT_DIST / "index.html").exists():
        return HTMLResponse((REACT_DIST / "index.html").read_text(encoding="utf-8"))
    # 开发模式：serve 最小 HTML（用户通过 Vite dev server 访问）
    return _EDITOR_HTML


# ═══════════════════════════════════════════════════════
# API
# ═══════════════════════════════════════════════════════

class CompileRequest(BaseModel):
    source: str
    target: str = "react"
    previousFiles: dict[str, str] | None = None
    debug: bool = False  # Enable runtime debug injection (useTrackedSignal etc.)


@app.post("/api/compile")
async def api_compile(req: CompileRequest, request: Request):
    """编译 .omni 源码 → { files, diff, errors }。（G2 错误格式）"""
    from omnilang.api import compile_omni

    result = compile_omni(req.source, target=req.target,
                          previous_files=req.previousFiles,
                          debug_mode=req.debug)

    # 保存本次结果用于下次 diff
    request.app.state.previous_compile = result.files

    return {
        "success": result.success,
        "files": result.files,
        "diff": {k: v for k, v in (result.diff or {}).items()},
        "errors": [e.to_dict() for e in result.errors],
    }


class CompileFromIRRequest(BaseModel):
    """POST /api/compile-from-ir 请求体 — 从 IR JSON 编译到目标框架"""
    ir: dict  # IRProgram dict (Rust WASM output)
    target: str = "react"


@app.post("/api/compile-from-ir")
async def api_compile_from_ir(req: CompileFromIRRequest):
    """从 IR JSON 编译到目标框架代码（Deprecated — Rust handles all codegen now）。

    该端点已弃用，因为 Python codegen 已被移除。保留以兼容旧客户端。
    """
    return {"success": False, "files": {}, "errors": ["compile-from-ir is deprecated: Rust handles all codegen"]}


@app.get("/api/wasm/version")
async def api_wasm_version():
    """返回 Rust 编译器版本信息（用于检查 WASM 是否可用）。"""
    from omnilang.rust_bridge import RustBridge
    return {
        "rust_available": RustBridge.is_available(),
        "wasm_supported": True,  # Browser feature-check at runtime
    }


class GenerateSourceRequest(BaseModel):
    """POST /api/generate-source 请求体 — 从项目状态生成 .omni 源码"""
    document: dict  # 完整 .omni.json document
    componentName: str = "App"


@app.post("/api/generate-source")
async def api_generate_source(req: GenerateSourceRequest):
    """从编辑器画布状态反向生成 .omni 源码。

    注意：Python 反向 codegen 已移除。Rust 编译器 handle 所有代码生成。
    """
    return {"success": False, "source": "", "errors": ["generate-source is deprecated: Rust handles all codegen"]}


class FsReadRequest(BaseModel):
    filePath: str


class FsListRequest(BaseModel):
    dirPath: str = "."


@app.post("/api/fs/list")
async def api_fs_list(req: FsListRequest, request: Request):
    """列出目录内容。"""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"entries": [], "error": "No project directory"}

    target_dir = (Path(project_dir) / req.dirPath).resolve()
    if not str(target_dir).startswith(str(Path(project_dir).resolve())):
        return {"entries": [], "error": "Access denied"}

    SOURCE_EXTS = {".omni", ".tsx", ".ts", ".jsx", ".js", ".json", ".css", ".vue", ".py", ".rs", ".dart"}
    entries = []
    try:
        for entry in sorted(target_dir.iterdir()):
            if entry.name.startswith(".") or entry.name == "node_modules":
                continue
            entries.append({
                "name": entry.name,
                "path": str(entry.relative_to(project_dir)).replace("\\", "/"),
                "type": "directory" if entry.is_dir() else "file",
                "ext": entry.suffix if entry.is_file() else "",
            })
    except PermissionError:
        return {"entries": [], "error": "Permission denied"}
    return {"entries": entries}


class FsWriteRequest(BaseModel):
    filePath: str
    content: str


@app.post("/api/fs/write")
async def api_fs_write(req: FsWriteRequest, request: Request):
    """原子写入文件。"""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"ok": False, "error": "Access denied"}

    file_path.parent.mkdir(parents=True, exist_ok=True)
    import uuid
    tmp_path = file_path.parent / f"{file_path.name}.tmp.{uuid.uuid4().hex[:8]}"
    try:
        tmp_path.write_text(req.content, encoding="utf-8")
        # fsync on Windows requires a writable handle
        with open(tmp_path, "r+b") as f:
            os.fsync(f.fileno())
        os.replace(tmp_path, file_path)
        return {"ok": True}
    except Exception as e:
        if tmp_path.exists():
            tmp_path.unlink()
        return {"ok": False, "error": str(e)}


@app.post("/api/parse-view")
async def api_parse_view(req: CompileRequest, request: Request):
    """解析 .omni 源码 → 返回视图元素树（用于画布渲染）。"""
    from omnilang.api import parse_view_tree
    try:
        nodes = parse_view_tree(req.source)
        return {
            "success": True,
            "nodes": [
                {
                    "id": n.id, "tag": n.tag, "props": n.props,
                    "testId": n.test_id, "sourceLine": n.source_line,
                    "children": _serialize_tree(n.children),
                }
                for n in nodes
            ],
        }
    except Exception as e:
        return {"success": False, "nodes": [], "error": str(e)}


@app.post("/api/parse-codeblocks")
async def api_parse_codeblocks(req: CompileRequest, request: Request):
    """解析 .omni 源码 → 提取代码块（store/signal/computed/action）用于画布。"""
    from omnilang.api import parse_code_blocks
    try:
        blocks = parse_code_blocks(req.source)
        return {
            "success": True,
            "blocks": [
                {
                    "id": b.id, "blockType": b.blockType,
                    "name": b.name, "parentStore": b.parentStore,
                    "typeAnnotation": b.typeAnnotation,
                    "sourceLine": b.sourceLine,
                }
                for b in blocks
            ],
        }
    except Exception as e:
        return {"success": False, "blocks": [], "error": str(e)}


class ConnectorRequest(BaseModel):
    """POST /api/connector/generate 请求体"""
    type: str  # "rest" | "graphql" | "firebase"
    name: str
    baseUrl: str = ""
    endpoints: list[dict] = []
    query: str = ""
    collection: str = ""


@app.post("/api/connector/generate")
async def api_connector_generate(req: ConnectorRequest, request: Request):
    """生成数据连接器脚手架代码。支持 REST / GraphQL / Firebase。"""
    from editor.server.connectors import (
        ConnectorConfig, EndpointDef, FieldDef,
        generate_rest_store, generate_graphql_store, generate_firebase_store,
        infer_type_from_json,
    )

    try:
        # Parse endpoints
        endpoints: list[EndpointDef] = []
        for ep in req.endpoints:
            response_fields: list[FieldDef] = []
            if ep.get("responseJson"):
                response_fields = infer_type_from_json(ep["responseJson"])
            endpoints.append(EndpointDef(
                method=ep.get("method", "GET"),
                path=ep.get("path", ""),
                action=ep.get("action", "fetch"),
                requestType=ep.get("requestType", ""),
                responseFields=response_fields,
            ))

        config = ConnectorConfig(
            type=req.type,
            name=req.name,
            baseUrl=req.baseUrl,
            endpoints=endpoints,
            query=req.query,
            collection=req.collection,
        )

        if req.type == "rest":
            code = generate_rest_store(config)
        elif req.type == "graphql":
            code = generate_graphql_store(config)
        elif req.type == "firebase":
            code = generate_firebase_store(config)
        else:
            return {"success": False, "code": "", "error": f"Unsupported connector type: {req.type}"}

        return {"success": True, "code": code}
    except Exception as e:
        return {"success": False, "code": "", "error": str(e)}


def _serialize_tree(nodes: list) -> list:
    """递归序列化 ViewTreeNode 为 JSON。"""
    result = []
    for n in nodes:
        result.append({
            "id": n.id, "tag": n.tag, "props": n.props,
            "testId": n.test_id, "sourceLine": n.source_line,
            "children": _serialize_tree(n.children) if n.children else [],
        })
    return result


@app.post("/api/fs/read")
async def api_fs_read(req: FsReadRequest, request: Request):
    """读取项目文件内容。"""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"content": "", "error": "No project directory set"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"content": "", "error": "Access denied"}

    try:
        return {"content": file_path.read_text(encoding="utf-8")}
    except Exception as e:
        return {"content": "", "error": str(e)}


@app.get("/api/project")
async def api_project(request: Request):
    """获取项目信息（G5）。"""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"dir": None, "target": "react", "mainFile": None}

    # 查找 main.omni 或第一个有效 .omni 文件（按字母排序，优先 main.omni）
    main_file = None
    omni_files = sorted(
        [f for f in Path(project_dir).rglob("*.omni")],
        key=lambda f: (f.name != "main.omni", str(f))  # main.omni first, then alphabetical
    )
    for f in omni_files:
        try:
            content = f.read_text(encoding="utf-8")
            # Prefer files that start with "component" (valid .omni source)
            if content.strip().startswith("component"):
                main_file = str(f.relative_to(project_dir)).replace("\\", "/")
                break
        except Exception:
            continue
    # Fallback: use first available .omni file
    if not main_file and omni_files:
        main_file = str(omni_files[0].relative_to(project_dir)).replace("\\", "/")

    return {"dir": str(project_dir), "target": "react", "mainFile": main_file}


# ═══════════════════════════════════════════════════════
# Welcome screen API — directory browser + project management
# ═══════════════════════════════════════════════════════

from pydantic import BaseModel

class ListDirectoryRequest(BaseModel):
    path: str = "."

@app.post("/api/fs/list-directory")
async def api_list_directory(req: ListDirectoryRequest):
    """列出指定目录内容（供欢迎页目录浏览器使用）。"""
    try:
        target = Path(req.path).expanduser().resolve()
        if not target.is_dir():
            return {"ok": False, "error": "目录不存在"}
        entries = []
        for entry in sorted(target.iterdir()):
            if entry.name.startswith("."):
                continue
            entries.append({
                "name": entry.name,
                "isDir": entry.is_dir(),
            })
        return {"ok": True, "current": str(target), "entries": entries}
    except PermissionError:
        return {"ok": False, "error": "权限不足"}
    except Exception as e:
        return {"ok": False, "error": str(e)}

class SetProjectDirRequest(BaseModel):
    dir: str

@app.post("/api/project/set-dir")
async def api_project_set_dir(req: SetProjectDirRequest, request: Request):
    """设置项目目录。"""
    try:
        target = Path(req.dir).expanduser().resolve()
        if not target.is_dir():
            return {"ok": False, "error": "目录不存在"}
        request.app.state.project_dir = str(target)
        return {"ok": True, "dir": str(target)}
    except Exception as e:
        return {"ok": False, "error": str(e)}

class CreateProjectRequest(BaseModel):
    dir: str
    name: str

@app.post("/api/project/create")
async def api_project_create(req: CreateProjectRequest, request: Request):
    """创建新项目（创建目录并初始化配置）。"""
    try:
        target = Path(req.dir).expanduser().resolve() / req.name
        target.mkdir(parents=True, exist_ok=True)
        src_dir = target / "src"
        src_dir.mkdir(exist_ok=True)
        # Create main.omni with empty component
        main_omni = src_dir / "main.omni"
        main_omni.write_text(f"import omniview;\n\ncomponent {req.name} {{\n    view {{\n        <Container id=\"{req.name}.root.0\">\n            <Text id=\"{req.name}.root.0.text.0\">Hello</Text>\n        </Container>\n    }}\n}}\n", encoding="utf-8")
        request.app.state.project_dir = str(target)
        return {"ok": True, "dir": str(target)}
    except Exception as e:
        return {"ok": False, "error": str(e)}

# ═══════════════════════════════════════════════════════
# M5 API — 部署 + 代码所有权
# ═══════════════════════════════════════════════════════

from editor.server.api.deploy import router as deploy_router
app.include_router(deploy_router, prefix="/api")

from editor.server.api.watch import router as watch_router
app.include_router(watch_router, prefix="/api")

from editor.server.api.run import router as run_router
app.include_router(run_router, prefix="/api/run")

# Phase 2: Debug WebSocket bridge
from .api.debug import router as debug_router
app.include_router(debug_router, prefix="/api/debug")


class CodeOwnershipRequest(BaseModel):
    source: str
    action: str = "parse"  # "parse" | "apply-edit"
    editStartLine: int = 0
    editEndLine: int = 0
    newContent: str = ""


@app.post("/api/code-ownership")
async def api_code_ownership(req: CodeOwnershipRequest):
    """解析或应用代码所有权区域 (M5.3)。"""
    from editor.server.api.code_ownership import (
        parse_ownership_regions,
        apply_canvas_edit,
        OwnershipConflictError,
        get_managed_regions,
        get_handwritten_regions,
    )

    try:
        if req.action == "parse":
            regions = parse_ownership_regions(req.source)
            return {
                "success": True,
                "regions": [
                    {
                        "type": r.type,
                        "startLine": r.start_line,
                        "endLine": r.end_line,
                        "content": r.content[:200] if len(r.content) > 200 else r.content,
                    }
                    for r in regions
                ],
                "managedCount": len(get_managed_regions(req.source)),
                "handwrittenCount": len(get_handwritten_regions(req.source)),
            }
        elif req.action == "parse-ast":
            # M6: AST-anchored ownership
            from editor.server.api.code_ownership import parse_ownership_ast_nodes
            ast_regions = parse_ownership_ast_nodes(req.source)
            return {"success": True, "regions": ast_regions}
        elif req.action == "apply-edit":
            new_source = apply_canvas_edit(
                req.source,
                req.editStartLine,
                req.editEndLine,
                req.newContent,
            )
            return {"success": True, "source": new_source}
        else:
            return {"success": False, "error": f"Unknown action: {req.action}"}
    except OwnershipConflictError as e:
        return {"success": False, "error": str(e), "conflict": True}
    except Exception as e:
        return {"success": False, "error": str(e)}


# ═══════════════════════════════════════════════════════
# M7 API — @native ports
# ═══════════════════════════════════════════════════════


@app.post("/api/parse-native-ports")
async def api_parse_native_ports(req: CompileRequest):
    """Parse @native blocks → extract @in/@out port definitions (M7)."""
    from omnilang.api import parse_native_ports
    try:
        ports = parse_native_ports(req.source)
        return {
            "success": True,
            "ports": [
                {
                    "id": p.id, "target": p.target,
                    "inputs": p.inputs, "outputs": p.outputs,
                    "sourceLine": p.source_line,
                }
                for p in ports
            ],
        }
    except Exception as e:
        return {"success": False, "ports": [], "error": str(e)}


# ═══════════════════════════════════════════════════════
# M6 API — .omni.json 原生格式 (导出/导入/检测)
# ═══════════════════════════════════════════════════════


class OmniJsonExportRequest(BaseModel):
    source: str
    filepath: str = "<input>"
    target: str = "react"
    existingCodeElements: list[dict] = []
    existingCodeWires: list[dict] = []


@app.post("/api/omni-json/export")
async def api_omni_json_export(req: OmniJsonExportRequest):
    """Regenerate code canvas from .omni source, returning only code canvas data (v2)."""
    from editor.server.omni_json.exporter import OmniJsonExporter
    from editor.server.omni_json.serializer import OmniJsonSerializer

    try:
        exporter = OmniJsonExporter()
        code_canvas = exporter.regenerate_code_canvas(
            source=req.source,
            filepath=req.filepath,
            target=req.target,
            existing_code_elements=req.existingCodeElements,
            existing_code_wires=req.existingCodeWires,
        )
        serializer = OmniJsonSerializer()
        return {"success": True, "codeCanvas": serializer.to_dict(code_canvas)}
    except Exception as e:
        return {"success": False, "codeCanvas": None, "error": str(e)}


class OmniJsonSaveRequest(BaseModel):
    filePath: str
    document: dict


@app.post("/api/omni-json/save")
async def api_omni_json_save(req: OmniJsonSaveRequest, request: Request):
    """保存 .omni.json 到磁盘（原子写入）(v2)."""
    from editor.server.omni_json.serializer import OmniJsonSerializer

    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"ok": False, "error": "Access denied"}

    try:
        serializer = OmniJsonSerializer()
        doc = serializer.from_dict(req.document)
        file_path.parent.mkdir(parents=True, exist_ok=True)
        serializer.save(doc, file_path)
        return {"ok": True, "path": str(file_path.relative_to(project_dir)).replace("\\", "/")}
    except Exception as e:
        return {"ok": False, "error": str(e)}


class OmniJsonLoadRequest(BaseModel):
    filePath: str


@app.post("/api/omni-json/load")
async def api_omni_json_load(req: OmniJsonLoadRequest, request: Request):
    """加载 .omni.json 从磁盘 (v2)."""
    from editor.server.omni_json.serializer import OmniJsonSerializer

    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"success": False, "document": None, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"success": False, "document": None, "error": "Access denied"}

    if not file_path.exists():
        return {"success": False, "document": None, "error": "File not found"}

    try:
        serializer = OmniJsonSerializer()
        doc = serializer.load(file_path)
        return {"success": True, "document": serializer.to_dict(doc)}
    except Exception as e:
        return {"success": False, "document": None, "error": str(e)}


class OmniJsonDetectRequest(BaseModel):
    source: str


@app.post("/api/omni-json/detect-handwritten")
async def api_detect_handwritten(req: OmniJsonDetectRequest):
    """检测 .omni 源码是否为手写."""
    from editor.server.omni_json.exporter import OmniJsonExporter

    exporter = OmniJsonExporter()
    is_hw = exporter.detect_hand_written(req.source)
    warnings: list[str] = []
    if is_hw:
        warnings.append(
            "This file was hand-written. Comments and custom formatting "
            "will be lost when converting to visual mode."
        )
        warnings.append(
            "Only 'omnic build' can compile this file. "
            "The editor cannot write to it."
        )
    return {"isHandWritten": is_hw, "warnings": warnings}


# ═══════════════════════════════════════════════════════
# M6 API — 组件拆分 / 语义合并 (P1)
# ═══════════════════════════════════════════════════════


class OmniJsonSplitRequest(BaseModel):
    filePath: str = "project.omni.json"


@app.post("/api/omni-json/split")
async def api_omni_json_split(req: OmniJsonSplitRequest, request: Request):
    """将单体 .omni.json 拆分为按组件分文件 (M6.6)."""
    from editor.server.omni_json.serializer import OmniJsonSerializer
    from editor.server.omni_json.project import OmniProjectManager

    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    try:
        serializer = OmniJsonSerializer()
        doc_path = Path(project_dir) / req.filePath
        if not doc_path.exists():
            return {"ok": False, "error": f"File not found: {req.filePath}"}

        doc = serializer.load(doc_path)
        manager = OmniProjectManager(project_dir)
        written = manager.split_document(doc)
        return {
            "ok": True,
            "files": {name: str(p.relative_to(project_dir)).replace("\\", "/")
                       for name, p in written.items()},
        }
    except Exception as e:
        return {"ok": False, "error": str(e)}


class MergeRequest(BaseModel):
    base: dict
    local: dict
    remote: dict


@app.post("/api/merge")
async def api_merge(req: MergeRequest):
    """3-way 语义合并 .omni.json 文档 (M6.7)."""
    from editor.server.omni_json.merger import OmniJsonMerger

    try:
        merger = OmniJsonMerger()
        result = merger.three_way_merge(req.base, req.local, req.remote)
        return {"success": result["success"], "merged": result["merged"], "conflicts": result["conflicts"]}
    except Exception as e:
        return {"success": False, "merged": None, "conflicts": [], "error": str(e)}


@app.post("/api/omni-json/merge")
async def api_omni_json_merge(request: Request):
    """从拆分的组件文件重构单体文档 (M6.6)."""
    from editor.server.omni_json.project import OmniProjectManager

    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"success": False, "document": None, "error": "No project directory"}

    try:
        manager = OmniProjectManager(project_dir)
        doc = manager.merge_documents()
        if doc is None:
            return {"success": False, "document": None, "error": "No manifest found"}
        from editor.server.omni_json.serializer import OmniJsonSerializer
        serializer = OmniJsonSerializer()
        return {"success": True, "document": serializer.to_dict(doc)}
    except Exception as e:
        return {"success": False, "document": None, "error": str(e)}


# ═══════════════════════════════════════════════════════
# M8 API — 自定义组件定义 (.omni-components.json)
# ═══════════════════════════════════════════════════════

import json as _json

_COMPONENTS_FILE = ".omni-components.json"


class ComponentDefinitionsPayload(BaseModel):
    definitions: dict = {}


@app.post("/api/component-definitions/save")
async def api_save_component_definitions(req: ComponentDefinitionsPayload, request: Request):
    """保存自定义组件定义到项目目录下的 .omni-components.json。"""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}
    config_path = Path(project_dir) / _COMPONENTS_FILE
    try:
        config_path.write_text(_json.dumps(req.definitions, indent=2, ensure_ascii=False), encoding="utf-8")
        return {"ok": True}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.get("/api/component-definitions/load")
async def api_load_component_definitions(request: Request):
    """从项目目录加载自定义组件定义。"""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"definitions": {}, "error": "No project directory"}
    config_path = Path(project_dir) / _COMPONENTS_FILE
    if not config_path.exists():
        return {"definitions": {}}
    try:
        data = _json.loads(config_path.read_text(encoding="utf-8"))
        return {"definitions": data}
    except Exception as e:
        return {"definitions": {}, "error": str(e)}


# ═══════════════════════════════════════════════════════
# Annotation API — .omni source with embedded editor annotations
# ═══════════════════════════════════════════════════════


class AnnotatedSaveRequest(BaseModel):
    """POST /api/annotated-save request body"""
    filePath: str
    annotations: dict  # Serialized AnnotationFile
    source: str         # The .omni source content


class AnnotatedLoadRequest(BaseModel):
    """POST /api/annotated-load request body"""
    filePath: str


class EditorTomlSaveRequest(BaseModel):
    """POST /api/editor-toml/save request body"""
    state: dict  # Serialized EditorTomlState


@app.post("/api/annotated-save")
async def api_annotated_save(req: AnnotatedSaveRequest, request: Request):
    """Save .omni source with editor annotations injected.

    Takes source + annotation data -> injects @code-tab, @edit, @chain blocks -> writes .omni file.
    """
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"ok": False, "error": "Access denied"}

    try:
        # Reconstruct AnnotationFile from dict
        ann = _dict_to_annotation_file(req.annotations, req.filePath)
        ann.raw_source = req.source
        # Validate before saving
        errors = validate_annotations(ann)
        if errors:
            return {
                "ok": False,
                "error": "Validation failed",
                "validationErrors": [
                    {"code": e.code.value, "message": e.message, "line": e.line}
                    for e in errors
                ],
            }
        # Generate annotated source
        annotated_source = generate_annotations(ann, req.source)
        # Atomic write
        import uuid
        tmp_path = file_path.parent / f"{file_path.name}.tmp.{uuid.uuid4().hex[:8]}"
        tmp_path.write_text(annotated_source, encoding="utf-8")
        with open(tmp_path, "r+b") as f:
            import os
            os.fsync(f.fileno())
        os.replace(tmp_path, file_path)
        return {"ok": True, "path": str(file_path.relative_to(project_dir)).replace("\\", "/")}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.post("/api/annotated-load")
async def api_annotated_load(req: AnnotatedLoadRequest, request: Request):
    """Load .omni source and extract editor annotations.

    Returns: { success, source, annotations, validationErrors }
    """
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"success": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"success": False, "error": "Access denied"}

    if not file_path.exists():
        return {"success": False, "error": "File not found"}

    try:
        source = file_path.read_text(encoding="utf-8")
        ann = parse_annotations(source, req.filePath)
        errors = validate_annotations(ann)

        return {
            "success": True,
            "source": source,
            "annotations": _annotation_file_to_dict(ann),
            "validationErrors": [
                {"code": e.code.value, "message": e.message, "line": e.line}
                for e in errors
            ],
        }
    except ValueError as e:
        return {"success": False, "error": str(e)}
    except Exception as e:
        return {"success": False, "error": str(e)}


@app.post("/api/editor-toml/save")
async def api_editor_toml_save(req: EditorTomlSaveRequest, request: Request):
    """Save .omni-editor.toml."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    try:
        state = EditorTomlState(
            active_file=req.state.get("active_file", ""),
            active_mode=req.state.get("active_mode", "design"),
            open_files=req.state.get("open_files", []),
            left_open=req.state.get("left_open", True),
            right_width=req.state.get("right_width", 320),
            viewports=req.state.get("viewports", {}),
        )
        toml_path = Path(project_dir) / ".omni-editor.toml"
        write_editor_toml(toml_path, state)
        return {"ok": True}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.get("/api/editor-toml/load")
async def api_editor_toml_load(request: Request):
    """Load .omni-editor.toml."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"state": None, "error": "No project directory"}

    try:
        toml_path = Path(project_dir) / ".omni-editor.toml"
        state = read_editor_toml(toml_path)
        return {
            "state": {
                "active_file": state.active_file,
                "active_mode": state.active_mode,
                "open_files": state.open_files,
                "left_open": state.left_open,
                "right_width": state.right_width,
                "viewports": state.viewports,
            }
        }
    except Exception as e:
        return {"state": None, "error": str(e)}


# -- Helpers --------------------------------------------------


def _annotation_file_to_dict(ann: AnnotationFile) -> dict:
    """Serialize AnnotationFile to JSON-safe dict."""
    return {
        "codeTabType": ann.code_tab_type.value,
        "codeTabLine": ann.code_tab_line,
        "viewMarkers": [
            {
                "elementId": m.element_id,
                "lineNumber": m.line_number,
                "isFolded": m.is_folded,
                "renderMode": m.render_mode.value,
            }
            for m in ann.view_markers
        ],
        "chains": [
            {
                "name": c.name,
                "startLine": c.start_line,
                "endLine": c.end_line,
                "nodes": [
                    {
                        "id": n.id,
                        "tag": n.tag,
                        "attrs": n.attrs,
                        "posX": n.pos_x,
                        "posY": n.pos_y,
                        "lineNumber": n.line_number,
                    }
                    for n in c.nodes
                ],
                "wires": [
                    {
                        "id": w.id,
                        "fromId": w.from_id,
                        "fromPort": w.from_port,
                        "toId": w.to_id,
                        "toPort": w.to_port,
                        "isCollapsed": w.is_collapsed,
                        "collapseGroupId": w.collapse_group_id,
                        "lineNumber": w.line_number,
                    }
                    for w in c.wires
                ],
            }
            for c in ann.chains
        ],
    }


def _dict_to_annotation_file(data: dict, filepath: str) -> AnnotationFile:
    """Deserialize dict back to AnnotationFile."""
    return AnnotationFile(
        filepath=filepath,
        code_tab_type=CodeTabType(data.get("codeTabType", "logic")),
        code_tab_line=data.get("codeTabLine", 0),
        view_markers=[
            ViewMarker(
                element_id=m.get("elementId", ""),
                line_number=m.get("lineNumber", 0),
                is_folded=m.get("isFolded", False),
                render_mode=ViewRenderMode(m.get("renderMode", "dom")),
            )
            for m in data.get("viewMarkers", [])
        ],
        chains=[
            Chain(
                name=c.get("name", ""),
                start_line=c.get("startLine", 0),
                end_line=c.get("endLine", 0),
                nodes=[
                    NodeEntry(
                        id=n.get("id", ""),
                        tag=n.get("tag", ""),
                        attrs=n.get("attrs", {}),
                        pos_x=n.get("posX", 0),
                        pos_y=n.get("posY", 0),
                        line_number=n.get("lineNumber", 0),
                    )
                    for n in c.get("nodes", [])
                ],
                wires=[
                    WireEntry(
                        id=w.get("id", ""),
                        from_id=w.get("fromId", ""),
                        from_port=w.get("fromPort", ""),
                        to_id=w.get("toId", ""),
                        to_port=w.get("toPort", ""),
                        is_collapsed=w.get("isCollapsed", False),
                        collapse_group_id=w.get("collapseGroupId", ""),
                        line_number=w.get("lineNumber", 0),
                    )
                    for w in c.get("wires", [])
                ],
            )
            for c in data.get("chains", [])
        ],
    )


# ═══════════════════════════════════════════════════════
# CLI — omnic serve 命令
# ═══════════════════════════════════════════════════════


def cmd_serve(args):
    """omnic serve command entry."""
    port = getattr(args, 'port', None) or 5173
    project_dir = getattr(args, 'dir', None)
    no_frontend = getattr(args, 'no_frontend', False)
    dev_mode = getattr(args, 'dev', False)

    if not project_dir:
        print("Error: --dir is required. Usage: omnic serve --dir ./my-project", file=sys.stderr)
        sys.exit(1)

    project_dir = str(Path(project_dir).resolve())
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
        # Start Vite dev server (auto when no prod build, or forced via --dev)
        import subprocess
        client_dir = Path(__file__).parent.parent / "client"
        tag = " — forced" if dev_mode and has_prod_build else ""
        print(f"  Frontend: http://localhost:{vite_port} (Vite dev{tag})")
        print(f"            (proxies /api → localhost:{port})")
        try:
            # Use shell=True on Windows for npx resolution
            frontend_proc = subprocess.Popen(
                f'npx vite --port {vite_port} --host',
                cwd=str(client_dir),
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                shell=True,
            )
        except Exception as e:
            print(f"  [WARN] Vite startup failed: {e}")
            print(f"  Manual: cd editor/client && npm run dev")
    else:
        print(f"  Editor:  cd editor/client && npm run dev  →  http://localhost:{vite_port}")
        print(f"           (Vite proxies /api → localhost:{port})")

    print(f"  Press Ctrl+C to stop.")
    print()

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

