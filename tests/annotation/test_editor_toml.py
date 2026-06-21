# tests/annotation/test_editor_toml.py
"""Tests for .omni-editor.toml read/write."""

import tempfile
from pathlib import Path
from editor.server.annotation.editor_toml import EditorTomlState, read_editor_toml, write_editor_toml

SAMPLE_TOML = """[editor]
active_file = "src/main.omni"
active_mode = "design"

[editor.open_files]
files = ["src/main.omni", "src/lib/validation.omni"]

[editor.panels]
left_open = true
right_width = 320

[editor.viewports."src/main.omni"]
zoom = 1.0
scroll_x = 0
scroll_y = 100

[editor.viewports."src/lib/validation.omni"]
zoom = 0.8
scroll_x = 0
scroll_y = 0
"""

class TestReadEditorToml:
    def test_read_full_state(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".toml", delete=False) as f:
            f.write(SAMPLE_TOML)
            f.close()
            state = read_editor_toml(Path(f.name))
            Path(f.name).unlink()

        assert state.active_file == "src/main.omni"
        assert state.active_mode == "design"
        assert "src/main.omni" in state.open_files
        assert "src/lib/validation.omni" in state.open_files
        assert state.left_open is True
        assert state.right_width == 320
        assert state.viewports["src/main.omni"]["zoom"] == 1.0
        assert state.viewports["src/lib/validation.omni"]["scroll_y"] == 0

    def test_read_non_existent_returns_default(self):
        state = read_editor_toml(Path("/nonexistent/path.toml"))
        assert state.active_file == ""
        assert state.open_files == []

class TestWriteEditorToml:
    def test_write_and_read_roundtrip(self):
        state = EditorTomlState(
            active_file="src/main.omni",
            active_mode="design",
            open_files=["src/main.omni", "src/lib/validation.omni"],
            left_open=True,
            right_width=320,
            viewports={
                "src/main.omni": {"zoom": 1.0, "scroll_x": 0, "scroll_y": 100},
            },
        )
        with tempfile.NamedTemporaryFile(mode="w", suffix=".toml", delete=False) as f:
            f.close()
            path = Path(f.name)
            write_editor_toml(path, state)
            read_back = read_editor_toml(path)
            Path(f.name).unlink()

        assert read_back.active_file == state.active_file
        assert read_back.active_mode == state.active_mode
        assert read_back.open_files == state.open_files
