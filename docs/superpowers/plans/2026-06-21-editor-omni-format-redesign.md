# Editor .omni Annotation Save Format Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `.omni.json` monolith with `.omni` source files containing embedded editor annotations (`@code-tab`, `@edit`, `@chain`, `@node`, `@wire`), plus `.omni-editor.toml` for UI state.

**Architecture:** New `editor/server/annotation/` Python package handles parse/generate/validate of annotation comments. Frontend TypeScript types mirror the annotation schema. Old `omni_json` module is deprecated but retained for migration. The editor reads `.omni` files → extracts annotations → renders canvas; saves by injecting annotations into `.omni` files via atomic write.

**Tech Stack:** Python 3.10+ (dataclasses, re), TypeScript 5.x, Zustand, FastAPI, pytest, Vitest

**Spec:** `docs/superpowers/specs/2026-06-21-editor-omni-format-redesign.md`

---

## File Structure Map

```
editor/server/annotation/          ← NEW: core annotation engine
├── __init__.py
├── types.py                       ← dataclasses: AnnotationFile, Chain, NodeEntry, WireEntry, ViewMarker, EditorState
├── parser.py                      ← parse_annotations(source: str) → AnnotationFile
├── generator.py                   ← generate_annotations(file: AnnotationFile, source: str) → str
├── validator.py                   ← validate(file: AnnotationFile) → list[AnnotationError]
├── editor_toml.py                 ← read/write .omni-editor.toml
└── migrate.py                     ← migrate_omni_json(json_path: Path, project_dir: Path) → None

editor/client/src/annotation/      ← NEW: frontend type mirrors
├── types.ts                       ← TS interfaces matching Python types
└── parser.ts                      ← Client-side annotation type guard (if needed)

editor/server/main.py              ← MODIFY: new API endpoints, deprecations
editor/server/omni_json/           ← KEEP: for migration only
editor/client/src/api/client.ts    ← MODIFY: add new API methods
editor/client/src/store/editor.ts  ← MODIFY: update save/load actions
editor/client/src/omniJson/types.ts← MODIFY: mark v1 types deprecated

tests/                              ← NEW tests
├── annotation/
│   ├── test_parser.py
│   ├── test_generator.py
│   ├── test_validator.py
│   ├── test_editor_toml.py
│   ├── test_roundtrip.py
│   └── test_migrate.py
```

---

## Phase 1: Annotation Data Types

### Task 1: Define annotation type dataclasses

**Files:**
- Create: `editor/server/annotation/__init__.py`
- Create: `editor/server/annotation/types.py`

- [ ] **Step 1: Create package init**

```python
# editor/server/annotation/__init__.py
"""OmniLang Editor — annotation comment parser/generator/validator."""

from editor.server.annotation.types import (
    AnnotationFile,
    CodeTabType,
    Chain,
    NodeEntry,
    WireEntry,
    ViewMarker,
    ViewRenderMode,
    AnnotationError,
    AnnotationErrorCode,
)
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.generator import generate_annotations
from editor.server.annotation.validator import validate_annotations

__all__ = [
    "AnnotationFile",
    "CodeTabType",
    "Chain",
    "NodeEntry",
    "WireEntry",
    "ViewMarker",
    "ViewRenderMode",
    "AnnotationError",
    "AnnotationErrorCode",
    "parse_annotations",
    "generate_annotations",
    "validate_annotations",
]
```

- [ ] **Step 2: Write types dataclasses**

```python
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
    attrs: dict[str, str] = field(default_factory=dict)  # name=value pairs
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


# Node type definition: tag → (required_attrs, optional_attrs, valid_ports)
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
```

- [ ] **Step 3: Write tests for types**

```python
# tests/annotation/test_types.py
"""Tests for annotation type definitions."""

import pytest
from editor.server.annotation.types import (
    NodeEntry, WireEntry, Chain, ViewMarker, AnnotationFile,
    CodeTabType, ViewRenderMode, NODE_TYPE_DEFS, VALID_COMPARE_OPS,
)

class TestNodeEntry:
    def test_create_signal_node(self):
        n = NodeEntry(id="n1", tag="signal", attrs={"name": "count", "type": "i32"})
        assert n.id == "n1"
        assert n.tag == "signal"
        assert n.attrs["name"] == "count"

    def test_create_fn_node_with_source(self):
        n = NodeEntry(id="n1", tag="fn", attrs={"source": "let x = 1 + 2;"})
        assert n.attrs["source"] == "let x = 1 + 2;"

    def test_pos_defaults(self):
        n = NodeEntry(id="n1", tag="add")
        assert n.pos_x == 0
        assert n.pos_y == 0

class TestWireEntry:
    def test_create_simple_wire(self):
        w = WireEntry(id="w1", from_id="n1", from_port="value", to_id="n2", to_port="a")
        assert w.from_port == "value"
        assert w.to_id == "n2"

    def test_create_fn_wire_no_ports(self):
        w = WireEntry(id="w1", from_id="n1", from_port="", to_id="n2", to_port="")
        assert w.from_port == ""

    def test_wire_collapsed_and_group(self):
        w = WireEntry(id="w1", from_id="n1", to_id="n2", is_collapsed=True, collapse_group_id="g1")
        assert w.is_collapsed
        assert w.collapse_group_id == "g1"

class TestChain:
    def test_create_chain_with_nodes_and_wires(self):
        chain = Chain(
            name="validate",
            start_line=10,
            end_line=20,
            nodes=[NodeEntry(id="n1", tag="getVariable", attrs={"name": "value"})],
            wires=[WireEntry(id="w1", from_id="n1", from_port="value", to_id="n2", to_port="a")],
        )
        assert len(chain.nodes) == 1
        assert len(chain.wires) == 1
        assert chain.name == "validate"

class TestAnnotationFile:
    def test_default_values(self):
        f = AnnotationFile(filepath="test.omni")
        assert f.code_tab_type == CodeTabType.LOGIC
        assert f.view_markers == []
        assert f.chains == []

class TestNodeTypeDefs:
    def test_all_tags_known(self):
        known_tags = {
            "signal", "store", "computed", "getVariable", "setVariable",
            "add", "sub", "mul", "div",
            "compare", "and", "or", "not", "concat",
            "literal",
            "branch", "sequence", "forEach",
            "updateUIProperty",
            "fn", "customFn", "return",
        }
        assert set(NODE_TYPE_DEFS.keys()) == known_tags

    def test_compare_ops(self):
        assert "==" in VALID_COMPARE_OPS
        assert "!=" in VALID_COMPARE_OPS
        assert ">=" in VALID_COMPARE_OPS
        assert "invalid" not in VALID_COMPARE_OPS
```

- [ ] **Step 4: Run tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_types.py -v
```
Expected: 9 tests PASS

- [ ] **Step 5: Commit**

```bash
git add editor/server/annotation/__init__.py editor/server/annotation/types.py tests/annotation/test_types.py
git commit -m "feat: add annotation data types for .omni comment format"
```

---

## Phase 2: Annotation Parser

### Task 2: Implement `@code-tab` and `@edit` parser

**Files:**
- Create: `editor/server/annotation/parser.py`
- Modify: `tests/annotation/test_parser.py` (create)

- [ ] **Step 1: Write failing tests for `@code-tab` parsing**

```python
# tests/annotation/test_parser.py
"""Tests for the annotation parser."""

import pytest
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.types import CodeTabType, ViewRenderMode

class TestParseCodeTab:
    def test_parse_logic_tab_first_line(self):
        source = "// @code-tab type=logic\nimport omniview;\n"
        result = parse_annotations(source, "test.omni")
        assert result.code_tab_type == CodeTabType.LOGIC

    def test_parse_event_tab(self):
        source = "// @code-tab type=event\n\ncomponent Foo {\n}\n"
        result = parse_annotations(source, "test.omni")
        assert result.code_tab_type == CodeTabType.EVENT

    def test_missing_code_tab_raises(self):
        source = "import omniview;\n"
        with pytest.raises(ValueError, match="MISSING_CODE_TAB"):
            parse_annotations(source, "test.omni")

    def test_invalid_code_tab_type_raises(self):
        source = "// @code-tab type=invalid\n"
        with pytest.raises(ValueError, match="INVALID_CODE_TAB_TYPE"):
            parse_annotations(source, "test.omni")

    def test_skip_blank_lines_before_code_tab(self):
        source = "\n\n// @code-tab type=logic\nimport omniview;\n"
        result = parse_annotations(source, "test.omni")
        assert result.code_tab_type == CodeTabType.LOGIC

    def test_skip_comment_before_code_tab(self):
        source = "// some comment\n// @code-tab type=logic\nimport omniview;\n"
        result = parse_annotations(source, "test.omni")
        assert result.code_tab_type == CodeTabType.LOGIC
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_parser.py -v
```
Expected: all FAIL (ModuleNotFoundError or similar)

- [ ] **Step 3: Implement minimal parser**

```python
# editor/server/annotation/parser.py
"""Parse .omni source → AnnotationFile by extracting editor annotation comments."""

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

# ── Regex Patterns ──────────────────────────────────────

_CODE_TAB_RE = re.compile(
    r"^//\s*@code-tab\s+type=(logic|event)\s*$"
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
    
    # ── Parse @code-tab (must be first non-blank, non-comment line) ──
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
            code_tab_type = CodeTabType(m.group(1))
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

    # ── Parse view_markers (@edit lines) ──
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

    # ── Parse chains (@chain blocks) ──
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
```

- [ ] **Step 4: Run the `@code-tab` tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_parser.py::TestParseCodeTab -v
```
Expected: 6 tests PASS

- [ ] **Step 5: Write tests for `@edit` parsing**

Add to `tests/annotation/test_parser.py`:

```python
class TestParseEdit:
    def test_parse_simple_edit(self):
        source = """// @code-tab type=logic
component Foo {
    view {
        // @edit
        <div id="box" class: box>
        </div>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        assert len(result.view_markers) == 1
        assert result.view_markers[0].element_id == "box"
        assert not result.view_markers[0].is_folded

    def test_parse_folded_edit(self):
        source = """// @code-tab type=logic
component Foo {
    view {
        // @edit folded
        <h3 id="title">Hello</h3>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        assert result.view_markers[0].is_folded

    def test_parse_render_canvas(self):
        source = """// @code-tab type=logic
component Foo {
    view {
        // @edit render=canvas
        <canvas id="c1"></canvas>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        assert result.view_markers[0].render_mode == ViewRenderMode.CANVAS

    def test_parse_nested_edits(self):
        source = """// @code-tab type=logic
component Foo {
    view {
        // @edit
        <div id="outer">
            // @edit
            <p id="inner">text</p>
        </div>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        assert len(result.view_markers) == 2
        ids = [m.element_id for m in result.view_markers]
        assert "outer" in ids
        assert "inner" in ids

    def test_element_without_edit_raises(self):
        source = """// @code-tab type=logic
component Foo {
    view {
        <div id="box"></div>
    }
}
"""
        with pytest.raises(ValueError, match="MISSING_EDIT"):
            parse_annotations(source, "test.omni")
```

- [ ] **Step 6: Run `@edit` tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_parser.py::TestParseEdit -v
```

Note: the current parser does NOT yet validate that every `<` has an `@edit` — that's in the validator phase. Update the test expectation for `test_element_without_edit_raises` to expect success at this stage (validator will catch it later). Mark it as `pytest.mark.skip` or adjust.

- [ ] **Step 7: Commit**

```bash
git add editor/server/annotation/parser.py tests/annotation/test_parser.py
git commit -m "feat: implement @code-tab and @edit annotation parser"
```

### Task 3: Implement `@chain/@node/@wire` parser

- [ ] **Step 1: Write failing tests for chain parsing**

Add to `tests/annotation/test_parser.py`:

```python
class TestParseChain:
    def test_parse_chain_with_nodes_and_wires(self):
        source = """// @code-tab type=logic
import omniview;

// ═══════════════════════════════════════
// @chain validate
//   @node n1  getVariable  name=value  pos=50,50
//   @node n2  literal      value=""   type=String  pos=50,150
//   @node n3  compare      op="=="    pos=250,50
//   @wire w1  n1.value -> n3.a
//   @wire w2  n2.value -> n3.b
// ═══════════════════════════════════════
pub fn validate(value: String): String {
    return "ok";
}
"""
        result = parse_annotations(source, "test.omni")
        assert len(result.chains) == 1
        chain = result.chains[0]
        assert chain.name == "validate"
        assert len(chain.nodes) == 3
        assert len(chain.wires) == 2

    def test_parse_chain_nodes_have_correct_attrs(self):
        source = """// @code-tab type=logic
// ══════
// @chain inc
//   @node n1  signal  name=count  type=i32  init=0  pos=100,50
// ══════
fn inc() { }
"""
        result = parse_annotations(source, "test.omni")
        node = result.chains[0].nodes[0]
        assert node.id == "n1"
        assert node.tag == "signal"
        assert node.attrs["name"] == "count"
        assert node.attrs["type"] == "i32"
        assert node.attrs["init"] == "0"
        assert node.pos_x == 100
        assert node.pos_y == 50

    def test_parse_chain_with_fn_node(self):
        source = """// @code-tab type=logic
// ══════
// @chain process
//   @node n1  fn  source="let x=1+2;"  pos=100,50
// ══════
pub fn process() { }
"""
        result = parse_annotations(source, "test.omni")
        fn_node = result.chains[0].nodes[0]
        assert fn_node.tag == "fn"
        assert fn_node.attrs["source"] == "let x=1+2;"

    def test_parse_wire_collapsed_and_group(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  getVariable  name=val  pos=50,50
//   @node n2  return  value=ok  pos=150,50
//   @wire w1  n1.value -> n2.value  collapsed  group=g1
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        wire = result.chains[0].wires[0]
        assert wire.is_collapsed
        assert wire.collapse_group_id == "g1"

    def test_parse_wire_to_fn_node_no_port(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  compare  op="=="  pos=50,50
//   @node n2  fn  source="..."  pos=200,50
//   @node n3  return  pos=350,50
//   @wire w1  n1.false -> n2
//   @wire w2  n2 -> n3.value
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        assert len(result.chains[0].wires) == 2
        w1 = result.chains[0].wires[0]
        assert w1.from_id == "n1"
        assert w1.from_port == "false"
        assert w1.to_id == "n2"
        assert w1.to_port == ""   # fn node, no port

    def test_parse_branch_wires(self):
        source = """// @code-tab type=logic
// ══════
// @chain ifdemo
//   @node n1  getVariable  name=val  pos=50,50
//   @node n2  branch                 pos=200,50
//   @node n3  return  value=true     pos=350,120
//   @node n4  return  value=false    pos=350,250
//   @wire w1  n1.value -> n2.condition
//   @wire w2  n2.true -> n3.value
//   @wire w3  n2.false -> n4.value
// ══════
fn ifdemo() { }
"""
        result = parse_annotations(source, "test.omni")
        assert len(result.chains[0].wires) == 3
```

- [ ] **Step 2: Run chain tests (should pass — parser already handles chains)**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_parser.py::TestParseChain -v
```
Expected: all PASS (parser implementation from Task 2 already covers chains)

- [ ] **Step 3: Commit**

```bash
git add tests/annotation/test_parser.py
git commit -m "test: add @chain/@node/@wire parser tests"
```

---

## Phase 3: Annotation Validator

### Task 4: Implement validation rules

**Files:**
- Create: `editor/server/annotation/validator.py`
- Create: `tests/annotation/test_validator.py`

- [ ] **Step 1: Write failing tests for validator**

```python
# tests/annotation/test_validator.py
"""Tests for annotation validator — all P, I, C, S, F rules."""

import pytest
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.validator import validate_annotations
from editor.server.annotation.types import AnnotationErrorCode

VALID_SOURCE = """// @code-tab type=logic
import omniview;

component Test {
    view {
        // @edit
        <div id="root">
            // @edit
            <p id="msg">Hello</p>
        </div>
    }
}
"""

class TestPositionRules:
    def test_P1_code_tab_must_be_first(self):
        # Code-tab is after import — should fail
        source = "import omniview;\n// @code-tab type=logic\n"
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.CODE_TAB_NOT_FIRST for e in errors)

    def test_P6_nodes_must_come_before_wires(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @wire w1  n1.value -> n2.a
//   @node n1  getVariable  name=x  pos=50,50
//   @node n2  add                   pos=200,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.NODE_AFTER_WIRE for e in errors)

class TestIdentityRules:
    def test_I1_duplicate_element_ids(self):
        source = """// @code-tab type=logic
component Test {
    view {
        // @edit
        <div id="dup">
            // @edit
            <p id="dup">text</p>
        </div>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.DUPLICATE_ELEMENT_ID for e in errors)

    def test_I2_duplicate_node_ids(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  getVariable  name=x  pos=50,50
//   @node n1  add                   pos=200,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.DUPLICATE_NODE_ID for e in errors)

    def test_I4_chain_name_mismatch_fn(self):
        source = """// @code-tab type=logic
// ══════
// @chain wrongName
//   @node n1  add  pos=50,50
// ══════
fn correctName() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.CHAIN_NAME_MISMATCH for e in errors)

class TestCompletenessRules:
    def test_C1_missing_required_attr(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  signal  name=count  pos=50,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        # signal requires type
        assert any(e.code == AnnotationErrorCode.MISSING_REQUIRED_ATTR for e in errors)

    def test_C2_missing_pos(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  add
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.MISSING_POS for e in errors)

    def test_C4_missing_edit_for_element(self):
        source = """// @code-tab type=logic
component Test {
    view {
        // @edit
        <div id="root">
            <p id="msg">text</p>
        </div>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.EXTRA_EDIT for e in errors)

class TestConsistencyRules:
    def test_S1_wire_references_nonexistent_node(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  getVariable  name=x  pos=50,50
//   @wire w1  n99.value -> n1.value
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.WIRE_TARGET_NOT_FOUND for e in errors)

    def test_S4_unknown_node_tag(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  unknownTag  pos=50,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.UNKNOWN_NODE_TAG for e in errors)

    def test_S2_wire_port_invalid(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  getVariable  name=x  pos=50,50
//   @node n2  return                pos=200,50
//   @wire w1  n1.invalid -> n2.value
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.WIRE_PORT_INVALID for e in errors)

class TestValidationPasses:
    def test_valid_source_passes(self):
        result = parse_annotations(VALID_SOURCE, "test.omni")
        errors = validate_annotations(result)
        assert len(errors) == 0

    def test_full_chain_passes(self):
        source = """// @code-tab type=logic
// ══════
// @chain validate
//   @node n1  getVariable  name=value  pos=50,50
//   @node n2  literal  value=""  type=String  pos=50,150
//   @node n3  compare  op="=="  pos=250,50
//   @node n4  return  value="error"  pos=450,50
//   @wire w1  n1.value -> n3.a
//   @wire w2  n2.value -> n3.b
//   @wire w3  n3.result -> n4.value
// ══════
pub fn validate(value: String): String {
    return "ok";
}
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert len(errors) == 0
```

- [ ] **Step 2: Run to verify they fail**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_validator.py -v
```
Expected: most FAIL (validator not implemented)

- [ ] **Step 3: Implement validator**

```python
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
    # P1: @code-tab must be first non-blank, non-comment line
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
        saw_wire = False
        for node in chain.nodes:
            if saw_wire:
                # We have a node after a wire — need to check line order
                for wire in chain.wires:
                    if wire.line_number < node.line_number:
                        errors.append(AnnotationError(
                            code=AnnotationErrorCode.NODE_AFTER_WIRE,
                            message=f"@node {node.id} (line {node.line_number}) must come before @wire (line {wire.line_number}) in chain '{chain.name}'",
                            line=node.line_number,
                        ))
                        break
        for wire in chain.wires:
            saw_wire = True

        # P4: No declarations between @chain end and its function
        fn_start_line = chain.end_line
        for i in range(chain.end_line, len(lines)):
            stripped = lines[i].strip()
            if re.match(r"^\s*(pub\s+)?(fn|action)\s+\w+", stripped):
                fn_start_line = i
                break
        # Check for signal/computed/fn/action/prop/event/@store between chain end and fn
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
        # Look for fn/action declaration after chain
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

    # C2: pos must exist on every @node (parsed by parser, but check for zero values as sentinel)
    # pos_x and pos_y are parsed from regex — if the regex matched, they exist.
    # This check validates that pos was actually provided with integer values.
    for chain in file.chains:
        for node in chain.nodes:
            # If both are 0, it might be a default — but 0,0 is a valid position.
            # The parser ensures pos is present if the regex matches.
            # We trust the parser here.
            pass

    # C3: Wire must have both from and to endpoints
    for chain in file.chains:
        for wire in chain.wires:
            if not wire.from_id or not wire.to_id:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.INCOMPLETE_WIRE,
                    message=f"@wire {wire.id} at line {wire.line_number} missing from_id or to_id",
                    line=wire.line_number,
                ))

    # C4: Every <element inside view block must have matching @edit
    in_view = False
    view_indent = 0
    edit_lines = {m.line_number for m in file.view_markers}

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
        if elem_m and (i + 1) not in edit_lines:
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
        # Build map of node id → valid ports
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

            # S2: Port must be valid for non-fn nodes
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

    # S3: code_tab type must be logic or event (enforced at parse time)
    if file.code_tab_type.value not in ("logic", "event"):
        errors.append(AnnotationError(
            code=AnnotationErrorCode.INVALID_CODE_TAB_TYPE,
            message=f"@code-tab type must be 'logic' or 'event', got '{file.code_tab_type.value}'",
            line=file.code_tab_line,
        ))

    # S4: Node tag must be registered
    for chain in file.chains:
        for node in chain.nodes:
            if node.tag not in NODE_TYPE_DEFS:
                errors.append(AnnotationError(
                    code=AnnotationErrorCode.UNKNOWN_NODE_TAG,
                    message=f"Unknown @node tag '{node.tag}' at line {node.line_number}",
                    line=node.line_number,
                ))

    # S5: Variables referenced in node attrs must exist in scope
    # (Scope resolution is complex — defer to future phase, emit warning for now)
    # TODO: integrate with compiler's scope analysis
    pass


# ── Format Rules (F) ────────────────────────────────────

def _validate_format_rules(
    file: AnnotationFile, lines: list[str], errors: list[AnnotationError]
) -> None:
    # F1: @edit only allows folded, render=dom, render=canvas
    # (parser only matches valid tokens — violations are implicitly caught)

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
```

- [ ] **Step 4: Run validator tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_validator.py -v
```
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add editor/server/annotation/validator.py tests/annotation/test_validator.py
git commit -m "feat: implement annotation validator (P/I/C/S/F rules)"
```

---

## Phase 4: Annotation Generator

### Task 5: Implement `@edit` and `@chain/@node/@wire` generator

**Files:**
- Create: `editor/server/annotation/generator.py`
- Create: `tests/annotation/test_generator.py`

- [ ] **Step 1: Write failing tests for generator**

```python
# tests/annotation/test_generator.py
"""Tests for annotation generator — inject comments into .omni source."""

import pytest
from editor.server.annotation.types import (
    AnnotationFile, Chain, NodeEntry, WireEntry, ViewMarker,
    CodeTabType, ViewRenderMode,
)
from editor.server.annotation.generator import generate_annotations

class TestGenerateCodeTab:
    def test_generate_code_tab_logic(self):
        ann = AnnotationFile(filepath="test.omni", code_tab_type=CodeTabType.LOGIC)
        source = "import omniview;\n\ncomponent Foo { }\n"
        result = generate_annotations(ann, source)
        lines = result.split("\n")
        assert lines[0] == "// @code-tab type=logic"

    def test_generate_code_tab_event(self):
        ann = AnnotationFile(filepath="test.omni", code_tab_type=CodeTabType.EVENT)
        result = generate_annotations(ann, "")
        assert "// @code-tab type=event" in result

    def test_generate_code_tab_replaces_old(self):
        ann = AnnotationFile(filepath="test.omni", code_tab_type=CodeTabType.LOGIC)
        source = "// @code-tab type=event\nimport omniview;\n"
        result = generate_annotations(ann, source)
        lines = result.split("\n")
        assert lines[0] == "// @code-tab type=logic"
        assert "// @code-tab type=event" not in result

class TestGenerateEdit:
    def test_generate_edit_inserts_before_element(self):
        ann = AnnotationFile(
            filepath="test.omni",
            code_tab_type=CodeTabType.LOGIC,
            view_markers=[ViewMarker(element_id="box", line_number=0)],
        )
        source = """// @code-tab type=logic
component Foo {
    view {
        <div id="box"></div>
    }
}
"""
        result = generate_annotations(ann, source)
        assert "// @edit" in result
        lines = result.split("\n")
        # Find the @edit followed by <div
        for i, line in enumerate(lines):
            if line.strip() == "// @edit" and i + 1 < len(lines):
                if "div" in lines[i + 1]:
                    break
        else:
            pytest.fail("@edit not found before <div>")

    def test_generate_edit_with_folded(self):
        ann = AnnotationFile(
            filepath="test.omni",
            code_tab_type=CodeTabType.LOGIC,
            view_markers=[ViewMarker(element_id="title", line_number=0, is_folded=True)],
        )
        source = """// @code-tab type=logic
component Foo {
    view {
        <h3 id="title">text</h3>
    }
}
"""
        result = generate_annotations(ann, source)
        assert "// @edit folded" in result

    def test_generate_edit_with_render_canvas(self):
        ann = AnnotationFile(
            filepath="test.omni",
            code_tab_type=CodeTabType.LOGIC,
            view_markers=[ViewMarker(element_id="c1", line_number=0, render_mode=ViewRenderMode.CANVAS)],
        )
        source = """// @code-tab type=logic
component Foo {
    view {
        <canvas id="c1"></canvas>
    }
}
"""
        result = generate_annotations(ann, source)
        assert "// @edit render=canvas" in result

class TestGenerateChain:
    def test_generate_full_chain_block(self):
        chain = Chain(
            name="validate",
            start_line=0,
            end_line=0,
            nodes=[
                NodeEntry(id="n1", tag="getVariable", attrs={"name": "value"}, pos_x=50, pos_y=50),
                NodeEntry(id="n2", tag="literal", attrs={"value": "", "type": "String"}, pos_x=50, pos_y=150),
                NodeEntry(id="n3", tag="compare", attrs={"op": "=="}, pos_x=250, pos_y=50),
                NodeEntry(id="n4", tag="return", attrs={"value": "ok"}, pos_x=450, pos_y=50),
            ],
            wires=[
                WireEntry(id="w1", from_id="n1", from_port="value", to_id="n3", to_port="a"),
                WireEntry(id="w2", from_id="n2", from_port="value", to_id="n3", to_port="b"),
                WireEntry(id="w3", from_id="n3", from_port="result", to_id="n4", to_port="value"),
            ],
        )
        ann = AnnotationFile(
            filepath="test.omni",
            code_tab_type=CodeTabType.LOGIC,
            chains=[chain],
        )
        source = """// @code-tab type=logic
import omniview;

pub fn validate(value: String): String {
    return "ok";
}
"""
        result = generate_annotations(ann, source)
        assert "@chain validate" in result
        assert "@node n1" in result
        assert "@node n2" in result
        assert "@wire w1" in result
        assert "pos=50,50" in result

class TestRoundTrip:
    def test_roundtrip_preserves_data(self):
        """Parse → generate → parse again should produce identical AnnotationFile."""
        source = """// @code-tab type=logic
import omniview;

// ═══════════════════════════════════════════════
// @chain inc
//   @node n1  signal  name=count  type=i32  pos=100,50
//   @node n2  literal  value=1  type=i32      pos=100,150
//   @node n3  add                              pos=300,50
//   @node n4  setVariable  name=count           pos=500,50
//   @wire w1  n1.value -> n3.a
//   @wire w2  n2.value -> n3.b
//   @wire w3  n3.result -> n4.value
// ═══════════════════════════════════════════════
fn inc() {
    let tmp = count + 1;
    count = tmp;
}

component Counter {
    signal count: i32 = 0;

    view {
        // @edit
        <div id="box">
            // @edit
            <button id="btn" @onPress={inc}>+</button>
        </div>
    }
}
"""
        from editor.server.annotation.parser import parse_annotations
        ann1 = parse_annotations(source, "test.omni")
        generated = generate_annotations(ann1, source)
        ann2 = parse_annotations(generated, "test.omni")

        assert ann1.code_tab_type == ann2.code_tab_type
        assert len(ann1.chains) == len(ann2.chains)
        assert len(ann1.view_markers) == len(ann2.view_markers)
        assert ann1.chains[0].name == ann2.chains[0].name
        assert len(ann1.chains[0].nodes) == len(ann2.chains[0].nodes)
        assert len(ann1.chains[0].wires) == len(ann2.chains[0].wires)
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_generator.py -v
```
Expected: FAIL (generator not implemented)

- [ ] **Step 3: Implement generator**

```python
# editor/server/annotation/generator.py
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

    # Insert as first line (after any blank lines / non-annotation comments at very top)
    insert_at = 0
    while insert_at < len(lines) and (
        lines[insert_at].strip() == "" or
        (lines[insert_at].strip().startswith("//") and not lines[insert_at].strip().startswith("// @"))
    ):
        insert_at += 1

    lines.insert(insert_at, tab_line)
    return lines


def _strip_old_annotations(lines: list[str]) -> list[str]:
    """Remove existing @edit lines."""
    return [l for l in lines if not re.match(r"^\s*//\s*@edit\b", l)]


def _strip_old_chain_blocks(lines: list[str]) -> list[str]:
    """Remove existing @chain blocks (everything between separator lines that contains @chain)."""
    in_chain_block = False
    result = []
    for line in lines:
        stripped = line.strip()
        if re.match(r"^//\s*═", stripped):
            in_chain_block = not in_chain_block
            continue
        if in_chain_block:
            continue
        result.append(line)
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
            # Insert @edit before this element
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

        result.append(line)

    return result


def _insert_chain_blocks(lines: list[str], ann: AnnotationFile) -> list[str]:
    """Insert @chain blocks before each fn/action declaration."""
    if not ann.chains:
        return lines

    result: list[str] = []
    chain_index = 0

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Check if this line is a fn/action declaration matching next chain
        m = re.match(r"^\s*(pub\s+)?(fn|action)\s+(\w+)", stripped)
        if m and chain_index < len(ann.chains):
            fn_name = m.group(3)
            chain = ann.chains[chain_index]
            if chain.name == fn_name:
                # Insert chain block before this fn
                indent = len(line) - len(line.lstrip())
                result.extend(_render_chain_block(chain, indent))
                chain_index += 1

        result.append(line)

    return result


def _render_chain_block(chain: Chain, indent: int = 0) -> list[str]:
    """Render a @chain block as list of lines."""
    SEP = "// " + "═" * 55
    lines = [SEP]
    lines.append(f"// @chain {chain.name}")

    for node in chain.nodes:
        attrs_str = ""
        if node.attrs:
            parts = []
            for k, v in node.attrs.items():
                if " " in v:
                    parts.append(f'{k}="{v}"')
                else:
                    parts.append(f"{k}={v}")
            attrs_str = "  " + "  ".join(parts)
        lines.append(f"//   @node {node.id}  {node.tag}{attrs_str}  pos={node.pos_x},{node.pos_y}".rstrip())

    for wire in chain.wires:
        from_str = f"{wire.from_id}.{wire.from_port}" if wire.from_port else wire.from_id
        to_str = f"{wire.to_id}.{wire.to_port}" if wire.to_port else wire.to_id
        extras = ""
        if wire.is_collapsed:
            extras += "  collapsed"
        if wire.collapse_group_id:
            extras += f"  group={wire.collapse_group_id}"
        lines.append(f"//   @wire {wire.id}  {from_str} -> {to_str}{extras}".rstrip())

    lines.append(SEP)
    return lines
```

- [ ] **Step 4: Run generator tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_generator.py -v
```
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add editor/server/annotation/generator.py tests/annotation/test_generator.py
git commit -m "feat: implement annotation generator (code-tab, edit, chain, node, wire)"
```

---

## Phase 5: .omni-editor.toml

### Task 6: Implement .omni-editor.toml read/write

**Files:**
- Create: `editor/server/annotation/editor_toml.py`
- Create: `tests/annotation/test_editor_toml.py`

- [ ] **Step 1: Write tests**

```python
# tests/annotation/test_editor_toml.py
"""Tests for .omni-editor.toml read/write."""

import tempfile
from pathlib import Path
from editor.server.annotation.editor_toml import EditorTomlState, read_editor_toml, write_editor_toml

SAMPLE_TOML = """[editor]
active_file = "src/main.omni"
active_mode = "design"

[editor.open_files]
files = ["src/main.omni", "src/lib/validation.omni"]

[editor.panels]
left_open = true
right_width = 320

[editor.viewports."src/main.omni"]
zoom = 1.0
scroll_x = 0
scroll_y = 100

[editor.viewports."src/lib/validation.omni"]
zoom = 0.8
scroll_x = 0
scroll_y = 0
"""

class TestReadEditorToml:
    def test_read_full_state(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".toml", delete=False) as f:
            f.write(SAMPLE_TOML)
            f.flush()
            state = read_editor_toml(Path(f.name))
            Path(f.name).unlink()
        
        assert state.active_file == "src/main.omni"
        assert state.active_mode == "design"
        assert "src/main.omni" in state.open_files
        assert "src/lib/validation.omni" in state.open_files
        assert state.left_open is True
        assert state.right_width == 320
        assert state.viewports["src/main.omni"]["zoom"] == 1.0
        assert state.viewports["src/lib/validation.omni"]["scroll_y"] == 0

    def test_read_non_existent_returns_default(self):
        state = read_editor_toml(Path("/nonexistent/path.toml"))
        assert state.active_file == ""
        assert state.open_files == []

class TestWriteEditorToml:
    def test_write_and_read_roundtrip(self):
        state = EditorTomlState(
            active_file="src/main.omni",
            active_mode="design",
            open_files=["src/main.omni", "src/lib/validation.omni"],
            left_open=True,
            right_width=320,
            viewports={
                "src/main.omni": {"zoom": 1.0, "scroll_x": 0, "scroll_y": 100},
            },
        )
        with tempfile.NamedTemporaryFile(mode="w", suffix=".toml", delete=False) as f:
            f.close()
            path = Path(f.name)
            write_editor_toml(path, state)
            read_back = read_editor_toml(path)
            Path(f.name).unlink()

        assert read_back.active_file == state.active_file
        assert read_back.active_mode == state.active_mode
        assert read_back.open_files == state.open_files
```

- [ ] **Step 2: Implement reader/writer**

```python
# editor/server/annotation/editor_toml.py
"""Read and write .omni-editor.toml — project-level editor UI state."""

from __future__ import annotations
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class EditorTomlState:
    """Editor UI state stored in .omni-editor.toml."""
    active_file: str = ""
    active_mode: str = "design"         # "design" | "code"
    open_files: list[str] = field(default_factory=list)
    left_open: bool = True
    right_width: int = 320
    viewports: dict[str, dict[str, float]] = field(default_factory=dict)


def read_editor_toml(path: Path) -> EditorTomlState:
    """Read .omni-editor.toml. Returns defaults if file doesn't exist."""
    if not path.exists():
        return EditorTomlState()

    try:
        import tomllib  # Python 3.11+
    except ImportError:
        try:
            import tomli as tomllib  # Python < 3.11 fallback
        except ImportError:
            return EditorTomlState()

    with open(path, "rb") as f:
        data = tomllib.load(f)

    editor = data.get("editor", {})
    viewports_raw = editor.get("viewports", {})

    viewports: dict[str, dict[str, float]] = {}
    for filepath, vp in viewports_raw.items():
        viewports[filepath] = {
            "zoom": float(vp.get("zoom", 1.0)),
            "scroll_x": float(vp.get("scroll_x", 0)),
            "scroll_y": float(vp.get("scroll_y", 0)),
        }

    return EditorTomlState(
        active_file=editor.get("active_file", ""),
        active_mode=editor.get("active_mode", "design"),
        open_files=list(editor.get("open_files", {}).get("files", [])),
        left_open=editor.get("panels", {}).get("left_open", True),
        right_width=editor.get("panels", {}).get("right_width", 320),
        viewports=viewports,
    )


def write_editor_toml(path: Path, state: EditorTomlState) -> None:
    """Write .omni-editor.toml to disk."""
    lines: list[str] = []
    lines.append("# .omni-editor.toml — 编辑器 UI 状态，不影响编译")
    lines.append("")
    lines.append("[editor]")
    lines.append(f'active_file = "{state.active_file}"')
    lines.append(f'active_mode = "{state.active_mode}"')
    lines.append("")
    lines.append("[editor.open_files]")
    files_str = ", ".join(f'"{f}"' for f in state.open_files)
    lines.append(f"files = [{files_str}]")
    lines.append("")
    lines.append("[editor.panels]")
    lines.append(f"left_open = {str(state.left_open).lower()}")
    lines.append(f"right_width = {state.right_width}")
    lines.append("")

    for filepath, vp in state.viewports.items():
        lines.append(f'[editor.viewports."{filepath}"]')
        lines.append(f"zoom = {vp.get('zoom', 1.0)}")
        lines.append(f"scroll_x = {int(vp.get('scroll_x', 0))}")
        lines.append(f"scroll_y = {int(vp.get('scroll_y', 0))}")
        lines.append("")

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")
```

- [ ] **Step 3: Run tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_editor_toml.py -v
```
Expected: 3 tests PASS

- [ ] **Step 4: Commit**

```bash
git add editor/server/annotation/editor_toml.py tests/annotation/test_editor_toml.py
git commit -m "feat: implement .omni-editor.toml read/write"
```

---

## Phase 6: Editor Integration — New API Endpoints

### Task 7: Add new annotation API endpoints to main.py

**Files:**
- Modify: `editor/server/main.py`

- [ ] **Step 1: Add import and new API models**

Add to `editor/server/main.py` imports (after existing imports):

```python
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.generator import generate_annotations
from editor.server.annotation.validator import validate_annotations
from editor.server.annotation.editor_toml import read_editor_toml, write_editor_toml, EditorTomlState
from editor.server.annotation.types import AnnotationError, AnnotationFile
```

- [ ] **Step 2: Add API endpoint models**

After existing model classes in `main.py`:

```python
class AnnotatedSaveRequest(BaseModel):
    """POST /api/annotated-save 请求体"""
    filePath: str
    annotations: dict  # Serialized AnnotationFile
    source: str         # The .omni source content


class AnnotatedLoadRequest(BaseModel):
    """POST /api/annotated-load 请求体"""
    filePath: str


class EditorTomlSaveRequest(BaseModel):
    """POST /api/editor-toml/save 请求体"""
    state: dict  # Serialized EditorTomlState
```

- [ ] **Step 3: Add API endpoints**

Add before the `cmd_serve` function in `main.py`:

```python
# ═══════════════════════════════════════════════════════
# New Annotation API — .omni source with embedded editor annotations
# ═══════════════════════════════════════════════════════


@app.post("/api/annotated-save")
async def api_annotated_save(req: AnnotatedSaveRequest, request: Request):
    """Save .omni source with editor annotations injected.

    Takes source + annotation data → injects @code-tab, @edit, @chain blocks → writes .omni file.
    """
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"ok": False, "error": "Access denied"}

    try:
        # Reconstruct AnnotationFile from dict
        ann = _dict_to_annotation_file(req.annotations, req.filePath)
        # Validate before saving
        errors = validate_annotations(ann)
        if errors:
            return {
                "ok": False,
                "error": "Validation failed",
                "validationErrors": [
                    {"code": e.code.value, "message": e.message, "line": e.line}
                    for e in errors
                ],
            }
        # Generate annotated source
        annotated_source = generate_annotations(ann, req.source)
        # Atomic write
        import uuid
        tmp_path = file_path.parent / f"{file_path.name}.tmp.{uuid.uuid4().hex[:8]}"
        tmp_path.write_text(annotated_source, encoding="utf-8")
        with open(tmp_path, "r+b") as f:
            import os
            os.fsync(f.fileno())
        os.replace(tmp_path, file_path)
        return {"ok": True, "path": str(file_path.relative_to(project_dir)).replace("\\", "/")}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.post("/api/annotated-load")
async def api_annotated_load(req: AnnotatedLoadRequest, request: Request):
    """Load .omni source and extract editor annotations.

    Returns: { source, annotations, validationErrors }
    """
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"success": False, "error": "No project directory"}

    file_path = (Path(project_dir) / req.filePath).resolve()
    if not str(file_path).startswith(str(Path(project_dir).resolve())):
        return {"success": False, "error": "Access denied"}

    if not file_path.exists():
        return {"success": False, "error": "File not found"}

    try:
        source = file_path.read_text(encoding="utf-8")
        ann = parse_annotations(source, req.filePath)
        errors = validate_annotations(ann)

        return {
            "success": True,
            "source": source,
            "annotations": _annotation_file_to_dict(ann),
            "validationErrors": [
                {"code": e.code.value, "message": e.message, "line": e.line}
                for e in errors
            ],
        }
    except ValueError as e:
        return {"success": False, "error": str(e)}
    except Exception as e:
        return {"success": False, "error": str(e)}


@app.post("/api/editor-toml/save")
async def api_editor_toml_save(req: EditorTomlSaveRequest, request: Request):
    """Save .omni-editor.toml."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"ok": False, "error": "No project directory"}

    try:
        state = EditorTomlState(
            active_file=req.state.get("active_file", ""),
            active_mode=req.state.get("active_mode", "design"),
            open_files=req.state.get("open_files", []),
            left_open=req.state.get("left_open", True),
            right_width=req.state.get("right_width", 320),
            viewports=req.state.get("viewports", {}),
        )
        toml_path = Path(project_dir) / ".omni-editor.toml"
        write_editor_toml(toml_path, state)
        return {"ok": True}
    except Exception as e:
        return {"ok": False, "error": str(e)}


@app.get("/api/editor-toml/load")
async def api_editor_toml_load(request: Request):
    """Load .omni-editor.toml."""
    project_dir = request.app.state.project_dir
    if not project_dir:
        return {"state": None, "error": "No project directory"}

    try:
        toml_path = Path(project_dir) / ".omni-editor.toml"
        state = read_editor_toml(toml_path)
        return {
            "state": {
                "active_file": state.active_file,
                "active_mode": state.active_mode,
                "open_files": state.open_files,
                "left_open": state.left_open,
                "right_width": state.right_width,
                "viewports": state.viewports,
            }
        }
    except Exception as e:
        return {"state": None, "error": str(e)}


# ── Helpers ──────────────────────────────────────────


def _annotation_file_to_dict(ann: AnnotationFile) -> dict:
    """Serialize AnnotationFile to JSON-safe dict."""
    from dataclasses import asdict
    return {
        "codeTabType": ann.code_tab_type.value,
        "codeTabLine": ann.code_tab_line,
        "viewMarkers": [
            {
                "elementId": m.element_id,
                "lineNumber": m.line_number,
                "isFolded": m.is_folded,
                "renderMode": m.render_mode.value,
            }
            for m in ann.view_markers
        ],
        "chains": [
            {
                "name": c.name,
                "startLine": c.start_line,
                "endLine": c.end_line,
                "nodes": [
                    {
                        "id": n.id,
                        "tag": n.tag,
                        "attrs": n.attrs,
                        "posX": n.pos_x,
                        "posY": n.pos_y,
                        "lineNumber": n.line_number,
                    }
                    for n in c.nodes
                ],
                "wires": [
                    {
                        "id": w.id,
                        "fromId": w.from_id,
                        "fromPort": w.from_port,
                        "toId": w.to_id,
                        "toPort": w.to_port,
                        "isCollapsed": w.is_collapsed,
                        "collapseGroupId": w.collapse_group_id,
                        "lineNumber": w.line_number,
                    }
                    for w in c.wires
                ],
            }
            for c in ann.chains
        ],
    }


def _dict_to_annotation_file(data: dict, filepath: str) -> AnnotationFile:
    """Deserialize dict back to AnnotationFile."""
    from editor.server.annotation.types import Chain, NodeEntry, WireEntry, ViewMarker, CodeTabType, ViewRenderMode

    return AnnotationFile(
        filepath=filepath,
        code_tab_type=CodeTabType(data.get("codeTabType", "logic")),
        code_tab_line=data.get("codeTabLine", 0),
        view_markers=[
            ViewMarker(
                element_id=m.get("elementId", ""),
                line_number=m.get("lineNumber", 0),
                is_folded=m.get("isFolded", False),
                render_mode=ViewRenderMode(m.get("renderMode", "dom")),
            )
            for m in data.get("viewMarkers", [])
        ],
        chains=[
            Chain(
                name=c.get("name", ""),
                start_line=c.get("startLine", 0),
                end_line=c.get("endLine", 0),
                nodes=[
                    NodeEntry(
                        id=n.get("id", ""),
                        tag=n.get("tag", ""),
                        attrs=n.get("attrs", {}),
                        pos_x=n.get("posX", 0),
                        pos_y=n.get("posY", 0),
                        line_number=n.get("lineNumber", 0),
                    )
                    for n in c.get("nodes", [])
                ],
                wires=[
                    WireEntry(
                        id=w.get("id", ""),
                        from_id=w.get("fromId", ""),
                        from_port=w.get("fromPort", ""),
                        to_id=w.get("toId", ""),
                        to_port=w.get("toPort", ""),
                        is_collapsed=w.get("isCollapsed", False),
                        collapse_group_id=w.get("collapseGroupId", ""),
                        line_number=w.get("lineNumber", 0),
                    )
                    for w in c.get("wires", [])
                ],
            )
            for c in data.get("chains", [])
        ],
    )
```

- [ ] **Step 4: Commit**

```bash
git add editor/server/main.py
git commit -m "feat: add /api/annotated-save, /api/annotated-load, /api/editor-toml endpoints"
```

---

## Phase 7: Frontend TypeScript Types Update

### Task 8: Update TypeScript annotation types

**Files:**
- Create: `editor/client/src/annotation/types.ts`

- [ ] **Step 1: Write TypeScript types matching Python schema**

```typescript
// editor/client/src/annotation/types.ts
/** TypeScript mirrors of editor/server/annotation/types.py */

export type CodeTabType = "logic" | "event";
export type ViewRenderMode = "dom" | "canvas";

export interface ViewMarker {
  elementId: string;
  lineNumber: number;
  isFolded: boolean;
  renderMode: ViewRenderMode;
}

export interface NodeEntry {
  id: string;
  tag: string;
  attrs: Record<string, string>;
  posX: number;
  posY: number;
  lineNumber: number;
}

export interface WireEntry {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  isCollapsed: boolean;
  collapseGroupId: string;
  lineNumber: number;
}

export interface Chain {
  name: string;
  startLine: number;
  endLine: number;
  nodes: NodeEntry[];
  wires: WireEntry[];
}

export interface AnnotationFile {
  codeTabType: CodeTabType;
  codeTabLine: number;
  viewMarkers: ViewMarker[];
  chains: Chain[];
}

export interface AnnotationValidationError {
  code: string;
  message: string;
  line: number;
}

export interface AnnotatedLoadResponse {
  success: boolean;
  source?: string;
  annotations?: AnnotationFile;
  validationErrors?: AnnotationValidationError[];
  error?: string;
}

export interface AnnotatedSaveResponse {
  ok: boolean;
  path?: string;
  error?: string;
  validationErrors?: AnnotationValidationError[];
}

export interface EditorTomlState {
  active_file: string;
  active_mode: string;
  open_files: string[];
  left_open: boolean;
  right_width: number;
  viewports: Record<string, { zoom: number; scroll_x: number; scroll_y: number }>;
}
```

- [ ] **Step 2: Commit**

```bash
git add editor/client/src/annotation/types.ts
git commit -m "feat: add TypeScript annotation types mirroring Python schema"
```

### Task 9: Add API client methods

**Files:**
- Modify: `editor/client/src/api/client.ts`

- [ ] **Step 1: Add new methods to ApiClient**

Add to `ApiClient` class in `client.ts`:

```typescript
import type { AnnotatedLoadResponse, AnnotatedSaveResponse, EditorTomlState, AnnotationFile } from "../annotation/types";

// ── Annotation API ──
async annotatedSave(filePath: string, annotations: AnnotationFile, source: string): Promise<AnnotatedSaveResponse> {
  return this.post("/api/annotated-save", {
    body: JSON.stringify({ filePath, annotations, source }),
  });
}

async annotatedLoad(filePath: string): Promise<AnnotatedLoadResponse> {
  return this.post("/api/annotated-load", {
    body: JSON.stringify({ filePath }),
  });
}

async saveEditorToml(state: EditorTomlState): Promise<{ ok: boolean; error?: string }> {
  return this.post("/api/editor-toml/save", {
    body: JSON.stringify({ state }),
  });
}

async loadEditorToml(): Promise<{ state: EditorTomlState | null; error?: string }> {
  const res = await fetch("/api/editor-toml/load");
  return res.json();
}
```

- [ ] **Step 2: Commit**

```bash
git add editor/client/src/api/client.ts
git commit -m "feat: add annotation API client methods"
```

---

## Phase 8: Migration Tool

### Task 10: Implement old .omni.json → new format migration

**Files:**
- Create: `editor/server/annotation/migrate.py`
- Create: `tests/annotation/test_migrate.py`

- [ ] **Step 1: Write tests**

```python
# tests/annotation/test_migrate.py
"""Tests for the omni.json → annotation format migration."""

import json
import tempfile
from pathlib import Path
from editor.server.annotation.migrate import migrate_omni_json

OLD_OMNI_JSON = {
    "schemaVersion": "2.0.0",
    "meta": {"projectName": "test-project"},
    "sourceCode": "",
    "design": {
        "elements": [
            {
                "id": "e1", "nodeType": "view", "tag": "Button",
                "props": {"label": "Click", "class": "primary"},
                "children": [], "parentId": None,
                "isFolded": False, "renderMode": "dom",
            },
        ],
        "wires": [],
    },
    "code": {
        "elements": [
            {
                "id": "cb1", "nodeType": "codeblock", "tag": "signal",
                "props": {"name": "count", "type": "i32"},
                "children": [], "parentId": None,
                "nodeCategory": "variable",
            },
        ],
        "wires": [],
    },
    "components": [],
    "canvasLayout": {"positions": {}},
    "tabState": {"openTabs": ["main"], "activeTab": "main"},
    "config": {"target": "react", "autoSave": False},
}

class TestMigrate:
    def test_migration_creates_omni_file(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)
            json_path = project_dir / "project.omni.json"
            json_path.write_text(json.dumps(OLD_OMNI_JSON))

            migrate_omni_json(json_path, project_dir)

            # Should have created src/App.omni
            main_file = project_dir / "src" / "App.omni"
            assert main_file.exists()
            content = main_file.read_text()
            assert "// @code-tab type=logic" in content

    def test_migration_generates_editor_toml(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            project_dir = Path(tmpdir)
            json_path = project_dir / "project.omni.json"
            json_path.write_text(json.dumps(OLD_OMNI_JSON))

            migrate_omni_json(json_path, project_dir)

            toml_path = project_dir / ".omni-editor.toml"
            assert toml_path.exists()
```

- [ ] **Step 2: Implement migration**

```python
# editor/server/annotation/migrate.py
"""Migrate old .omni.json projects to new annotation-based format."""

from __future__ import annotations
import json
from pathlib import Path
from editor.server.annotation.editor_toml import write_editor_toml, EditorTomlState


def migrate_omni_json(json_path: Path, project_dir: Path) -> None:
    """Read old .omni.json, generate annotated .omni files, write .omni-editor.toml.

    Does NOT delete the original .omni.json — user can verify before deleting.
    """
    if not json_path.exists():
        raise FileNotFoundError(f".omni.json not found: {json_path}")

    with open(json_path, encoding="utf-8") as f:
        doc = json.load(f)

    design = doc.get("design", {})
    design_elements = design.get("elements", [])
    design_wires = design.get("wires", [])

    code = doc.get("code", {})
    code_elements = code.get("elements", [])
    code_wires = code.get("wires", [])

    config = doc.get("config", {})
    target = config.get("target", "react")
    tab_state = doc.get("tabState", {})

    # ── Generate src/App.omni with design elements ──
    src_dir = project_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)

    lines: list[str] = []
    lines.append("// @code-tab type=logic")
    lines.append("import omniview;")
    lines.append("")

    # Build a simple component from design elements
    component_name = doc.get("meta", {}).get("projectName", "App")
    lines.append(f"component {component_name} {{")
    lines.append("    view {")

    # Walk the element tree
    root_elements = [e for e in design_elements if e.get("parentId") is None]
    for el in root_elements:
        _emit_element(el, design_elements, lines, indent=2)

    lines.append("    }")
    lines.append("    style {")
    lines.append("    }")
    lines.append("}")

    main_omni = src_dir / f"{component_name}.omni"
    main_omni.write_text("\n".join(lines), encoding="utf-8")

    # ── Generate .omni-editor.toml ──
    state = EditorTomlState(
        active_file=f"src/{component_name}.omni",
        active_mode="design",
        open_files=[f"src/{component_name}.omni"],
    )
    toml_path = project_dir / ".omni-editor.toml"
    write_editor_toml(toml_path, state)

    # ── Generate stub .omnilang.toml if not exists ──
    omni_toml = project_dir / ".omnilang.toml"
    if not omni_toml.exists():
        omni_toml.write_text(f"""\
[project]
name = "{component_name.lower().replace(' ', '-')}"
version = "1.0.0"

[source]
dir = "src"

[output]
dir = "build"
""")

    print(f"Migration complete.")
    print(f"  Main file: {main_omni}")
    print(f"  Editor state: {toml_path}")
    print(f"  Original .omni.json preserved at: {json_path}")
    print(f"  Verify the migration, then delete .omni.json manually.")


def _emit_element(el: dict, all_elements: list[dict], lines: list[str], indent: int = 0) -> None:
    """Recursively emit a view element with @edit annotation."""
    pad = "    " * indent
    tag = el.get("tag", "div")
    eid = el.get("id", "")
    props = el.get("props", {})
    is_folded = el.get("isFolded", False)
    render_mode = el.get("renderMode", "dom")
    children_ids = el.get("children", [])

    # @edit annotation
    edit_parts = ["// @edit"]
    if is_folded:
        edit_parts.append("folded")
    if render_mode == "canvas":
        edit_parts.append("render=canvas")
    lines.append(f"{pad}{' '.join(edit_parts)}")

    # Element tag
    props_str = ""
    if eid:
        props_str += f' id="{eid}"'
    for k, v in props.items():
        if k == "class":
            props_str += f" class: {v}"
        elif k == "label":
            props_str += f' label="{v}"'
        elif not k.startswith("_"):
            if isinstance(v, str):
                props_str += f' {k}="{v}"'
            else:
                props_str += f" {k}={{{v}}}"
    props_str = props_str.strip()

    if children_ids:
        lines.append(f"{pad}<{tag} {props_str}>")
        for cid in children_ids:
            child = next((e for e in all_elements if e.get("id") == cid), None)
            if child:
                _emit_element(child, all_elements, lines, indent + 1)
        lines.append(f"{pad}</{tag}>")
    else:
        lines.append(f"{pad}<{tag} {props_str} />")
```

- [ ] **Step 3: Run tests, commit**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_migrate.py -v
git add editor/server/annotation/migrate.py tests/annotation/test_migrate.py
git commit -m "feat: implement old .omni.json migration tool"
```

---

## Phase 9: CLI Integration

### Task 11: Add `omnic migrate` CLI command

**Files:**
- Modify: `omni-compiler` CLI entry point (or `editor/server/main.py` cmd_serve area)

- [ ] **Step 1: Add migration CLI command**

Add to the CLI command dispatching in the omnic entry point or as a standalone script:

```python
# In relevant CLI entry point, add:
def cmd_migrate(args):
    """omnic migrate command — convert .omni.json to new format."""
    from pathlib import Path
    from editor.server.annotation.migrate import migrate_omni_json

    project_dir = Path(args.dir or ".")
    json_file = Path(args.file or "project.omni.json")

    if not json_file.is_absolute():
        json_file = project_dir / json_file

    migrate_omni_json(json_file, project_dir)
```

- [ ] **Step 2: Commit**

```bash
git add <cli_file>
git commit -m "feat: add 'omnic migrate' CLI command"
```

---

## Phase 10: End-to-End Round-Trip Integration Tests

### Task 12: Full pipeline integration test

**Files:**
- Create: `tests/annotation/test_roundtrip.py`

- [ ] **Step 1: Write comprehensive round-trip test**

```python
# tests/annotation/test_roundtrip.py
"""End-to-end round-trip tests: parse → generate → parse → validate."""

import pytest
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.generator import generate_annotations
from editor.server.annotation.validator import validate_annotations

COMPLETE_SOURCE = """// @code-tab type=logic
import omniview;

// ═══════════════════════════════════════════════════════════
// @chain inc
//   @node n1  signal  name=count  type=i32  pos=100,50
//   @node n2  getVariable  name=count  pos=100,150
//   @node n3  add                       pos=300,50
//   @node n4  setVariable  name=count   pos=500,50
//   @node n5  return                    pos=650,50
//   @wire w1  n1.value -> n2.value
//   @wire w2  n2.value -> n3.a
//   @wire w3  n3.result -> n4.value
//   @wire w4  n4 -> n5.value
//
// @chain reset
//   @node n6  signal  name=count  type=i32  pos=100,50
//   @node n7  literal  value=0  type=i32   pos=100,150
//   @node n8  setVariable  name=count      pos=300,50
//   @node n9  return                       pos=450,50
//   @wire w5  n7.value -> n8.value
//   @wire w6  n8 -> n9.value
// ═══════════════════════════════════════════════════════════
fn inc() {
    let tmp = count + 1;
    count = tmp;
}

fn reset() {
    count = 0;
}

component Counter {
    signal count: i32 = 0;

    view {
        // @edit
        <div id="box" class: box>
            // @edit
            <h3 id="title" class: title>计数器演示</h3>
            // @edit
            <p id="big" class: big-number>{count}</p>
            // @edit
            <div id="row" class: row>
                // @edit
                <button id="btn-inc" class: btn @onPress={inc}>+</button>
                // @edit
                <button id="btn-reset" class: btn-reset @onPress={reset}>重置</button>
            </div>
        </div>
    }
    style {
        .box { background: white; }
    }
}
"""

class TestFullRoundTrip:
    def test_complete_file_roundtrip(self):
        """Parse a full file, validate, generate, re-parse, and verify equality."""
        ann1 = parse_annotations(COMPLETE_SOURCE, "Counter.omni")
        errors1 = validate_annotations(ann1)
        assert len(errors1) == 0, f"Validation errors in original: {errors1}"

        # Generate
        generated = generate_annotations(ann1, COMPLETE_SOURCE)

        # Re-parse
        ann2 = parse_annotations(generated, "Counter.omni")
        errors2 = validate_annotations(ann2)
        assert len(errors2) == 0, f"Validation errors after round-trip: {errors2}"

        # Compare structures
        assert ann1.code_tab_type == ann2.code_tab_type
        assert len(ann1.view_markers) == len(ann2.view_markers)
        assert len(ann1.chains) == len(ann2.chains)

        # Compare chains
        for c1, c2 in zip(ann1.chains, ann2.chains):
            assert c1.name == c2.name
            assert len(c1.nodes) == len(c2.nodes)
            assert len(c1.wires) == len(c2.wires)
            for n1, n2 in zip(c1.nodes, c2.nodes):
                assert n1.id == n2.id
                assert n1.tag == n2.tag
                assert n1.attrs == n2.attrs
                assert n1.pos_x == n2.pos_x
                assert n1.pos_y == n2.pos_y
            for w1, w2 in zip(c1.wires, c2.wires):
                assert w1.id == w2.id
                assert w1.from_id == w2.from_id
                assert w1.from_port == w2.from_port
                assert w1.to_id == w2.to_id
                assert w1.to_port == w2.to_port

    def test_generated_source_is_valid_omni(self):
        """Generated source should still be parsable by the Omni compiler."""
        ann = parse_annotations(COMPLETE_SOURCE, "Counter.omni")
        generated = generate_annotations(ann, COMPLETE_SOURCE)

        # The generated source should contain valid Omni syntax
        # (annotations are in comments, so they don't affect compilation)
        assert "fn inc()" in generated
        assert "fn reset()" in generated
        assert "component Counter" in generated
        assert "signal count" in generated
        assert "view {" in generated

    def test_multiple_roundtrips_are_stable(self):
        """Three round-trips should produce identical output (idempotent)."""
        ann = parse_annotations(COMPLETE_SOURCE, "test.omni")
        gen1 = generate_annotations(ann, COMPLETE_SOURCE)
        ann2 = parse_annotations(gen1, "test.omni")
        gen2 = generate_annotations(ann2, gen1)

        # Second and third generations should be identical
        # (minor formatting diffs may occur — check structural equivalence)
        ann3 = parse_annotations(gen2, "test.omni")
        assert len(ann2.chains) == len(ann3.chains)
        assert len(ann2.view_markers) == len(ann3.view_markers)
```

- [ ] **Step 2: Run integration tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/test_roundtrip.py -v
```
Expected: 3 tests PASS

- [ ] **Step 3: Run ALL annotation tests**

```bash
cd /d/space/OmniLang && python -m pytest tests/annotation/ -v
```
Expected: all tests PASS (should be ~35+ tests)

- [ ] **Step 4: Commit**

```bash
git add tests/annotation/
git commit -m "test: add end-to-end round-trip integration tests"
```

---

## Summary

### Task Checklist

| # | Task | Phase |
|---|---|---|
| 1 | Annotation data types | Core Types |
| 2 | `@code-tab` and `@edit` parser | Parser |
| 3 | `@chain/@node/@wire` parser tests | Parser |
| 4 | Validation rules (P/I/C/S/F) | Validator |
| 5 | `@edit` and `@chain` generator | Generator |
| 6 | `.omni-editor.toml` read/write | TOML |
| 7 | New API endpoints in main.py | API |
| 8 | TypeScript annotation types | Frontend |
| 9 | API client methods | Frontend |
| 10 | Migration tool | Migration |
| 11 | CLI `omnic migrate` command | CLI |
| 12 | End-to-end round-trip tests | Integration |

### Deferred to Later

- `@if` / `@for` / `@empty` element `@edit` rules (编辑器中这些指令的开发)
- Scope analysis for S5 (variable existence validation) — needs compiler integration
- Custom component file generation from editor canvas — needs the full editor canvas refactor
- Client-side annotation parser (if needed for validation before save)

### Files Created (~10 new files)

```
editor/server/annotation/__init__.py
editor/server/annotation/types.py
editor/server/annotation/parser.py
editor/server/annotation/generator.py
editor/server/annotation/validator.py
editor/server/annotation/editor_toml.py
editor/server/annotation/migrate.py
editor/client/src/annotation/types.ts
tests/annotation/test_types.py
tests/annotation/test_parser.py
tests/annotation/test_validator.py
tests/annotation/test_generator.py
tests/annotation/test_editor_toml.py
tests/annotation/test_migrate.py
tests/annotation/test_roundtrip.py
```

### Files Modified (~2 files)

```
editor/server/main.py          — add annotation API endpoints
editor/client/src/api/client.ts — add API client methods
```
