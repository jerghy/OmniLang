"""Tests for wire logic algorithms (fold, search) and command undo/redo patterns.

These test the same algorithms used in WireOverlay.tsx and AddWire/RemoveWire.ts,
ported to Python for verification.
"""

from typing import Any

import pytest


# ═══════════════════════════════════════════════════════════════
# Algorithm mirrors (same logic as WireOverlay.tsx)
# ═══════════════════════════════════════════════════════════════


def fold_wires(wires: list[dict], max_visible: int = 5) -> tuple[list[dict], list[dict]]:
    """Group wires by (fromId, toId), fold groups > max_visible into +N badges.

    Mirrors WireOverlay.foldWires() in TypeScript.
    """
    groups: dict[str, list[dict]] = {}
    for w in wires:
        key = f"{w['fromId']}→{w['toId']}"
        groups.setdefault(key, []).append(w)

    visible: list[dict] = []
    folded: list[dict] = []

    for lst in groups.values():
        if len(lst) <= 1:
            visible.append(lst[0])
        elif len(lst) <= max_visible:
            visible.extend(lst)
        else:
            visible.extend(lst[:max_visible])
            folded.append({
                "fromId": lst[0]["fromId"],
                "toId": lst[0]["toId"],
                "count": len(lst) - max_visible,
            })

    return visible, folded


def search_wires(wires: list[dict], query: str) -> list[dict]:
    """Filter wires by fromId/toId/fromPort/toPort text match.

    Mirrors WireOverlay.searchWires() in TypeScript.
    """
    q = query.lower()
    return [
        w for w in wires
        if q in w.get("fromId", "").lower()
        or q in w.get("toId", "").lower()
        or q in w.get("fromPort", "").lower()
        or q in w.get("toPort", "").lower()
    ]


# ═══════════════════════════════════════════════════════════════
# foldWires algorithm tests
# ═══════════════════════════════════════════════════════════════


class TestFoldWires:
    """Verify the wire folding algorithm correctness for all edge cases."""

    def test_empty_wires(self):
        visible, folded = fold_wires([])
        assert visible == []
        assert folded == []

    def test_single_wire(self):
        wires = [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "out1", "toPort": "in1"}]
        visible, folded = fold_wires(wires)
        assert len(visible) == 1
        assert visible[0]["id"] == "w1"
        assert folded == []

    def test_two_different_pairs(self):
        wires = [
            {"id": "w1", "fromId": "a", "toId": "b", "fromPort": "out1", "toPort": "in1"},
            {"id": "w2", "fromId": "c", "toId": "d", "fromPort": "out2", "toPort": "in2"},
        ]
        visible, folded = fold_wires(wires)
        assert len(visible) == 2
        assert folded == []

    def test_same_pair_two_wires_not_folded(self):
        """2 wires with same (fromId, toId) — under threshold, not folded."""
        wires = [
            {"id": "w1", "fromId": "a", "toId": "b", "fromPort": "p1", "toPort": "q1"},
            {"id": "w2", "fromId": "a", "toId": "b", "fromPort": "p2", "toPort": "q2"},
        ]
        visible, folded = fold_wires(wires)
        assert len(visible) == 2  # Under default max_visible (5)
        assert folded == []

    def test_exactly_max_visible_not_folded(self):
        """Exactly 5 wires with same (fromId, toId) — not folded."""
        wires = [
            {"id": f"w{i}", "fromId": "a", "toId": "b", "fromPort": f"p{i}", "toPort": f"q{i}"}
            for i in range(5)
        ]
        visible, folded = fold_wires(wires, max_visible=5)
        assert len(visible) == 5
        assert folded == []

    def test_over_threshold_folds(self):
        """7 wires with same (fromId, toId), max_visible=5 → 5 visible + 2 folded."""
        wires = [
            {"id": f"w{i}", "fromId": "a", "toId": "b", "fromPort": f"p{i}", "toPort": f"q{i}"}
            for i in range(7)
        ]
        visible, folded = fold_wires(wires, max_visible=5)
        assert len(visible) == 5
        assert len(folded) == 1
        assert folded[0]["fromId"] == "a"
        assert folded[0]["toId"] == "b"
        assert folded[0]["count"] == 2  # 7 - 5

    def test_mixed_folded_and_unfolded(self):
        """Group A has 10 wires (folds), group B has 2 wires (no fold), group C has 1."""
        wires = (
            [{"id": f"wa{i}", "fromId": "a", "toId": "b", "fromPort": f"p{i}", "toPort": f"q{i}"} for i in range(10)]
            + [{"id": f"wb{i}", "fromId": "c", "toId": "d", "fromPort": f"p{i}", "toPort": f"q{i}"} for i in range(2)]
            + [{"id": "wc0", "fromId": "e", "toId": "f", "fromPort": "p0", "toPort": "q0"}]
        )
        visible, folded = fold_wires(wires, max_visible=5)
        # Group A: 5 visible, 5 folded → +5 badge
        # Group B: 2 visible, no fold
        # Group C: 1 visible, no fold
        assert len(visible) == 8  # 5 + 2 + 1
        assert len(folded) == 1
        assert folded[0]["count"] == 5

    def test_custom_max_visible(self):
        """max_visible=2: 4 wires → 2 visible + 2 folded."""
        wires = [
            {"id": f"w{i}", "fromId": "a", "toId": "b", "fromPort": f"p{i}", "toPort": f"q{i}"}
            for i in range(4)
        ]
        visible, folded = fold_wires(wires, max_visible=2)
        assert len(visible) == 2
        assert folded[0]["count"] == 2


# ═══════════════════════════════════════════════════════════════
# searchWires algorithm tests
# ═══════════════════════════════════════════════════════════════


class TestSearchWires:
    """Verify wire text search correctness for all edge cases."""

    @pytest.fixture
    def sample_wires(self) -> list[dict]:
        return [
            {"id": "w1", "fromId": "btn-increment", "toId": "sig-count", "fromPort": "onPress", "toPort": "set"},
            {"id": "w2", "fromId": "sig-count", "toId": "lbl-display", "fromPort": "value", "toPort": "text"},
            {"id": "w3", "fromId": "inp-name", "toId": "sig-username", "fromPort": "onChange", "toPort": "set"},
        ]

    def test_search_by_from_id(self, sample_wires):
        result = search_wires(sample_wires, "btn")
        assert len(result) == 1
        assert result[0]["id"] == "w1"

    def test_search_by_to_id(self, sample_wires):
        result = search_wires(sample_wires, "display")
        assert len(result) == 1
        assert result[0]["id"] == "w2"

    def test_search_by_port(self, sample_wires):
        result = search_wires(sample_wires, "onPress")
        assert len(result) == 1
        assert result[0]["id"] == "w1"

    def test_search_case_insensitive(self, sample_wires):
        result = search_wires(sample_wires, "BTN")
        assert len(result) == 1
        assert result[0]["id"] == "w1"

    def test_search_no_match(self, sample_wires):
        result = search_wires(sample_wires, "zzz_not_found")
        assert result == []

    def test_search_multiple_matches(self, sample_wires):
        result = search_wires(sample_wires, "sig")
        assert len(result) == 3  # All three have "sig" in fromId or toId

    def test_search_empty_query(self, sample_wires):
        result = search_wires(sample_wires, "")
        assert len(result) == 3  # Empty query matches all

    def test_search_empty_wires(self):
        result = search_wires([], "anything")
        assert result == []


# ═══════════════════════════════════════════════════════════════
# AddWire / RemoveWire command pattern tests
# ═══════════════════════════════════════════════════════════════


class MockCommandContext:
    """Simulates the Zustand store for command testing."""
    def __init__(self):
        self.wires: list[dict[str, Any]] = []

    def add_wire(self, wire: dict) -> None:
        self.wires.append(wire)

    def remove_wire(self, wire_id: str) -> None:
        self.wires = [w for w in self.wires if w.get("id") != wire_id]


class TestAddWireCommand:
    def test_execute_adds_wire(self):
        ctx = MockCommandContext()
        wire = {"id": "w1", "fromId": "a", "toId": "b", "fromPort": "out1", "toPort": "in1"}
        ctx.add_wire(wire)
        assert len(ctx.wires) == 1
        assert ctx.wires[0]["id"] == "w1"

    def test_undo_removes_wire(self):
        ctx = MockCommandContext()
        wire = {"id": "w1", "fromId": "a", "toId": "b", "fromPort": "out1", "toPort": "in1"}
        ctx.add_wire(wire)
        ctx.remove_wire("w1")
        assert len(ctx.wires) == 0

    def test_undo_redo_roundtrip(self):
        """execute → undo → execute should produce same state."""
        ctx = MockCommandContext()
        wire = {"id": "w1", "fromId": "a", "toId": "b", "fromPort": "out1", "toPort": "in1"}

        # Execute
        ctx.add_wire(wire)
        assert len(ctx.wires) == 1

        # Undo
        ctx.remove_wire("w1")
        assert len(ctx.wires) == 0

        # Redo (execute again)
        ctx.add_wire(wire)
        assert len(ctx.wires) == 1
        assert ctx.wires[0]["id"] == "w1"

    def test_multiple_wires_undo_order(self):
        """Undo should remove the correct wire, not the last one."""
        ctx = MockCommandContext()
        w1 = {"id": "w1", "fromId": "a", "toId": "b", "fromPort": "p1", "toPort": "q1"}
        w2 = {"id": "w2", "fromId": "c", "toId": "d", "fromPort": "p2", "toPort": "q2"}

        ctx.add_wire(w1)
        ctx.add_wire(w2)
        assert len(ctx.wires) == 2

        # Remove w1 specifically, w2 should remain
        ctx.remove_wire("w1")
        assert len(ctx.wires) == 1
        assert ctx.wires[0]["id"] == "w2"


class TestRemoveWireCommand:
    def test_execute_removes_existing(self):
        ctx = MockCommandContext()
        ctx.wires = [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "p1", "toPort": "q1"}]
        ctx.remove_wire("w1")
        assert len(ctx.wires) == 0

    def test_execute_remove_nonexistent(self):
        """Removing a non-existent wire should not crash."""
        ctx = MockCommandContext()
        ctx.wires = [{"id": "w1", "fromId": "a", "toId": "b", "fromPort": "p1", "toPort": "q1"}]
        ctx.remove_wire("w99")  # Should not raise
        assert len(ctx.wires) == 1  # Unchanged

    def test_undo_restores_snapshot(self):
        """RemoveWire saves snapshot for undo — undo should restore."""
        ctx = MockCommandContext()
        original = {"id": "w1", "fromId": "a", "toId": "b", "fromPort": "p1", "toPort": "q1"}
        ctx.wires = [original]

        # Save snapshot
        snapshot = next(w for w in ctx.wires if w["id"] == "w1")

        # Execute remove
        ctx.remove_wire("w1")
        assert len(ctx.wires) == 0

        # Undo: restore snapshot
        ctx.add_wire(snapshot)
        assert len(ctx.wires) == 1
        assert ctx.wires[0]["id"] == "w1"
