"""Tests for RustBridge — works both with and without Rust installed."""
from __future__ import annotations

import pytest
from omniview.codegen.rust_bridge import RustBridge


def test_is_available_returns_bool():
    """Rust compiler not required — always returns a bool."""
    result = RustBridge.is_available()
    assert isinstance(result, bool)


def test_compile_to_ir_raises_when_not_available():
    """When Rust is absent, compile_to_ir must raise RuntimeError."""
    if not RustBridge.is_available():
        with pytest.raises(RuntimeError, match="not available"):
            RustBridge.compile_to_ir("component App { view {} }")


def test_compile_to_omniview_raises_when_not_available():
    """When Rust is absent, compile_to_omniview must raise RuntimeError."""
    if not RustBridge.is_available():
        with pytest.raises(RuntimeError, match="not available"):
            RustBridge.compile_to_omniview("component App { view {} }")


@pytest.mark.skipif(not RustBridge.is_available(), reason="Rust compiler not installed")
def test_compile_to_ir_success():
    """Rust available -> compile_to_ir returns valid IR dict."""
    result = RustBridge.compile_to_ir("component App { view {} }")
    assert isinstance(result, dict)
    assert "components" in result or "stores" in result
    assert "version" in result
    assert result["version"] == "1.0"


@pytest.mark.skipif(not RustBridge.is_available(), reason="Rust compiler not installed")
def test_compile_to_ir_reports_errors():
    """Rust returns errors for broken .omni."""
    result = RustBridge.compile_to_ir("not valid omni @@@")
    assert "_rust_errors" in result or "errors" in result
