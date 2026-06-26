"""Router: compile, parse-view, parse-codeblocks, wasm/version, code-ownership, parse-native-ports."""

from fastapi import APIRouter, Request

from editor.server.models.requests import CompileRequest, CodeOwnershipRequest
from editor.server.services.compiler import serialize_tree

router = APIRouter()


@router.post("/api/compile")
async def api_compile(req: CompileRequest, request: Request):
    """Compile .omni source -> { files, diff, errors }."""
    from omnilang.api import compile_omni

    result = compile_omni(req.source, target=req.target,
                          previous_files=req.previousFiles,
                          debug_mode=req.debug)

    request.app.state.previous_compile = result.files

    return {
        "success": result.success,
        "files": result.files,
        "diff": {k: v for k, v in (result.diff or {}).items()},
        "errors": [e.to_dict() for e in result.errors],
    }


@router.post("/api/parse-view")
async def api_parse_view(req: CompileRequest, request: Request):
    """Parse .omni source -> return view element tree (for canvas rendering)."""
    from omnilang.api import parse_view_tree
    try:
        nodes = parse_view_tree(req.source)
        return {
            "success": True,
            "nodes": [
                {
                    "id": n.id, "tag": n.tag, "props": n.props,
                    "testId": n.test_id, "sourceLine": n.source_line,
                    "children": serialize_tree(n.children),
                }
                for n in nodes
            ],
        }
    except Exception as e:
        return {"success": False, "nodes": [], "error": str(e)}


@router.post("/api/parse-codeblocks")
async def api_parse_codeblocks(req: CompileRequest, request: Request):
    """Parse .omni source -> extract code blocks (store/signal/computed/action) for canvas."""
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


@router.get("/api/wasm/version")
async def api_wasm_version():
    """Return Rust compiler version info (to check WASM availability)."""
    from omnilang.rust_bridge import RustBridge
    return {
        "rust_available": RustBridge.is_available(),
        "wasm_supported": True,
    }


@router.post("/api/code-ownership")
async def api_code_ownership(req: CodeOwnershipRequest):
    """Parse or apply code ownership regions."""
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


@router.post("/api/parse-native-ports")
async def api_parse_native_ports(req: CompileRequest):
    """Parse @native blocks -> extract @in/@out port definitions."""
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
