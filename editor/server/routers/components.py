"""Router: /api/component-definitions/save, /api/component-definitions/load, /api/component-definitions/delete."""

from pathlib import Path

from fastapi import APIRouter, Request

from editor.server.models.requests import ComponentDefinitionsPayload, ComponentDeletePayload

router = APIRouter()

_COMPONENTS_DIR = "src/components"


@router.post("/api/component-definitions/save")
async def api_save_component_definitions(req: ComponentDefinitionsPayload, request: Request):
    """Save custom component definitions to src/components/<Name>.omni,
    cleaning up stale files."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}
    comp_dir = Path(project_dir) / _COMPONENTS_DIR
    try:
        comp_dir.mkdir(parents=True, exist_ok=True)
        defined_names: set[str] = set()
        for name, definition in req.definitions.items():
            omni_source = definition.get("omniSource") or definition.get("source")
            if omni_source:
                defined_names.add(name)
                omni_path = comp_dir / f"{name}.omni"
                omni_path.write_text(omni_source, encoding="utf-8")
        for f in list(comp_dir.iterdir()):
            if f.suffix == ".omni" and f.stem not in defined_names:
                f.unlink()
        return {"ok": True}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@router.get("/api/component-definitions/load")
async def api_load_component_definitions(request: Request):
    """Load all custom component definitions from src/components/ directory."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"definitions": {}, "error": "No project directory"}
    comp_dir = Path(project_dir) / _COMPONENTS_DIR
    if not comp_dir.exists() or not comp_dir.is_dir():
        return {"definitions": {}}
    try:
        definitions = {}
        for f in comp_dir.iterdir():
            if f.suffix == ".omni" and f.stem:
                source = f.read_text(encoding="utf-8")
                definitions[f.stem] = {"source": source}
        return {"definitions": definitions}
    except Exception as e:
        return {"definitions": {}, "error": str(e)}


@router.post("/api/component-definitions/delete")
async def api_delete_component_definitions(req: ComponentDeletePayload, request: Request):
    """Delete custom component .omni files from src/components/."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}
    comp_dir = Path(project_dir) / _COMPONENTS_DIR
    if not comp_dir.exists():
        return {"ok": True}
    try:
        for name in req.names:
            omni_path = comp_dir / f"{name}.omni"
            if omni_path.exists():
                omni_path.unlink()
        return {"ok": True}
    except Exception as e:
        return {"ok": False, "error": str(e)}
