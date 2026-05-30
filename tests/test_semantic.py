"""Semantic error tests: verify that invalid OmniLang programs are rejected."""

import os
import pytest
from omnilang.parser import parse
from omnilang.semantic import analyze

ERRORS_DIR = os.path.join(os.path.dirname(__file__), "fixtures", "errors")


def _compile_should_fail(filename: str) -> str:
    """Attempt to compile a .omni file and return the error message if it fails."""
    import io
    import sys
    path = os.path.join(ERRORS_DIR, filename)
    with open(path, "r", encoding="utf-8") as f:
        source = f.read()
    try:
        prog = parse(source, filename=path)
        source_lines = {path: source.split("\n")}
        # Capture stderr
        old_stderr = sys.stderr
        sys.stderr = io.StringIO()
        try:
            analyze(prog, source_lines)
            sys.stderr = old_stderr
            return ""  # should not succeed
        except SystemExit:
            error_output = sys.stderr.getvalue()
            sys.stderr = old_stderr
            return error_output.strip() or "compilation failed"
    except Exception as e:
        return str(e)


# Each test case: (filename, expected_error_substring)
SEMANTIC_ERROR_TESTS = [
    # v0.2: type annotation is now optional (type inference support)
    # ("err_var_no_type.omni", ""),
    ("err_var_no_init.omni", ""),
    ("err_type_mismatch.omni", "Type mismatch"),
    ("err_shadow_same_scope.omni", "already declared"),
    ("err_underscore_prefix.omni", ""),
    ("err_float_div.omni", "forbidden"),
    ("err_float_mod.omni", "forbidden"),
    ("err_param_mutate.omni", ""),
    ("err_for_mutate_container.omni", ""),
    ("err_for_mutate_var.omni", ""),  # may not be caught by current restrict_checker
    ("err_main_with_return.omni", ""),
    ("err_print_not_string.omni", "must be String"),
    ("err_f64_key.omni", ""),
    ("err_nested_container.omni", ""),
    ("err_break_outside_loop.omni", "outside of loop"),
]


@pytest.mark.parametrize("filename,expected_error", SEMANTIC_ERROR_TESTS)
def test_semantic_error(filename, expected_error):
    """Verify that invalid OmniLang programs are rejected by the compiler."""
    result = _compile_should_fail(filename)
    assert result != "", f"Expected {filename} to fail compilation, but it succeeded!"
    if expected_error:
        assert expected_error.lower() in result.lower(), (
            f"Expected error containing '{expected_error}' for {filename}, got: {result}"
        )
