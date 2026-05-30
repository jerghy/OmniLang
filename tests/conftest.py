"""Shared test fixtures for OmniLang tests."""

import os
import sys
import subprocess
import tempfile
import pytest

# Ensure the project root is on sys.path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "fixtures")
BUILD_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "build", "test")

import uuid
_RUN_ID = uuid.uuid4().hex[:8]

# ── v0.1 fixtures (preserved) ────────────────────────


@pytest.fixture
def compile_and_run():
    """Fixture that compiles an .omni file to a target and returns stdout."""
    def _run(omni_path: str, target: str) -> str:
        from omnilang.parser import parse
        from omnilang.semantic import analyze
        from omnilang.codegen import generate

        with open(omni_path, "r", encoding="utf-8") as f:
            source = f.read()

        prog = parse(source, filename=omni_path)
        source_lines = {omni_path: source.split("\n")}
        type_map, rename_map = analyze(prog, source_lines)
        code = generate(prog, target, rename_map, type_map)

        run_dir = os.path.join(BUILD_DIR, _RUN_ID)
        os.makedirs(run_dir, exist_ok=True)
        base = os.path.splitext(os.path.basename(omni_path))[0]
        ext_map = {"rust": ".rs", "dart": ".dart", "js": ".js", "python": ".py"}
        out_path = os.path.join(run_dir, f"{base}{ext_map[target]}")
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(code)

        if target == "python":
            result = subprocess.run(["python", out_path], capture_output=True, text=True, timeout=10)
        elif target == "js":
            result = subprocess.run(["node", out_path], capture_output=True, text=True, timeout=10)
        elif target == "dart":
            result = subprocess.run(["dart", "run", out_path], capture_output=True, text=True, timeout=10)
        elif target == "rust":
            rust_dir = os.path.join(run_dir, f"rust_{base}")
            os.makedirs(rust_dir, exist_ok=True)
            import shutil
            src_dir = os.path.join(rust_dir, "src")
            os.makedirs(src_dir, exist_ok=True)
            shutil.copy(out_path, os.path.join(src_dir, "main.rs"))
            cargo_path = os.path.join(rust_dir, "Cargo.toml")
            if not os.path.exists(cargo_path):
                safe_name = f"omni_{base.lstrip('0123456789_')}_{_RUN_ID}"
                with open(cargo_path, "w") as f:
                    f.write(f'[package]\nname = "{safe_name}"\nversion = "0.1.0"\nedition = "2021"\n')
            build_result = subprocess.run(
                ["cargo", "build", "--quiet"], cwd=rust_dir,
                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60
            )
            if build_result.returncode != 0:
                raise RuntimeError(f"Rust build failed for {base}:\n{build_result.stderr}")
            cargo_run = subprocess.run(
                ["cargo", "run", "--quiet"], cwd=rust_dir,
                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=10
            )
            result = cargo_run
        else:
            raise ValueError(f"Unknown target: {target}")

        return result.stdout

    return _run


# ── v0.2 new fixtures ────────────────────────────────


@pytest.fixture
def multi_file_fixture(tmp_path):
    """Create a temporary multi-file OmniLang project.

    Usage:
        def test_modules(multi_file_fixture):
            project_dir = multi_file_fixture({
                "src/main.omni": '''
                    import { greet } from "./greet";
                    fn main() { greet("World"); }
                ''',
                "src/greet.omni": '''
                    export fn greet(name: String) { println(name); }
                ''',
                ".omnilang.toml": '''
                    [project]
                    name = "test_project"
                ''',
            })
            # project_dir is a pathlib.Path
    """
    import pathlib

    def _create_project(files: dict[str, str]) -> pathlib.Path:
        project_dir = tmp_path / f"omni_project_{uuid.uuid4().hex[:8]}"
        project_dir.mkdir(parents=True, exist_ok=True)

        for rel_path, content in files.items():
            full_path = project_dir / rel_path
            full_path.parent.mkdir(parents=True, exist_ok=True)
            full_path.write_text(content, encoding="utf-8")

        return project_dir

    return _create_project


@pytest.fixture
def async_test_runner():
    """Compile and run async OmniLang code for a specific target.

    Wraps the compile pipeline with async runtime orchestration.

    Usage:
        def test_async_python(async_test_runner):
            output = async_test_runner('''
                async fn fetch() -> String { return "ok"; }
                async fn main() { println(await fetch()); }
            ''', target="python")
            assert "ok" in output
    """
    def _run(source: str, target: str) -> str:
        from omnilang.parser import parse
        from omnilang.semantic import analyze
        from omnilang.codegen import generate

        prog = parse(source, filename="<test_async>")
        source_lines = {"<test_async>": source.split("\n")}
        type_map, rename_map = analyze(prog, source_lines)
        code = generate(prog, target, rename_map, type_map)

        run_dir = os.path.join(BUILD_DIR, f"async_{_RUN_ID}")
        os.makedirs(run_dir, exist_ok=True)
        ext_map = {"rust": ".rs", "dart": ".dart", "js": ".js", "python": ".py"}
        out_path = os.path.join(run_dir, f"test_async{ext_map[target]}")
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(code)

        if target == "python":
            result = subprocess.run(["python", out_path], capture_output=True, text=True, timeout=15)
        elif target == "js":
            result = subprocess.run(["node", out_path], capture_output=True, text=True, timeout=15)
        elif target == "dart":
            result = subprocess.run(["dart", "run", out_path], capture_output=True, text=True, timeout=15)
        elif target == "rust":
            rust_dir = os.path.join(run_dir, "rust_test_async")
            os.makedirs(rust_dir, exist_ok=True)
            import shutil
            src_dir = os.path.join(rust_dir, "src")
            os.makedirs(src_dir, exist_ok=True)
            shutil.copy(out_path, os.path.join(src_dir, "main.rs"))
            cargo_path = os.path.join(rust_dir, "Cargo.toml")
            safe_name = f"async_test_{_RUN_ID}"
            with open(cargo_path, "w") as f:
                f.write(f'[package]\nname = "{safe_name}"\nversion = "0.1.0"\nedition = "2021"\n')
                f.write('[dependencies]\ntokio = { version = "1", features = ["full"] }\n')
            build_result = subprocess.run(
                ["cargo", "build", "--quiet"], cwd=rust_dir,
                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=120
            )
            if build_result.returncode != 0:
                raise RuntimeError(f"Rust async build failed:\n{build_result.stderr}")
            result = subprocess.run(
                ["cargo", "run", "--quiet"], cwd=rust_dir,
                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=15
            )
        else:
            raise ValueError(f"Unknown target: {target}")

        return result.stdout

    return _run


@pytest.fixture
def analyze_fixture():
    """Run the analyze pipeline on OmniLang source and return diagnostics.

    Usage:
        def test_portability_rule(analyze_fixture):
            diags = analyze_fixture('''
                fn main() { let a: i32 = 5 / 2; }
            ''', targets=["js"])
            assert any(d.code == "P0101" for d in diags)
    """
    def _analyze(source: str, targets: list[str] | None = None) -> list:
        from omnilang.parser import parse
        from omnilang.analyze.diagnostic import Diagnostic

        prog = parse(source, filename="<analyze_test>")
        source_lines = {"<analyze_test>": source.split("\n")}

        # TODO: Replace with full analyze pipeline once Phase 2.5 is implemented.
        # For now, returns an empty list (no-op pipeline for Phase 0 verification).
        diagnostics: list[Diagnostic] = []
        return diagnostics

    return _analyze
