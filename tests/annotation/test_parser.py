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

    def test_element_without_edit_parses_ok(self):
        """Parser extracts elements regardless of @edit presence -- validator catches missing @edit."""
        source = """// @code-tab type=logic
component Foo {
    view {
        <div id="box"></div>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        # Parser successfully parsed (no view_markers since no @edit lines)
        assert len(result.view_markers) == 0


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

    def test_parse_chain_without_separator(self):
        """Chain block that ends at EOF without closing ══ separator."""
        source = """// @code-tab type=logic
// @chain eofChain
//   @node n1  add  pos=50,50
//   @node n2  return  pos=150,50
//   @wire w1  n1.result -> n2.value
fn eofChain() { }
"""
        result = parse_annotations(source, "test.omni")
        assert len(result.chains) == 1
        chain = result.chains[0]
        assert chain.name == "eofChain"
        assert len(chain.nodes) == 2
        assert len(chain.wires) == 1
