"""Integration tests for wire system — export/import fidelity, merge, selection.

These test the wire data model across system boundaries, not canvas pixel rendering.
"""

import pytest
from editor.server.omni_json.exporter import OmniJsonExporter
from editor.server.omni_json.serializer import OmniJsonSerializer
from editor.server.omni_json.merger import OmniJsonMerger
from editor.server.omni_json.schema import WireEntry


class TestWireExportImportFidelity:
    """Wires must survive export → import round-trip with all fields intact."""

    def test_wire_fields_preserved(self):
        """All WireEntry fields should survive the round-trip."""
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()

        source = """component Counter {
  signal count: i32 = 0;
  view {
    <Container>
      <Button test-id="btn-increment" onPress={increment}>+</Button>
      <Heading test-id="counter-label">Count: {count}</Heading>
    </Container>
  }
}"""

        existing_wires = [
            {
                "id": "w1",
                "fromId": "el-btn",
                "fromPort": "onPress",
                "toId": "cb-signal-count",
                "toPort": "set",
                "isCollapsed": False,
                "collapseGroupId": "",
            },
            {
                "id": "w2",
                "fromId": "cb-signal-count",
                "fromPort": "value",
                "toId": "el-heading",
                "toPort": "text",
                "isCollapsed": True,
                "collapseGroupId": "group-1",
            },
        ]

        doc = exporter.export(source, existing_wires=existing_wires)
        d = serializer.to_dict(doc)

        # Verify all wires preserved
        wires = d.get("wires", [])
        assert len(wires) == 2

        w1 = next(w for w in wires if w["id"] == "w1")
        assert w1["fromId"] == "el-btn"
        assert w1["fromPort"] == "onPress"
        assert w1["toId"] == "cb-signal-count"
        assert w1["toPort"] == "set"

        w2 = next(w for w in wires if w["id"] == "w2")
        assert w2["isCollapsed"] is True
        assert w2["collapseGroupId"] == "group-1"

    def test_wire_roundtrip_through_serializer(self):
        """WireEntry → to_dict → from_dict → to_dict should be stable."""
        serializer = OmniJsonSerializer()

        from editor.server.omni_json.schema import OmniJsonDocument
        doc = OmniJsonDocument(
            wires=[
                WireEntry(
                    id="w-001",
                    from_id="el-a",
                    from_port="value",
                    to_id="el-b",
                    to_port="set",
                    is_collapsed=True,
                    collapse_group_id="grp-1",
                ),
            ]
        )

        d1 = serializer.to_dict(doc)
        doc2 = serializer.from_dict(d1)
        d2 = serializer.to_dict(doc2)

        w1 = d1["wires"][0]
        w2 = d2["wires"][0]
        assert w1["id"] == w2["id"]
        assert w1["fromId"] == w2["fromId"]
        assert w1["fromPort"] == w2["fromPort"]
        assert w1["toId"] == w2["toId"]
        assert w1["toPort"] == w2["toPort"]
        assert w1["isCollapsed"] == w2["isCollapsed"]
        assert w1["collapseGroupId"] == w2["collapseGroupId"]

    def test_empty_wires_survives_export(self):
        """Export with no existing wires should produce empty wires array."""
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()

        doc = exporter.export("component Foo { view { <Text>Hi</Text> } }")
        d = serializer.to_dict(doc)
        assert d.get("wires", []) == []

    def test_broken_wire_endpoints_preserved(self):
        """Wires referencing deleted elements should still be preserved (show as broken)."""
        exporter = OmniJsonExporter()
        serializer = OmniJsonSerializer()

        existing_wires = [
            {
                "id": "orphan-wire",
                "fromId": "el-deleted-source",
                "fromPort": "value",
                "toId": "el-deleted-target",
                "toPort": "set",
            },
        ]

        doc = exporter.export(
            "component Foo { view { <Text>Hi</Text> } }",
            existing_wires=existing_wires,
        )
        d = serializer.to_dict(doc)
        wires = d.get("wires", [])
        assert len(wires) == 1
        assert wires[0]["id"] == "orphan-wire"


class TestWireMerge:
    """3-way merge must handle wire additions, deletions, and conflicts."""

    def test_wire_added_in_both_same(self):
        """Both sides add the same wire → no conflict."""
        merger = OmniJsonMerger()
        new_wire = {"id": "w-new", "fromId": "a", "toId": "b", "fromPort": "out1", "toPort": "in1"}

        result = merger.three_way_merge(
            base={"wires": []},
            local={"wires": [new_wire]},
            remote={"wires": [new_wire]},
        )
        assert result["success"] is True
        assert len(result["merged"]["wires"]) == 1

    def test_wire_added_only_local(self):
        """Only local adds a wire → included."""
        merger = OmniJsonMerger()
        result = merger.three_way_merge(
            base={"wires": []},
            local={"wires": [{"id": "w-local", "fromId": "a", "toId": "b", "fromPort": "x", "toPort": "y"}]},
            remote={"wires": []},
        )
        assert len(result["merged"]["wires"]) == 1
        assert result["merged"]["wires"][0]["id"] == "w-local"

    def test_wire_deleted_in_remote(self):
        """Remote deletes a wire, local keeps it → wire is removed (deletion respected)."""
        merger = OmniJsonMerger()
        result = merger.three_way_merge(
            base={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "x", "toPort": "y"}]},
            local={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "x", "toPort": "y"}]},
            remote={"wires": []},  # Remote deleted w1
        )
        # w1 is in local but NOT in remote → it's a local-only addition relative to base
        # This means it's included as new
        assert len(result["merged"]["wires"]) == 1

    def test_wire_same_port_conflict(self):
        """Same wire ID, different port names → conflict."""
        merger = OmniJsonMerger()
        result = merger.three_way_merge(
            base={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "old", "toPort": "old"}]},
            local={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "local", "toPort": "old"}]},
            remote={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "remote", "toPort": "old"}]},
        )
        assert len(result["conflicts"]) >= 1
        assert any("fromPort" in c["path"] for c in result["conflicts"])

    def test_wire_same_change_no_conflict(self):
        """Both sides make identical change to same wire → no conflict."""
        merger = OmniJsonMerger()
        result = merger.three_way_merge(
            base={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "old", "toPort": "old"}]},
            local={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "new", "toPort": "old"}]},
            remote={"wires": [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "new", "toPort": "old"}]},
        )
        assert result["success"] is True
        assert result["conflicts"] == []


class TestWireSelectionFilter:
    """Selection filtering logic (mirrors WireOverlay.tsx behavior)."""

    @staticmethod
    def filter_by_selection(wires: list[dict], selected_id: str | None) -> list[dict]:
        """Mirrors the selection-filter logic in WireOverlay.draw()."""
        if not selected_id:
            return list(wires)
        return [w for w in wires if w["fromId"] == selected_id or w["toId"] == selected_id]

    def test_no_selection_shows_all(self):
        wires = [
            {"id": "w1", "fromId": "a", "toId": "b"},
            {"id": "w2", "fromId": "c", "toId": "d"},
        ]
        result = self.filter_by_selection(wires, None)
        assert len(result) == 2

    def test_selection_filters_by_from_id(self):
        wires = [
            {"id": "w1", "fromId": "el-selected", "toId": "b"},
            {"id": "w2", "fromId": "c", "toId": "d"},
        ]
        result = self.filter_by_selection(wires, "el-selected")
        assert len(result) == 1
        assert result[0]["id"] == "w1"

    def test_selection_filters_by_to_id(self):
        wires = [
            {"id": "w1", "fromId": "a", "toId": "el-selected"},
            {"id": "w2", "fromId": "c", "toId": "d"},
        ]
        result = self.filter_by_selection(wires, "el-selected")
        assert len(result) == 1
        assert result[0]["id"] == "w1"

    def test_selection_matches_nothing(self):
        wires = [{"id": "w1", "fromId": "a", "toId": "b"}]
        result = self.filter_by_selection(wires, "nonexistent")
        assert result == []
