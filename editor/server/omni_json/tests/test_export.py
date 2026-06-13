"""Tests for .omni.json export pipeline (.omni → .omni.json)."""

import pytest
from editor.server.omni_json.exporter import OmniJsonExporter
from editor.server.omni_json.serializer import OmniJsonSerializer


COUNTER_SOURCE = """component Counter {
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
  signal todos: String = "";
  signal loading: bool = false;

  action addTodo(text: String) {
    loading = true;
  }

  action clearTodos() {
    loading = true;
  }
}

component TodoApp {
  @store TodoStore;

  view {
    <Container>
      <Input test-id="todo-input" bind:value={newTodo} />
      <Button onPress={store.addTodo} test-id="btn-add">Add</Button>
    </Container>
  }
}"""


class TestExport:
    """Test .omni → .omni.json export."""

    def test_export_counter_component(self):
        exporter = OmniJsonExporter()
        doc = exporter.export(COUNTER_SOURCE, filepath="Counter.omni")
        s = OmniJsonSerializer()
        d = s.to_dict(doc)

        # Basic structure
        assert d["schemaVersion"] == "1.0.0"
        assert d["meta"]["projectName"] == "Counter"

        # Should have at least 1 component
        assert len(d["components"]) >= 1

        # Should have elements from view tree
        assert len(d["elements"]) >= 1

        # Should have search index entries
        entries = d["searchIndex"]["entries"]
        assert len(entries) >= 1

    def test_export_store_fixture(self):
        exporter = OmniJsonExporter()
        doc = exporter.export(STORE_SOURCE, filepath="TodoApp.omni")
        s = OmniJsonSerializer()
        d = s.to_dict(doc)

        # Should have store data
        stores = d.get("stores", [])
        assert len(stores) >= 1
        store = next((s for s in stores if s["name"] == "TodoStore"), None)
        assert store is not None
        assert "todos" in store.get("signals", {})
        assert "loading" in store.get("signals", {})
        assert len(store.get("actions", [])) >= 2

    def test_export_preserves_existing_wires(self):
        exporter = OmniJsonExporter()
        existing_wires = [
            {"id": "w1", "fromId": "el-a", "fromPort": "out", "toId": "el-b", "toPort": "in"},
        ]
        doc = exporter.export(COUNTER_SOURCE, existing_wires=existing_wires)
        s = OmniJsonSerializer()
        d = s.to_dict(doc)

        wires = d.get("wires", [])
        # Should preserve existing wire
        assert any(w["id"] == "w1" for w in wires)

    def test_export_preserves_canvas_positions(self):
        exporter = OmniJsonExporter()
        existing_canvas = {"positions": {"el-1": {"x": 10, "y": 20, "w": 100, "h": 50}}}
        doc = exporter.export(COUNTER_SOURCE, existing_canvas=existing_canvas)
        s = OmniJsonSerializer()
        d = s.to_dict(doc)

        positions = d.get("canvasLayout", {}).get("positions", {})
        assert "el-1" in positions
        assert positions["el-1"]["x"] == 10

    def test_export_invalid_source_does_not_crash(self):
        """Export should not crash on invalid .omni source."""
        exporter = OmniJsonExporter()
        doc = exporter.export("this is not valid omni source {{")
        assert doc is not None
        assert doc.schema_version == "1.0.0"

    def test_detect_hand_written(self):
        exporter = OmniJsonExporter()

        # Hand-written: has comments but no @editor:managed marker
        assert exporter._detect_hand_written("// Hello\ncomponent Foo {}") is True

        # Editor-generated: has @editor:managed marker
        assert exporter._detect_hand_written("// @editor:managed\ncomponent Foo {}") is False

        # Clean: no comments at all
        assert exporter._detect_hand_written("component Bar {}") is False

    def test_json_output_is_valid_utf8(self):
        exporter = OmniJsonExporter()
        doc = exporter.export(COUNTER_SOURCE)
        s = OmniJsonSerializer()
        json_str = s.to_json(doc)
        # Should be valid JSON
        import json
        parsed = json.loads(json_str)
        assert parsed["schemaVersion"] == "1.0.0"
