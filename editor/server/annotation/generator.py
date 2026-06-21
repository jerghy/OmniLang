"""Generate editor annotation comments and inject them into .omni source."""

from __future__ import annotations
import re

from editor.server.annotation.types import (
    AnnotationFile,
    Chain,
    NodeEntry,
    WireEntry,
    ViewMarker,
    ViewRenderMode,
)


def generate_annotations(ann: AnnotationFile, source: str) -> str:
    """Inject editor annotations into .omni source. Modifies source in-place:
    1. Insert/replace @code-tab at file top
    2. Insert @edit before each view <element>
    3. Insert @chain blocks before each fn/action
    """
    lines = source.split("\n")

    # 1. Insert/replace @code-tab
    lines = _insert_code_tab(lines, ann)

    # 2. Remove old @edit annotations, then re-insert
    lines = _strip_old_annotations(lines)
    lines = _insert_edit_markers(lines, ann)

    # 3. Remove old @chain blocks, then re-insert
    lines = _strip_old_chain_blocks(lines)
    lines = _insert_chain_blocks(lines, ann)

    return "\n".join(lines)


def _insert_code_tab(lines: list[str], ann: AnnotationFile) -> list[str]:
    """Ensure @code-tab is the first line."""
    tab_line = f"// @code-tab type={ann.code_tab_type.value}"

    # Remove existing @code-tab lines
    lines = [l for l in lines if not re.match(r"^//\s*@code-tab\s", l.strip())]

    # Insert as first meaningful line
    insert_at = 0
    while insert_at < len(lines) and (
        lines[insert_at].strip() == ""
        or (
            lines[insert_at].strip().startswith("//")
            and not lines[insert_at].strip().startswith("// @")
        )
    ):
        insert_at += 1

    lines.insert(insert_at, tab_line)
    return lines


def _strip_old_annotations(lines: list[str]) -> list[str]:
    """Remove existing @edit lines."""
    return [l for l in lines if not re.match(r"^\s*//\s*@edit\b", l)]


def _strip_old_chain_blocks(lines: list[str]) -> list[str]:
    """Remove existing @chain blocks delimited by paired ══ separators."""
    result: list[str] = []
    i = 0
    while i < len(lines):
        stripped = lines[i].strip()
        if re.match(r"^//\s*═", stripped):
            j = i + 1
            found_annotation = False
            while j < len(lines):
                s = lines[j].strip()
                if re.match(r"^//\s*═", s):
                    break
                if re.match(r"^//\s*@(chain|node|wire)\b", s):
                    found_annotation = True
                j += 1
            if found_annotation and j < len(lines):
                i = j + 1
                continue
        result.append(lines[i])
        i += 1
    return result


def _insert_edit_markers(lines: list[str], ann: AnnotationFile) -> list[str]:
    """Insert @edit annotations before each <element in view block."""
    result: list[str] = []
    in_view = False
    view_indent = 0
    marker_index = 0

    for i, line in enumerate(lines):
        stripped = line.strip()

        if not in_view and re.match(r"^\s*view\s*\{", line):
            in_view = True
            view_indent = len(line) - len(line.lstrip())

        if in_view:
            indent = len(line) - len(line.lstrip())
            if stripped == "}" and indent == view_indent:
                in_view = False

        if in_view and re.match(r"^\s*<(\w+)", stripped):
            elem_indent = len(line) - len(line.lstrip())
            if marker_index < len(ann.view_markers):
                marker = ann.view_markers[marker_index]
                marker_index += 1
                edit_parts = ["// @edit"]
                if marker.is_folded:
                    edit_parts.append("folded")
                if marker.render_mode == ViewRenderMode.CANVAS:
                    edit_parts.append("render=canvas")
                edit_line = " " * elem_indent + " ".join(edit_parts)
                result.append(edit_line)
            else:
                raise ValueError(
                    f"view_markers count ({len(ann.view_markers)}) < view elements; "
                    f"annotation data out of sync with source"
                )

        result.append(line)

    if marker_index != len(ann.view_markers):
        raise ValueError(
            f"view_markers count ({len(ann.view_markers)}) != view elements ({marker_index} matched); "
            f"annotation data out of sync with source"
        )

    return result


def _insert_chain_blocks(lines: list[str], ann: AnnotationFile) -> list[str]:
    """Insert @chain blocks before each fn/action declaration."""
    if not ann.chains:
        return lines

    result: list[str] = []
    chain_index = 0

    for i, line in enumerate(lines):
        stripped = line.strip()

        m = re.match(r"^\s*(pub\s+)?(fn|action)\s+(\w+)", stripped)
        if m and chain_index < len(ann.chains):
            fn_name = m.group(3)
            chain = ann.chains[chain_index]
            if chain.name == fn_name:
                indent = len(line) - len(line.lstrip())
                result.extend(_render_chain_block(chain, indent))
                chain_index += 1

        result.append(line)

    return result


def _render_chain_block(chain: Chain, indent: int = 0) -> list[str]:
    """Render a @chain block as list of lines."""
    SEP = "// " + "═" * 55
    pad = " " * indent
    lines: list[str] = [pad + SEP]
    lines.append(pad + f"// @chain {chain.name}")

    for node in chain.nodes:
        attrs_str = ""
        if node.attrs:
            parts: list[str] = []
            for k, v in node.attrs.items():
                if " " in v:
                    parts.append(f'{k}="{v}"')
                else:
                    parts.append(f"{k}={v}")
            attrs_str = "  " + "  ".join(parts)
        line = f"//   @node {node.id}  {node.tag}{attrs_str}  pos={node.pos_x},{node.pos_y}".rstrip()
        lines.append(pad + line)

    for wire in chain.wires:
        from_str = f"{wire.from_id}.{wire.from_port}" if wire.from_port else wire.from_id
        to_str = f"{wire.to_id}.{wire.to_port}" if wire.to_port else wire.to_id
        extras = ""
        if wire.is_collapsed:
            extras += "  collapsed"
        if wire.collapse_group_id:
            extras += f"  group={wire.collapse_group_id}"
        line = f"//   @wire {wire.id}  {from_str} -> {to_str}{extras}".rstrip()
        lines.append(pad + line)

    lines.append(pad + SEP)
    return lines
