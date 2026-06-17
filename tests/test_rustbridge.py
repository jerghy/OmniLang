"""Tests for the DLL-only RustBridge."""
from __future__ import annotations

from pathlib import Path

import pytest

from omnilang.errors import CompilerUnavailable


def test_compiler_unavailable_when_dll_missing():
    """RustBridge._load() raises CompilerUnavailable when no DLL is found."""
    import omnilang.rust_bridge as rb

    old_paths = list(rb.RustBridge._dll_search_paths)
    try:
        rb.RustBridge._dll_search_paths = [Path("/nonexistent/omni_compiler.dll")]
        rb.RustBridge._checked = False
        rb.RustBridge._dll = None
        with pytest.raises(CompilerUnavailable) as exc:
            rb.RustBridge._load()
        assert "omni_compiler.dll" in exc.value.format()
    finally:
        rb.RustBridge._dll_search_paths = old_paths
        rb.RustBridge._dll = None
        rb.RustBridge._checked = False


def test_compile_raises_compiler_unavailable():
    """RustBridge.compile() propagates CompilerUnavailable when DLL is missing."""
    import omnilang.rust_bridge as rb

    old_paths = list(rb.RustBridge._dll_search_paths)
    try:
        rb.RustBridge._dll_search_paths = [Path("/nonexistent/omni_compiler.dll")]
        rb.RustBridge._checked = False
        rb.RustBridge._dll = None
        with pytest.raises(CompilerUnavailable):
            rb.RustBridge.compile({"test.omni": ""}, "python")
    finally:
        rb.RustBridge._dll_search_paths = old_paths
        rb.RustBridge._dll = None
        rb.RustBridge._checked = False


def test_analyse_raises_compiler_unavailable():
    """RustBridge.analyse() propagates CompilerUnavailable when DLL is missing."""
    import omnilang.rust_bridge as rb

    old_paths = list(rb.RustBridge._dll_search_paths)
    try:
        rb.RustBridge._dll_search_paths = [Path("/nonexistent/omni_compiler.dll")]
        rb.RustBridge._checked = False
        rb.RustBridge._dll = None
        with pytest.raises(CompilerUnavailable):
            rb.RustBridge.analyse({"test.omni": ""})
    finally:
        rb.RustBridge._dll_search_paths = old_paths
        rb.RustBridge._dll = None
        rb.RustBridge._checked = False


def test_compile_state_scoped_per_test():
    """State isolation: each test starts with a clean RustBridge."""
    import omnilang.rust_bridge as rb

    assert rb.RustBridge._checked is False
    assert rb.RustBridge._dll is None
