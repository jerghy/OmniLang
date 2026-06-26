"""Router: /api/annotated-save, /api/annotated-load, /api/editor-toml/save, /api/editor-toml/load."""

import os
import uuid
from pathlib import Path

from fastapi import APIRouter, Request

from editor.server.models.requests import AnnotatedSaveRequest, AnnotatedLoadRequest, EditorTomlSaveRequest
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.generator import generate_annotations
from editor.server.annotation.validator import validate_annotations
from editor.server.annotation.editor_toml import read_editor_toml, write_editor_toml, EditorTomlState
from editor.server.annotation.types import AnnotationFile, CodeTabType, ViewRenderMode, Chain, NodeEntry, WireEntry, ViewMarker

router = APIRouter()


# ── Helpers ───────────────────────────────────────────────


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


# ── Endpoints ─────────────────────────────────────────────


@router.post("/api/annotated-save")
async def api_annotated_save(req: AnnotatedSaveRequest, request: Request):
    """Save .omni source with editor annotations injected."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"ok": False, "error": "Access denied"}

    try:
        ann = _dict_to_annotation_file(req.annotations, req.filePath)
        ann.raw_source = req.source
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
        annotated_source = generate_annotations(ann, req.source)
        tmp_path = file_path.parent / f"{file_path.name}.tmp.{uuid.uuid4().hex[:8]}"
        tmp_path.write_text(annotated_source, encoding="utf-8")
        with open(tmp_path, "r+b") as f:
            os.fsync(f.fileno())
        os.replace(tmp_path, file_path)
        return {"ok": True, "path": str(file_path.relative_to(project_dir)).replace("\\", "/")}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@router.post("/api/annotated-load")
async def api_annotated_load(req: AnnotatedLoadRequest, request: Request):
    """Load .omni source and extract editor annotations."""
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


@router.post("/api/editor-toml/save")
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


@router.get("/api/editor-toml/load")
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
