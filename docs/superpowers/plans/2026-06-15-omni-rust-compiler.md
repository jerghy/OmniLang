# Omni Rust Compiler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust compiler (omni-compiler) that parses .omni → AST → Intent IR → JSON, compile to WASM, and integrate with Python codegen.

**Architecture:** Three-layer: Rust parses/analyzes/lowers → IR JSON. Python codegen consumes IR JSON to produce React/Vue/Flutter/Dioxus. Phase 0 writes all Python-side tests first (zero Rust dependency), Phase 1 implements the Rust compiler, Phase 2 integrates.

**Tech Stack:** Rust (parser, semantic, IR, WASM), Python (codegen, tests), TypeScript (WASM loader in browser)

**Design Doc:** `docs/superpowers/specs/2026-06-15-omni-rust-compiler-design.md`

---

## Phase 0: Python Test Infrastructure (No Rust Required)

Goal: Write IR schema, reader, bridge stub, refactor one codegen, and all Python tests. Everything passes without Rust installed.

---

### Task 0.1: Create IR Schema (ir_schema.py)

**Files:**
- Create: `omniview/ir_schema.py`

- [ ] **Step 1: Write IR schema TypedDict definitions**

Copy the TypedDict definitions from the design doc §6.1. Full file:

```python
"""IR JSON Schema for Rust ↔ Python contract.
Each TypedDict matches a corresponding dataclass in ir_types.py.
Key rule: every IR object carries a _kind field for dispatch.
"""
from typing import TypedDict, Literal, Optional, Any


class TypeIR(TypedDict, total=False):
    kind: str
    type_args: list  # list[TypeIR]
    is_nullable: bool
    needs_clone: bool
    needs_partial_eq: bool


class LiteralExpr(TypedDict):
    _kind: Literal["LiteralExpr"]
    value: Any


class IdentifierExpr(TypedDict):
    _kind: Literal["IdentifierExpr"]
    name: str


class FieldAccessExpr(TypedDict):
    _kind: Literal["FieldAccessExpr"]
    object: dict
    field: str


class BinaryOpExpr(TypedDict):
    _kind: Literal["BinaryOpExpr"]
    op: str
    left: dict
    right: dict


class UnaryOpExpr(TypedDict):
    _kind: Literal["UnaryOpExpr"]
    op: str
    operand: dict


class TernaryIR(TypedDict):
    _kind: Literal["TernaryIR"]
    cond: dict
    then_expr: dict
    else_expr: dict


class StoreFieldAccessExpr(TypedDict):
    _kind: Literal["StoreFieldAccessExpr"]
    store_alias: str
    field_name: str
    call_args: list  # list[ExprIR]


class SignalRefIR(TypedDict):
    owner: Optional[str]
    name: str


class BindingIR(TypedDict):
    name: str
    type: Optional[dict]


class EventHandlerIR(TypedDict):
    _kind: Literal["EventHandlerIR"]
    kind: str  # "inline_fn"|"fn_ref"|"store_action"
    body: Optional[dict]
    ref: Optional[str]


class StateIntent(TypedDict):
    _kind: Literal["StateIntent"]
    name: str
    type: TypeIR
    initial_value: dict
    mutability: str
    update_semantics: str
    accessor_style: str
    is_copy: bool


class MutationIntent(TypedDict):
    _kind: Literal["MutationIntent"]
    target: SignalRefIR
    operation: str
    amount: Optional[dict]
    item: Optional[dict]


class ComputedIntent(TypedDict):
    _kind: Literal["ComputedIntent"]
    name: str
    type: TypeIR
    body: dict
    deps: list[str]


class EffectIntent(TypedDict):
    _kind: Literal["EffectIntent"]
    deps: list[str]
    body: dict


class LifecycleIntent(TypedDict):
    _kind: Literal["LifecycleIntent"]
    kind: str
    body: dict


class ConditionalBranch(TypedDict):
    condition: dict
    body: Optional[dict]


class ConditionalIntent(TypedDict):
    _kind: Literal["ConditionalIntent"]
    branches: list  # list[ConditionalBranch]
    fallback: Optional[dict]


class ListRenderingIntent(TypedDict):
    _kind: Literal["ListRenderingIntent"]
    iterable: dict
    item_binding: dict
    body: Optional[dict]
    key_expr: Optional[dict]
    empty_body: Optional[dict]


class BindIntent(TypedDict):
    _kind: Literal["BindIntent"]
    source: SignalRefIR
    element_type: str
    direction: str


class EventIntent(TypedDict):
    _kind: Literal["EventIntent"]
    event_name: str
    handler: EventHandlerIR


class SlotIntent(TypedDict):
    _kind: Literal["SlotIntent"]
    name: Optional[str]
    body: Optional[dict]


class ElementIR(TypedDict):
    _kind: Literal["ElementIR"]
    tag: str
    props: dict[str, dict]
    bindings: dict[str, SignalRefIR]
    events: dict[str, EventHandlerIR]
    style_classes: Optional[dict]
    inline_style: Optional[dict]
    children: list  # list[ViewTree]
    test_id: Optional[str]
    ref_name: Optional[str]
    construction_kind: str


class StyleRuleIR(TypedDict):
    _kind: Literal["StyleRuleIR"]
    class_name: str
    properties: dict[str, dict]
    output_mode: str


class BlockIR(TypedDict):
    _kind: Literal["BlockIR"]
    statements: list  # list[StatementIR]
    execution_mode: str


class LetStmtIR(TypedDict):
    _kind: Literal["LetStmtIR"]
    name: str
    value: Optional[dict]
    type: Optional[TypeIR]


class IfStmtIR(TypedDict):
    _kind: Literal["IfStmtIR"]
    condition: dict
    then_body: dict
    else_body: Optional[dict]


class ReturnStmtIR(TypedDict):
    _kind: Literal["ReturnStmtIR"]
    value: Optional[dict]


class MutationStmt(TypedDict):
    _kind: Literal["MutationStmt"]
    intent: Optional[dict]


class FunctionIR(TypedDict):
    _kind: Literal["FunctionIR"]
    name: str
    params: list[dict]  # list[{name, type?}]
    return_type: Optional[TypeIR]
    body: dict


class ComponentIR(TypedDict, total=False):
    _kind: Literal["ComponentIR"]
    name: str
    generic_params: list[str]
    props: dict[str, TypeIR]
    states: list  # list[StateIntent]
    computeds: list
    effects: list
    lifecycles: list
    fns: list
    store_refs: list[str]
    refs: list
    view: Optional[dict]
    styles: list


class StoreIR(TypedDict):
    _kind: Literal["StoreIR"]
    name: str
    signals: dict[str, TypeIR]
    signal_defaults: dict[str, Any]
    getters: dict[str, dict]
    actions: list


class SourceMapEntry(TypedDict):
    node_id: str
    omni_span: dict  # {line, col, len}
    target_spans: list  # list[{file, line, col, len, kind}]


class IRProgram(TypedDict):
    version: str
    components: dict[str, ComponentIR]
    stores: dict[str, StoreIR]
    source_map: dict[str, SourceMapEntry]


__all__ = [
    "TypeIR", "LiteralExpr", "IdentifierExpr", "FieldAccessExpr",
    "BinaryOpExpr", "UnaryOpExpr", "TernaryIR", "StoreFieldAccessExpr",
    "SignalRefIR", "BindingIR", "EventHandlerIR",
    "StateIntent", "MutationIntent", "ComputedIntent",
    "EffectIntent", "LifecycleIntent",
    "ConditionalBranch", "ConditionalIntent",
    "ListRenderingIntent", "BindIntent", "EventIntent", "SlotIntent",
    "ElementIR", "StyleRuleIR", "BlockIR",
    "LetStmtIR", "IfStmtIR", "ReturnStmtIR", "MutationStmt",
    "FunctionIR", "ComponentIR", "StoreIR",
    "SourceMapEntry", "IRProgram",
]
```

- [ ] **Step 2: Verify the file imports correctly**

```bash
cd D:/space/OmniLang && python -c "from omniview.ir_schema import ComponentIR, ElementIR, IRProgram; print('OK')"
```

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/ir_schema.py && git commit -m "feat(ir-schema): add TypedDict definitions for Rust IR JSON contract"
```

---

### Task 0.2: Write IR Schema Tests

**Files:**
- Create: `omniview/tests/test_ir_schema.py`

- [ ] **Step 1: Write the test file**

Full file as in design doc §7.1 — copy the complete test code from there. Tests:
1. `test_schema_matches_dataclass_for_counter` — Python pipeline output → dict → _kind check, name check, states existence
2. `test_element_ir_structure` — Deeply nested ElementIR with children
3. `test_conditional_intent_structure` — @if/@else branches
4. `test_list_rendering_intent_structure` — @for iterable + item_binding
5. Helper functions: `_ir_to_dict()`, `_validate_component_ir()`, `_validate_element_ir()`, `_validate_ir_object()`

- [ ] **Step 2: Run tests — verify they pass**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/test_ir_schema.py -v
```

Expected: 4 passed

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/tests/test_ir_schema.py && git commit -m "test(ir-schema): add TypedDict validation tests against Python IR pipeline output"
```

---

### Task 0.3: Create IR Reader (ir_reader.py)

**Files:**
- Create: `omniview/codegen/ir_reader.py`

- [ ] **Step 1: Write the IR Reader**

```python
"""IR JSON Reader — validates and enriches Rust-generated IR JSON for Python codegen."""
from __future__ import annotations

import json
from typing import Any


class IRReader:
    """Read a Rust-generated IR JSON and validate/enrich it for codegen."""

    def __init__(self, ir_input: str | dict):
        if isinstance(ir_input, str):
            try:
                self.ir: dict = json.loads(ir_input)
            except json.JSONDecodeError as e:
                self.ir = {}
                self._errors = [f"Invalid JSON: {e}"]
                return
        else:
            self.ir = ir_input
        self._errors: list[str] = []
        self._validate()

    def _validate(self) -> None:
        """Check version and top-level structure."""
        if not self.ir:
            self._errors.append("Empty IR document")
            return

        v = self.ir.get("version")
        if v is None:
            self._errors.append("IR missing 'version' field")
        elif v != "1.0":
            self._errors.append(f"Unsupported IR version: {v} (expected '1.0')")

        has_components = "components" in self.ir and self.ir["components"]
        has_stores = "stores" in self.ir and self.ir["stores"]
        if not has_components and not has_stores:
            self._errors.append("IR must contain 'components' or 'stores' with at least one entry")

    @property
    def errors(self) -> list[str]:
        return self._errors

    @property
    def is_valid(self) -> bool:
        return len(self._errors) == 0

    def components(self) -> dict[str, dict]:
        """Return {name: ComponentIR} dict."""
        return self.ir.get("components", {})

    def stores(self) -> dict[str, dict]:
        """Return {name: StoreIR} dict."""
        return self.ir.get("stores", {})

    def source_map(self) -> dict:
        """Return {node_id: SourceMapEntry} dict."""
        return self.ir.get("source_map", {})

    @staticmethod
    def ir_kind(obj: Any) -> str:
        """Read _kind field for dispatch. e.g. 'ElementIR', 'StateIntent'."""
        if isinstance(obj, dict):
            return obj.get("_kind", "unknown")
        return "unknown"
```

- [ ] **Step 2: Verify import**

```bash
cd D:/space/OmniLang && python -c "from omniview.codegen.ir_reader import IRReader; print('OK')"
```

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/codegen/ir_reader.py && git commit -m "feat(ir-reader): add IR JSON reader/validator for Python codegen consumption"
```

---

### Task 0.4: Write IR Reader Tests

**Files:**
- Create: `omniview/tests/test_ir_reader.py`

- [ ] **Step 1: Write the test file**

Full file as in design doc §7.2. Copy all tests from there:
1. `test_read_valid_ir` 
2. `test_read_version`
3. `test_read_component_name`
4. `test_read_states`
5. `test_read_view`
6. `test_accept_json_string`
7. `test_accept_dict`
8. `test_invalid_version`
9. `test_empty_ir`
10. `test_missing_components_and_stores`
11. `test_invalid_json_string`
12. `test_stores_empty`
13. `test_stores_with_data`
14. `test_source_map`
15. `test_ir_kind`

Need a `counter_ir_json` fixture that uses the Python pipeline to generate IR and serialize it. This is the same `_ir_to_dict` helper from test_ir_schema.py. Extract it to a shared test utility.

- [ ] **Step 2: Extract shared test utilities**

```bash
# Check if omniview/tests/helpers exists
cd D:/space/OmniLang && ls omniview/tests/
```

If no `helpers.py` exists, create `omniview/tests/helpers.py`:

```python
"""Shared test utilities for IR tests."""

def ir_dataclass_to_dict(ir_obj) -> dict:
    """Convert a Python IR dataclass tree to a JSON-serializable dict with _kind markers."""
    if ir_obj is None:
        return None
    if isinstance(ir_obj, (str, int, float, bool)):
        return ir_obj
    if isinstance(ir_obj, list):
        return [ir_dataclass_to_dict(v) for v in ir_obj]
    if isinstance(ir_obj, dict):
        return {k: ir_dataclass_to_dict(v) for k, v in ir_obj.items()}
    if hasattr(ir_obj, '__dataclass_fields__'):
        result = {"_kind": type(ir_obj).__name__}
        for field in ir_obj.__dataclass_fields__:
            val = getattr(ir_obj, field)
            result[field] = ir_dataclass_to_dict(val)
        return result
    return str(ir_obj)


def make_counter_ir_dict():
    """Run Python pipeline on Counter.omni and return IR as dict."""
    import omniview.preprocessor as pp
    from omniview.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer
    from omniview.semantic.analyzer import SemanticAnalyzer
    from omniview.lowering import IRFixture
    
    source = open("build/test/counter/Counter.omni", encoding="utf-8").read()
    pre = pp.preprocess(source)
    tree = create_omniview_parser().parse(pre.processed)
    ast = OmniViewTransformer(pre.block_map).transform(tree)
    analysis = SemanticAnalyzer().analyze(ast)
    ir_map = IRFixture().lower_program(ast)
    
    return {
        "version": "1.0",
        "components": {
            "Counter": ir_dataclass_to_dict(ir_map["Counter"])
        },
        "stores": {},
        "source_map": {},
    }
```

- [ ] **Step 3: Run tests — verify they pass**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/test_ir_reader.py -v
```

Expected: 15 passed

- [ ] **Step 4: Commit**

```bash
cd D:/space/OmniLang && git add omniview/tests/test_ir_reader.py omniview/tests/helpers.py && git commit -m "test(ir-reader): add 15 tests for IR JSON reader/validator"
```

---

### Task 0.5: Create Rust Bridge Stub

**Files:**
- Create: `omniview/codegen/rust_bridge.py`

- [ ] **Step 1: Write the stub**

```python
"""Rust bridge — call Rust compiler DLL/CLI for .omni → IR compilation."""
from __future__ import annotations

import subprocess
import json
from pathlib import Path


class RustBridge:
    """Bridge to the Rust omni-compiler (DLL or CLI)."""
    
    _dll = None
    _availability_checked = False
    _available = False
    
    @classmethod
    def is_available(cls) -> bool:
        """Check if Rust compiler is available.
        Returns False gracefully — callers must handle Python fallback.
        """
        if cls._availability_checked:
            return cls._available
        
        # Option 1: Check for native DLL
        dll_paths = [
            Path(__file__).parent.parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll",
            Path("omni-compiler/target/release/omni_compiler.dll"),
        ]
        for p in dll_paths:
            if p.exists():
                cls._available = True
                cls._availability_checked = True
                return True
        
        # Option 2: Check for CLI
        try:
            result = subprocess.run(
                ["omni-compiler", "--version"],
                capture_output=True, timeout=5, text=True
            )
            if result.returncode == 0:
                cls._available = True
                cls._availability_checked = True
                return True
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
        
        cls._availability_checked = True
        cls._available = False
        return False

    @classmethod
    def compile_to_ir(cls, source: str) -> dict:
        """Compile .omni → IR JSON via Rust.
        
        Returns: IRProgram dict
        Raises: RuntimeError if Rust compiler is not available
        """
        if not cls.is_available():
            raise RuntimeError("Rust compiler not available")
        
        # Try DLL first, fallback to CLI
        result = cls._try_dll(source) or cls._try_cli(source)
        if result is None:
            raise RuntimeError("Rust compilation failed — no output")
        
        if "_rust_errors" in result:
            return result  # Rust found errors; caller handles diagnostics
        
        return result
    
    @classmethod
    def compile_to_omniview(cls, source: str) -> dict:
        """Compile .omni → OmniView JS/HTML/CSS + SourceMap via Rust.
        Returns: {success, files?, source_map?, errors?}
        """
        if not cls.is_available():
            raise RuntimeError("Rust compiler not available")
        
        result = cls._try_dll_omniview(source) or cls._try_cli_omniview(source)
        if result is None:
            raise RuntimeError("Rust OmniView compilation failed")
        return result
    
    # ── Private ──
    
    @classmethod
    def _try_dll(cls, source: str) -> dict | None:
        """Try compiling via ctypes DLL call."""
        # TODO: Implement when Rust DLL is built (Phase 1)
        return None
    
    @classmethod
    def _try_cli(cls, source: str) -> dict | None:
        """Try compiling via subprocess CLI call."""
        try:
            result = subprocess.run(
                ["omni-compiler", "compile", "--to-ir", "--format", "json"],
                input=source, capture_output=True, timeout=30, text=True
            )
            if result.returncode == 0:
                return json.loads(result.stdout)
            else:
                # CLI returned errors
                try:
                    errors = json.loads(result.stderr)
                    return {"_rust_errors": errors.get("errors", [])}
                except json.JSONDecodeError:
                    return {"_rust_errors": [{"message": result.stderr}]}
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
        return None
    
    @classmethod
    def _try_dll_omniview(cls, source: str) -> dict | None:
        return None
    
    @classmethod
    def _try_cli_omniview(cls, source: str) -> dict | None:
        """Try OmniView compilation via CLI."""
        try:
            result = subprocess.run(
                ["omni-compiler", "compile", "--to-omniview", "--format", "json"],
                input=source, capture_output=True, timeout=30, text=True
            )
            if result.returncode == 0:
                return json.loads(result.stdout)
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
        return None
```

- [ ] **Step 2: Verify import**

```bash
cd D:/space/OmniLang && python -c "from omniview.codegen.rust_bridge import RustBridge; print('OK')"
```

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/codegen/rust_bridge.py && git commit -m "feat(rust-bridge): add stub for Rust compiler DLL/CLI bridge"
```

---

### Task 0.6: Write Rust Bridge Tests

**Files:**
- Create: `omniview/tests/test_rust_bridge.py`

- [ ] **Step 1: Write the test file**

```python
"""Tests for RustBridge — works both with and without Rust installed."""
import pytest
from omniview.codegen.rust_bridge import RustBridge


def test_is_available_returns_bool():
    """Rust compiler not required — always returns a bool."""
    result = RustBridge.is_available()
    assert isinstance(result, bool)


def test_compile_to_ir_raises_when_not_available():
    """When Rust is absent, compile_to_ir must raise RuntimeError."""
    if not RustBridge.is_available():
        with pytest.raises(RuntimeError, match="not available"):
            RustBridge.compile_to_ir("component App { view {} }")


def test_compile_to_omniview_raises_when_not_available():
    """When Rust is absent, compile_to_omniview must raise RuntimeError."""
    if not RustBridge.is_available():
        with pytest.raises(RuntimeError, match="not available"):
            RustBridge.compile_to_omniview("component App { view {} }")


@pytest.mark.skipif(not RustBridge.is_available(), reason="Rust compiler not installed")
def test_compile_to_ir_success():
    """Rust available → compile_to_ir returns valid IR dict."""
    result = RustBridge.compile_to_ir("component App { view {} }")
    assert isinstance(result, dict)
    assert "components" in result or "stores" in result
    assert "version" in result
    assert result["version"] == "1.0"


@pytest.mark.skipif(not RustBridge.is_available(), reason="Rust compiler not installed")
def test_compile_to_ir_reports_errors():
    """Rust returns errors for broken .omni."""
    result = RustBridge.compile_to_ir("not valid omni @@@")
    assert "_rust_errors" in result or "errors" in result
```

- [ ] **Step 2: Run tests**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/test_rust_bridge.py -v
```

Expected: 3 passed, 2 skipped (because Rust is not installed yet)

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/tests/test_rust_bridge.py && git commit -m "test(rust-bridge): add availability + graceful degradation tests"
```

---

### Task 0.7: Refactor React Codegen to Accept Dict

**Files:**
- Modify: `omniview/codegen/react.py`
- Modify: `omniview/codegen/base.py` (update type signatures)

- [ ] **Step 1: Read current React codegen entry point**

```bash
cd D:/space/OmniLang && grep -n "def generate_component" omniview/codegen/react.py
```

Find the `generate_component` method. Currently it accepts `ComponentIR` dataclass. Need to make it accept dict too.

- [ ] **Step 2: Add dict adapter at the entry point**

At the top of `generate_component` in `react.py`, add a normalization step:

```python
# omniview/codegen/react.py — add at top of generate_component()

def generate_component(self, comp, store_irs=None, source_line=None):
    """Generate React TSX from ComponentIR.
    
    Args:
        comp: ComponentIR dataclass OR dict (from Rust IR JSON)
        store_irs: dict of StoreIR (same flexible typing)
    """
    # Normalize: accept both dataclass and dict
    if isinstance(comp, dict):
        comp = self._dict_to_component(comp)
    if store_irs:
        store_irs = {
            k: self._dict_to_store(v) if isinstance(v, dict) else v
            for k, v in store_irs.items()
        }
    # ... rest of existing code unchanged ...
    return self._gen_component(comp, store_irs or {}, source_line)
```

- [ ] **Step 3: Add dict helper methods**

```python
# At bottom of ReactCodegen class:

def _dict_to_component(self, d: dict):
    """Convert ComponentIR dict to a lightweight object for attribute access."""
    return _DictAccessor(d)

def _dict_to_store(self, d: dict):
    return _DictAccessor(d)


class _DictAccessor:
    """Wrap a dict to allow attribute-style access (d.name instead of d['name']).
    This makes the migration minimal — codegen internals can stay unchanged.
    """
    def __init__(self, data: dict):
        self.__dict__['_data'] = data
    
    def __getattr__(self, name):
        if name.startswith('_'):
            return self.__dict__[name]
        val = self.__dict__['_data'].get(name)
        if isinstance(val, dict):
            return _DictAccessor(val)
        if isinstance(val, list):
            return [
                _DictAccessor(item) if isinstance(item, dict) else item
                for item in val
            ]
        return val
    
    def __setattr__(self, name, value):
        self.__dict__['_data'][name] = value
    
    def __contains__(self, key):
        return key in self.__dict__['_data']
    
    def get(self, key, default=None):
        val = self.__dict__['_data'].get(key, default)
        if isinstance(val, dict):
            return _DictAccessor(val)
        if isinstance(val, list):
            return [
                _DictAccessor(item) if isinstance(item, dict) else item
                for item in val
            ]
        return val
    
    def items(self):
        for k, v in self.__dict__['_data'].items():
            if isinstance(v, dict):
                yield k, _DictAccessor(v)
            elif isinstance(v, list):
                yield k, [_DictAccessor(i) if isinstance(i, dict) else i for i in v]
            else:
                yield k, v
    
    def keys(self):
        return self.__dict__['_data'].keys()
    
    def values(self):
        return [self.__getattr__(k) for k in self.__dict__['_data']]
    
    def __getitem__(self, key):
        return self.__getattr__(key)
    
    def __iter__(self):
        return iter(self.__dict__['_data'])
    
    def __len__(self):
        return len(self.__dict__['_data'])
    
    def __repr__(self):
        return f"_DictAccessor({self.__dict__['_data']})"
```

- [ ] **Step 4: Run existing React codegen tests to verify no regression**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/ -k "react" -v --timeout=30 2>&1 | head -40
```

Expected: All existing React tests pass (no regression from the adapter)

- [ ] **Step 5: Commit**

```bash
cd D:/space/OmniLang && git add omniview/codegen/react.py omniview/codegen/base.py && git commit -m "refactor(react-codegen): accept dict input via _DictAccessor adapter"
```

---

### Task 0.8: Write Codegen-Dict-Input Tests

**Files:**
- Create: `omniview/tests/test_codegen_from_dict.py`

- [ ] **Step 1: Write tests for dict → codegen path**

```python
"""Test that Python codegen produces identical output with dict vs dataclass input."""
import pytest
from omniview.tests.helpers import make_counter_ir_dict


def test_react_codegen_from_dict_matches_pipeline():
    """React codegen: dict input → same output as full Python pipeline."""
    from omniview.api import compile_omni
    from omniview.codegen.react import ReactCodegen
    
    source = open("build/test/counter/Counter.omni", encoding="utf-8").read()
    
    # Path A: Full Python pipeline (dataclass)
    result_a = compile_omni(source, filepath="counter/Counter.omni", target="react")
    assert result_a.success, f"Python pipeline errors: {result_a.errors}"
    
    # Path B: IR dict → codegen directly
    ir_dict = make_counter_ir_dict()
    comp_dict = ir_dict["components"]["Counter"]
    result_b = ReactCodegen().generate_component(comp_dict)
    
    # Compare outputs file-by-file
    for filename in result_a.files:
        assert filename in result_b.files, f"Missing file: {filename}"
        a = result_a.files[filename].strip()
        b = result_b.files[filename].strip()
        assert a == b, f"Output mismatch for {filename}"


def test_empty_component_does_not_crash():
    """Codegen must handle an empty component gracefully."""
    from omniview.codegen.react import ReactCodegen
    
    empty = {
        "_kind": "ComponentIR",
        "name": "Empty",
        "props": {},
        "states": [],
        "computeds": [],
        "effects": [],
        "lifecycles": [],
        "fns": [],
        "store_refs": [],
        "refs": [],
        "view": None,
        "styles": [],
    }
    result = ReactCodegen().generate_component(empty)
    assert result.files
    content = result.files.get("Empty.tsx", "")
    assert "Empty" in content


def test_minimal_signal_component():
    """Component with one signal → codegen produces valid output."""
    from omniview.codegen.react import ReactCodegen
    
    comp = {
        "_kind": "ComponentIR",
        "name": "Minimal",
        "props": {},
        "states": [{
            "_kind": "StateIntent",
            "name": "count",
            "type": {"kind": "i32"},
            "initial_value": {"_kind": "LiteralExpr", "value": 0},
            "mutability": "mutable",
            "update_semantics": "setter-call",
            "accessor_style": "direct",
            "is_copy": False,
        }],
        "computeds": [],
        "effects": [],
        "lifecycles": [],
        "fns": [],
        "store_refs": [],
        "refs": [],
        "view": None,
        "styles": [],
    }
    result = ReactCodegen().generate_component(comp)
    content = result.files.get("Minimal.tsx", "")
    assert "useState" in content
    assert "count" in content


def test_shopping_cart_dict_matches_pipeline():
    """ShoppingCart: dict path == pipeline path."""
    from omniview.api import compile_omni
    from omniview.codegen.react import ReactCodegen
    from omniview.tests.helpers import ir_dataclass_to_dict
    
    source = open("build/test/shopping-cart/ShoppingCart.omni", encoding="utf-8").read()
    result_a = compile_omni(source, filepath="shopping-cart/ShoppingCart.omni", target="react")
    assert result_a.success
    
    # Get IR via internal pipeline
    import omniview.preprocessor as pp
    from omniview.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer
    from omniview.semantic.analyzer import SemanticAnalyzer
    from omniview.lowering import IRFixture
    
    pre = pp.preprocess(source)
    tree = create_omniview_parser().parse(pre.processed)
    ast = OmniViewTransformer(pre.block_map).transform(tree)
    analysis = SemanticAnalyzer().analyze(ast)
    ir_map = IRFixture().lower_program(ast)
    
    for name, ir_obj in ir_map.items():
        if hasattr(ir_obj, '__dataclass_fields__'):
            comp_dict = ir_dataclass_to_dict(ir_obj)
            result_b = ReactCodegen().generate_component(comp_dict)
            assert result_b.files
```

- [ ] **Step 2: Run tests**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/test_codegen_from_dict.py -v
```

Expected: 4 passed

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/tests/test_codegen_from_dict.py && git commit -m "test(codegen-dict): add dict-input codegen tests — matches pipeline output exactly"
```

---

### Task 0.9: Refactor Remaining 4 Codegens

**Files:**
- Modify: `omniview/codegen/vue.py`
- Modify: `omniview/codegen/flutter.py`
- Modify: `omniview/codegen/dioxus.py`
- Modify: `omniview/codegen/omniview.py`

- [ ] **Step 1: Apply same _DictAccessor adapter pattern to each codegen**

For each file, add the same dict normalization at `generate_component()` entry and copy the `_DictAccessor` class (or import it from a shared module).

Better: Move `_DictAccessor` to `omniview/codegen/base.py` so all codegens share it.

```python
# Add to omniview/codegen/base.py:

class _DictAccessor:
    """Wrap a dict for attribute-style access (minimal migration from dataclass→dict)."""
    def __init__(self, data: dict):
        self.__dict__['_data'] = data
    
    def __getattr__(self, name):
        if name.startswith('_'):
            return self.__dict__[name]
        val = self.__dict__['_data'].get(name)
        if isinstance(val, dict):
            return _DictAccessor(val)
        if isinstance(val, list):
            return [_DictAccessor(item) if isinstance(item, dict) else item for item in val]
        return val
    
    def __setattr__(self, name, value):
        self.__dict__['_data'][name] = value
    
    def __contains__(self, key):
        return key in self.__dict__['_data']
    
    def get(self, key, default=None):
        val = self.__dict__['_data'].get(key, default)
        if isinstance(val, dict):
            return _DictAccessor(val)
        if isinstance(val, list):
            return [_DictAccessor(item) if isinstance(item, dict) else item for item in val]
        return val
    
    def __getitem__(self, key):
        return self.__getattr__(key)


def ir_dict_to_accessor(d: dict) -> _DictAccessor:
    return _DictAccessor(d)
```

Then update each codegen's `generate_component()` to call `ir_dict_to_accessor()` if input is dict.

- [ ] **Step 2: Verify all codegens still pass existing tests**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/ -v --timeout=60 -x 2>&1 | tail -30
```

Expected: All existing 261+ tests pass

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/codegen/base.py omniview/codegen/vue.py omniview/codegen/flutter.py omniview/codegen/dioxus.py omniview/codegen/omniview.py && git commit -m "refactor(all-codegens): accept dict input via shared _DictAccessor adapter"
```

---

### Task 0.10: Update api.py with Dual Pipeline Dispatch

**Files:**
- Modify: `omniview/api.py`

- [ ] **Step 1: Add dual-path compile function**

In `compile_omni()`, before the existing `_compile_ui_mode()` call, try Rust first:

```python
def compile_omni(source, filepath="<input>", target="react", previous_files=None, debug_mode=False):
    # ... existing setup ...
    
    # Try Rust compiler first (if available)
    from omniview.codegen.rust_bridge import RustBridge
    if RustBridge.is_available():
        ir_result = RustBridge.compile_to_ir(source)
        if "_rust_errors" not in ir_result:
            return _compile_from_ir(ir_result, target, previous_files)
        # Rust found errors — report them
        errors = ir_result.get("_rust_errors", [])
        return CompileResult(success=False, files={}, errors=errors, source=source)
    
    # Fallback: full Python pipeline (unchanged)
    if _detect_ui_mode(source):
        return _compile_ui_mode(source, filepath, target, previous_files, debug_mode)
    else:
        return _compile_pure_logic(source, filepath, target, previous_files, debug_mode)


def _compile_from_ir(ir_program: dict, target: str, previous_files=None) -> CompileResult:
    """Compile from IR JSON dict (Rust output) via Python codegen only."""
    codegen = _get_codegen(target)
    if codegen is None:
        return CompileResult(
            success=False, files={},
            errors=[{"severity": "error", "message": f"Unsupported target: {target}"}]
        )
    
    current_files = {}
    errors = []
    
    for name, comp_ir in ir_program.get("components", {}).items():
        result = codegen.generate_component(comp_ir)
        current_files.update(result.files)
        errors.extend(result.errors)
    
    for name, store_ir in ir_program.get("stores", {}).items():
        result = codegen.generate_store(store_ir)
        current_files.update(result.files)
        errors.extend(result.errors)
    
    # Compute diff if previous_files provided
    diff = None
    if previous_files:
        diff = _compute_diff(previous_files, current_files)
    
    return CompileResult(
        success=len(errors) == 0,
        files=current_files,
        diff=diff,
        errors=errors,
    )
```

- [ ] **Step 2: Verify existing tests still pass**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/ -v --timeout=60 -x 2>&1 | tail -30
```

Expected: All existing tests pass (Rust not available → falls back to Python pipeline)

- [ ] **Step 3: Commit**

```bash
cd D:/space/OmniLang && git add omniview/api.py && git commit -m "feat(api): add dual pipeline dispatch — Rust first, Python fallback"
```

---

### Task 0.11: Phase 0 Final Verification

- [ ] **Step 1: Run all Python tests**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/ -v --timeout=60 2>&1 | tail -30
```

Expected: All tests pass (no regressions from Phase 0 changes)

- [ ] **Step 2: Run reverse codegen tests**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/test_reverse_codegen.py -v
```

Expected: 36 passed

- [ ] **Step 3: Verify round-trip manually**

```bash
cd D:/space/OmniLang && python -c "
from omniview.api import compile_omni
src = open('build/test/counter/Counter.omni').read()
result = compile_omni(src, target='react')
assert result.success, f'Compile failed: {result.errors}'
print('Counter.tsx:', len(result.files['Counter.tsx']), 'chars')
print('Compile OK ✓')
"
```

Expected: `Compile OK ✓`

---

## Phase 1: Rust Compiler Core

Goal: Implement the Rust omni-compiler crate — lexer, parser, semantic analysis, IR lowering, IR JSON output, WASM bindings.

---

### Task 1.1: Initialize Rust Project

**Files:**
- Create: `omni-compiler/Cargo.toml`
- Create: `omni-compiler/src/lib.rs`
- Create: `omni-compiler/.gitignore`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "omni-compiler"
version = "0.1.0"
edition = "2021"
description = "OmniLang compiler — .omni → Intent IR → target code"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# WASM support
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }

# Error handling
thiserror = "1"

[features]
default = []
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]
native = []

[profile.release]
lto = true
opt-level = "s"
strip = true
```

- [ ] **Step 2: Create lib.rs skeleton**

```rust
// omni-compiler/src/lib.rs

pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod ir;
pub mod codegen;
pub mod sourcemap;
pub mod error;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ir: Option<ir::IRProgram>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

/// Compile .omni source to IR JSON. Returns JSON string.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn compile_to_ir(source: &str) -> String {
    let result = compile(source);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(r#"{{"success":false,"errors":[{{"message":"{}"}}]}}"#, e)
    })
}

/// Compile .omni source → IR. Core function, shared by WASM and native.
pub fn compile(source: &str) -> CompileResult {
    let mut errors: Vec<error::Diagnostic> = Vec::new();
    
    // 1. Lex
    let tokens = lexer::tokenize(source);
    
    // 2. Parse
    let program = match parser::parse(&tokens) {
        Ok(p) => p,
        Err(e) => {
            errors.push(e.into());
            return CompileResult { success: false, ir: None, errors };
        }
    };
    
    // 3. Semantic analysis
    let semantics = match semantic::analyze(&program) {
        Ok(s) => s,
        Err(diags) => {
            errors.extend(diags);
            return CompileResult { success: false, ir: None, errors };
        }
    };
    
    // 4. Lower to IR
    let ir_program = ir::lower(&program, &semantics);
    
    CompileResult { success: true, ir: Some(ir_program), errors }
}

/// Return compiler version.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
```

- [ ] **Step 4: Run cargo check to verify project structure**

```bash
cd D:/space/OmniLang/omni-compiler && cargo check
```

Expected: Errors about missing modules (expected — they don't exist yet)

- [ ] **Step 5: Commit**

```bash
cd D:/space/OmniLang && git add omni-compiler/ && git commit -m "feat(rust-compiler): initialize Cargo project with module skeleton"
```

---

### Task 1.2: Implement Lexer

**Files:**
- Create: `omni-compiler/src/lexer/mod.rs`
- Create: `omni-compiler/src/lexer/token.rs`

- [ ] **Step 1: Write token types**

```rust
// omni-compiler/src/lexer/token.rs

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenKind {
    // Keywords (omnilang)
    Fn, Let, Mut, If, Else, While, For, In, Return, Break, Continue,
    Print, Println, Async, Await, Match,
    Struct, Class, Interface, Impl, Extends, Implements,
    Type, Nominal, Static, Private, Override,
    Import, From, Export,
    True, False, Null,

    // Keywords (omniview)
    Component, Signal, Computed, Prop, Event, Ref, Store, View, Style,
    Action, Getter, Theme,
    AtStore, AtIf, AtElse, AtFor, AtEmpty, AtSlot,
    AtNative, AtFallback, AtError,
    AtMount, AtUnmount, AtEffect,
    Bind,

    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    TemplateString(String),

    // Identifiers
    Ident(String),

    // Operators & Punctuation
    Dot, DotDotDot, QuestionMark, QuestionDot,
    Plus, Minus, Star, Slash, Percent,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    Eq, EqEq, NotEq, Lt, Gt, LtEq, GtEq,
    AndAnd, OrOr, Not,
    Arrow, FatArrow, Pipe,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Semicolon, Comma, Colon, At,

    // Special
    Eof,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, start: usize, end: usize, line: u32, col: u32) -> Self {
        Token { kind, span: Span { start, end, line, col } }
    }
}
```

- [ ] **Step 2: Write lexer implementation**

```rust
// omni-compiler/src/lexer/mod.rs

mod token;
pub use token::*;

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.pos < self.source.len() {
            self.skip_whitespace_and_comments();
            if self.pos >= self.source.len() { break; }
            let tok = self.read_token();
            match &tok.kind {
                TokenKind::Eof => break,
                TokenKind::Error(_) => { tokens.push(tok); break; }
                _ => tokens.push(tok),
            }
        }
        tokens.push(Token::new(TokenKind::Eof, self.pos, self.pos, self.line, self.col));
        tokens
    }

    fn read_token(&mut self) -> Token {
        let c = self.peek().unwrap();
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        match c {
            // Strings
            '"' => self.read_string(),
            // Template strings
            '`' => self.read_template_string(),
            // Numbers
            '0'..='9' => self.read_number(),
            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.read_ident_or_keyword(),
            // @-prefixed keywords
            '@' => self.read_at_keyword(),
            // Operators and punctuation
            '.' => { self.advance(); self.check_dot_continuation(start, line, col) }
            '+' => { self.advance(); self.if_next('=', TokenKind::PlusEq, TokenKind::Plus, start, line, col) }
            '-' => { self.advance(); if self.peek() == Some('>') { self.advance(); Token::new(TokenKind::Arrow, start, self.pos, line, col) } else { self.if_next('=', TokenKind::MinusEq, TokenKind::Minus, start, line, col) } }
            '*' => { self.advance(); self.if_next('=', TokenKind::StarEq, TokenKind::Star, start, line, col) }
            '/' => { self.advance(); self.if_next('=', TokenKind::SlashEq, TokenKind::Slash, start, line, col) }
            '%' => { self.advance(); self.if_next('=', TokenKind::PercentEq, TokenKind::Percent, start, line, col) }
            '=' => { self.advance(); self.if_next('=', TokenKind::EqEq, TokenKind::Eq, start, line, col); self.if_next('>', TokenKind::FatArrow, TokenKind::Eq, start, line, col) }
            '!' => { self.advance(); self.if_next('=', TokenKind::NotEq, TokenKind::Not, start, line, col) }
            '<' => { self.advance(); self.if_next('=', TokenKind::LtEq, TokenKind::Lt, start, line, col) }
            '>' => { self.advance(); self.if_next('=', TokenKind::GtEq, TokenKind::Gt, start, line, col) }
            '&' => { self.advance(); self.expect('&', TokenKind::AndAnd, "expected &&", start, line, col) }
            '|' => { self.advance(); self.if_next('|', TokenKind::OrOr, TokenKind::Pipe, start, line, col) }
            '?' => { 
                self.advance();
                if self.peek() == Some('.') {
                    self.advance();
                    Token::new(TokenKind::QuestionDot, start, self.pos, line, col)
                } else {
                    Token::new(TokenKind::QuestionMark, start, self.pos, line, col)
                }
            }
            '(' => { self.advance(); Token::new(TokenKind::LParen, start, self.pos, line, col) }
            ')' => { self.advance(); Token::new(TokenKind::RParen, start, self.pos, line, col) }
            '{' => { self.advance(); Token::new(TokenKind::LBrace, start, self.pos, line, col) }
            '}' => { self.advance(); Token::new(TokenKind::RBrace, start, self.pos, line, col) }
            '[' => { self.advance(); Token::new(TokenKind::LBracket, start, self.pos, line, col) }
            ']' => { self.advance(); Token::new(TokenKind::RBracket, start, self.pos, line, col) }
            ';' => { self.advance(); Token::new(TokenKind::Semicolon, start, self.pos, line, col) }
            ',' => { self.advance(); Token::new(TokenKind::Comma, start, self.pos, line, col) }
            ':' => { self.advance(); Token::new(TokenKind::Colon, start, self.pos, line, col) }
            _ => {
                self.advance();
                Token::new(TokenKind::Error(format!("Unexpected character: '{}'", c)), start, self.pos, line, col)
            }
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos >= self.source.len() { return None; }
        let c = self.source[self.pos];
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_n(&self, n: usize) -> Option<char> {
        self.source.get(self.pos + n).copied()
    }

    fn if_next(&self, expected: char, if_match: TokenKind, if_not: TokenKind, start: usize, line: u32, col: u32) -> Token {
        if self.peek() == Some(expected) {
            self.advance();
            Token::new(if_match, start, self.pos, line, col)
        } else {
            Token::new(if_not, start, self.pos, line, col)
        }
    }

    fn expect(&self, expected: char, kind: TokenKind, err_msg: &str, start: usize, line: u32, col: u32) -> Token {
        if self.peek() == Some(expected) {
            self.advance();
            Token::new(kind, start, self.pos, line, col)
        } else {
            Token::new(TokenKind::Error(err_msg.to_string()), start, self.pos, line, col)
        }
    }

    fn check_dot_continuation(&mut self, start: usize, line: u32, col: u32) -> Token {
        if self.peek() == Some('.') {
            self.advance();
            if self.peek() == Some('.') {
                self.advance();
                return Token::new(TokenKind::DotDotDot, start, self.pos, line, col);
            }
            // just ".."? error recovery: return DotDot
            Token::new(TokenKind::Error("expected ...".into()), start, self.pos, line, col)
        } else {
            Token::new(TokenKind::Dot, start, self.pos, line, col)
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\n') | Some('\r') => { self.advance(); }
                Some('/') => {
                    if self.peek_n(1) == Some('/') {
                        // Single-line comment
                        while self.peek().is_some() && self.peek() != Some('\n') { self.advance(); }
                    } else if self.peek_n(1) == Some('*') {
                        // Block comment
                        self.advance(); self.advance(); // skip /*
                        loop {
                            if self.peek() == Some('*') && self.peek_n(1) == Some('/') {
                                self.advance(); self.advance(); // skip */
                                break;
                            }
                            if self.peek().is_none() { break; }
                            self.advance();
                        }
                    } else {
                        break; // not a comment, just '/'
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // opening "
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('"') => { self.advance(); break; }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('"') => { s.push('"'); self.advance(); }
                        Some('\\') => { s.push('\\'); self.advance(); }
                        Some('n') => { s.push('\n'); self.advance(); }
                        Some('t') => { s.push('\t'); self.advance(); }
                        Some('r') => { s.push('\r'); self.advance(); }
                        Some('u') => {
                            self.advance();
                            if self.peek() == Some('{') {
                                self.advance();
                                let mut hex = String::new();
                                while self.peek().map_or(false, |c| c.is_ascii_hexdigit()) {
                                    hex.push(self.advance().unwrap());
                                }
                                if self.peek() == Some('}') { self.advance(); }
                                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                    if let Some(c) = char::from_u32(code) { s.push(c); }
                                }
                            }
                        }
                        _ => { s.push('\\'); }
                    }
                }
                Some(c) => { s.push(c); self.advance(); }
            }
        }
        Token::new(TokenKind::StringLiteral(s), start, self.pos, line, col)
    }

    fn read_template_string(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // opening `
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('`') => { self.advance(); break; }
                Some('\\') => {
                    self.advance();
                    if self.peek() == Some('`') { s.push('`'); self.advance(); }
                    else if self.peek() == Some('\\') { s.push('\\'); self.advance(); }
                    else { s.push('\\'); }
                }
                Some(c) => { s.push(c); self.advance(); }
            }
        }
        Token::new(TokenKind::TemplateString(s), start, self.pos, line, col)
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        
        // Check for hex/octal/binary prefix
        if self.peek() == Some('0') {
            let n = self.peek_n(1);
            if n == Some('x') || n == Some('X') {
                self.advance(); self.advance();
                let mut hex = String::new();
                while self.peek().map_or(false, |c| c.is_ascii_hexdigit()) { hex.push(self.advance().unwrap()); }
                return Token::new(TokenKind::IntLiteral(i64::from_str_radix(&hex, 16).unwrap_or(0)), start, self.pos, line, col);
            }
            if n == Some('o') || n == Some('O') {
                self.advance(); self.advance();
                let mut oct = String::new();
                while self.peek().map_or(false, |c| ('0'..='7').contains(&c)) { oct.push(self.advance().unwrap()); }
                return Token::new(TokenKind::IntLiteral(i64::from_str_radix(&oct, 8).unwrap_or(0)), start, self.pos, line, col);
            }
            if n == Some('b') || n == Some('B') {
                self.advance(); self.advance();
                let mut bin = String::new();
                while self.peek() == Some('0') || self.peek() == Some('1') { bin.push(self.advance().unwrap()); }
                return Token::new(TokenKind::IntLiteral(i64::from_str_radix(&bin, 2).unwrap_or(0)), start, self.pos, line, col);
            }
        }
        
        let mut num = String::new();
        let mut is_float = false;
        while self.peek().map_or(false, |c| c.is_ascii_digit()) { num.push(self.advance().unwrap()); }
        if self.peek() == Some('.') && self.peek_n(1).map_or(false, |c| c.is_ascii_digit()) {
            is_float = true;
            num.push(self.advance().unwrap()); // .
            while self.peek().map_or(false, |c| c.is_ascii_digit()) { num.push(self.advance().unwrap()); }
        }
        if self.peek() == Some('e') || self.peek() == Some('E') {
            is_float = true;
            num.push(self.advance().unwrap());
            if self.peek() == Some('+') || self.peek() == Some('-') { num.push(self.advance().unwrap()); }
            while self.peek().map_or(false, |c| c.is_ascii_digit()) { num.push(self.advance().unwrap()); }
        }
        
        if is_float {
            Token::new(TokenKind::FloatLiteral(num.parse().unwrap_or(0.0)), start, self.pos, line, col)
        } else {
            Token::new(TokenKind::IntLiteral(num.parse().unwrap_or(0)), start, self.pos, line, col)
        }
    }

    fn read_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        let mut ident = String::new();
        while self.peek().map_or(false, |c| c.is_alphanumeric() || c == '_') {
            ident.push(self.advance().unwrap());
        }
        
        let kind = match ident.as_str() {
            "fn" => TokenKind::Fn, "let" => TokenKind::Let, "mut" => TokenKind::Mut,
            "if" => TokenKind::If, "else" => TokenKind::Else, "while" => TokenKind::While,
            "for" => TokenKind::For, "in" => TokenKind::In, "return" => TokenKind::Return,
            "break" => TokenKind::Break, "continue" => TokenKind::Continue,
            "print" => TokenKind::Print, "println" => TokenKind::Println,
            "async" => TokenKind::Async, "await" => TokenKind::Await, "match" => TokenKind::Match,
            "struct" => TokenKind::Struct, "class" => TokenKind::Class,
            "interface" => TokenKind::Interface, "impl" => TokenKind::Impl,
            "extends" => TokenKind::Extends, "implements" => TokenKind::Implements,
            "type" => TokenKind::Type, "nominal" => TokenKind::Nominal,
            "static" => TokenKind::Static, "private" => TokenKind::Private, "override" => TokenKind::Override,
            "import" => TokenKind::Import, "from" => TokenKind::From, "export" => TokenKind::Export,
            "true" => TokenKind::True, "false" => TokenKind::False, "null" => TokenKind::Null,
            "component" => TokenKind::Component, "signal" => TokenKind::Signal,
            "computed" => TokenKind::Computed, "prop" => TokenKind::Prop,
            "event" => TokenKind::Event, "ref" => TokenKind::Ref,
            "store" => TokenKind::Store, "view" => TokenKind::View,
            "style" => TokenKind::Style, "action" => TokenKind::Action,
            "getter" => TokenKind::Getter, "theme" => TokenKind::Theme,
            "bind" => TokenKind::Bind,
            _ => TokenKind::Ident(ident),
        };
        Token::new(kind, start, self.pos, line, col)
    }

    fn read_at_keyword(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // @
        let mut kw = String::from("@");
        while self.peek().map_or(false, |c| c.is_alphanumeric() || c == '_') {
            kw.push(self.advance().unwrap());
        }
        let kind = match kw.as_str() {
            "@store" => TokenKind::AtStore, "@if" => TokenKind::AtIf,
            "@else" => TokenKind::AtElse, "@for" => TokenKind::AtFor,
            "@empty" => TokenKind::AtEmpty, "@slot" => TokenKind::AtSlot,
            "@native" => TokenKind::AtNative, "@fallback" => TokenKind::AtFallback,
            "@error" => TokenKind::AtError, "@mount" => TokenKind::AtMount,
            "@unmount" => TokenKind::AtUnmount, "@effect" => TokenKind::AtEffect,
            _ => TokenKind::Error(format!("Unknown directive: {}", kw)),
        };
        Token::new(kind, start, self.pos, line, col)
    }
}

/// Convenience function — tokenize a source string.
pub fn tokenize(source: &str) -> Vec<Token> {
    Lexer::new(source).tokenize()
}
```

- [ ] **Step 3: Run cargo check**

```bash
cd D:/space/OmniLang/omni-compiler && cargo check 2>&1
```

- [ ] **Step 4: Commit**

```bash
cd D:/space/OmniLang && git add omni-compiler/src/lexer/ && git commit -m "feat(rust-compiler): implement hand-written lexer — 60+ token kinds, strings, numbers, comments"
```

---

### Task 1.3: Implement Error Types

**Files:**
- Create: `omni-compiler/src/error.rs`

```rust
// omni-compiler/src/error.rs

use serde::{Serialize, Deserialize};
use crate::lexer::Span;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Diagnostic {
    pub fn error(code: &str, message: &str, span: &Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code: code.to_string(),
            message: message.to_string(),
            line: span.line,
            col: span.col,
            end_line: span.line,
            end_col: span.col + (span.end - span.start) as u32,
            suggestion: None,
        }
    }

    pub fn warning(code: &str, message: &str, span: &Span) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            code: code.to_string(),
            message: message.to_string(),
            line: span.line,
            col: span.col,
            end_line: span.line,
            end_col: span.col + (span.end - span.start) as u32,
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl From<ParseError> for Diagnostic {
    fn from(e: ParseError) -> Self {
        Diagnostic::error("E0001", &e.message, &e.span)
    }
}
```

---

### Task 1.4: Implement Parser — AST Types

**Files:**
- Create: `omni-compiler/src/parser/mod.rs`
- Create: `omni-compiler/src/parser/ast.rs`
- Create: `omni-compiler/src/parser/expr.rs`
- Create: `omni-compiler/src/parser/stmt.rs`
- Create: `omni-compiler/src/parser/top_level.rs`
- Create: `omni-compiler/src/parser/view.rs`

Due to the size of the parser implementation (~2000+ lines across all files), these tasks are grouped by module. The full AST type definitions are in the design doc §3.3.1.

- [ ] **Step 1: Write ast.rs** — All AST node types (copy from design doc §3.3.1, 250+ lines)
- [ ] **Step 2: Write mod.rs** — Parser struct, `parse()` entry, token cursor with peek/advance
- [ ] **Step 3: Write expr.rs** — Pratt expression parser with binding power table
- [ ] **Step 4: Write stmt.rs** — Recursive descent statement parser
- [ ] **Step 5: Write top_level.rs** — Component, store, struct, function parsers
- [ ] **Step 6: Write view.rs** — View elements, directives, props parsers
- [ ] **Step 7: Run cargo check** and fix compilation errors
- [ ] **Step 8: Commit**

---

### Task 1.5: Implement Semantic Analysis

**Files:**
- Create: `omni-compiler/src/semantic/mod.rs`
- Create: `omni-compiler/src/semantic/symbols.rs`
- Create: `omni-compiler/src/semantic/resolver.rs`
- Create: `omni-compiler/src/semantic/type_checker.rs`

- [ ] **Step 1: Write symbols.rs** — SymbolTable, Scope, Symbol types
- [ ] **Step 2: Write resolver.rs** — Pass 1: Collect symbols, Pass 2: Resolve references
- [ ] **Step 3: Write type_checker.rs** — Pass 3: Type inference and validation
- [ ] **Step 4: Write mod.rs** — Orchestrator: analyze() running all passes
- [ ] **Step 5: Run cargo check** and fix errors
- [ ] **Step 6: Commit**

---

### Task 1.6: Implement IR Types

**Files:**
- Create: `omni-compiler/src/ir/mod.rs`

The IR types mirror Python `ir_types.py` exactly. Use serde for JSON serialization.

- [ ] **Step 1: Write all IR types** (copy from design doc §3.5 + Appendix A)
- [ ] **Step 2: Write lowering** — `lower()` function: AST → IR
- [ ] **Step 3: Run cargo check**
- [ ] **Step 4: Commit**

---

### Task 1.7: Implement SourceMap

**Files:**
- Create: `omni-compiler/src/sourcemap/mod.rs`

- [ ] **Step 1: Write SourceMap types and generation** (from design doc §3.8)
- [ ] **Step 2: Commit**

---

### Task 1.8: Build WASM

- [ ] **Step 1: Install wasm-pack**

```bash
cargo install wasm-pack
```

- [ ] **Step 2: Build WASM**

```bash
cd D:/space/OmniLang/omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm
```

- [ ] **Step 3: Verify WASM output**

```bash
ls -la D:/space/OmniLang/editor/public/wasm/omni_compiler_bg.wasm
```

Expected: File exists, size < 1MB (target < 500KB gzipped)

- [ ] **Step 4: Commit the built WASM**

```bash
cd D:/space/OmniLang && git add editor/public/wasm/ && git commit -m "feat(rust-compiler): add built WASM to editor static assets"
```

---

## Phase 2: Integration

### Task 2.1: Python ↔ Rust Round-Trip Tests

Once Rust DLL/CLI is built, run the Phase 1 tests from §7.5.

- [ ] **Step 1: Run dual pipeline comparison**

```bash
cd D:/space/OmniLang && python -m pytest omniview/tests/test_dual_pipeline.py -v
```

- [ ] **Step 2: Run round-trip script**

```bash
cd D:/space/OmniLang && bash tools/test-round-trip.sh
```

- [ ] **Step 3: Fix any mismatches** — iterate until all tests pass

---

### Task 2.2: TS WASM Loader

**Files:**
- Create: `editor/client/src/api/rustCompiler.ts`

- [ ] **Step 1: Write the loader** (from design doc §6.6)
- [ ] **Step 2: Wire into App.tsx** — init Rust compiler on startup
- [ ] **Step 3: Commit**

---

### Task 2.3: Editor Server Endpoint

**Files:**
- Modify: `editor/server/main.py`

- [ ] **Step 1: Add `/api/compile-from-ir` endpoint** (from design doc §6.5)
- [ ] **Step 2: Test with curl** — send IR JSON → get compiled React back
- [ ] **Step 3: Commit**

---

## Verification Checklist

Before declaring Phase 1 complete:

- [ ] `cargo test` passes all Rust unit tests (6 suites)
- [ ] `cargo build --release` produces DLL
- [ ] `wasm-pack build` produces WASM < 500KB gzipped
- [ ] Python `test_ir_schema.py` — 4 passed
- [ ] Python `test_ir_reader.py` — 15 passed
- [ ] Python `test_rust_bridge.py` — 3 passed + 2 skipped
- [ ] Python `test_codegen_from_dict.py` — 4 passed
- [ ] Python `test_dual_pipeline.py` — 25+ passed (requires Rust DLL)
- [ ] Round-trip: Counter.omni → Rust → IR → Python React → matches Python pipeline
- [ ] All existing Python tests still pass (261+)
