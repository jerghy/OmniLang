"""Tests for omniview.codegen.source — reverse codegen (editor state → .omni source)."""

import json
import pytest
from omniview.codegen.source import OmniViewSourceCodegen


def make_document(
    design_elements=None,
    design_wires=None,
    code_tabs=None,
    component_name="App",
    config=None,
):
    """Helper: construct a minimal .omni.json-like document dict."""
    return {
        "design": {
            "elements": design_elements or [],
            "wires": design_wires or [],
        },
        "codeTabs": {
            "tabs": code_tabs or [],
            "openTabs": [t["id"] for t in (code_tabs or [])] or ["main"],
            "activeTab": "main",
        },
        "config": config or {"target": "react"},
        "meta": {"projectName": component_name},
    }


# ── Helpers for building element dicts ──

def _el(id_, tag, props=None, children=None, parentId=None, **kwargs):
    """Build an ElementData-like dict."""
    el = {
        "id": id_,
        "tag": tag,
        "nodeType": kwargs.pop("nodeType", "view" if tag[0].isupper() else "codeblock"),
        "props": props or {},
        "children": children or [],
        "parentId": parentId,
        **kwargs,
    }
    return el


def _wire(id_, fromId, fromPort, toId, toPort):
    """Build a WireData-like dict."""
    return {
        "id": id_,
        "fromId": fromId,
        "fromPort": fromPort,
        "toId": toId,
        "toPort": toPort,
    }


def _tab(id_, type_, label, elements=None, wires=None, variables=None, icon="📄"):
    """Build a CodeTab-like dict."""
    return {
        "id": id_,
        "type": type_,
        "label": label,
        "icon": icon,
        "graph": {
            "elements": elements or [],
            "wires": wires or [],
        },
        "variables": variables or [],
    }


# ═══════════════════════════════════════════════════════════
# Test Cases
# ═══════════════════════════════════════════════════════════

class TestEmptyProject:
    """Test 1: Empty project → minimal component."""

    def test_empty(self):
        doc = make_document()
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc, "Empty")
        assert "component Empty {" in result
        assert "view {" in result

    def test_empty_uses_app_default(self):
        doc = make_document()
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "component App {" in result


class TestSignalDeclarations:
    """Test 2: Signal declarations from code tabs."""

    def test_single_signal(self):
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("s1", "signal", {"name": "count", "typeAnnotation": "i32"}),
            ],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "signal count: i32 = 0;" in result

    def test_signal_with_type(self):
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("s1", "signal", {"name": "price", "typeAnnotation": "f64"}),
            ],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "signal price: f64 = 0;" in result

    def test_signal_no_type_annotation_defaults_to_i32(self):
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("s1", "signal", {"name": "x"}),
            ],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "signal x: i32 = 0;" in result

    def test_signal_from_tab_variables(self):
        tab = _tab(
            "main", "logic", "Main",
            elements=[],
            variables=[{"name": "username", "type": "String", "defaultValue": "guest"}],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert 'signal username: String = "guest";' in result

    def test_signal_deduplication(self):
        """Signal with same name in graph + variables only appears once."""
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("s1", "signal", {"name": "count", "typeAnnotation": "i32"}),
            ],
            variables=[{"name": "count", "type": "i32", "defaultValue": 5}],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        # Should only appear once
        assert result.count("signal count") == 1

    def test_signal_from_source_field(self):
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("s1", "signal", {"name": "total"},
                    source="signal total: f64 = 0.0;"),
            ],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "signal total: f64 = 0.0;" in result

    def test_multiple_signals(self):
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("s1", "signal", {"name": "count", "typeAnnotation": "i32"}),
                _el("s2", "signal", {"name": "total", "typeAnnotation": "f64"}),
                _el("s3", "signal", {"name": "name", "typeAnnotation": "String"}),
            ],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "signal count: i32 = 0;" in result
        assert "signal total: f64 = 0;" in result
        assert "signal name: String = 0;" in result  # String defaults to 0


class TestViewTree:
    """Test 3: View tree generation."""

    def test_single_card(self):
        doc = make_document(design_elements=[
            _el("root", "Card", {"padding": "16px", "background": "#fff"}),
        ])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert '<Card padding="16px" background="#fff" />' in result

    def test_nested_elements(self):
        doc = make_document(design_elements=[
            _el("root", "Card", {"padding": "24px"}, children=["h1", "btn"]),
            _el("h1", "Heading", {"label": "Hello", "level": "2"}, parentId="root"),
            _el("btn", "Button", {"label": "Click", "variant": "primary"}, parentId="root"),
        ])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert '<Card padding="24px">' in result
        assert '<Heading level="2">' in result
        assert "Hello" in result
        assert '<Button variant="primary">Click</Button>' in result
        assert "</Card>" in result

    def test_self_closing_when_no_children(self):
        doc = make_document(design_elements=[
            _el("c1", "Container", {"width": "100%"}),
        ])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert '<Container width="100%" />' in result

    def test_button_with_label(self):
        doc = make_document(design_elements=[
            _el("b1", "Button", {"label": "Submit", "variant": "primary"}),
        ])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert '<Button variant="primary">Submit</Button>' in result

    def test_heading_with_level_string(self):
        doc = make_document(design_elements=[
            _el("h1", "Heading", {"label": "Title", "level": "2"}),
        ])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert 'level="2"' in result

    def test_flex_with_props(self):
        doc = make_document(design_elements=[
            _el("f1", "Flex", {"direction": "row", "gap": "16px"}),
        ])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert 'direction="row"' in result
        assert 'gap="16px"' in result


class TestEventBindings:
    """Test 4: Event bindings in view elements."""

    def test_onpress_binding(self):
        el = _el("b1", "Button", {"label": "+1", "variant": "primary"})
        el["config"] = {
            "_eventBindings": {
                "onPress": {"fnName": "handleIncrement", "paramBindings": {}},
            }
        }
        doc = make_document(design_elements=[el])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "onPress={handleIncrement}" in result

    def test_multiple_event_bindings(self):
        el = _el("input1", "Input", {"placeholder": "Enter text"})
        el["config"] = {
            "_eventBindings": {
                "onChange": {"fnName": "handleChange", "paramBindings": {}},
                "onFocus": {"fnName": "handleFocus", "paramBindings": {}},
            }
        }
        doc = make_document(design_elements=[el])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "onChange={handleChange}" in result
        assert "onFocus={handleFocus}" in result


class TestFunctionBodyReconstruction:
    """Test 5: customFn body reconstruction from wire graph."""

    def test_simple_set_from_literal(self):
        """literal(1) → signal.set → count.set(1);"""
        elements = [
            _el("fn1", "customFn", {"name": "increment"},
                config={"fnName": "increment", "fnParams": [], "fnReturnType": "void"}),
            _el("lit1", "literal", {}, config={"value": 1, "type": "i32"}),
            _el("sig1", "signal", {"name": "count", "typeAnnotation": "i32"}),
        ]
        wires = [
            _wire("w1", "lit1", "value", "sig1", "set"),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "action increment() {" in result
        assert "count.set(1);" in result

    def test_add_then_set(self):
        """signal.value → add.a, literal(1) → add.b, add.result → signal.set
        → count.set(count.value + 1);"""
        elements = [
            _el("fn1", "customFn", {"name": "inc"},
                config={"fnName": "inc", "fnParams": [], "fnReturnType": "void"}),
            _el("sig1", "signal", {"name": "count", "typeAnnotation": "i32"}),
            _el("lit1", "literal", {}, config={"value": 1, "type": "i32"}),
            _el("add1", "add"),
        ]
        wires = [
            _wire("w1", "sig1", "value", "add1", "a"),
            _wire("w2", "lit1", "value", "add1", "b"),
            _wire("w3", "add1", "result", "sig1", "set"),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "count.set(" in result
        # Should contain count.value + 1 (or similar expression)
        assert "count.value" in result

    def test_signal_set_with_complex_value(self):
        """signal.value → add.a, literal(999.99) → add.b, add.result → signal.set
        → cartTotal.set(cartTotal.value + 999.99);"""
        elements = [
            _el("fn1", "customFn", {"name": "handleAdd"},
                config={"fnName": "handleAdd", "fnParams": [], "fnReturnType": "void"}),
            _el("sig1", "signal", {"name": "cartTotal", "typeAnnotation": "f64"}),
            _el("lit1", "literal", {}, config={"value": 999.99, "type": "f64"}),
            _el("add1", "add"),
        ]
        wires = [
            _wire("w1", "sig1", "value", "add1", "a"),
            _wire("w2", "lit1", "value", "add1", "b"),
            _wire("w3", "add1", "result", "sig1", "set"),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "cartTotal.set(" in result
        assert "cartTotal.value" in result
        assert "999.99" in result

    def test_multiple_set_operations(self):
        """Two signals being set in the same function."""
        elements = [
            _el("fn1", "customFn", {"name": "handleClick"},
                config={"fnName": "handleClick", "fnParams": [], "fnReturnType": "void"}),
            _el("sigA", "signal", {"name": "a", "typeAnnotation": "i32"}),
            _el("sigB", "signal", {"name": "b", "typeAnnotation": "i32"}),
            _el("lit1", "literal", {}, config={"value": 5, "type": "i32"}),
            _el("lit2", "literal", {}, config={"value": 10, "type": "i32"}),
        ]
        wires = [
            _wire("w1", "lit1", "value", "sigA", "set"),
            _wire("w2", "lit2", "value", "sigB", "set"),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "a.set(5);" in result
        assert "b.set(10);" in result

    def test_customfn_with_params(self):
        """Function with parameters."""
        elements = [
            _el("fn1", "customFn", {"name": "handleSubmit"},
                config={
                    "fnName": "handleSubmit",
                    "fnParams": [{"name": "event", "type": "MouseEvent"}],
                    "fnReturnType": "void",
                }),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=[])
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "action handleSubmit(event: MouseEvent) {" in result

    def test_return_node(self):
        """customFn → literal → return.value"""
        elements = [
            _el("fn1", "customFn", {"name": "getValue"},
                config={
                    "fnName": "getValue",
                    "fnParams": [],
                    "fnReturnType": "i32",
                }),
            _el("lit1", "literal", {}, config={"value": 42, "type": "i32"}),
            _el("ret1", "return", {"name": "return"}),
        ]
        wires = [
            _wire("w1", "lit1", "value", "ret1", "value"),
        ]
        tab = _tab("main", "logic", "Main", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "fn getValue() -> i32 {" in result
        assert "return 42;" in result

    def test_fn_with_source_uses_source_verbatim(self):
        """When customFn has source field, use it instead of reconstructing."""
        elements = [
            _el("fn1", "customFn", {"name": "inc"},
                config={"fnName": "inc", "fnParams": [], "fnReturnType": "void"},
                source="action inc() {\n    count.set(count.value + 1);\n}"),
            _el("sig1", "signal", {"name": "count", "typeAnnotation": "i32"}),
            _el("lit1", "literal", {}, config={"value": 1}),
        ]
        wires = [
            _wire("w1", "lit1", "value", "sig1", "set"),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        # Should use the source verbatim
        assert "count.set(count.value + 1);" in result
        # Should NOT have the reconstructed version
        assert result.count("count.set") == 1

    def test_compare_node(self):
        """signal.value → compare.a, literal → compare.b → signal.set"""
        elements = [
            _el("fn1", "customFn", {"name": "check"},
                config={"fnName": "check", "fnParams": [], "fnReturnType": "void"}),
            _el("sig1", "signal", {"name": "count", "typeAnnotation": "i32"}),
            _el("lit1", "literal", {}, config={"value": 100, "type": "i32"}),
            _el("cmp1", "compare", {}, config={"op": ">"}),
            _el("sig2", "signal", {"name": "isMax", "typeAnnotation": "bool"}),
        ]
        wires = [
            _wire("w1", "sig1", "value", "cmp1", "a"),
            _wire("w2", "lit1", "value", "cmp1", "b"),
            _wire("w3", "cmp1", "result", "sig2", "set"),
        ]
        tab = _tab("logic1", "logic", "Logic", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "isMax.set" in result
        assert "count.value" in result

    def test_subtraction_operation(self):
        """count.value → sub.a, literal(1) → sub.b, sub → count.set"""
        elements = [
            _el("fn1", "customFn", {"name": "dec"},
                config={"fnName": "dec", "fnParams": [], "fnReturnType": "void"}),
            _el("sig1", "signal", {"name": "count", "typeAnnotation": "i32"}),
            _el("lit1", "literal", {}, config={"value": 1, "type": "i32"}),
            _el("sub1", "sub"),
        ]
        wires = [
            _wire("w1", "sig1", "value", "sub1", "a"),
            _wire("w2", "lit1", "value", "sub1", "b"),
            _wire("w3", "sub1", "result", "sig1", "set"),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "count.set" in result
        assert "-" in result


class TestRoundTrip:
    """Test 6: Round-trip consistency."""

    def test_counter_round_trip_structure(self):
        """Build a document matching Counter.omni structure and check output."""
        elements = [
            _el("root", "Card", {"padding": "24px"}, children=["h1", "txt", "btn"]),
            _el("h1", "Heading", {"label": "Counter Test", "level": "2"},
                parentId="root"),
            _el("txt", "Text", {"label": "Count: {count}", "fontSize": "16px"},
                parentId="root"),
            _el("btn", "Button", {"label": "+1", "variant": "primary"},
                parentId="root"),
        ]
        btn = elements[3]
        btn["config"] = {
            "_eventBindings": {
                "onPress": {"fnName": "increment", "paramBindings": {}},
            }
        }

        sig_el = _el("sig1", "signal", {"name": "count", "typeAnnotation": "i32"})

        code_elements = [
            _el("fn1", "customFn", {"name": "increment"},
                config={
                    "fnName": "increment",
                    "fnParams": [],
                    "fnReturnType": "void",
                }),
            sig_el,
            _el("lit1", "literal", {}, config={"value": 1, "type": "i32"}),
            _el("add1", "add"),
        ]
        code_wires = [
            _wire("w1", "sig1", "value", "add1", "a"),
            _wire("w2", "lit1", "value", "add1", "b"),
            _wire("w3", "add1", "result", "sig1", "set"),
        ]

        tab = _tab("event1", "event", "Increment", elements=code_elements, wires=code_wires)
        doc = make_document(
            design_elements=elements,
            code_tabs=[tab],
            component_name="Counter",
        )
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc, component_name="Counter")

        # Check structure
        assert "component Counter {" in result
        assert "signal count: i32 = 0;" in result
        assert "action increment() {" in result
        assert "view {" in result
        assert "<Card" in result
        assert "onPress={increment}" in result


class TestEdgeCases:
    """Test 7: Edge cases."""

    def test_empty_function_body(self):
        """customFn with no wires → empty body."""
        elements = [
            _el("fn1", "customFn", {"name": "empty"},
                config={"fnName": "empty", "fnParams": [], "fnReturnType": "void"}),
        ]
        tab = _tab("event1", "event", "Empty", elements=elements, wires=[])
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "action empty() {" in result

    def test_disconnected_nodes_ignored(self):
        """Orphan literal node (no wires) → not emitted in function body."""
        elements = [
            _el("fn1", "customFn", {"name": "fn"},
                config={"fnName": "fn", "fnParams": [], "fnReturnType": "void"}),
            _el("lit1", "literal", {}, config={"value": 999}),
            # No wires connecting lit1 to anything
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=[])
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        # Should not crash or include orphan nodes
        assert "action fn() {" in result

    def test_unknown_node_tag(self):
        """Unknown tag in expression trace should not crash."""
        elements = [
            _el("fn1", "customFn", {"name": "fn"},
                config={"fnName": "fn", "fnParams": [], "fnReturnType": "void"}),
            _el("unk1", "unknown_custom_tag", {}),
            _el("sig1", "signal", {"name": "x", "typeAnnotation": "i32"}),
        ]
        wires = [
            _wire("w1", "unk1", "value", "sig1", "set"),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        # Should not crash
        assert "component App {" in result

    def test_wire_with_missing_elements(self):
        """Wire referencing non-existent elements."""
        wires = [
            _wire("w1", "missing_a", "value", "missing_b", "set"),
        ]
        elements = [
            _el("fn1", "customFn", {"name": "fn"},
                config={"fnName": "fn", "fnParams": [], "fnReturnType": "void"}),
        ]
        tab = _tab("event1", "event", "Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        # Should not crash on missing elements
        assert "action fn() {" in result

    def test_multiple_tabs(self):
        """Multiple code tabs with different functions."""
        tab1 = _tab(
            "event1", "event", "Inc",
            elements=[
                _el("fn1", "customFn", {"name": "inc"},
                    config={"fnName": "inc", "fnParams": [], "fnReturnType": "void"}),
                _el("sig1", "signal", {"name": "count", "typeAnnotation": "i32"}),
            ],
        )
        tab2 = _tab(
            "event2", "event", "Dec",
            elements=[
                _el("fn2", "customFn", {"name": "dec"},
                    config={"fnName": "dec", "fnParams": [], "fnReturnType": "void"}),
            ],
        )
        doc = make_document(code_tabs=[tab1, tab2])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "action inc() {" in result
        assert "action dec() {" in result

    def test_logic_tab_generates_fn_not_action(self):
        """'logic' tab → fn keyword, 'event' tab → action keyword."""
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("fn1", "customFn", {"name": "compute"},
                    config={
                        "fnName": "compute",
                        "fnParams": [],
                        "fnReturnType": "f64",
                    }),
            ],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "fn compute() -> f64 {" in result
        assert "action " not in result

    def test_computed_node(self):
        """computed node in graph → computed declaration."""
        tab = _tab(
            "main", "logic", "Main",
            elements=[
                _el("c1", "computed", {"name": "total", "typeAnnotation": "f64"}),
            ],
        )
        doc = make_document(code_tabs=[tab])
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        assert "computed total() -> f64" in result


# ═══════════════════════════════════════════════════════════
# Integration test: Shopping Cart
# ═══════════════════════════════════════════════════════════

class TestShoppingCartIntegration:
    """Integration test matching the real ShoppingCart.omni structure."""

    def test_shopping_cart_signals(self):
        """All 5 signals from ShoppingCart appear."""
        signals = [
            ("cartCount", "i32"),
            ("cartTotal", "f64"),
            ("laptopQty", "i32"),
            ("headphoneQty", "i32"),
            ("mouseQty", "i32"),
        ]
        sig_els = [
            _el(f"s_{name}", "signal", {"name": name, "typeAnnotation": typ})
            for name, typ in signals
        ]
        tab = _tab("main", "logic", "Main", elements=sig_els)
        doc = make_document(code_tabs=[tab], component_name="ShoppingCart")
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)
        for name, typ in signals:
            assert f"signal {name}: {typ} = 0;" in result

    def test_shopping_cart_add_to_cart_function(self):
        """Reconstruct handleAddLaptop: 3 signals set with complex expressions."""
        elements = [
            _el("fn1", "customFn", {"name": "handleAddLaptop"},
                config={
                    "fnName": "handleAddLaptop",
                    "fnParams": [{"name": "event", "type": "MouseEvent"}],
                    "fnReturnType": "void",
                }),
            # cartCount signal
            _el("s_cc", "signal", {"name": "cartCount", "typeAnnotation": "i32"}),
            # cartTotal signal
            _el("s_ct", "signal", {"name": "cartTotal", "typeAnnotation": "f64"}),
            # laptopQty signal
            _el("s_lq", "signal", {"name": "laptopQty", "typeAnnotation": "i32"}),
            # Literals
            _el("lit1", "literal", {}, config={"value": 1, "type": "i32"}),
            _el("lit999", "literal", {}, config={"value": 999.99, "type": "f64"}),
            # Add nodes
            _el("add_cc", "add"),
            _el("add_ct", "add"),
            _el("add_lq", "add"),
        ]
        wires = [
            # cartCount += 1
            _wire("w1", "s_cc", "value", "add_cc", "a"),
            _wire("w2", "lit1", "value", "add_cc", "b"),
            _wire("w3", "add_cc", "result", "s_cc", "set"),
            # cartTotal += 999.99
            _wire("w4", "s_ct", "value", "add_ct", "a"),
            _wire("w5", "lit999", "value", "add_ct", "b"),
            _wire("w6", "add_ct", "result", "s_ct", "set"),
            # laptopQty += 1
            _wire("w7", "s_lq", "value", "add_lq", "a"),
            _wire("w8", "lit1", "value", "add_lq", "b"),
            _wire("w9", "add_lq", "result", "s_lq", "set"),
        ]
        tab = _tab("event1", "event", "Cart Events", elements=elements, wires=wires)
        doc = make_document(code_tabs=[tab], component_name="ShoppingCart")
        codegen = OmniViewSourceCodegen()
        result = codegen.generate(doc)

        # Should have the function signature
        assert "action handleAddLaptop(event: MouseEvent) {" in result

        # Should have all three set operations
        assert "cartCount.set" in result
        assert "cartTotal.set" in result
        assert "laptopQty.set" in result

        # Should contain the signal.value references
        assert "cartCount.value" in result
        assert "cartTotal.value" in result
        assert "laptopQty.value" in result

        # Should contain the literal values
        assert "999.99" in result
