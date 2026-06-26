"""Router: /api/project, /api/project/set-dir, /api/project/create."""

from pathlib import Path

from fastapi import APIRouter, Request

from editor.server.models.requests import SetProjectDirRequest, CreateProjectRequest
from editor.server.services.project_manager import normalize_path

router = APIRouter()


@router.get("/api/project")
async def api_project(request: Request):
    """Get project info."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"dir": None, "target": "react", "mainFile": None}

    # Find main.omni or first valid .omni file (sorted alphabetically, main.omni first)
    main_file = None
    omni_files = sorted(
        [f for f in Path(project_dir).rglob("*.omni")],
        key=lambda f: (f.name != "main.omni", str(f))
    )
    for f in omni_files:
        try:
            content = f.read_text(encoding="utf-8")
            if content.strip().startswith("component"):
                main_file = str(f.relative_to(project_dir)).replace("\\", "/")
                break
        except Exception:
            continue
    if not main_file and omni_files:
        main_file = str(omni_files[0].relative_to(project_dir)).replace("\\", "/")

    return {"dir": str(project_dir), "target": "react", "mainFile": main_file}


@router.post("/api/project/set-dir")
async def api_project_set_dir(req: SetProjectDirRequest, request: Request):
    """Set project directory."""
    try:
        target = normalize_path(req.dir)
        if not Path(target).is_dir():
            return {"ok": False, "error": "Directory does not exist"}
        request.app.state.project_dir = target
        return {"ok": True, "dir": target}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@router.post("/api/project/create")
async def api_project_create(req: CreateProjectRequest, request: Request):
    """Create a new project (create directories and init config)."""
    try:
        target = Path(req.dir).expanduser().resolve() / req.name
        target.mkdir(parents=True, exist_ok=True)
        src_dir = target / "src"
        src_dir.mkdir(exist_ok=True)
        main_omni = src_dir / "main.omni"
        main_omni.write_text(
            f"import omniview;\n\n"
            f"component {req.name} {{\n"
            f"    view {{\n"
            f"        <Container id=\"{req.name}.root.0\">\n"
            f"            <Text id=\"{req.name}.root.0.text.0\">Hello</Text>\n"
            f"        </Container>\n"
            f"    }}\n"
            f"}}\n",
            encoding="utf-8",
        )
        request.app.state.project_dir = str(target)
        return {"ok": True, "dir": str(target)}
    except Exception as e:
        return {"ok": False, "error": str(e)}
