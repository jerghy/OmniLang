"""Round-trip integrity tests: .omni .omni.json .omni compile.

Catches the most probable silent failure mode identified by Council Critic:
importer formatter parser round-trip silently dropping fields.
"""

import os
import pytest
from editor.server.omni_json.exporter import OmniJsonExporter
from editor.server.omni_json.importer import OmniJsonImporter
from editor.server.omni_json.serializer import OmniJsonSerializer
from editor.server.omni_json.schema import OmniJsonDocument, ComponentSnapshot, OmniMetadata
from omniview.api import compile_omni

FIXTURE_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "tests", "frameworks", "behavior")


def _read_fixture(name):
    with open(os.path.join(FIXTURE_DIR, name), encoding="utf-8") as f:
        return f.read()


class TestRoundTripIntegrity:

    @pytest.fixture
    def counter_source(self):
        return _read_fixture("01_counter.omni")

    @pytest.fixture
    def store_source(self):
        return _read_fixture("05_store_read.omni")

    def test_counter_roundtrip(self, counter_source):
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()
        importer = OmniJsonImporter()
        doc = exporter.export(counter_source)
        doc_dict = serializer.to_dict(doc)
        raw = importer.import_document(serializer.from_dict(doc_dict), format=False)
        result = compile_omni(raw, target="react")
        assert result.success, f"Round-trip should compile: {[e.message for e in result.errors]}"

    def test_store_roundtrip(self, store_source):
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()
        importer = OmniJsonImporter()
        doc = exporter.export(store_source)
        doc_dict = serializer.to_dict(doc)
        raw = importer.import_document(serializer.from_dict(doc_dict), format=False)
        result = compile_omni(raw, target="react")
        assert result.success, f"Round-trip should compile: {[e.message for e in result.errors]}"

    def test_roundtrip_preserves_component_names(self, counter_source):
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()
        importer = OmniJsonImporter()
        doc = exporter.export(counter_source)
        doc_dict = serializer.to_dict(doc)
        comp_names = [c["name"] for c in doc_dict.get("components", [])]
        assert "Counter" in comp_names
        doc2 = serializer.from_dict(doc_dict)
        reconstructed = importer.import_document(doc2, format=True)
        assert "Counter" in reconstructed

    def test_roundtrip_preserves_element_count(self, counter_source):
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()
        doc = exporter.export(counter_source)
        d1 = serializer.to_dict(doc)
        elem_count = len(d1.get("elements", []))
        assert elem_count > 0
        doc2 = serializer.from_dict(d1)
        d2 = serializer.to_dict(doc2)
        assert elem_count == len(d2.get("elements", []))

    def test_empty_document_roundtrip(self):
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()
        importer = OmniJsonImporter()
        source = "component Empty { view { <Text>Hello</Text> } }"
        doc = exporter.export(source)
        doc_dict = serializer.to_dict(doc)
        doc2 = serializer.from_dict(doc_dict)
        assert doc2.schema_version == "1.0.0"
        reconstructed = importer.import_document(doc2, format=True)
        result = compile_omni(reconstructed, target="react")
        orig_result = compile_omni(source, target="react")
        assert result.success == orig_result.success

    def test_serializer_idempotent(self):
        serializer = OmniJsonSerializer()
        doc = OmniJsonDocument(
            meta=OmniMetadata(project_name="RoundtripTest"),
            components=[ComponentSnapshot(name="Test")],
        )
        d1 = serializer.to_dict(doc)
        doc2 = serializer.from_dict(d1)
        d2 = serializer.to_dict(doc2)
        assert d1["schemaVersion"] == d2["schemaVersion"]
        assert d1["components"] == d2["components"]
        assert d2["meta"]["projectName"] == "RoundtripTest"
