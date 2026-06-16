"""Test that IR dict structure matches ir_schema TypedDict definitions.

These tests validate: (1) Python pipeline IR output -> dict has _kind markers,
(2) nested IR structures are recursively valid, (3) all required fields exist.
No Rust dependency needed.

Note: Helper types (SignalRefIR, BindingIR, ConditionalBranch, TypeIR, etc.)
do NOT carry _kind markers - they are pure data containers, not dispatch targets.
Only dispatch-target IR types (ElementIR, StateIntent, ConditionalIntent, etc.)
have _kind markers.
"""
from __future__ import annotations

import pytest
from omniview.tests.helpers import ir_dataclass_to_dict, make_counter_ir_dict


# IR types that carry _kind markers (matches dataclass names in ir_types.py)
_KIND_TYPES = {
    # Expressions
    "ExprIR", "LiteralExpr", "IdentifierExpr", "FieldAccessExpr", "BinaryOpExpr",
    "UnaryOpExpr", "TernaryIR", "StoreFieldAccessExpr",
    # References & handlers
    "SignalRefIR", "BindingIR", "EventHandlerIR", "ComponentInstanceIR",
    # Types
    "TypeIR",
    # Blocks & Statements
    "BlockIR", "MutationStmt", "EventEmitStmtIR", "ReturnStmtIR",
    "ExprStmtIR", "IfStmtIR", "LetStmtIR",
    # Style
    "ClassNameExpr", "StyleIntent", "LayoutIntent", "StyleRuleIR",
    # Core Intents
    "StateIntent", "MutationIntent", "ComputedIntent",
    "EffectIntent", "LifecycleIntent",
    "ConditionalBranch", "ConditionalIntent",
    "ListRenderingIntent", "BindIntent", "EventIntent", "SlotIntent",
    # Element
    "ElementIR",
    # Top-level
    "ComponentIR", "StoreIR",
    # Codegen
    "FunctionIR",
}


def validate_ir_tree(obj, path: str = ""):
    """Recursively validate: every dict with _kind must have a recognized kind value."""
    if isinstance(obj, dict):
        if "_kind" in obj:
            kind = obj["_kind"]
            assert kind in _KIND_TYPES, \
                f"Unknown _kind '{kind}' at {path}"
        for k, v in obj.items():
            validate_ir_tree(v, f"{path}.{k}")
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            validate_ir_tree(item, f"{path}[{i}]")


def validate_component_ir(d: dict):
    """Validate ComponentIR structure."""
    assert d["_kind"] == "ComponentIR"
    assert isinstance(d["name"], str)
    assert isinstance(d["states"], list)
    assert isinstance(d["computeds"], list)
    assert isinstance(d["fns"], list)
    assert isinstance(d["view"], (dict, type(None)))


def validate_element_ir(d: dict):
    """Validate ElementIR and its children recursively."""
    assert d["_kind"] == "ElementIR"
    assert isinstance(d["tag"], str)
    assert isinstance(d["props"], dict)
    assert isinstance(d["children"], list)
    # Recursively validate all children (should be IR types)
    for child in d["children"]:
        if isinstance(child, dict):
            validate_ir_tree(child)


# ═══════════════════════════════════════════════════════════
# Tests
# ═══════════════════════════════════════════════════════════

def test_schema_matches_dataclass_for_counter():
    """Python pipeline IR output -> dict -> _kind check, name check, states existence."""
    ir = make_counter_ir_dict()
    comp_ir = ir["components"]["Counter"]

    validate_component_ir(comp_ir)
    assert comp_ir["_kind"] == "ComponentIR"
    assert comp_ir["name"] == "Counter"
    assert len(comp_ir["states"]) >= 1

    for state in comp_ir["states"]:
        assert state["_kind"] == "StateIntent"
        assert "name" in state
        assert "type" in state
        assert "initial_value" in state


def test_element_ir_structure():
    """ElementIR dict structure correct - all required fields present, children recursive."""
    ir = {
        "_kind": "ElementIR",
        "tag": "Button",
        "props": {"label": {"_kind": "LiteralExpr", "value": "Submit"}},
        "bindings": {},
        "events": {"onPress": {"_kind": "EventHandlerIR", "kind": "fn_ref", "ref": "handleClick"}},
        "style_classes": None,
        "inline_style": None,
        "children": [
            {"_kind": "ElementIR", "tag": "Text", "props": {}, "bindings": {},
             "events": {}, "style_classes": None, "inline_style": None,
             "children": [], "test_id": None, "ref_name": None,
             "construction_kind": "tag-mapping"}
        ],
        "test_id": None,
        "ref_name": None,
        "construction_kind": "tag-mapping",
    }
    validate_element_ir(ir)
    validate_ir_tree(ir)


def test_conditional_intent_structure():
    """@if/@else -> ConditionalIntent branches list structure."""
    ir = {
        "_kind": "ConditionalIntent",
        "branches": [
            {
                "condition": {"_kind": "IdentifierExpr", "name": "isLoggedIn"},
                "body": {"_kind": "ElementIR", "tag": "Text", "props": {},
                         "bindings": {}, "events": {}, "children": [], "test_id": None,
                         "ref_name": None, "construction_kind": "tag-mapping",
                         "style_classes": None, "inline_style": None},
            },
        ],
        "fallback": None,
    }
    # ConditionalIntent should have _kind
    assert ir["_kind"] == "ConditionalIntent"
    assert len(ir["branches"]) == 1
    assert ir["branches"][0]["condition"]["_kind"] == "IdentifierExpr"
    assert ir["branches"][0]["body"]["_kind"] == "ElementIR"
    validate_ir_tree(ir)


def test_list_rendering_intent_structure():
    """@for item in items -> ListRenderingIntent structure."""
    ir = {
        "_kind": "ListRenderingIntent",
        "iterable": {"_kind": "IdentifierExpr", "name": "items"},
        "item_binding": {"name": "item"},
        "body": None,
        "key_expr": None,
        "empty_body": None,
    }
    assert ir["_kind"] == "ListRenderingIntent"
    assert ir["iterable"]["_kind"] == "IdentifierExpr"
    assert ir["item_binding"]["name"] == "item"
    validate_ir_tree(ir)


def test_all_counter_states_are_state_intents():
    """Every state in Counter IR is a StateIntent with required fields."""
    ir = make_counter_ir_dict()
    comp = ir["components"]["Counter"]
    for state in comp["states"]:
        assert state["_kind"] == "StateIntent"
        assert isinstance(state["name"], str)
        assert isinstance(state["mutability"], str)
        assert isinstance(state["update_semantics"], str)
        assert isinstance(state["accessor_style"], str)


def test_counter_view_is_element_or_conditional():
    """Counter view is an ElementIR or ConditionalIntent."""
    ir = make_counter_ir_dict()
    comp = ir["components"]["Counter"]
    view = comp.get("view")
    if view is not None:
        assert view["_kind"] in ("ElementIR", "ConditionalIntent", "ListRenderingIntent")
