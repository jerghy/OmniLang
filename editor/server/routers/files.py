"""Router: /api/fs/list, /api/fs/read, /api/fs/write, /api/fs/list-directory."""

from pathlib import Path

from fastapi import APIRouter, Request

from editor.server.models.requests import FsReadRequest, FsListRequest, FsWriteRequest, ListDirectoryRequest

router = APIRouter()

SOURCE_EXTS = {".omni", ".tsx", ".ts", ".jsx", ".js", ".json", ".css", ".vue", ".py", ".rs", ".dart"}


@router.post("/api/fs/list")
async def api_fs_list(req: FsListRequest, request: Request):
    """List directory contents."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"entries": [], "error": "No project directory"}

    target_dir = (Path(project_dir) / req.dirPath).resolve()
    if not str(target_dir).startswith(str(Path(project_dir).resolve())):
        return {"entries": [], "error": "Access denied"}

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


@router.post("/api/fs/read")
async def api_fs_read(req: FsReadRequest, request: Request):
    """Read project file contents."""
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


@router.post("/api/fs/write")
async def api_fs_write(req: FsWriteRequest, request: Request):
    """Atomically write a file."""
    import os
    import uuid

    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"ok": False, "error": "Access denied"}

    file_path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = file_path.parent / f"{file_path.name}.tmp.{uuid.uuid4().hex[:8]}"
    try:
        tmp_path.write_text(req.content, encoding="utf-8")
        with open(tmp_path, "r+b") as f:
            os.fsync(f.fileno())
        os.replace(tmp_path, file_path)
        return {"ok": True}
    except Exception as e:
        if tmp_path.exists():
            tmp_path.unlink()
        return {"ok": False, "error": str(e)}


@router.post("/api/fs/list-directory")
async def api_list_directory(req: ListDirectoryRequest):
    """List directory contents (for welcome screen directory browser)."""
    try:
        target = Path(req.path).expanduser().resolve()
        if not target.is_dir():
            return {"ok": False, "error": "Directory does not exist"}
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
        return {"ok": False, "error": "Permission denied"}
    except Exception as e:
        return {"ok": False, "error": str(e)}
