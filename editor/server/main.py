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


@app.post("/api/compile")
async def api_compile(req: CompileRequest, request: Request):
    """编译 .omni 源码 → { files, diff, errors }。（G2 错误格式）"""
    from omniview.api import compile_omni

    result = compile_omni(req.source, target=req.target, previous_files=req.previousFiles)

    # 保存本次结果用于下次 diff
    request.app.state.previous_compile = result.files

    return {
        "success": result.success,
        "files": result.files,
        "diff": {k: v for k, v in (result.diff or {}).items()},
        "errors": [e.to_dict() for e in result.errors],
    }


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
    from omniview.api import parse_view_tree
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
    from omniview.api import parse_code_blocks
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

    # 查找 main.omni 或第一个 .omni 文件
    main_file = None
    omni_files = list(Path(project_dir).rglob("*.omni"))
    if omni_files:
        main_file = str(omni_files[0].relative_to(project_dir)).replace("\\", "/")

    return {"dir": str(project_dir), "target": "react", "mainFile": main_file}


# ═══════════════════════════════════════════════════════
# M5 API — 部署 + 代码所有权
# ═══════════════════════════════════════════════════════

from editor.server.api.deploy import router as deploy_router
app.include_router(deploy_router, prefix="/api")

from editor.server.api.watch import router as watch_router
app.include_router(watch_router, prefix="/api")


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
    from omniview.api import parse_native_ports
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
    existingElements: list[dict] = []
    existingWires: list[dict] = []
    existingCanvas: dict = {}


@app.post("/api/omni-json/export")
async def api_omni_json_export(req: OmniJsonExportRequest):
    """导出 .omni 源码 → .omni.json 文档快照 (M6)."""
    from editor.server.omni_json.exporter import OmniJsonExporter
    from editor.server.omni_json.serializer import OmniJsonSerializer

    try:
        exporter = OmniJsonExporter()
        doc = exporter.export(
            source=req.source,
            filepath=req.filepath,
            target=req.target,
            existing_elements=req.existingElements,
            existing_wires=req.existingWires,
            existing_canvas=req.existingCanvas,
        )
        serializer = OmniJsonSerializer()
        return {"success": True, "document": serializer.to_dict(doc)}
    except Exception as e:
        return {"success": False, "document": None, "error": str(e)}


class OmniJsonImportRequest(BaseModel):
    document: dict


@app.post("/api/omni-json/import")
async def api_omni_json_import(req: OmniJsonImportRequest):
    """从 .omni.json 文档重构 .omni 源码 (M6)."""
    from editor.server.omni_json.serializer import OmniJsonSerializer

    try:
        from editor.server.omni_json.importer import OmniJsonImporter
        serializer = OmniJsonSerializer()
        doc = serializer.from_dict(req.document)
        importer = OmniJsonImporter()
        source = importer.import_document(doc, format=True)
        return {"success": True, "source": source}
    except Exception as e:
        return {"success": False, "source": "", "error": str(e)}


class OmniJsonSaveRequest(BaseModel):
    filePath: str
    document: dict


@app.post("/api/omni-json/save")
async def api_omni_json_save(req: OmniJsonSaveRequest, request: Request):
    """保存 .omni.json 到磁盘（原子写入）(M6)."""
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
    """加载 .omni.json 从磁盘 (M6)."""
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
    """检测 .omni 源码是否为手写 (M6.5)."""
    from editor.server.omni_json.exporter import OmniJsonExporter

    exporter = OmniJsonExporter()
    is_hw = exporter._detect_hand_written(req.source)
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

