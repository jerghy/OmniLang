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
