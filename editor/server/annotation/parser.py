# editor/server/annotation/parser.py
"""Parse .omni source -> AnnotationFile by extracting editor annotation comments."""

from __future__ import annotations
import re
from typing import Any

from editor.server.annotation.types import (
    AnnotationFile,
    AnnotationError,
    AnnotationErrorCode,
    Chain,
    CodeTabType,
    NodeEntry,
    ViewMarker,
    ViewRenderMode,
    WireEntry,
)

# -- Regex Patterns ----------------------------------------------------------

_CODE_TAB_RE = re.compile(
    r"^//\s*@code-tab\s+type=(\w+)\s*$"
)
_EDIT_RE = re.compile(
    r"^//\s*@edit(\s+folded)?(\s+render=(dom|canvas))?\s*$"
)
_CHAIN_RE = re.compile(
    r"^//\s*@chain\s+(\w+)\s*$"
)
_NODE_RE = re.compile(
    r"^//\s*@node\s+(\w+)\s+(\w+)(.*?)\s+pos=(\d+),(\d+)\s*$"
)
_WIRE_RE = re.compile(
    r"^//\s*@wire\s+(\w+)\s+(\w+)(?:\.(\w+))?\s*->\s*(\w+)(?:\.(\w+))?\s*(collapsed)?\s*(?:group=(\w+))?\s*$"
)
_BLOCK_SEP_RE = re.compile(r"^//\s*═+\s*$")
_ELEMENT_RE = re.compile(r"^(\s*)<(\w+)\b")
_FN_DECL_RE = re.compile(r"^\s*(pub\s+)?fn\s+(\w+)")
_ACTION_DECL_RE = re.compile(r"^\s*action\s+(\w+)")
_ID_ATTR_RE = re.compile(r'\bid="([^"]*)"')


def parse_annotations(source: str, filepath: str = "<input>") -> AnnotationFile:
    """Parse editor annotations from .omni source. Raises ValueError on parse failure."""
    lines = source.split("\n")
    errors: list[AnnotationError] = []

    # -- Parse @code-tab (must be first non-blank, non-comment line) ----------
    code_tab_type = CodeTabType.LOGIC
    code_tab_line = 0
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "":
            continue
        if stripped.startswith("//") and not stripped.startswith("// @"):
            continue  # skip non-annotation comments
        m = _CODE_TAB_RE.match(stripped)
        if m:
            raw_type = m.group(1)
            if raw_type not in {e.value for e in CodeTabType}:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.INVALID_CODE_TAB_TYPE,
                    message=f"Invalid code-tab type '{raw_type}'; expected logic or event",
                    line=i + 1,
                ))
                break
            code_tab_type = CodeTabType(raw_type)
            code_tab_line = i + 1
            break
        else:
            errors.append(AnnotationError(
                code=AnnotationErrorCode.MISSING_CODE_TAB,
                message="File must start with '// @code-tab type=logic|event'",
                line=i + 1,
            ))
            break
    else:
        errors.append(AnnotationError(
            code=AnnotationErrorCode.MISSING_CODE_TAB,
            message="File must start with '// @code-tab type=logic|event'",
            line=1,
        ))

    # -- Parse view_markers (@edit lines) ------------------------------------
    view_markers: list[ViewMarker] = []
    in_view_block = False
    view_indent = 0

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Detect view block start
        if not in_view_block and re.match(r"^\s*view\s*\{", line):
            in_view_block = True
            view_indent = len(line) - len(line.lstrip())
            continue

        # Detect view block end
        if in_view_block:
            indent = len(line) - len(line.lstrip())
            if stripped == "}" and indent == view_indent:
                in_view_block = False
                continue

        if not in_view_block:
            continue

        # Match @edit annotation
        edit_m = _EDIT_RE.match(stripped)
        if edit_m:
            is_folded = edit_m.group(1) is not None and "folded" in edit_m.group(1)
            render_str = edit_m.group(3)
            render_mode = ViewRenderMode.DOM
            if render_str == "canvas":
                render_mode = ViewRenderMode.CANVAS
            elif render_str == "dom":
                render_mode = ViewRenderMode.DOM

            # Next non-blank line must be an <element
            next_line_idx = i + 1
            while next_line_idx < len(lines) and lines[next_line_idx].strip() == "":
                next_line_idx += 1
            if next_line_idx >= len(lines):
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.MISSING_EDIT,
                    message=f"@edit at line {i+1} has no following XML element",
                    line=i + 1,
                ))
                continue

            elem_m = _ELEMENT_RE.match(lines[next_line_idx])
            if not elem_m:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.MISSING_EDIT,
                    message=f"@edit at line {i+1} not followed by XML element",
                    line=i + 1,
                ))
                continue

            # Extract element id
            id_match = _ID_ATTR_RE.search(lines[next_line_idx])
            element_id = id_match.group(1) if id_match else ""

            # Check indent consistency
            elem_indent = len(elem_m.group(1))
            edit_indent = len(line) - len(line.lstrip())
            if edit_indent != elem_indent:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.MISMATCHED_INDENT,
                    message=f"@edit indent ({edit_indent}) != element indent ({elem_indent}) at line {i+1}",
                    line=i + 1,
                ))

            view_markers.append(ViewMarker(
                element_id=element_id,
                line_number=i + 1,
                is_folded=is_folded,
                render_mode=render_mode,
            ))

    # -- Parse chains (@chain blocks) ----------------------------------------
    chains: list[Chain] = []
    current_chain: Chain | None = None
    in_chain = False

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Check for @chain start
        chain_m = _CHAIN_RE.match(stripped)
        if chain_m:
            if current_chain:
                current_chain.end_line = i
                chains.append(current_chain)
            current_chain = Chain(
                name=chain_m.group(1),
                start_line=i + 1,
                end_line=i + 1,
            )
            in_chain = True
            continue

        # Check for block separator (end of chain block)
        if in_chain and _BLOCK_SEP_RE.match(stripped):
            if current_chain:
                current_chain.end_line = i + 1
                chains.append(current_chain)
                current_chain = None
            in_chain = False
            continue

        if not in_chain or current_chain is None:
            continue

        # Parse @node within chain
        node_m = _NODE_RE.match(stripped)
        if node_m:
            node_id = node_m.group(1)
            tag = node_m.group(2)
            attrs_str = node_m.group(3).strip()
            pos_x = int(node_m.group(4))
            pos_y = int(node_m.group(5))

            # Parse attributes
            attrs: dict[str, str] = {}
            if attrs_str:
                # attr=value pairs (value may be quoted)
                attr_matches = re.findall(r'(\w+)=("[^"]*"|\S+)', attrs_str)
                for k, v in attr_matches:
                    attrs[k] = v.strip('"')

            current_chain.nodes.append(NodeEntry(
                id=node_id,
                tag=tag,
                attrs=attrs,
                pos_x=pos_x,
                pos_y=pos_y,
                line_number=i + 1,
            ))
            continue

        # Parse @wire within chain
        wire_m = _WIRE_RE.match(stripped)
        if wire_m:
            wire_id = wire_m.group(1)
            from_id = wire_m.group(2)
            from_port = wire_m.group(3) or ""
            to_id = wire_m.group(4)
            to_port = wire_m.group(5) or ""
            is_collapsed = wire_m.group(6) is not None
            collapse_group = wire_m.group(7) or ""

            current_chain.wires.append(WireEntry(
                id=wire_id,
                from_id=from_id,
                from_port=from_port,
                to_id=to_id,
                to_port=to_port,
                is_collapsed=is_collapsed,
                collapse_group_id=collapse_group,
                line_number=i + 1,
            ))
            continue

    # Don't forget the last chain
    if current_chain:
        current_chain.end_line = len(lines)
        chains.append(current_chain)

    if errors:
        raise ValueError(_format_errors(errors))

    return AnnotationFile(
        filepath=filepath,
        code_tab_type=code_tab_type,
        code_tab_line=code_tab_line,
        view_markers=view_markers,
        chains=chains,
        raw_source=source,
    )


def _format_errors(errors: list[AnnotationError]) -> str:
    """Format validation errors into a readable error message."""
    lines = []
    for e in errors:
        lines.append(f"[{e.code.value}] line {e.line}: {e.message}")
    return "\n".join(lines)
