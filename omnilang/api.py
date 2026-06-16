"""OmniLang compilation API entry point.

Used by omniview/cli.py and editor/server/.
Pure function compile_omni() — no file I/O, input source string, output dict.

Usage:
    from omnilang.api import compile_omni, CompileResult, CompileError

    result = compile_omni(source, filepath="/abs/path/to/App.omni", target="react")
    if result.success:
        for filename, content in result.files.items():
            ...
"""

from __future__ import annotations

import re as _re
from dataclasses import dataclass, field
from pathlib import Path


# ── Error protocol ─────────────────────────────────────────

@dataclass
class CompileError:
    """Unified compilation error format."""
    file: str                           # Source filename
    line: int                           # Line number (1-based)
    col: int = 0                        # Column number (1-based)
    severity: str = "error"             # "error" | "warning"
    code: str = ""                      # Error code, e.g. "E0101"
    message: str = ""                   # Human-readable description
    suggestion: str | None = None       # Fix suggestion

    def to_dict(self) -> dict:
        return {
            "file": self.file,
            "line": self.line,
            "col": self.col,
            "severity": self.severity,
            "code": self.code,
            "message": self.message,
            "suggestion": self.suggestion,
        }


@dataclass
class CompileResult:
    """Compilation result. success=True -> files/diff; success=False -> errors."""
    success: bool
    files: dict[str, str] = field(default_factory=dict)
    diff: dict[str, str] | None = None
    errors: list[CompileError] = field(default_factory=list)
    source: str = ""


# ── UI mode detection ──────────────────────────────────────

def _detect_ui_mode(source: str) -> bool:
    """Detect whether source contains OmniView UI components."""
    ui_keywords = ("component ", "store ", "theme ", "view {", "@if", "@for", "signal ", "computed ")
    has_ui_keywords = any(kw in source for kw in ui_keywords)
    has_ui_keywords = has_ui_keywords or bool(_re.search(r'<[A-Z]', source))
    has_import_omniview = "import omniview" in source
    return has_import_omniview or has_ui_keywords


# ── Main compile function ──────────────────────────────────

def compile_omni(source: str, filepath: str = "<input>", target: str = "react",
                 previous_files: dict[str, str] | None = None,
                 debug_mode: bool = False) -> CompileResult:
    """Compile .omni source -> {filename: content} dict. Pure function, no file I/O.

    Routes all compilation through the Rust compiler (omni-compiler) via RustBridge.

    Args:
        source: .omni source string.
        filepath: Source file path (for error messages).
        target: Target framework ("react"|"vue"|"dioxus"|"flutter"|"omniview"
                |"python"|"js"|"typescript"|"rust"|"dart").
        previous_files: Previous compilation result (for diff computation).
        debug_mode: Enable runtime debug injection.

    Returns:
        CompileResult — success/files/diff/errors.
    """
    errors: list[CompileError] = []
    is_ui_mode = _detect_ui_mode(source)

    # Pure logic mode auto-fallback target
    PURE_LOGIC_TARGETS = {"python", "javascript", "js", "typescript", "ts",
                          "rust", "rs", "dart"}
    ALL_TARGETS = PURE_LOGIC_TARGETS | {"react", "vue", "dioxus", "flutter", "omniview"}
    if target not in ALL_TARGETS:
        errors.append(CompileError(
            file=filepath, line=1, severity="error", code="UNSUPPORTED",
            message=f"Unknown target '{target}'. Supported: {', '.join(sorted(ALL_TARGETS))}",
        ))
        return CompileResult(success=False, errors=errors, source=source)

    if not is_ui_mode and target not in PURE_LOGIC_TARGETS:
        target = "python"

    from omnilang.rust_bridge import RustBridge

    if not RustBridge.is_available():
        errors.append(CompileError(
            file=filepath, line=1, severity="error", code="RUST_UNAVAILABLE",
            message="Rust compiler (omni-compiler) is not available. "
                    "Build the Rust compiler or ensure omni_compiler.dll is in the release directory.",
        ))
        return CompileResult(success=False, errors=errors, source=source)

    # Route through Rust compiler
    rust_result = RustBridge.compile_to_target({filepath: source}, target)
    if rust_result is None:
        # compile_to_target not available (older DLL), try compile_to_ir + codegen
        # but we no longer have Python codegen, so this is an error
        errors.append(CompileError(
            file=filepath, line=1, severity="error", code="RUST_CODEGEN_UNAVAILABLE",
            message="Rust compiler compile_to_target function is not available. "
                    "Update the Rust compiler to the latest version.",
        ))
        return CompileResult(success=False, errors=errors, source=source)

    if rust_result.get("success"):
        # Compute diff
        diff: dict[str, str] | None = None
        if previous_files is not None:
            diff = _compute_diff(previous_files, rust_result.get("files", {}))

        result = CompileResult(
            success=True,
            files=rust_result.get("files", {}),
            diff=diff,
            source=source,
        )

        # Forward Rust warnings
        for warn in rust_result.get("warnings", []):
            if isinstance(warn, dict):
                result.errors.append(CompileError(
                    file=warn.get("file", filepath),
                    line=warn.get("line", 1),
                    col=warn.get("col", 0),
                    severity="warning",
                    code=warn.get("code", "RUST_WARN"),
                    message=str(warn.get("message", warn)),
                ))

        return result
    else:
        # Rust compilation failed
        for err in rust_result.get("errors", []):
            if isinstance(err, dict):
                errors.append(CompileError(
                    file=err.get("file", filepath),
                    line=err.get("line", 1),
                    col=err.get("col", 0),
                    severity=err.get("severity", "error"),
                    code=err.get("code", "RUST_ERROR"),
                    message=str(err.get("message", err)),
                ))
            else:
                errors.append(CompileError(
                    file=filepath, line=1, severity="error",
                    code="RUST_ERROR", message=str(err),
                ))
        return CompileResult(success=False, errors=errors, source=source)


# ── Diff helper ────────────────────────────────────────────

def _compute_diff(previous: dict[str, str], current: dict[str, str]) -> dict[str, str]:
    """Compute diff between two compilation results."""
    import difflib
    result: dict[str, str] = {}
    all_files = set(previous.keys()) | set(current.keys())
    for filename in sorted(all_files):
        old_text = previous.get(filename, "")
        new_text = current.get(filename, "")
        if old_text == new_text:
            continue
        diff_lines = difflib.unified_diff(
            old_text.splitlines(keepends=True),
            new_text.splitlines(keepends=True),
            fromfile=filename, tofile=filename,
        )
        result[filename] = "".join(diff_lines)
    return result


# ── View tree extraction (editor utility) ──────────────────

@dataclass
class ViewTreeNode:
    """View tree node, for canvas rendering."""
    id: str
    tag: str
    props: dict = field(default_factory=dict)
    children: list["ViewTreeNode"] = field(default_factory=list)
    test_id: str | None = None
    source_line: int = 1


def parse_view_tree(source: str, filepath: str = "<input>") -> list[ViewTreeNode]:
    """Parse .omni source -> extract view element tree (for canvas rendering).

    Uses Rust compiler IR if available. Returns empty list if Rust is unavailable.
    Only parses UI mode sources.
    """
    try:
        from omnilang.rust_bridge import RustBridge
        if RustBridge.is_available():
            ir_result = RustBridge.compile_to_ir(source)
            if ir_result and "_rust_errors" not in ir_result:
                components = ir_result.get("components", {}) if isinstance(ir_result, dict) else {}
                result: list[ViewTreeNode] = []
                for name, comp in components.items():
                    view = comp.get("view") if isinstance(comp, dict) else None
                    if view:
                        root = _ir_view_to_tree(view, name)
                        if root:
                            result.append(root)
                return result
    except Exception:
        pass

    # Fallback: regex-based view tree extraction
    return _extract_view_tree_regex(source)


def _ir_view_to_tree(node, name: str = "") -> ViewTreeNode | None:
    """Convert Rust IR view node dict to ViewTreeNode.

    Handles both tagged and _kind-dispatched IR formats.
    String nodes are converted to Text ViewTreeNodes.
    """
    # Handle string children (text nodes)
    if isinstance(node, str):
        return ViewTreeNode(
            id=f"text-{id(node)}",
            tag="Text",
            props={"label": node},
        )

    if not node or not isinstance(node, dict):
        return None

    kind = node.get("_kind", "")
    tag = node.get("tag", "")

    # Element: has a tag (with or without _kind)
    if tag:
        props = {}
        for k, v in node.get("props", {}).items():
            if isinstance(v, dict):
                val = v.get("value", str(v))
                props[k] = str(val) if not isinstance(val, (int, float, bool)) else str(val)
            else:
                props[k] = str(v) if v is not None else ""

        # Add bindings as props
        for k, v in node.get("bindings", {}).items():
            owner = v.get("owner") if isinstance(v, dict) else None
            vname = v.get("name", "?") if isinstance(v, dict) else str(v)
            props[k] = f"{{bind:{vname}}}" if not owner else f"{{bind:{owner}.{vname}}}"

        # Add events as props
        for k, v in node.get("events", {}).items():
            ref = v.get("ref", "") if isinstance(v, dict) else ""
            props[k] = f"->{ref}()" if ref else "->fn()"

        children = []
        for child in node.get("children", []):
            c = _ir_view_to_tree(child)
            if c:
                children.append(c)

        test_id = node.get("test_id")

        return ViewTreeNode(
            id=f"el-{tag}-{id(node)}",
            tag=tag,
            props=props,
            children=children,
            test_id=test_id,
        )

    # Conditional
    if kind == "ConditionalIntent" or "branches" in node:
        container = ViewTreeNode(
            id=f"if-{id(node)}",
            tag="Container",
            props={"data-directive": "@if"},
        )
        for branch in node.get("branches", []):
            body = branch.get("body") if isinstance(branch, dict) else None
            if isinstance(body, dict):
                child = _ir_view_to_tree(body)
                if child:
                    container.children.append(child)
        return container

    # List rendering
    if kind == "ListRenderingIntent" or "iterable" in node:
        container = ViewTreeNode(
            id=f"for-{id(node)}",
            tag="Container",
            props={"data-directive": f"@for {node.get('item_binding', {}).get('name', 'item')} in ..."},
        )
        body = node.get("body")
        if isinstance(body, dict):
            child = _ir_view_to_tree(body)
            if child:
                container.children.append(child)
        return container

    return None


def _extract_view_tree_regex(source: str) -> list[ViewTreeNode]:
    """Fallback regex-based view tree extraction when Rust is unavailable."""
    result: list[ViewTreeNode] = []
    # Find top-level component definitions
    comp_pattern = _re.compile(r'component\s+(\w+)')
    for match in comp_pattern.finditer(source):
        name = match.group(1)
        result.append(ViewTreeNode(
            id=f"comp-{name}",
            tag="Container",
            props={"data-component": name},
        ))
    return result


# ── Code block extraction (editor utility) ─────────────────

@dataclass
class CodeBlockNode:
    """Code block extracted from .omni source (signal/computed/action/store)."""
    id: str
    blockType: str  # "store" | "signal" | "computed" | "action"
    name: str
    parentStore: str | None = None
    typeAnnotation: str | None = None
    sourceLine: int = 0


def parse_code_blocks(source: str, filepath: str = "<input>") -> list[CodeBlockNode]:
    """Parse .omni source -> extract store/signal/computed/action declarations.

    Uses Rust compiler IR if available. Falls back to regex parsing.
    These blocks can be rendered as canvas nodes with ports for visual wiring.
    """
    try:
        from omnilang.rust_bridge import RustBridge
        if RustBridge.is_available():
            ir_result = RustBridge.compile_to_ir(source)
            if ir_result and "_rust_errors" not in ir_result and isinstance(ir_result, dict):
                return _extract_blocks_from_ir(ir_result)
    except Exception:
        pass

    # Fallback: regex-based extraction
    return _extract_blocks_regex(source)


def _extract_blocks_from_ir(ir: dict) -> list[CodeBlockNode]:
    """Extract code blocks from Rust IR."""
    blocks: list[CodeBlockNode] = []
    node_id = 0

    # Process stores
    for store_name, store in ir.get("stores", {}).items():
        if not isinstance(store, dict):
            continue
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-store-{store_name}",
            blockType="store",
            name=store_name,
        ))

        for sig in store.get("signals", []):
            if not isinstance(sig, dict):
                continue
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{store_name}-signal-{sig.get('name', '?')}",
                blockType="signal",
                name=sig.get("name", "?"),
                parentStore=store_name,
                typeAnnotation=_type_to_str(sig.get("type")),
            ))

        for getter in store.get("getters", []):
            if not isinstance(getter, dict):
                continue
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{store_name}-computed-{getter.get('name', '?')}",
                blockType="computed",
                name=getter.get("name", "?"),
                parentStore=store_name,
                typeAnnotation=_type_to_str(getter.get("type", getter.get("return_type"))),
            ))

        for action in store.get("actions", []):
            if not isinstance(action, dict):
                continue
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{store_name}-action-{action.get('name', '?')}",
                blockType="action",
                name=action.get("name", "?"),
                parentStore=store_name,
            ))

    # Process component-level declarations
    for comp_name, comp in ir.get("components", {}).items():
        if not isinstance(comp, dict):
            continue
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-store-{comp_name}",
            blockType="store",
            name=comp_name,
        ))

        for state in comp.get("states", []):
            if not isinstance(state, dict):
                continue
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{comp_name}-signal-{state.get('name', '?')}",
                blockType="signal",
                name=state.get("name", "?"),
                parentStore=comp_name,
                typeAnnotation=_type_to_str(state.get("type")),
            ))

        for computed in comp.get("computeds", []):
            if not isinstance(computed, dict):
                continue
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{comp_name}-computed-{computed.get('name', '?')}",
                blockType="computed",
                name=computed.get("name", "?"),
                parentStore=comp_name,
                typeAnnotation=_type_to_str(computed.get("type")),
            ))

        for fn in comp.get("fns", []):
            if not isinstance(fn, dict):
                continue
            node_id += 1
            blocks.append(CodeBlockNode(
                id=f"cb-{comp_name}-action-{fn.get('name', '?')}",
                blockType="action",
                name=fn.get("name", "?"),
                parentStore=comp_name,
            ))

    return blocks


def _type_to_str(type_obj) -> str | None:
    """Convert a Rust IR type object to a string representation."""
    if type_obj is None:
        return None
    if isinstance(type_obj, str):
        return type_obj
    if isinstance(type_obj, dict):
        kind = type_obj.get("kind", "")
        if kind and kind != "void":
            return kind
    return None


def _extract_blocks_regex(source: str) -> list[CodeBlockNode]:
    """Fallback regex-based code block extraction."""
    import re
    blocks: list[CodeBlockNode] = []
    node_id = 0

    # Match store definitions: store Name { ... }
    store_pattern = re.compile(r'store\s+(\w+)\s*\{')
    for m in store_pattern.finditer(source):
        store_name = m.group(1)
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-store-{store_name}",
            blockType="store",
            name=store_name,
            sourceLine=source[:m.start()].count('\n') + 1,
        ))

    # Match component definitions: component Name { ... }
    comp_pattern = re.compile(r'component\s+(\w+)\s*\{')
    for m in comp_pattern.finditer(source):
        comp_name = m.group(1)
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-store-{comp_name}",
            blockType="store",
            name=comp_name,
            sourceLine=source[:m.start()].count('\n') + 1,
        ))

    # Match signal declarations: signal name: Type
    signal_pattern = re.compile(r'(?:signal|let\s+mut)\s+(\w+)(?:\s*:\s*(\w+))?')
    for m in signal_pattern.finditer(source):
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-signal-{node_id}",
            blockType="signal",
            name=m.group(1),
            typeAnnotation=m.group(2),
            sourceLine=source[:m.start()].count('\n') + 1,
        ))

    # Match computed/getter declarations: computed name => expr
    computed_pattern = re.compile(r'(?:computed|get)\s+(\w+)(?:\s*:\s*(\w+))?\s*=')
    for m in computed_pattern.finditer(source):
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-computed-{node_id}",
            blockType="computed",
            name=m.group(1),
            typeAnnotation=m.group(2),
            sourceLine=source[:m.start()].count('\n') + 1,
        ))

    # Match action/fn declarations: fn name(...)
    fn_pattern = re.compile(r'(?:fn|action)\s+(\w+)\s*\(')
    for m in fn_pattern.finditer(source):
        node_id += 1
        blocks.append(CodeBlockNode(
            id=f"cb-action-{node_id}",
            blockType="action",
            name=m.group(1),
            sourceLine=source[:m.start()].count('\n') + 1,
        ))

    return blocks


# ── Native port extraction (editor utility) ────────────────

@dataclass
class NativePortInfo:
    """Extracted @in/@out port info from @native block annotations."""
    id: str
    target: str            # e.g. "react", "flutter"
    inputs: list[dict]     # [{name, type}]
    outputs: list[dict]    # [{name, type}]
    source_line: int = 0


def parse_native_ports(source: str, filepath: str = "<input>") -> list[NativePortInfo]:
    """Parse @native blocks and extract @in/@out comment annotations for canvas ports.

    Scans for patterns like:
        // @in: videoUrl (String)
        // @out: isPlaying (bool)
        @native(react) { ... }

    Returns list of NativePortInfo with extracted port definitions.

    Note: This function uses regex only, no parser dependency.
    """
    import re

    results: list[NativePortInfo] = []
    node_id = 0

    # Scan line by line
    lines = source.split("\n")
    current_ins: list[dict] = []
    current_outs: list[dict] = []

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Detect @in annotation
        in_match = re.match(r"//\s*@in:\s*(\w+)\s*\((\w+)\)", stripped)
        if in_match:
            current_ins.append({"name": in_match.group(1), "type": in_match.group(2)})
            continue

        # Detect @out annotation
        out_match = re.match(r"//\s*@out:\s*(\w+)\s*\((\w+)\)", stripped)
        if out_match:
            current_outs.append({"name": out_match.group(1), "type": out_match.group(2)})
            continue

        # Detect @native block start
        native_match = re.match(r"@native\((\w+)\)\s*\{", stripped)
        if native_match:
            node_id += 1
            target = native_match.group(1)
            results.append(NativePortInfo(
                id=f"native-{target}-{node_id}",
                target=target,
                inputs=list(current_ins),
                outputs=list(current_outs),
                source_line=i + 1,
            ))
            current_ins = []
            current_outs = []
            continue

        # Reset annotations on blank lines or non-comment lines
        if not stripped or (not stripped.startswith("//") and not stripped.startswith("@")):
            current_ins = []
            current_outs = []

    return results
