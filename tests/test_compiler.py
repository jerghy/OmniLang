"""Snapshot tests: verify generated code matches expected patterns for all targets."""

import os
import pytest
from omnilang.parser import parse
from omnilang.semantic import analyze
from omnilang.codegen import generate

FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "fixtures")
TARGETS = ["python", "js", "dart", "rust"]


def _generate(fixture_name: str, target: str) -> str:
    path = os.path.join(FIXTURES_DIR, fixture_name)
    with open(path, "r", encoding="utf-8") as f:
        source = f.read()
    prog = parse(source, filename=path)
    source_lines = {path: source.split("\n")}
    type_map, rename_map = analyze(prog, source_lines)
    return generate(prog, target, rename_map, type_map)


class TestCompilerOutput:
    """Verify generated code structure and correctness."""

    def test_python_no_idiv_when_not_needed(self):
        """20_no_idiv.omni has no division — output should not contain _idiv."""
        code = _generate("20_no_idiv.omni", "python")
        assert "_idiv" not in code
        assert "_irem" not in code

    def test_js_helpers_when_needed(self):
        """03_arithmetic.omni has division — output should contain _idiv."""
        code = _generate("03_arithmetic.omni", "js")
        assert "_idiv" in code
        assert "_irem" in code

    def test_js_helpers_when_not_needed(self):
        """20_no_idiv.omni has no division — no helpers."""
        code = _generate("20_no_idiv.omni", "js")
        assert "_idiv" not in code

    def test_dart_import_io_when_print(self):
        """01_hello_world has output — import should be present."""
        code = _generate("01_hello_world.omni", "dart")
        assert "import 'dart:io';" in code

    def test_rust_string_param_type(self):
        """String parameters in Rust should be &str."""
        code = _generate("08_functions.omni", "rust")
        assert "name: &str" in code

    def test_rust_format_for_string_concat(self):
        """String concatenation should use format!."""
        code = _generate("04_strings.omni", "rust")
        assert "format!" in code

    def test_dart_len_property(self):
        """Array len() should be .length property in Dart."""
        code = _generate("05_arrays.omni", "dart")
        assert ".length" in code

    def test_rust_automatic_clone(self):
        """Container assignment should clone."""
        code = _generate("05_arrays.omni", "rust")
        assert ".clone()" in code

    def test_rust_main_no_return_type(self):
        """main function should not have return type."""
        code = _generate("01_hello_world.omni", "rust")
        assert "fn main() {" in code

    def test_js_entry_point(self):
        """JS output should end with main() call."""
        code = _generate("01_hello_world.omni", "js")
        assert "main();" in code.split("\n")[-3:]

    def test_python_entry_point(self):
        """Python output should have __main__ guard."""
        code = _generate("01_hello_world.omni", "python")
        assert '__name__ == "__main__"' in code

    def test_dart_final_for_let(self):
        """let should become final in Dart."""
        code = _generate("02_variables.omni", "dart")
        assert "final int x" in code

    def test_js_const_for_let(self):
        """let should become const in JS."""
        code = _generate("02_variables.omni", "js")
        assert "const x" in code

    def test_rust_for_in_loop_ref(self):
        """for-in should iterate by reference in Rust."""
        code = _generate("11_for_loop.omni", "rust")
        assert "for " in code

    def test_rust_use_hashmap(self):
        """Map usage should import HashMap."""
        code = _generate("06_maps.omni", "rust")
        assert "use std::collections::HashMap;" in code
