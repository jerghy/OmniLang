"""Unit tests for module system."""
import pytest
from pathlib import Path
from omnilang.modules.resolver import ModuleResolver
from omnilang.modules.graph import ModuleGraph


class TestModuleResolver:
    def test_resolve_relative(self, tmp_path):
        root = tmp_path / "project"
        root.mkdir()
        (root / "lib.omni").write_text("fn f() {}")
        resolver = ModuleResolver(str(root))
        result = resolver.resolve("./lib", from_file=str(root / "main.omni"))
        assert result is not None
        assert result.path.name == "lib.omni"

    def test_resolve_alias(self, tmp_path):
        root = tmp_path / "project"
        root.mkdir()
        libs = tmp_path / "libs"
        libs.mkdir()
        (libs / "utils.omni").write_text("fn f() {}")
        resolver = ModuleResolver(str(root), aliases={"@libs": str(libs)})
        result = resolver.resolve("@libs/utils")
        assert result is not None
        assert result.path.name == "utils.omni"


class TestModuleGraph:
    def test_no_cycle(self):
        g = ModuleGraph()
        g.add_module("a.omni", ["b.omni"], ["a"])
        g.add_module("b.omni", [], ["b"])
        assert g.detect_cycles() == []

    def test_simple_cycle(self):
        g = ModuleGraph()
        g.add_module("a.omni", ["b.omni"], ["a"])
        g.add_module("b.omni", ["a.omni"], ["b"])
        cycles = g.detect_cycles()
        assert len(cycles) == 1

    def test_compilation_order(self):
        g = ModuleGraph()
        g.add_module("main.omni", ["lib.omni"], [])
        g.add_module("lib.omni", [], [])
        order = g.compilation_order()
        # lib should come before main
        assert order.index("lib.omni") < order.index("main.omni")
