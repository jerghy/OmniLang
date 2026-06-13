"""Shared test fixtures for editor server tests."""

import os
import sys
import tempfile
import pytest
from pathlib import Path

# Ensure project root on sys.path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))


@pytest.fixture
def editor_app():
    """Return the FastAPI app with a temporary project directory."""
    from editor.server.main import app

    with tempfile.TemporaryDirectory() as tmpdir:
        app.state.project_dir = tmpdir
        app.state.previous_compile = None
        yield app


@pytest.fixture
def client(editor_app):
    """FastAPI TestClient bound to the editor app."""
    from fastapi.testclient import TestClient

    with TestClient(editor_app) as c:
        yield c


@pytest.fixture
def project_dir(editor_app):
    """Path to the temporary project directory."""
    return Path(editor_app.state.project_dir)


# ── Sample .omni sources for testing ──────────────────

PURE_LOGIC_SOURCE = """fn main() {
    println("hello");
}"""

UI_SOURCE = """component Counter {
    signal count: i32 = 0;

    fn increment() {
        count = count + 1;
    }

    view {
        <Container>
            <Heading level="1" test-id="counter-label">Count: {count}</Heading>
            <Button onPress={increment} test-id="btn-increment">+</Button>
        </Container>
    }
}"""

STORE_SOURCE = """store TodoStore {
    signal items: String = "";
    signal loading: bool = false;

    action addItem(text: String) {
        loading = true;
    }

    action clearItems() {
        loading = true;
    }
}

component TodoApp {
    @store TodoStore;

    signal newItem: String = "";

    view {
        <Container>
            <Input test-id="todo-input" bind:value={newItem} />
            <Button onPress={store.addItem} test-id="btn-add">Add</Button>
        </Container>
    }
}"""
