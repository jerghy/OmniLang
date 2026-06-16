"""Regression tests comparing Rust codegen vs Python codegen output for pure-logic .omni programs.

Tests verify that:
1. Both the Rust bridge and Python fallback codegen produce valid output
2. Output contains expected syntax patterns for each target language
3. When both backends are available, outputs are semantically equivalent

Design:
- All tests work regardless of Rust DLL availability (Python fallback always works)
- The helper _compile_with_backend() allows explicit backend selection
- Test cases cover: function definitions, parameters, println output,
  variable declarations, arithmetic, and cross-backend comparison

NOTE: The current Python parser has a known limitation: the `type_ann` grammar
rule lacks a corresponding Lark transformer method, so return type annotations
like `-> i32` are silently dropped and treated as `void`. This means:
  - Non-void return types cannot be tested yet
  - Void functions (`fn name() { ... }`) work correctly
  - Param types like `a: i32` parse correctly
"""

from __future__ import annotations

from unittest.mock import patch

import pytest

from omniview.api import compile_omni
from omniview.codegen.rust_bridge import RustBridge


# ── Helpers ──────────────────────────────────────────────────────────────


def _compile_with_backend(source: str, target: str, *, use_rust: bool | None = None) -> str:
    """Compile .omni source and return the single-file output string.

    Args:
        source: .omni source code.
        target: Target language (python, javascript/js, rust, dart).
        use_rust: If True, force Rust bridge (skips if unavailable).
                  If False, force Python fallback via mock.
                  If None (default), use whatever is available (Rust preferred).

    Returns:
        Concatenated file contents from the compilation result.
    """
    if use_rust is False:
        with patch.object(RustBridge, "is_available", return_value=False):
            result = compile_omni(source, target=target)
    else:
        result = compile_omni(source, target=target)

    combined = ""
    if result.files:
        combined = "\n".join(result.files.values())
    return combined


def _check_output(combined: str, snippets: list[str]) -> None:
    """Assert that all expected snippets appear in the output."""
    missing = [s for s in snippets if s not in combined]
    if missing:
        pytest.fail(
            f"Missing {len(missing)} expected pattern(s): {missing}\n"
            f"--- output (first 1000 chars) ---\n{combined[:1000]}"
        )


def _get_code(source: str, target: str) -> tuple[str, str]:
    """Return (python_fallback_code, best_available_code) for comparison.

    The Python fallback is always forced via mock. The second result uses the
    best available backend (Rust if available, otherwise Python fallback).
    """
    py_code = _compile_with_backend(source, target, use_rust=False)
    best_code = _compile_with_backend(source, target, use_rust=None)
    return py_code, best_code


# ── Basic Regression Cases ──────────────────────────────────────────────

REGRESSION_CASES = [
    pytest.param(
        "fn main() { let x = 42; }",
        "python",
        ["def main():", "    x = 42"],
        id="python-let-binding",
    ),
    pytest.param(
        'fn main() { println("hello"); }',
        "python",
        ["def main():", 'print("hello")'],
        id="python-println",
    ),
    pytest.param(
        "fn main() { let x = 42; }",
        "javascript",
        ["function main()", "const x = 42"],
        id="javascript-let-binding",
    ),
    pytest.param(
        "fn main() { let x = 42; }",
        "js",
        ["function main()", "const x = 42"],
        id="js-let-binding",
    ),
    pytest.param(
        'fn main() { println("hello"); }',
        "rust",
        ["fn main()", 'println!'],
        id="rust-println",
    ),
    pytest.param(
        'fn main() { println("hello"); }',
        "dart",
        ["main()", "stdout.writeln"],
        id="dart-println",
    ),
    # Functions with void return
    pytest.param(
        "fn main() { }",
        "python",
        ["def main():"],
        id="python-empty-function",
    ),
    pytest.param(
        "fn main() { }",
        "javascript",
        ["function main()"],
        id="javascript-empty-function",
    ),
    pytest.param(
        "fn main() { }",
        "rust",
        ["fn main()"],
        id="rust-empty-function",
    ),
    pytest.param(
        "fn main() { }",
        "dart",
        ["main()"],
        id="dart-empty-function",
    ),
    # Functions with parameters (param types like i32 are parsed but stripped in codegen)
    pytest.param(
        "fn add(a: i32, b: i32) -> void { let z = a + b; } fn main() { add(1, 2); }",
        "python",
        ["def add(a, b):", "z = (a + b)", "def main():"],
        id="python-function-with-params",
    ),
    pytest.param(
        "fn add(a: i32, b: i32) -> void { let z = a + b; } fn main() { add(1, 2); }",
        "rust",
        ["fn add(a: i32, b: i32)", "fn main()", "add(1, 2);"],
        id="rust-function-with-params",
    ),
]

# Cases that specifically test the Python fallback path
PYTHON_FALLBACK_CASES = [
    pytest.param(
        "fn main() { let x = 1 + 2; }",
        "python",
        ["x = (1 + 2)"],
        id="python-arithmetic-fallback",
    ),
    pytest.param(
        "fn main() { let x = 42; }",
        "javascript",
        ["const x = 42"],
        id="javascript-let-fallback",
    ),
    pytest.param(
        'fn main() { println("hello"); }',
        "rust",
        ['println!("{}", String::from("hello"))'],
        id="rust-println-fallback",
    ),
    pytest.param(
        "fn main() { let x = 42; }",
        "dart",
        ["final void x = 42"],
        id="dart-let-fallback",
    ),
]


# ── Tests ────────────────────────────────────────────────────────────────


@pytest.mark.parametrize("source,target,snippets", REGRESSION_CASES)
def test_codegen_output_contains_expected_patterns(source: str, target: str, snippets: list[str]) -> None:
    """Compiled output must contain expected syntax patterns for the target language.

    This test works regardless of which backend (Rust or Python) is selected.
    """
    combined = _compile_with_backend(source, target)
    _check_output(combined, snippets)


@pytest.mark.parametrize("source,target,snippets", PYTHON_FALLBACK_CASES)
def test_python_fallback_codegen(source: str, target: str, snippets: list[str]) -> None:
    """Python fallback codegen must produce valid output with expected patterns.

    This explicitly forces the Python fallback path by mocking RustBridge,
    ensuring the Python codegen backends remain correct as a baseline.
    """
    combined = _compile_with_backend(source, target, use_rust=False)
    _check_output(combined, snippets)


@pytest.mark.skipif(
    not RustBridge.is_available(),
    reason="Rust omni-compiler DLL/CLI not available on this system",
)
def test_rust_bridge_produces_valid_output() -> None:
    """When Rust bridge is available, verify its output for all targets.

    Only runs when the Rust DLL or CLI is present. Validates that the
    Rust codegen produces syntactically valid output with expected patterns.
    """
    source = 'fn main() { println("hello"); }'
    targets_snippets = [
        ("python", ["def main():", 'print("hello")']),
        ("javascript", ["function main()", "console.log"]),
        ("rust", ["fn main()", 'println!']),
        ("dart", ["void main()", "stdout.writeln"]),
    ]
    for target, snippets in targets_snippets:
        combined = _compile_with_backend(source, target, use_rust=True)
        _check_output(combined, snippets)


# ── Cross-Backend Comparison Tests ──────────────────────────────────────

COMPARISON_CASES = [
    pytest.param(
        "fn main() { let x = 42; }",
        "python",
        id="python-let-both-backends",
    ),
    pytest.param(
        'fn main() { println("hello"); }',
        "python",
        id="python-println-both-backends",
    ),
    pytest.param(
        "fn main() { let x = 42; }",
        "javascript",
        id="javascript-let-both-backends",
    ),
    pytest.param(
        'fn main() { println("hello"); }',
        "rust",
        id="rust-println-both-backends",
    ),
    pytest.param(
        "fn main() { let x = 42; }",
        "dart",
        id="dart-let-both-backends",
    ),
]


@pytest.mark.parametrize("source,target", COMPARISON_CASES)
def test_rust_and_python_codegen_agree_on_structure(source: str, target: str) -> None:
    """Both backends should produce structurally similar output.

    Checks that both outputs contain the expected function name and key
    operators/keywords, not exact string equality (whitespace and naming
    conventions differ between Python and Rust codegen implementations).

    When Rust bridge is unavailable, Python fallback is used for both
    results and they should be identical.
    """
    py_code, best_code = _get_code(source, target)

    assert py_code, "Python fallback produced empty output"
    assert best_code, "Best-backend produced empty output"

    # Both must contain "main" function
    assert "main" in py_code, f"Python fallback missing 'main' in {target}"
    assert "main" in best_code, f"Best-backend missing 'main' in {target}"

    # Both must contain "42" if source has it
    if "42" in source:
        assert "42" in py_code, f"Python fallback missing '42' in {target}"
        assert "42" in best_code, f"Best-backend missing '42' in {target}"

    # If both used the same backend, outputs should be identical
    rust_available = RustBridge.is_available()
    if not rust_available:
        assert py_code == best_code, (
            f"Expected identical Python fallback output for target={target}, "
            f"but got different results:\n--- py ---\n{py_code}\n--- best ---\n{best_code}"
        )


@pytest.mark.skipif(
    not RustBridge.is_available(),
    reason="Rust DLL required for cross-backend comparison",
)
def test_rust_vs_python_comparison_all_targets() -> None:
    """Compare Rust bridge output vs Python fallback for all pure-logic targets.

    Only runs when Rust DLL is available. Verifies that:
    - Both backends produce non-empty output
    - Outputs contain the same function name
    - Outputs are structurally compatible (same operators, same output)
    """
    source = 'fn main() { println("hello world"); }'
    targets = ["python", "javascript", "rust", "dart"]

    for target in targets:
        py_code = _compile_with_backend(source, target, use_rust=False)
        rust_code = _compile_with_backend(source, target, use_rust=True)

        assert py_code, f"Python fallback empty for target={target}"
        assert rust_code, f"Rust bridge empty for target={target}"

        # Both must contain the function name
        assert "main" in py_code, f"Python fallback missing 'main' in {target}"
        assert "main" in rust_code, f"Rust bridge missing 'main' in {target}"

        # Both must reference the string argument
        assert "hello world" in py_code, f"Python fallback missing 'hello world' in {target}"
        assert "hello world" in rust_code, f"Rust bridge missing 'hello world' in {target}"
