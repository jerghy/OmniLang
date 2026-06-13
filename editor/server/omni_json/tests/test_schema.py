"""Tests for .omni.json schema serialization, validation, and round-trips."""

import pytest
from editor.server.omni_json.schema import (
    OmniJsonDocument, OmniMetadata, ComponentSnapshot, StoreSnapshot, ThemeSnapshot,
    ElementEntry, WireEntry, ViewNodeSnapshot, OwnershipRegionEntry,
    SearchIndexEntry, SearchIndexSnapshot, CanvasLayoutSnapshot,
)
from editor.server.omni_json.serializer import OmniJsonSerializer


class TestSchemaRoundTrip:
    """Test that documents survive serialization round-trips."""

    def test_empty_document(self):
        doc = OmniJsonDocument()
        s = OmniJsonSerializer()
        d = s.to_dict(doc)
        doc2 = s.from_dict(d)
        assert doc2.schema_version == "1.0.0"
        assert doc2.meta.project_name == ""
        assert doc2.components == []
        assert doc2.elements == []

    def test_document_with_component(self):
        doc = OmniJsonDocument(
            meta=OmniMetadata(project_name="TestProject", omni_version="0.6.0"),
            components=[
                ComponentSnapshot(
                    name="Counter",
                    props={"label": "String"},
                    signals={"count": {"type": "i32", "init": 0}},
                    events=["onUpdate"],
                    view=ViewNodeSnapshot(
                        _kind="element",
                        tag="Container",
                        children=[
                            ViewNodeSnapshot(_kind="text", text="Hello"),
                            ViewNodeSnapshot(_kind="interpolation", expression="{count}"),
                        ],
                    ),
                )
            ],
        )
        s = OmniJsonSerializer()
        d = s.to_dict(doc)
        doc2 = s.from_dict(d)

        assert len(doc2.components) == 1
        comp = doc2.components[0]
        assert comp.name == "Counter"
        assert comp.signals == {"count": {"type": "i32", "init": 0}}
        assert comp.view is not None
        assert comp.view.tag == "Container"
        assert len(comp.view.children) == 2
        assert comp.view.children[0]._kind == "text"
        assert comp.view.children[0].text == "Hello"
        assert comp.view.children[1]._kind == "interpolation"
        assert comp.view.children[1].expression == "{count}"

    def test_document_with_store(self):
        doc = OmniJsonDocument(
            stores=[
                StoreSnapshot(
                    name="UserStore",
                    signals={"username": "String", "loading": "bool"},
                    signal_defaults={"username": "Guest", "loading": False},
                    actions=[
                        {"name": "login", "params": [{"name": "user", "type": "String"}], "body": ""},
                        {"name": "logout", "params": [], "body": ""},
                    ],
                )
            ],
        )
        s = OmniJsonSerializer()
        d = s.to_dict(doc)
        doc2 = s.from_dict(d)

        assert len(doc2.stores) == 1
        store = doc2.stores[0]
        assert store.name == "UserStore"
        assert len(store.signals) == 2
        assert store.signal_defaults["username"] == "Guest"
        assert len(store.actions) == 2

    def test_document_with_elements_and_wires(self):
        doc = OmniJsonDocument(
            elements=[
                ElementEntry(id="el-btn-1", node_type="view", tag="Button",
                             props={"label": "Click"}, children=[]),
                ElementEntry(id="cb-signal-count", node_type="codeblock", tag="signal",
                             props={"name": "count", "typeAnnotation": "i32"}),
            ],
            wires=[
                WireEntry(id="w1", from_id="el-btn-1", from_port="onPress",
                          to_id="cb-signal-count", to_port="set"),
            ],
        )
        s = OmniJsonSerializer()
        d = s.to_dict(doc)
        doc2 = s.from_dict(d)

        assert len(doc2.elements) == 2
        assert doc2.elements[0].id == "el-btn-1"
        assert doc2.elements[0].tag == "Button"
        assert doc2.elements[1].node_type == "codeblock"
        assert len(doc2.wires) == 1
        assert doc2.wires[0].from_port == "onPress"
        assert doc2.wires[0].to_port == "set"

    def test_json_serialization(self):
        doc = OmniJsonDocument(
            meta=OmniMetadata(project_name="Test"),
            components=[ComponentSnapshot(name="Foo")],
        )
        s = OmniJsonSerializer()
        json_str = s.to_json(doc)
        assert '"schemaVersion"' in json_str
        assert '"components"' in json_str
        assert '"Foo"' in json_str

        doc2 = s.from_json(json_str)
        assert doc2.meta.project_name == "Test"
        assert doc2.components[0].name == "Foo"

    def test_schema_version_validation(self):
        s = OmniJsonSerializer()
        with pytest.raises(ValueError, match="Unsupported schema version"):
            s.from_dict({"schema_version": "9.9.9", "meta": {}, "components": []})

    def test_camel_case_keys(self):
        """Verify JSON output uses camelCase keys."""
        doc = OmniJsonDocument(
            meta=OmniMetadata(project_name="Test", is_hand_written=True),
        )
        s = OmniJsonSerializer()
        d = s.to_dict(doc)
        assert "projectName" in d["meta"]
        assert "isHandWritten" in d["meta"]
        assert "schemaVersion" in d

    def test_unknown_kind_fallback(self):
        """Unknown _kind values should fall back to 'fragment' without crashing."""
        s = OmniJsonSerializer()
        # Simulate a future schema version with an unknown _kind
        data = {
            "schema_version": "1.0.0",
            "meta": {},
            "components": [{
                "name": "Test",
                "view": {
                    "_kind": "future-video-embed",  # Unknown kind
                    "tag": "Video",
                    "text": "should not crash",
                    "children": [
                        {"_kind": "text", "text": "Nested child still works"}
                    ]
                }
            }],
        }
        doc = s.from_dict(data)
        assert doc.components[0].view is not None
        # Unknown kind → fragment (children preserved)
        assert doc.components[0].view._kind == "fragment"
        # Children should still be intact
        assert len(doc.components[0].view.children) == 1
        assert doc.components[0].view.children[0]._kind == "text"
        assert doc.components[0].view.children[0].text == "Nested child still works"

    def test_unknown_kind_no_children(self):
        """Unknown _kind with no children should still not crash."""
        s = OmniJsonSerializer()
        data = {
            "schema_version": "1.0.0",
            "meta": {},
            "components": [{
                "name": "Test",
                "view": {"_kind": "future-widget", "tag": "Widget"},
            }],
        }
        doc = s.from_dict(data)
        assert doc.components[0].view._kind == "fragment"

    def test_double_round_trip(self):
        """Document → dict → doc → dict should be stable."""
        doc = OmniJsonDocument(
            components=[ComponentSnapshot(name="A")],
            stores=[StoreSnapshot(name="B")],
            elements=[ElementEntry(id="e1", tag="Text")],
        )
        s = OmniJsonSerializer()
        d1 = s.to_dict(doc)
        doc2 = s.from_dict(d1)
        d2 = s.to_dict(doc2)
        # Compare relevant fields (ignore timestamps)
        assert d1["components"] == d2["components"]
        assert d1["stores"] == d2["stores"]
        assert d1["elements"] == d2["elements"]
