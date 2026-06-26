"""Tests for annotation generator — inject comments into .omni source."""

import pytest
from editor.server.annotation.types import (
    AnnotationFile,
    Chain,
    NodeEntry,
    WireEntry,
    ViewMarker,
    CodeTabType,
    ViewRenderMode,
)
from editor.server.annotation.generator import generate_annotations


class TestGenerateCodeTab:
    def test_generate_code_tab_logic(self):
        ann = AnnotationFile(filepath="test.omni", code_tab_type=CodeTabType.LOGIC)
        source = "import omniview;\n\ncomponent Foo { }\n"
        result = generate_annotations(ann, source)
        lines = result.split("\n")
        assert lines[0] == "// @omni-file type=logic"

    def test_generate_code_tab_event(self):
        ann = AnnotationFile(filepath="test.omni", code_tab_type=CodeTabType.EVENT)
        result = generate_annotations(ann, "")
        assert "// @omni-file type=event" in result

    def test_generate_code_tab_replaces_old(self):
        ann = AnnotationFile(filepath="test.omni", code_tab_type=CodeTabType.LOGIC)
        source = "// @omni-file type=event\nimport omniview;\n"
        result = generate_annotations(ann, source)
        lines = result.split("\n")
        assert lines[0] == "// @omni-file type=logic"
        assert "// @omni-file type=event" not in result


class TestGenerateEdit:
    def test_generate_edit_inserts_before_element(self):
        ann = AnnotationFile(
            filepath="test.omni",
            code_tab_type=CodeTabType.LOGIC,
            view_markers=[ViewMarker(element_id="box", line_number=0)],
        )
        source = """// @omni-file type=logic
component Foo {
    view {
        <div id="box"></div>
    }
}
"""
        result = generate_annotations(ann, source)
        assert "// @edit" in result
        lines = result.split("\n")
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
        source = """// @omni-file type=logic
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
        source = """// @omni-file type=logic
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
        source = """// @omni-file type=logic
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


    def test_generate_with_orphaned_chain_raises(self):
        """Generation should raise if a chain cannot be matched to a function."""
        chain = Chain(
            name="nonexistentFn",
            start_line=0,
            end_line=0,
            nodes=[NodeEntry(id="n1", tag="add", pos_x=50, pos_y=50)],
        )
        ann = AnnotationFile(
            filepath="test.omni",
            code_tab_type=CodeTabType.LOGIC,
            chains=[chain],
        )
        source = """// @omni-file type=logic
fn actualFn() { }
"""
        with pytest.raises(ValueError, match="nonexistentFn"):
            generate_annotations(ann, source)


class TestRoundTrip:
    def test_roundtrip_preserves_data(self):
        """Parse -> generate -> parse again should produce identical AnnotationFile."""
        source = """// @omni-file type=logic
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
