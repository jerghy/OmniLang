# editor/server/annotation/validator.py
"""Validate AnnotationFile against all P, I, C, S, F rules."""

from __future__ import annotations
import re
from collections import Counter

from editor.server.annotation.types import (
    AnnotationError,
    AnnotationErrorCode,
    AnnotationFile,
    NODE_TYPE_DEFS,
    VALID_COMPARE_OPS,
)


def validate_annotations(file: AnnotationFile) -> list[AnnotationError]:
    """Run all validation rules on an AnnotationFile. Returns list of errors (empty = valid)."""
    errors: list[AnnotationError] = []
    lines = file.raw_source.split("\n")

    _validate_position_rules(file, lines, errors)
    _validate_identity_rules(file, errors)
    _validate_completeness_rules(file, lines, errors)
    _validate_consistency_rules(file, errors)
    _validate_format_rules(file, lines, errors)

    return errors


# ── Position Rules (P) ──────────────────────────────────


def _validate_position_rules(
    file: AnnotationFile, lines: list[str], errors: list[AnnotationError]
) -> None:
    # P1: @code-tab must be first non-blank, non-comment (non-annotation) line
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "":
            continue
        if stripped.startswith("//") and not stripped.startswith("// @"):
            continue
        if not stripped.startswith("// @code-tab"):
            errors.append(AnnotationError(
                code=AnnotationErrorCode.CODE_TAB_NOT_FIRST,
                message=f"@code-tab must be the first directive; found '{stripped[:40]}' at line {i + 1}",
                line=i + 1,
            ))
        break

    # P6: Within each chain, all @node must come before all @wire
    for chain in file.chains:
        if not chain.nodes or not chain.wires:
            continue
        min_wire_line = min(w.line_number for w in chain.wires)
        for node in chain.nodes:
            if node.line_number > min_wire_line:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.NODE_AFTER_WIRE,
                    message=f"@node {node.id} (line {node.line_number}) must come before @wire in chain '{chain.name}'",
                    line=node.line_number,
                ))
                break

        # P4: No declarations between @chain end and its function
        fn_start_line: int | None = None
        for i in range(chain.end_line, len(lines)):
            stripped = lines[i].strip()
            if re.match(r"^\s*(pub\s+)?(fn|action)\s+\w+", stripped):
                fn_start_line = i
                break
        if fn_start_line is not None:
            for i in range(chain.end_line, fn_start_line):
                stripped = lines[i].strip()
                if re.match(r"^\s*(signal|computed|fn|action|prop|event|@store)\s", stripped):
                    errors.append(AnnotationError(
                        code=AnnotationErrorCode.DECLARATION_BETWEEN_CHAIN_AND_FN,
                        message=f"Declaration at line {i + 1} between @chain '{chain.name}' and its function",
                        line=i + 1,
                    ))


# ── Identity Rules (I) ──────────────────────────────────


def _validate_identity_rules(
    file: AnnotationFile, errors: list[AnnotationError]
) -> None:
    # I1: Duplicate element IDs in view_markers
    ids = [m.element_id for m in file.view_markers if m.element_id]
    dupes = [eid for eid, count in Counter(ids).items() if count > 1]
    for eid in dupes:
        duplicates = [m for m in file.view_markers if m.element_id == eid]
        for m in duplicates[1:]:
            errors.append(AnnotationError(
                code=AnnotationErrorCode.DUPLICATE_ELEMENT_ID,
                message=f"Duplicate element id='{eid}' at line {m.line_number}",
                line=m.line_number,
            ))

    # I2: Duplicate @node IDs within each chain
    for chain in file.chains:
        node_ids = [n.id for n in chain.nodes]
        dupes = [nid for nid, count in Counter(node_ids).items() if count > 1]
        for nid in dupes:
            duplicates = [n for n in chain.nodes if n.id == nid]
            for n in duplicates[1:]:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.DUPLICATE_NODE_ID,
                    message=f"Duplicate @node id='{nid}' at line {n.line_number} in chain '{chain.name}'",
                    line=n.line_number,
                ))

    # I3: Duplicate @wire IDs within each chain
    for chain in file.chains:
        wire_ids = [w.id for w in chain.wires]
        dupes = [wid for wid, count in Counter(wire_ids).items() if count > 1]
        for wid in dupes:
            duplicates = [w for w in chain.wires if w.id == wid]
            for w in duplicates[1:]:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.DUPLICATE_WIRE_ID,
                    message=f"Duplicate @wire id='{wid}' at line {w.line_number} in chain '{chain.name}'",
                    line=w.line_number,
                ))

    # I4: Chain name must match function name
    for chain in file.chains:
        lines = file.raw_source.split("\n")
        for i in range(chain.end_line, len(lines)):
            m = re.match(r"^\s*(pub\s+)?(fn|action)\s+(\w+)", lines[i])
            if m:
                fn_name = m.group(3)
                if fn_name != chain.name:
                    errors.append(AnnotationError(
                        code=AnnotationErrorCode.CHAIN_NAME_MISMATCH,
                        message=f"Chain name '{chain.name}' does not match function name '{fn_name}' at line {i + 1}",
                        line=chain.start_line,
                    ))
                break


# ── Completeness Rules (C) ──────────────────────────────


def _validate_completeness_rules(
    file: AnnotationFile, lines: list[str], errors: list[AnnotationError]
) -> None:
    # C1: Required attributes for each node tag
    for chain in file.chains:
        for node in chain.nodes:
            tag_def = NODE_TYPE_DEFS.get(node.tag)
            if tag_def is None:
                continue  # caught by S4
            required_attrs, _, _ = tag_def
            for req in required_attrs:
                if req not in node.attrs:
                    errors.append(AnnotationError(
                        code=AnnotationErrorCode.MISSING_REQUIRED_ATTR,
                        message=f"@node {node.id} ({node.tag}) missing required attr '{req}' at line {node.line_number}",
                        line=node.line_number,
                    ))

    # C3: Wire must have both from and to endpoints
    for chain in file.chains:
        for wire in chain.wires:
            if not wire.from_id or not wire.to_id:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.INCOMPLETE_WIRE,
                    message=f"@wire {wire.id} at line {wire.line_number} missing from_id or to_id",
                    line=wire.line_number,
                ))

    # C4: Every element inside view block must have matching @edit
    in_view = False
    view_indent = 0

    for i, line in enumerate(lines):
        stripped = line.strip()

        if not in_view and re.match(r"^\s*view\s*\{", line):
            in_view = True
            view_indent = len(line) - len(line.lstrip())
            continue

        if in_view:
            indent = len(line) - len(line.lstrip())
            if stripped == "}" and indent == view_indent:
                in_view = False
                continue

        if not in_view:
            continue

        elem_m = re.match(r"^\s*<(\w+)", stripped)
        if elem_m:
            # Check if the previous non-blank line was @edit
            prev_line = i - 1
            while prev_line >= 0 and lines[prev_line].strip() == "":
                prev_line -= 1
            if prev_line >= 0:
                prev_stripped = lines[prev_line].strip()
                if not prev_stripped.startswith("// @edit"):
                    errors.append(AnnotationError(
                        code=AnnotationErrorCode.EXTRA_EDIT,
                        message=f"Element '<{elem_m.group(1)}>' at line {i + 1} missing // @edit annotation",
                        line=i + 1,
                    ))


# ── Consistency Rules (S) ───────────────────────────────


def _validate_consistency_rules(
    file: AnnotationFile, errors: list[AnnotationError]
) -> None:
    for chain in file.chains:
        node_ids = {n.id for n in chain.nodes}
        # Build map of node id -> valid ports
        node_ports: dict[str, set[str]] = {}
        for n in chain.nodes:
            tag_def = NODE_TYPE_DEFS.get(n.tag)
            if tag_def:
                _, _, ports = tag_def
                node_ports[n.id] = set(ports)
            else:
                node_ports[n.id] = set()

        for wire in chain.wires:
            # S1: Wire references must exist
            if wire.from_id not in node_ids:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.WIRE_TARGET_NOT_FOUND,
                    message=f"@wire {wire.id}: from_id '{wire.from_id}' not found in chain '{chain.name}'",
                    line=wire.line_number,
                ))
            if wire.to_id not in node_ids:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.WIRE_TARGET_NOT_FOUND,
                    message=f"@wire {wire.id}: to_id '{wire.to_id}' not found in chain '{chain.name}'",
                    line=wire.line_number,
                ))

            # S2: Port must be valid for the node type
            if wire.from_port and wire.from_id in node_ports:
                valid = node_ports[wire.from_id]
                if valid and wire.from_port not in valid:
                    errors.append(AnnotationError(
                        code=AnnotationErrorCode.WIRE_PORT_INVALID,
                        message=f"@wire {wire.id}: port '{wire.from_port}' invalid for node '{wire.from_id}'",
                        line=wire.line_number,
                    ))
            if wire.to_port and wire.to_id in node_ports:
                valid = node_ports[wire.to_id]
                if valid and wire.to_port not in valid:
                    errors.append(AnnotationError(
                        code=AnnotationErrorCode.WIRE_PORT_INVALID,
                        message=f"@wire {wire.id}: port '{wire.to_port}' invalid for node '{wire.to_id}'",
                        line=wire.line_number,
                    ))

    # S3: code_tab type must be logic or event (also enforced at parse time)
    if file.code_tab_type.value not in ("logic", "event"):
        errors.append(AnnotationError(
            code=AnnotationErrorCode.INVALID_CODE_TAB_TYPE,
            message=f"@code-tab type must be 'logic' or 'event', got '{file.code_tab_type.value}'",
            line=file.code_tab_line,
        ))

    # S4: Node tag must be registered in NODE_TYPE_DEFS
    for chain in file.chains:
        for node in chain.nodes:
            if node.tag not in NODE_TYPE_DEFS:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.UNKNOWN_NODE_TAG,
                    message=f"Unknown @node tag '{node.tag}' at line {node.line_number}",
                    line=node.line_number,
                ))

    # S5: Variables referenced in node attrs must exist in scope
    # (Scope resolution is complex -- defer to future phase)
    pass


# ── Format Rules (F) ────────────────────────────────────


def _validate_format_rules(
    file: AnnotationFile, lines: list[str], errors: list[AnnotationError]
) -> None:
    # F1: @edit only allows folded, render=dom, render=canvas
    # (parser only matches valid tokens -- violations are implicitly caught)

    # F2: Wire arrow must be "->" (parser already enforces this via regex)

    # F3: Attribute values with spaces must be quoted
    for chain in file.chains:
        for node in chain.nodes:
            for key, value in node.attrs.items():
                if " " in value and not (value.startswith('"') and value.endswith('"')):
                    errors.append(AnnotationError(
                        code=AnnotationErrorCode.UNQUOTED_SPACE_VALUE,
                        message=f"@node {node.id}: attr '{key}' value contains spaces but is not quoted at line {node.line_number}",
                        line=node.line_number,
                    ))
