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
