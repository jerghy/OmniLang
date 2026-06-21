# editor/server/annotation/types.py
"""Annotation data types — mirrors the .omni comment annotation schema."""

from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class CodeTabType(str, Enum):
    LOGIC = "logic"
    EVENT = "event"


class ViewRenderMode(str, Enum):
    DOM = "dom"
    CANVAS = "canvas"


class AnnotationErrorCode(str, Enum):
    MISSING_CODE_TAB = "MISSING_CODE_TAB"
    MISSING_EDIT = "MISSING_EDIT"
    EXTRA_EDIT = "EXTRA_EDIT"
    MISMATCHED_INDENT = "MISMATCHED_INDENT"
    EMPTY_LINE_BEFORE_ELEMENT = "EMPTY_LINE_BEFORE_ELEMENT"
    INVALID_EDIT_TOKEN = "INVALID_EDIT_TOKEN"
    INVALID_CODE_TAB_TYPE = "INVALID_CODE_TAB_TYPE"
    DUPLICATE_ELEMENT_ID = "DUPLICATE_ELEMENT_ID"
    DUPLICATE_NODE_ID = "DUPLICATE_NODE_ID"
    DUPLICATE_WIRE_ID = "DUPLICATE_WIRE_ID"
    CHAIN_NAME_MISMATCH = "CHAIN_NAME_MISMATCH"
    MISSING_REQUIRED_ATTR = "MISSING_REQUIRED_ATTR"
    MISSING_POS = "MISSING_POS"
    POS_NOT_INTEGER = "POS_NOT_INTEGER"
    INCOMPLETE_WIRE = "INCOMPLETE_WIRE"
    UNKNOWN_NODE_TAG = "UNKNOWN_NODE_TAG"
    WIRE_TARGET_NOT_FOUND = "WIRE_TARGET_NOT_FOUND"
    WIRE_PORT_INVALID = "WIRE_PORT_INVALID"
    NODE_OUTSIDE_CHAIN = "NODE_OUTSIDE_CHAIN"
    WIRE_OUTSIDE_CHAIN = "WIRE_OUTSIDE_CHAIN"
    NODE_AFTER_WIRE = "NODE_AFTER_WIRE"
    DECLARATION_BETWEEN_CHAIN_AND_FN = "DECLARATION_BETWEEN_CHAIN_AND_FN"
    UNREFERENCED_VARIABLE = "UNREFERENCED_VARIABLE"
    CODE_TAB_NOT_FIRST = "CODE_TAB_NOT_FIRST"
    INVALID_WIRE_ARROW = "INVALID_WIRE_ARROW"
    UNQUOTED_SPACE_VALUE = "UNQUOTED_SPACE_VALUE"


@dataclass
class AnnotationError:
    """A single annotation validation error."""
    code: AnnotationErrorCode
    message: str
    line: int = 0
    column: int = 0


@dataclass
class ViewMarker:
    """Represents a `// @edit` comment preceding a view element."""
    element_id: str                    # from <element id="...">
    line_number: int                   # line number in source
    is_folded: bool = False
    render_mode: ViewRenderMode = ViewRenderMode.DOM


@dataclass
class NodeEntry:
    """Represents a `// @node` declaration within a @chain block."""
    id: str                            # n1, n2, ...
    tag: str                           # signal, add, branch, fn, etc.
    attrs: dict[str, str] = field(default_factory=dict)
    pos_x: int = 0
    pos_y: int = 0
    line_number: int = 0


@dataclass
class WireEntry:
    """Represents a `// @wire` declaration within a @chain block."""
    id: str                            # w1, w2, ...
    from_id: str                       # source node id
    from_port: str = ""                # source port (empty for fn nodes)
    to_id: str = ""                    # target node id
    to_port: str = ""                  # target port (empty for fn nodes)
    is_collapsed: bool = False
    collapse_group_id: str = ""
    line_number: int = 0


@dataclass
class Chain:
    """Represents one `@chain` block — all nodes + wires that form a function."""
    name: str                          # chain name, must match fn name
    start_line: int
    end_line: int
    nodes: list[NodeEntry] = field(default_factory=list)
    wires: list[WireEntry] = field(default_factory=list)


@dataclass
class AnnotationFile:
    """Complete annotation data extracted from a single .omni file."""
    filepath: str
    code_tab_type: CodeTabType = CodeTabType.LOGIC
    code_tab_line: int = 0
    view_markers: list[ViewMarker] = field(default_factory=list)
    chains: list[Chain] = field(default_factory=list)
    raw_source: str = ""


# Node type definition: tag -> (required_attrs, optional_attrs, valid_ports)
NODE_TYPE_DEFS: dict[str, tuple[list[str], list[str], list[str]]] = {
    "signal":          (["name", "type"], ["init"], ["value", "set"]),
    "store":           (["name"], [], ["items"]),
    "computed":        (["name", "deps"], [], ["value", "deps"]),
    "getVariable":     (["name"], [], ["value"]),
    "setVariable":     (["name"], [], ["value"]),
    "add":             ([], [], ["a", "b", "result"]),
    "sub":             ([], [], ["a", "b", "result"]),
    "mul":             ([], [], ["a", "b", "result"]),
    "div":             ([], [], ["a", "b", "result"]),
    "compare":         (["op"], [], ["a", "b", "result"]),
    "and":             ([], [], ["a", "b", "result"]),
    "or":              ([], [], ["a", "b", "result"]),
    "not":             ([], [], ["a", "result"]),
    "concat":          ([], [], ["a", "b", "result"]),
    "literal":         (["value", "type"], [], ["value"]),
    "branch":          ([], [], ["condition", "true", "false"]),
    "sequence":        (["outputs"], [], ["input"]),
    "forEach":         ([], [], ["collection", "current", "index"]),
    "updateUIProperty":(["target", "property"], [], ["value"]),
    "fn":              (["source"], [], []),   # fn: no ports, connections implicit
    "customFn":        (["name", "params", "return"], ["desc"], ["return"]),
    "return":          ([], ["value"], ["value"]),
}

# Valid comparison operators for compare node
VALID_COMPARE_OPS = {"==", "!=", ">", "<", ">=", "<="}
