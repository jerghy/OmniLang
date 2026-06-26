# Task 2.10 Report: Split `editor/server/main.py` into routers + services + models

## Summary

Successfully refactored `editor/server/main.py` from 1096 lines down to 314 lines (141 lines of Python code + 163 lines of _EDITOR_HTML template).

## Files Created

### Models (`editor/server/models/`)
- `__init__.py` — package init
- `requests.py` — all 14 Pydantic models extracted from main.py (CompileRequest, FsReadRequest, FsListRequest, FsWriteRequest, ListDirectoryRequest, SetProjectDirRequest, CreateProjectRequest, CodeOwnershipRequest, ComponentDefinitionsPayload, ComponentDeletePayload, AnnotatedSaveRequest, AnnotatedLoadRequest, EditorTomlSaveRequest, ConnectorRequest)

### Services (`editor/server/services/`)
- `__init__.py` — package init
- `compiler.py` — `serialize_tree()` helper (renamed from `_serialize_tree`)
- `project_manager.py` — `normalize_path()` helper (renamed from `_normalize_path`)

### Routers (`editor/server/routers/`)
- `__init__.py` — package init
- `compile.py` — `/api/compile`, `/api/parse-view`, `/api/parse-codeblocks`, `/api/wasm/version`, `/api/code-ownership`, `/api/parse-native-ports`
- `files.py` — `/api/fs/list`, `/api/fs/read`, `/api/fs/write`, `/api/fs/list-directory`
- `project.py` — `/api/project`, `/api/project/set-dir`, `/api/project/create`
- `annotations.py` — `/api/annotated-save`, `/api/annotated-load`, `/api/editor-toml/save`, `/api/editor-toml/load` (also includes `_annotation_file_to_dict` and `_dict_to_annotation_file` helpers)
- `components.py` — `/api/component-definitions/save`, `/api/component-definitions/load`, `/api/component-definitions/delete`
- `connectors.py` — `/api/connector/generate`
- `deploy.py` — re-exports from `editor.server.api.deploy`
- `run.py` — re-exports from `editor.server.api.run`
- `debug.py` — re-exports from `editor.server.api.debug`
- `watch.py` — re-exports from `editor.server.api.watch`

## Files Modified

- `editor/server/main.py` — slimmed from 1096 to 314 lines. Removed all inline route handlers and Pydantic models. Now only handles: app creation, CORS, static mounts, `index()` route, router registration, and `cmd_serve()` entry point.

## Verification

- All 35 routes registered correctly (verified programmatically)
- `python -c "from editor.server.main import app"` — OK
- `cmd_serve` still importable for CLI entry point
- 16 existing tests pass (2 compiler-dependent tests have pre-existing failure due to missing Rust compiler)
- All new modules import correctly

## Route Count by Domain

| Domain | Routes |
|--------|--------|
| Core | /, /api/compile, /api/parse-view, /api/parse-codeblocks, /api/wasm/version, /api/code-ownership, /api/parse-native-ports |
| Files | /api/fs/list, /api/fs/read, /api/fs/write, /api/fs/list-directory |
| Project | /api/project, /api/project/set-dir, /api/project/create |
| Annotations | /api/annotated-save, /api/annotated-load, /api/editor-toml/save, /api/editor-toml/load |
| Components | /api/component-definitions/save, /api/component-definitions/load, /api/component-definitions/delete |
| Connectors | /api/connector/generate |
| Deploy | /api/deploy |
| Watch | /api/watch |
| Run | /api/run/start, /api/run/stop, /api/run/status, /api/run/write-files |
| Debug | /api/debug/ws, /api/debug/status |
| Framework | /docs, /docs/oauth2-redirect, /openapi.json, /redoc |
| Static | /wasm |
