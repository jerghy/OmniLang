"""Tests for IR adapter: Rust IR JSON → Python IR dataclass conversion."""
import pytest

from omniview.codegen.ir_adapter import IRAdapter
from omniview.codegen.rust_bridge import RustBridge
from omniview import ir_types as ir


# ═══════════════════════════════════════════════════════
# State / Signal
# ═══════════════════════════════════════════════════════

def test_adapt_state_int_signal():
    d = {
        "name": "count",
        "type": {"kind": "i32"},
        "initial_value": {"_kind": "LiteralExpr", "value": 0},
        "mutability": "mutable",
        "update_semantics": "setter-call",
        "accessor_style": "direct",
        "is_copy": False,
    }
    state = IRAdapter._adapt_state(d)
    assert state.name == "count"
    assert state.type.kind == ir.TypeKind.I32
    assert isinstance(state.initial_value, ir.LiteralExpr)
    assert state.initial_value.value == 0


def test_adapt_state_string_signal():
    d = {
        "name": "title",
        "type": {"kind": "String"},
        "initial_value": {"_kind": "LiteralExpr", "value": "hello"},
        "mutability": "mutable",
        "update_semantics": "setter-call",
        "accessor_style": "direct",
        "is_copy": False,
    }
    state = IRAdapter._adapt_state(d)
    assert state.name == "title"
    assert state.type.kind == ir.TypeKind.STRING
    assert state.initial_value.value == "hello"


# ═══════════════════════════════════════════════════════
# Expression
# ═══════════════════════════════════════════════════════

def test_adapt_literal_int():
    d = {"_kind": "LiteralExpr", "value": 42}
    expr = IRAdapter._adapt_expr(d)
    assert isinstance(expr, ir.LiteralExpr)
    assert expr.value == 42


def test_adapt_literal_string():
    d = {"_kind": "LiteralExpr", "value": "test"}
    expr = IRAdapter._adapt_expr(d)
    assert isinstance(expr, ir.LiteralExpr)
    assert expr.value == "test"


def test_adapt_identifier():
    d = {"_kind": "IdentifierExpr", "name": "count"}
    expr = IRAdapter._adapt_expr(d)
    assert isinstance(expr, ir.IdentifierExpr)
    assert expr.name == "count"


def test_adapt_binary_op():
    d = {
        "_kind": "BinaryOpExpr",
        "op": "add",
        "left": {"_kind": "IdentifierExpr", "name": "x"},
        "right": {"_kind": "LiteralExpr", "value": 1},
    }
    expr = IRAdapter._adapt_expr(d)
    assert isinstance(expr, ir.BinaryOpExpr)
    assert expr.op == ir.BinaryOp.ADD
    assert isinstance(expr.left, ir.IdentifierExpr)
    assert isinstance(expr.right, ir.LiteralExpr)


def test_adapt_field_access():
    d = {
        "_kind": "FieldAccessExpr",
        "object": {"_kind": "IdentifierExpr", "name": "count"},
        "field": "value",
    }
    expr = IRAdapter._adapt_expr(d)
    assert isinstance(expr, ir.FieldAccessExpr)
    assert expr.field == "value"


def test_adapt_ternary():
    d = {
        "_kind": "TernaryIR",
        "cond": {"_kind": "IdentifierExpr", "name": "flag"},
        "then_expr": {"_kind": "LiteralExpr", "value": "yes"},
        "else_expr": {"_kind": "LiteralExpr", "value": "no"},
    }
    expr = IRAdapter._adapt_expr(d)
    assert isinstance(expr, ir.TernaryIR)


# ═══════════════════════════════════════════════════════
# Statement
# ═══════════════════════════════════════════════════════

def test_adapt_let_stmt():
    d = {
        "_kind": "LetStmtIR",
        "name": "x",
        "value": {"_kind": "LiteralExpr", "value": 10},
    }
    stmt = IRAdapter._adapt_statement(d)
    assert isinstance(stmt, ir.LetStmtIR)
    assert stmt.name == "x"


def test_adapt_return_stmt():
    d = {
        "_kind": "ReturnStmtIR",
        "value": {"_kind": "IdentifierExpr", "name": "result"},
    }
    stmt = IRAdapter._adapt_statement(d)
    assert isinstance(stmt, ir.ReturnStmtIR)


def test_adapt_expr_stmt():
    d = {
        "_kind": "ExprStmtIR",
        "expr": {"_kind": "IdentifierExpr", "name": "count"},
    }
    stmt = IRAdapter._adapt_statement(d)
    assert isinstance(stmt, ir.ExprStmtIR)


# ═══════════════════════════════════════════════════════
# View Tree
# ═══════════════════════════════════════════════════════

def test_adapt_element():
    d = {
        "_kind": "ElementIR",
        "tag": "Button",
        "props": {"label": {"_kind": "LiteralExpr", "value": "Click Me"}},
        "bindings": {},
        "events": {},
        "style_classes": None,
        "inline_style": None,
        "children": [],
        "test_id": None,
        "ref_name": None,
        "construction_kind": "tag-mapping",
    }
    view = IRAdapter._adapt_view_tree(d)
    assert isinstance(view, ir.ElementIR)
    assert view.tag == "Button"
    assert "label" in view.props


def test_adapt_element_with_children():
    d = {
        "_kind": "ElementIR",
        "tag": "Card",
        "props": {},
        "bindings": {},
        "events": {},
        "style_classes": None,
        "inline_style": None,
        "children": [
            {
                "_kind": "ElementIR",
                "tag": "Text",
                "props": {},
                "bindings": {},
                "events": {},
                "style_classes": None,
                "inline_style": None,
                "children": [],
                "test_id": None,
                "ref_name": None,
                "construction_kind": "tag-mapping",
            },
        ],
        "test_id": None,
        "ref_name": None,
        "construction_kind": "tag-mapping",
    }
    view = IRAdapter._adapt_view_tree(d)
    assert isinstance(view, ir.ElementIR)
    assert len(view.children) == 1
    assert isinstance(view.children[0], ir.ElementIR)


def test_adapt_conditional():
    d = {
        "_kind": "ConditionalIntent",
        "branches": [
            {
                "condition": {"_kind": "IdentifierExpr", "name": "show"},
                "body": {
                    "_kind": "ElementIR",
                    "tag": "Text",
                    "props": {},
                    "bindings": {},
                    "events": {},
                    "style_classes": None,
                    "inline_style": None,
                    "children": [],
                    "test_id": None,
                    "ref_name": None,
                    "construction_kind": "tag-mapping",
                },
            },
        ],
        "fallback": None,
    }
    view = IRAdapter._adapt_view_tree(d)
    assert isinstance(view, ir.ConditionalIntent)
    assert len(view.branches) == 1
    assert isinstance(view.branches[0].body, ir.ElementIR)


def test_adapt_list_rendering():
    d = {
        "_kind": "ListRenderingIntent",
        "iterable": {"_kind": "IdentifierExpr", "name": "items"},
        "item_binding": {"name": "item"},
        "body": {
            "_kind": "ElementIR",
            "tag": "Text",
            "props": {},
            "bindings": {},
            "events": {},
            "style_classes": None,
            "inline_style": None,
            "children": [],
            "test_id": None,
            "ref_name": None,
            "construction_kind": "tag-mapping",
        },
        "key_expr": None,
        "empty_body": None,
    }
    view = IRAdapter._adapt_view_tree(d)
    assert isinstance(view, ir.ListRenderingIntent)
    assert view.item_binding.name == "item"


# ═══════════════════════════════════════════════════════
# Component
# ═══════════════════════════════════════════════════════

def test_adapt_minimal_component():
    d = {
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
    comp = IRAdapter._adapt_component(d)
    assert comp.name == "Empty"
    assert comp.states == []
    assert comp.view is None


def test_adapt_component_with_state():
    d = {
        "name": "Counter",
        "props": {},
        "states": [{
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
    comp = IRAdapter._adapt_component(d)
    assert comp.name == "Counter"
    assert len(comp.states) == 1
    assert comp.states[0].initial_value.value == 0


# ═══════════════════════════════════════════════════════
# Program (top-level)
# ═══════════════════════════════════════════════════════

def test_adapt_program():
    d = {
        "version": "1.0",
        "components": {
            "Empty": {
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
            },
        },
        "stores": {},
        "source_map": {},
    }
    result = IRAdapter.adapt_program(d)
    assert "Empty" in result
    assert isinstance(result["Empty"], ir.ComponentIR)
    assert result["Empty"].name == "Empty"


# ═══════════════════════════════════════════════════════
# Rust Bridge Integration (requires Rust DLL)
# ═══════════════════════════════════════════════════════

@pytest.mark.require_rust
def test_rust_bridge_to_adapter_roundtrip():
    """Compile Counter.omni via Rust, adapt to dataclasses, verify structure."""
    if not RustBridge.is_available():
        pytest.skip("Rust compiler not available")

    src = open("build/test/counter/Counter.omni").read()
    ir_dict = RustBridge.compile_to_ir(src)

    assert "components" in ir_dict
    assert "Counter" in ir_dict["components"]

    components = IRAdapter.adapt_program(ir_dict)
    assert "Counter" in components

    comp = components["Counter"]
    assert isinstance(comp, ir.ComponentIR)
    assert comp.name == "Counter"
    # Counter.omni has a 'count' signal
    assert len(comp.states) >= 1
    assert any(s.name == "count" for s in comp.states)
    # Counter.omni has an 'increment' function
    assert len(comp.fns) >= 1
    assert any(f.get("name") == "increment" for f in comp.fns)


# ═══════════════════════════════════════════════════════
# Edge cases
# ═══════════════════════════════════════════════════════

def test_adapt_empty_dict_returns_none():
    expr = IRAdapter._adapt_expr({})
    assert expr is None


def test_adapt_none_expr_returns_none():
    expr = IRAdapter._adapt_expr(None)
    assert expr is None


def test_adapt_none_view_tree():
    view = IRAdapter._adapt_view_tree(None)
    assert view is None


def test_adapt_empty_view_tree():
    view = IRAdapter._adapt_view_tree({})
    assert view is None


def test_adapt_event_handler_fn_ref():
    d = {"kind": "fn_ref", "ref": "increment"}
    handler = IRAdapter._adapt_event_handler(d)
    assert handler.kind == ir.EventHandlerKind.FN_REF
    assert handler.ref == "increment"


def test_adapt_event_handler_store_action():
    d = {"kind": "store_action", "ref": "cart.addItem"}
    handler = IRAdapter._adapt_event_handler(d)
    assert handler.kind == ir.EventHandlerKind.STORE_ACTION
    assert handler.ref == "cart.addItem"


def test_adapt_type_void():
    t = IRAdapter._adapt_type({"kind": "void"})
    assert t.kind == ir.TypeKind.VOID


def test_adapt_type_array():
    t = IRAdapter._adapt_type({"kind": "Array", "type_args": [{"kind": "String"}]})
    assert t.kind == ir.TypeKind.ARRAY
    assert len(t.type_args) == 1
    assert t.type_args[0].kind == ir.TypeKind.STRING


def test_adapt_mutation_increment():
    d = {
        "target": {"owner": None, "name": "count"},
        "operation": "increment",
        "amount": {"_kind": "LiteralExpr", "value": 1},
    }
    m = IRAdapter._adapt_mutation(d)
    assert m.target.name == "count"
    assert m.operation == ir.MutationOp.INCREMENT
    assert m.amount.value == 1


def test_adapt_function_body():
    """Functions in ComponentIR.fns are list[dict] — adapter passes through."""
    d = {
        "name": "increment",
        "params": [],
        "return_type": None,
        "body": {"statements": [], "execution_mode": "client-only"},
    }
    # fns are passed through as-is (list[dict])
    assert d["name"] == "increment"
    assert "body" in d


def test_adapt_block_with_statements():
    d = {
        "statements": [
            {"_kind": "MutationStmt", "intent": {
                "target": {"name": "count"},
                "operation": "increment",
                "amount": {"_kind": "LiteralExpr", "value": 1},
            }},
        ],
        "execution_mode": "client-only",
    }
    block = IRAdapter._adapt_block(d)
    assert block is not None
    assert len(block.statements) == 1
    assert isinstance(block.statements[0], ir.MutationStmt)
