"""OmniLang compilation API entry point.

Used by editor/server/ and omnilang/cli.py.
Pure function compile_omni() — no file I/O, input source string, output dict.

Usage:
    from omnilang.api import compile_omni, CompileResult

    result = compile_omni(source, filepath="/abs/path/to/App.omni", target="react")
    if result.success:
        for filename, content in result.files.items():
            ...
"""

from __future__ import annotations

import re as _re
from dataclasses import dataclass, field

from omnilang.errors import OmniError, CompileError, CompilerUnavailable
from omnilang.result import CompileResult


# ── UI mode detection ──────────────────────────────────────

def _detect_ui_mode(source: str) -> bool:
    """Detect whether source is an OmniView UI file.

    A file is a "view file" iff it contains 'import omniview'.
    This is the single source of truth — the Rust compiler enforces
    that any file with component/store/theme must also have this import.
    """
    return "import omniview" in source


# ── Constants ──────────────────────────────────────────────

# Valid targets
PURE_LOGIC_TARGETS = {"python", "javascript", "js", "typescript", "ts",
                      "rust", "rs", "dart"}
ALL_TARGETS = frozenset(PURE_LOGIC_TARGETS | {"react", "vue", "dioxus", "flutter", "omniview"})


# ── Main compile functions ─────────────────────────────────

def compile_omni(source: str, filepath: str = "<input>", target: str = "react",
                 previous_files: dict[str, str] | None = None,
                 debug_mode: bool = False) -> CompileResult:
    """Compile a single .omni source -> {filename: content} dict.

    Backward-compatible wrapper used by the editor server.
    Returns CompileResult with errors embedded (never raises on compile failure).

    Args:
        source: Source code text.
        filepath: Path for error reporting / file naming.
        target: Target language/framework.
        previous_files: Previous compilation result (for diff computation).
        debug_mode: Enable runtime debug injection.

    Returns:
        CompileResult — success/files/diff/errors.
    """
    # Backward-compatible: if source is pure logic but target is a UI framework,
    # fall back to python (same behavior as the old compile_omni_multi).
    if not _detect_ui_mode(source) and target not in PURE_LOGIC_TARGETS:
        target = "python"

    try:
        result = compile_omni_multi(sources={filepath: source}, target=target)
    except CompileError as e:
        diff = _compute_diff(previous_files or {}, {}) if previous_files is not None else None
        return CompileResult(
            success=False,
            diff=diff,
            errors=e.errors,
        )
    except CompilerUnavailable:
        return CompileResult(
            success=False,
            errors=[OmniError(
                severity="error", code="COMPILER_UNAVAILABLE",
                message="Rust compiler not available. "
                        "Build it with: cd omni-compiler && cargo build --release",
                file=filepath,
            )],
        )

    diff = _compute_diff(previous_files, result.files) if previous_files is not None else None
    return CompileResult(
        success=result.success,
        files=result.files,
        diff=diff,
        errors=result.errors,
        warnings=result.warnings,
    )


def compile_omni_multi(
    sources: dict[str, str],
    target: str = "python",
    path_mapping: dict[str, str] | None = None,
    **_kwargs,
) -> CompileResult:
    """Compile multiple .omni sources via Rust compiler.

    Routes all compilation through the Rust compiler (omni-compiler) via RustBridge.
    Supports cross-file imports.

    Args:
        sources: {filepath: source_text} dict.
        target: Target language/framework.
        path_mapping: {import_path: resolved_filepath} for resolving imports.

    Returns:
        CompileResult with success flag and generated files.

    Raises:
        CompilerUnavailable: If the Rust compiler DLL cannot be found.
        CompileError: If compilation fails with errors.
    """
    from omnilang.rust_bridge import RustBridge

    # Auto-derive path mapping for cross-file import resolution
    if path_mapping is None:
        path_mapping = _derive_path_mapping(sources)

    primary = next(iter(sources.keys()), "<input>")

    try:
        result = RustBridge.compile(sources, target, path_mapping)
    except CompilerUnavailable:
        raise
    except Exception as e:
        raise CompileError([OmniError(
            severity="error", code="BRIDGE_ERROR",
            message=f"Rust compiler call failed: {e}",
            file=primary,
        )])

    if not result.success and result.errors:
        raise CompileError(result.errors + result.warnings)
    return result


# ── IR stopgap (editor server) ─────────────────────────────

def _compile_ir_internal(sources: dict[str, str]) -> dict:
    """Stopgap for editor server — compile sources and return raw dict.

    Returns {"files": {...}, "success": True/False}.
    """
    from omnilang.rust_bridge import RustBridge
    result = RustBridge.compile(sources, "python")
    return {"files": result.files, "success": result.success}


# ── Path mapping / diff helpers ────────────────────────────

def _derive_path_mapping(sources: dict[str, str]) -> dict[str, str]:
    """Derive import path mapping from source file paths.

    Maps relative import paths to the actual file paths in the sources dict.
    Handles: './Component.omni', './Component', '../stores/Store',
    '../lib', basenames, and stems.

    For each importing file, computes the relative path from its directory
    to every other source file, so cross-file imports resolve correctly.
    """
    import os
    mapping: dict[str, str] = {}

    for filepath in sources:
        # Direct path → self
        mapping[filepath] = filepath

        # Basename (e.g. "TodoItem.omni")
        basename = os.path.basename(filepath)
        if basename not in mapping:
            mapping[basename] = filepath

        # Stem (e.g. "TodoItem")
        stem = os.path.splitext(basename)[0]
        if stem not in mapping:
            mapping[stem] = filepath

    # For every pair of files, compute relative-path keys
    for import_path in sources:
        importer_dir = os.path.dirname(import_path)
        for target_path in sources:
            if import_path == target_path:
                continue
            try:
                rel = os.path.relpath(target_path, importer_dir)
            except ValueError:
                continue

            rel_normalized = rel.replace("\\", "/")

            # Map "./Component.omni" → full path
            mapping["./" + rel_normalized] = target_path

            # Map "../lib" (without ./ prefix, for non-sibling imports)
            mapping[rel_normalized] = target_path

            # Map "./Component" (without .omni extension) → full path
            if rel.endswith(".omni"):
                rel_no_ext = rel_normalized[:-5]
                mapping["./" + rel_no_ext] = target_path
                mapping[rel_no_ext] = target_path

    return mapping


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

    Uses regex-based extraction. (Rust IR no longer available via DLL.)
    """
    return _extract_view_tree_regex(source)


def _extract_view_tree_regex(source: str) -> list[ViewTreeNode]:
    """Fallback regex-based view tree extraction."""
    result: list[ViewTreeNode] = []
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

    Uses regex-based extraction. (Rust IR no longer available via DLL.)
    """
    return _extract_blocks_regex(source)


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
