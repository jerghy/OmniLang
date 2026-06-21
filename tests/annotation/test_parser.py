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
