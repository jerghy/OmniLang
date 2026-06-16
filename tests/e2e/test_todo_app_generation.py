"""E2E: Verify todo-app.omni compiles and generates valid structure.

Validates that a real .omni application compiles without errors and
produces expected output files with correct structure.
"""
import pytest
from pathlib import Path
import subprocess
import sys


BUILD_DIR = Path(__file__).parent.parent.parent / "build_output" / "todo-app"
TODO_APP = Path(__file__).parent.parent.parent / "omniview" / "examples" / "todo-app.omni"


@pytest.fixture(scope="module")
def build_todo_app():
    """Build the todo app and return the output directory."""
    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(
        [sys.executable, "-m", "omnilang.cli", "build",
         str(TODO_APP), "--target", "react", "--output", str(BUILD_DIR)],
        capture_output=True, text=True, timeout=60,
    )
    return result, BUILD_DIR


class TestTodoAppCompilation:
    """Verify the todo app compiles successfully."""

    def test_compilation_succeeds(self, build_todo_app):
        """Compilation should exit with code 0."""
        result, _ = build_todo_app
        assert result.returncode == 0, f"Build failed:\n{result.stderr}"

    def test_output_files_exist(self, build_todo_app):
        """Should generate TodoApp.tsx, styles, and store."""
        _, out_dir = build_todo_app
        assert (out_dir / "TodoApp.tsx").exists(), "Missing TodoApp.tsx"
        assert (out_dir / "TodoApp.module.css").exists(), "Missing CSS module"
        assert (out_dir / "useTodoStore.ts").exists(), "Missing store"


class TestTodoAppGeneratedCode:
    """Verify generated code structure is correct."""

    @pytest.fixture(autouse=True)
    def setup(self, build_todo_app):
        _, self.out_dir = build_todo_app

    def _read(self, filename):
        return (self.out_dir / filename).read_text(encoding="utf-8")

    def test_tsx_has_component_export(self):
        """TodoApp.tsx should export a React component."""
        content = self._read("TodoApp.tsx")
        assert "export function TodoApp" in content

    def test_tsx_imports_react(self):
        """TodoApp.tsx should import React hooks."""
        content = self._read("TodoApp.tsx")
        assert "import React" in content or "from \"react\"" in content

    def test_tsx_has_data_testids(self):
        """All test-id attributes should be preserved in output."""
        content = self._read("TodoApp.tsx")
        expected_ids = [
            "app-title", "todo-input", "btn-add",
            "filter-bar", "filter-all", "filter-active", "filter-done",
            "task-list", "task-item", "btn-complete",
            "empty-message",
        ]
        for test_id in expected_ids:
            assert f'data-testid="{test_id}"' in content, \
                f"Missing test-id: {test_id}"

    def test_store_has_zustand_create(self):
        """Store file should use Zustand create."""
        content = self._read("useTodoStore.ts")
        assert "create<TodoStoreState>" in content
        assert "import { create }" in content

    def test_store_has_actions(self):
        """Store should expose CRUD actions."""
        content = self._read("useTodoStore.ts")
        assert "add_task:" in content
        assert "complete_task:" in content
        assert "uncomplete_task:" in content
        assert "clear_completed:" in content

    def test_css_module_exists(self):
        """CSS module should contain style rules."""
        content = self._read("TodoApp.module.css")
        assert len(content.strip()) > 0
        assert ".container" in content or ".heading" in content


# ── Known issues — tests that currently FAIL ──

class TestKnownGaps:
    """Tests for gaps documented in REAL_WORLD_GAPS.md.
    Marked xfail until fixes are implemented.
    """

    @pytest.fixture(autouse=True)
    def setup(self, build_todo_app):
        _, self.out_dir = build_todo_app

    def _read(self, filename):
        return (self.out_dir / filename).read_text(encoding="utf-8")

    @pytest.mark.xfail(reason="B1: Store getter uses bare vars instead of state.xxx")
    def test_store_getter_uses_state_prefix(self):
        """BUG: getter should access state.tasks, not bare tasks."""
        content = self._read("useTodoStore.ts")
        # Should NOT have bare variable access after const state = get()
        import re
        getter_section = content[content.find("active_count:"):content.find("add_task:")]
        # Current bug: returns `tasks.length` instead of `state.tasks.length`
        assert "state.tasks" in getter_section or "state.completed" in getter_section, \
            "Getter should use state.xxx not bare variable"

    @pytest.mark.xfail(reason="B2: Inline fn handler loses call arguments")
    def test_inline_handler_calls_function_with_args(self):
        """BUG: fn() { handle_complete(task); } should generate handle_complete(task)."""
        content = self._read("TodoApp.tsx")
        # Should have handle_complete(task) not just handle_complete;
        import re
        # Look for the inline onClick handler — should CALL the function
        assert "handle_complete(task)" in content or "handle_complete(" in content, \
            "Inline handler should call function with arguments"

    @pytest.mark.xfail(reason="B3: React style should be object not string")
    def test_style_is_react_object(self):
        """BUG: style should be {{textDecoration: ...}} not a string."""
        content = self._read("TodoApp.tsx")
        # Should NOT have style as string
        assert 'style="text_decoration' not in content, \
            "Style should be React object, not string"

    @pytest.mark.xfail(reason="G1: Array<String> should map to string[]")
    def test_array_string_maps_to_string_array(self):
        """GAP: Type info lost — Array<String> → any[] should be string[]."""
        content = self._read("useTodoStore.ts")
        assert "tasks: string[]" in content, \
            "Array<String> should map to string[], not any[]"
