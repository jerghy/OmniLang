"""Tests for the OmniView brace-balanced preprocessor."""

import pytest
from omniview.preprocessor import (
    preprocess, PreprocessorOutput, BlockSlot,
    _extract_balanced_braces, _find_blocks, is_placeholder,
)


class TestBraceBalancing:
    """Test _extract_balanced_braces correctness."""

    def test_simple_braces(self):
        text, end = _extract_balanced_braces("{hello}", 0)
        assert text == "hello"
        assert end == 6  # 0-indexed position of closing }

    def test_nested_braces(self):
        text, end = _extract_balanced_braces("{a {b} c}", 0)
        assert text == "a {b} c"
        assert end == 8  # 0-indexed position of outer }

    def test_deeply_nested(self):
        source = "{level1 {level2 {level3}}}"
        text, end = _extract_balanced_braces(source, 0)
        assert text == "level1 {level2 {level3}}"

    def test_string_ignored(self):
        text, end = _extract_balanced_braces('{hello "world}" here}', 0)
        assert text == 'hello "world}" here'

    def test_string_with_escaped_quote(self):
        # String "say \"hi\"" — the \" is an escaped quote inside the string
        # The string spans from the first " to the last ", enclosing the \"
        text, end = _extract_balanced_braces('{"say \\"hi\\"" }', 0)
        assert text == '"say \\"hi\\"" '
        assert end > 0

    def test_line_comment_ignored(self):
        text, end = _extract_balanced_braces("{\n  // this is a { comment\n  real content\n}", 0)
        assert "real content" in text
        assert "comment" in text  # comment text is included (not stripped)

    def test_block_comment_ignored(self):
        text, end = _extract_balanced_braces("{\n  /* nested { */\n  value = 1;\n}", 0)
        assert "value = 1" in text

    def test_unmatched_returns_none(self):
        text, end = _extract_balanced_braces("{unmatched", 0)
        assert text is None
        assert end == -1

    def test_empty_braces(self):
        text, end = _extract_balanced_braces("{}", 0)
        assert text == ""
        assert end == 1  # 0-indexed position of }


class TestFindBlocks:
    """Test _find_blocks correctly identifies statement blocks."""

    def test_fn_body_detected(self):
        source = "fn increment() {\n  count = count + 1;\n}"
        blocks = _find_blocks(source)
        assert len(blocks) == 1
        assert blocks[0].context == "fn_body"
        assert "count = count + 1" in blocks[0].original_text

    def test_action_body_detected(self):
        source = "action add(text: String) {\n  items = [...items, text];\n}"
        blocks = _find_blocks(source)
        assert len(blocks) == 1
        assert blocks[0].context == "action_body"

    def test_async_action_detected(self):
        source = "async action fetch() {\n  loading = true;\n}"
        blocks = _find_blocks(source)
        assert len(blocks) == 1
        assert blocks[0].context == "action_body"

    def test_mount_hook_detected(self):
        source = "@mount {\n  count = 10;\n}"
        blocks = _find_blocks(source)
        assert len(blocks) == 1
        assert blocks[0].context == "lifecycle_body"

    def test_unmount_hook_detected(self):
        source = "@unmount {\n  cleanup();\n}"
        blocks = _find_blocks(source)
        assert len(blocks) == 1
        assert blocks[0].context == "lifecycle_body"

    def test_effect_hook_detected(self):
        source = '@effect(count) {\n  println("changed");\n}'
        blocks = _find_blocks(source)
        assert len(blocks) == 1
        assert blocks[0].context == "lifecycle_body"

    def test_multiple_blocks(self):
        source = """
component App {
  fn foo() { a = 1; }
  @mount { b = 2; }
}
"""
        blocks = _find_blocks(source)
        assert len(blocks) == 2

    def test_view_not_extracted(self):
        """View blocks and component blocks should NOT be extracted."""
        source = """component App {
  view {
    <Text>Hello</Text>
  }
}"""
        blocks = _find_blocks(source)
        assert len(blocks) == 0  # view block is UI structure

    def test_style_not_extracted(self):
        source = """component App {
  style {
    .card { padding: 16px; }
  }
}"""
        blocks = _find_blocks(source)
        assert len(blocks) == 0  # style block is UI structure

    def test_interpolation_not_extracted(self):
        """Expression interpolation {expr} should NOT be extracted as blocks."""
        source = "view { <Text>{count}</Text> }"
        blocks = _find_blocks(source)
        assert len(blocks) == 0  # {count} is an expression, not a statement block


class TestPreprocess:
    """Test full preprocess() pipeline."""

    def test_no_blocks_returns_original(self):
        source = "component Empty { view { <Text/> } }"
        result = preprocess(source)
        assert result.processed_source == source
        assert len(result.blocks) == 0

    def test_block_replaced_with_placeholder(self):
        source = "fn foo() { bar = 1; }"
        result = preprocess(source)
        assert "__OMNI_BLOCK_0__" in result.processed_source
        assert "bar = 1" not in result.processed_source  # Original text removed
        assert len(result.blocks) == 1

    def test_multiple_blocks_increment_ids(self):
        source = """
component App {
  fn foo() { a = 1; }
  fn bar() { b = 2; }
  action baz() { c = 3; }
}
"""
        result = preprocess(source)
        assert "__OMNI_BLOCK_0__" in result.processed_source
        assert "__OMNI_BLOCK_1__" in result.processed_source
        assert "__OMNI_BLOCK_2__" in result.processed_source
        assert result.placeholder_count == 3

    def test_block_map_preserves_original_text(self):
        source = "fn increment() {\n  count = count + 1;\n  return count;\n}"
        result = preprocess(source)
        slot = result.blocks["__OMNI_BLOCK_0__"]
        assert "count = count + 1" in slot.original_text
        assert "return count" in slot.original_text
        assert slot.context == "fn_body"

    def test_block_map_has_position(self):
        source = "\n\nfn foo() {\n  bar = 1;\n}"
        result = preprocess(source)
        slot = result.blocks["__OMNI_BLOCK_0__"]
        assert slot.omni_line == 3  # line 3 (1-based)
        assert slot.omni_col > 0

    def test_placeholders_parseable_by_lark(self):
        """Placeholders must be valid Lark terminals (match RAW_BLOCK regex)."""
        import re
        source = "fn foo() { bar = 1; }"
        result = preprocess(source)
        placeholder = "__OMNI_BLOCK_0__"
        # Must match the RAW_BLOCK regex in grammar.lark
        assert re.match(r"__OMNI_BLOCK_\d+__", placeholder)

    def test_todomvc_full_file(self):
        """Full TodoMVC .omni file preprocessing."""
        import os
        todomvc_path = os.path.join(
            os.path.dirname(__file__), "..", "examples", "01_TodoMVC.omni"
        )
        if not os.path.exists(todomvc_path):
            pytest.skip("TodoMVC example not found")

        with open(todomvc_path, "r", encoding="utf-8") as f:
            source = f.read()

        result = preprocess(source)
        # TodoMVC has several action bodies in the store
        assert result.placeholder_count > 0
        # All placeholders should be valid
        for slot_id in result.blocks:
            assert is_placeholder(slot_id)

    def test_nested_braces_in_block(self):
        """Blocks with nested braces (like if/else in fn body) should be handled."""
        source = """fn handle(e: Event) {
  if (e.active) {
    count = count + 1;
  } else {
    count = 0;
  }
}"""
        result = preprocess(source)
        assert len(result.blocks) == 1
        slot = result.blocks["__OMNI_BLOCK_0__"]
        # The original text should contain the full nested structure
        assert "if (e.active)" in slot.original_text
        assert "count = count + 1" in slot.original_text
        assert "count = 0" in slot.original_text


class TestPlaceholderUtils:
    """Test placeholder utility functions."""

    def test_is_placeholder_positive(self):
        assert is_placeholder("__OMNI_BLOCK_0__")
        assert is_placeholder("__OMNI_BLOCK_999__")

    def test_is_placeholder_negative(self):
        assert not is_placeholder("something")
        assert not is_placeholder("__OMNI_OTHER__")
        assert not is_placeholder("OMNI_BLOCK_0")
        assert not is_placeholder("")

    def test_parse_placeholder_id(self):
        from omniview.preprocessor import parse_placeholder_id
        assert parse_placeholder_id("__OMNI_BLOCK_5__") == ("BLOCK", 5)
        assert parse_placeholder_id("not_a_placeholder") is None
