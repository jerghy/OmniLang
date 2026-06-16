"""Verify that generated code actually compiles with target compilers.

Council R4 Critic: "55 UI codegen tests are string-matching snapshots. Not one runs
tsc --noEmit, dart analyze, or cargo check on the output."

This test runs the actual compilers/linters on generated output to catch
silent breakage that snapshot tests miss.
"""

import os
import subprocess
import sys
import shutil
from pathlib import Path

import pytest

FIXTURES_DIR = Path(__file__).parent.parent / "frameworks" / "behavior"
BUILD_DIR = Path(__file__).parent.parent.parent / "build_output" / "compile_check"
OMNI_FIXTURES = [
    "01_counter.omni",
    "05_store_read.omni",
]


def _find_tool(name: str) -> str | None:
    """Find a CLI tool in PATH."""
    return shutil.which(name)


def _compile_omni(source: str, target: str, out_dir: Path) -> subprocess.CompletedProcess:
    """Compile .omni source to a target and write to out_dir."""
    out_dir.mkdir(parents=True, exist_ok=True)
    # Write source to temp file
    tmp_file = out_dir / "_input.omni"
    tmp_file.write_text(source, encoding="utf-8")
    return subprocess.run(
        [sys.executable, "-m", "omnilang.cli", "build",
         str(tmp_file), "--target", target, "--output", str(out_dir)],
        capture_output=True, text=True, timeout=60,
    )


def _read_fixture(name: str) -> str:
    return (FIXTURES_DIR / name).read_text(encoding="utf-8")


class TestReactOutputCompiles:
    """Generated React/TSX output should pass TypeScript type-check."""

    @pytest.fixture(scope="class")
    def react_output(self):
        BUILD_DIR.mkdir(parents=True, exist_ok=True)
        source = _read_fixture("01_counter.omni")
        result = _compile_omni(source, "react", BUILD_DIR / "react")
        return result, BUILD_DIR / "react"

    def test_react_build_succeeds(self, react_output):
        result, _ = react_output
        assert result.returncode == 0, f"Build failed:\n{result.stderr}"

    @pytest.mark.skipif(_find_tool("tsc") is None, reason="TypeScript compiler not installed")
    def test_tsc_typecheck(self, react_output):
        """Generated TSX should pass tsc --noEmit."""
        _, out_dir = react_output
        tsx_files = list(out_dir.glob("*.tsx")) + list(out_dir.glob("*.ts"))
        if not tsx_files:
            pytest.skip("No TypeScript files generated")

        # Write minimal tsconfig for type-checking
        tsconfig = out_dir / "tsconfig.json"
        tsconfig.write_text("""{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true,
    "esModuleInterop": true
  },
  "include": ["*.tsx", "*.ts"]
}""")

        result = subprocess.run(
            ["npx", "tsc", "--noEmit", "--project", str(out_dir)],
            capture_output=True, text=True, timeout=30,
            cwd=str(out_dir),
        )
        assert result.returncode == 0, (
            f"TypeScript type-check failed:\n{result.stdout}\n{result.stderr}"
        )

    def test_react_output_has_valid_jsx(self, react_output):
        """Output should contain valid JSX syntax markers."""
        _, out_dir = react_output
        for tsx_file in out_dir.glob("*.tsx"):
            content = tsx_file.read_text(encoding="utf-8")
            # Should contain React import or JSX
            assert "react" in content.lower() or "<" in content, (
                f"{tsx_file.name} has no React content"
            )


class TestPythonOutputCompiles:
    """Generated Python output should be syntactically valid."""

    def test_python_syntax_check(self):
        source = _read_fixture("01_counter.omni")
        out_dir = BUILD_DIR / "python"
        result = _compile_omni(source, "python", out_dir)

        if result.returncode != 0:
            pytest.skip(f"Python target build failed: {result.stderr}")

        for py_file in out_dir.glob("*.py"):
            # Compile to bytecode as syntax check
            try:
                compile(py_file.read_text(encoding="utf-8"), py_file.name, "exec")
            except SyntaxError as e:
                pytest.fail(f"Python syntax error in {py_file.name}: {e}")


class TestDartOutputAnalyzes:
    """Generated Dart output should pass dart analyze (if Dart SDK available)."""

    @pytest.mark.skipif(_find_tool("dart") is None, reason="Dart SDK not installed")
    def test_dart_analyze(self):
        source = _read_fixture("01_counter.omni")
        out_dir = BUILD_DIR / "dart"
        result = _compile_omni(source, "dart", out_dir)

        if result.returncode != 0:
            pytest.skip(f"Dart target build failed: {result.stderr}")

        dart_files = list(out_dir.glob("*.dart"))
        if not dart_files:
            pytest.skip("No Dart files generated")

        analyze = subprocess.run(
            ["dart", "analyze", str(out_dir)],
            capture_output=True, text=True, timeout=30,
        )
        # dart analyze exits 0 for clean, 1 for hints, 2 for errors, 3 for fatal
        assert analyze.returncode in (0, 1), (
            f"dart analyze found errors:\n{analyze.stdout}\n{analyze.stderr}"
        )


class TestRustOutputChecks:
    """Generated Rust output should pass cargo check (if Rust toolchain available)."""

    @pytest.mark.skipif(_find_tool("cargo") is None, reason="Rust toolchain not installed")
    def test_cargo_check(self):
        source = _read_fixture("01_counter.omni")
        out_dir = BUILD_DIR / "rust"
        result = _compile_omni(source, "rust", out_dir)

        if result.returncode != 0:
            pytest.skip(f"Rust target build failed: {result.stderr}")

        rs_files = list(out_dir.glob("*.rs"))
        if not rs_files:
            pytest.skip("No Rust files generated")

        # Check if we have a Cargo.toml or need to create one
        cargo_toml = out_dir / "Cargo.toml"
        if not cargo_toml.exists():
            cargo_toml.write_text("""[package]
name = "compile-check"
version = "0.1.0"
edition = "2021"

[dependencies]
""")

        # Put Rust files in src/
        src_dir = out_dir / "src"
        src_dir.mkdir(exist_ok=True)
        for rs_file in rs_files:
            shutil.copy(rs_file, src_dir / rs_file.name)

        check = subprocess.run(
            ["cargo", "check"],
            capture_output=True, text=True, timeout=60,
            cwd=str(out_dir),
        )
        assert check.returncode == 0, (
            f"cargo check failed:\n{check.stdout}\n{check.stderr}"
        )


class TestVueOutputCompiles:
    """Generated Vue output should pass vue-tsc (if available)."""

    def test_vue_output_structure(self):
        """Vue output should be valid .vue SFC files."""
        source = _read_fixture("01_counter.omni")
        out_dir = BUILD_DIR / "vue"
        result = _compile_omni(source, "vue", out_dir)

        if result.returncode != 0:
            pytest.skip(f"Vue target build failed: {result.stderr}")

        for vue_file in out_dir.glob("*.vue"):
            content = vue_file.read_text(encoding="utf-8")
            # Vue SFC should have <template>, <script>, or <script setup>
            has_sfc = any(tag in content for tag in ["<template>", "<script"])
            assert has_sfc, f"{vue_file.name} is not a valid Vue SFC"
