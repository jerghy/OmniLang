"""Regression tests for Rust codegen output across all 10 targets.

Since the Python parser/codegen was deleted in Phase 5, Rust is the sole
compiler. These tests require the Rust omni-compiler DLL (or CLI) to be built
and available. On platforms where the DLL hasn't been compiled yet, all tests
are skipped automatically.

To build the DLL: cd omni-compiler && cargo build --release
"""

from __future__ import annotations

import pytest

from omnilang.api import compile_omni
from omnilang.rust_bridge import RustBridge


# ── Helpers ──────────────────────────────────────────────────────────────


def _require_rust():
    """Skip test if the Rust compiler backend is not available."""
    if not RustBridge.is_available():
        pytest.skip("Rust omni-compiler DLL/CLI not built. Run: cd omni-compiler && cargo build --release")
    # Double-check: is_available may return True but compile_to_target returns None
    sources = {"_probe.omni": "fn main() {}"}
    if RustBridge.compile_to_target(sources, "python") is None:
        pytest.skip("Rust compiler found but compile_to_target function not available (DLL may need rebuild)")


def _compile(source: str, target: str) -> str:
    """Compile .omni source and return concatenated file contents."""
    _require_rust()
    result = compile_omni(source, target=target)
    if not result.success:
        error_msgs = [e.message for e in (result.errors or [])]
        pytest.fail(f"Compilation failed for target={target}: {error_msgs}")
    return "\n".join(result.files.values()) if result.files else ""


def _check_output(combined: str, snippets: list[str]) -> None:
    """Assert that all expected snippets appear in the output."""
    missing = [s for s in snippets if s not in combined]
    if missing:
        pytest.fail(
            f"Missing {len(missing)} expected pattern(s): {missing}\n"
            f"--- output (first 1000 chars) ---\n{combined[:1000]}"
        )


# ── Pure-Logic Targets ───────────────────────────────────────────────────

# The Rust compiler's cross-file validator requires `import omniview;` for any
# file containing component/store/theme declarations. Pure-logic targets
# (python/js/rust/dart) also support components (generating classes/structs).
# Use `import omniview;` prefix for all component-based tests.
PURE_LOGIC_CASES = [
    # Python
    pytest.param(
        'import omniview; component App { fn greet() -> string { return "hi"; } }',
        "python", ["class App:", "def greet(self) -> str:", "return 'hi'"], id="python-component-fn",
    ),
    # JavaScript (uses double-quote strings)
    pytest.param(
        'import omniview; component App { fn greet() -> string { return "hi"; } }',
        "javascript", ["class App", "greet()", 'return "hi"'], id="js-component-fn",
    ),
    # TypeScript (uses double-quote strings)
    pytest.param(
        'import omniview; component App { fn greet() -> string { return "hi"; } }',
        "typescript", ["class App", "greet()", 'return "hi"'], id="ts-component-fn",
    ),
    # Rust
    pytest.param(
        'import omniview; component App { fn greet() -> string { return "hi"; } }',
        "rust", ["struct App", "impl App", "fn greet", "String"], id="rust-component-fn",
    ),
    # Dart
    pytest.param(
        'import omniview; component App { fn greet() -> string { return "hi"; } }',
        "dart", ["class App", "greet()", "return 'hi'"], id="dart-component-fn",
    ),
]


# ── Framework Targets ────────────────────────────────────────────────────
# Require `import omniview;` per cross-file validation rules.

FRAMEWORK_CASES = [
    pytest.param(
        "import omniview; component App { view { <div>Hello</div> } }", "react",
        ["export function App(", "return (", "<div>", "Hello"],
        id="react-basic",
    ),
    pytest.param(
        "import omniview; component App { view { <div>Hello</div> } }", "vue",
        ["<template>", "<div>", "<script setup"],
        id="vue-basic",
    ),
    pytest.param(
        "import omniview; component App { view { <div>Hello</div> } }", "dioxus",
        ["#[component]", "pub fn App()", "rsx!"],
        id="dioxus-basic",
    ),
    pytest.param(
        "import omniview; component App { view { <div>Hello</div> } }", "flutter",
        ["class App extends StatefulWidget", "Widget build"],
        id="flutter-basic",
    ),
]


# ── OmniView Target ─────────────────────────────────────────────────────

OMNIVIEW_CASES = [
    pytest.param(
        "import omniview; component App { view { <div>Hello</div> } }", "omniview",
        ["index.js", "index.html", "index.css"],
        id="omniview-basic",
    ),
]


# ── Tests ────────────────────────────────────────────────────────────────


@pytest.mark.parametrize("source,target,snippets", PURE_LOGIC_CASES)
def test_pure_logic_codegen(source: str, target: str, snippets: list[str]) -> None:
    """Pure-logic .omni source must compile to valid target output."""
    output = _compile(source, target)
    _check_output(output, snippets)


@pytest.mark.parametrize("source,target,snippets", FRAMEWORK_CASES)
def test_framework_codegen(source: str, target: str, snippets: list[str]) -> None:
    """Framework .omni source must compile to valid target output."""
    output = _compile(source, target)
    _check_output(output, snippets)


@pytest.mark.parametrize("source,target,snippets", OMNIVIEW_CASES)
def test_omniview_codegen(source: str, target: str, snippets: list[str]) -> None:
    """OmniView .omni source must produce multi-file output."""
    _require_rust()
    result = compile_omni(source, target=target)
    assert result.success, f"OmniView compilation failed: {result.errors}"
    for snippet in snippets:
        assert snippet in result.files, f"Missing file '{snippet}' in {list(result.files.keys())}"


def test_all_ten_targets_produce_output() -> None:
    """Smoke test: all 10 targets should compile a simple component successfully."""
    _require_rust()
    source = "import omniview; component App { view { <div>ok</div> } }"
    targets = ["python", "javascript", "typescript", "rust", "dart",
               "react", "vue", "dioxus", "flutter", "omniview"]
    for target in targets:
        result = compile_omni(source, target=target)
        assert result.success, f"Target '{target}' failed: {result.errors}"
        assert result.files, f"Target '{target}' produced no files"
