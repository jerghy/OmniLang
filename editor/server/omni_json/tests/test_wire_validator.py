"""Tests for wire validators — port type compatibility, cycle detection, duplicate prevention."""

import pytest
from editor.server.omni_json.wire_validator import (
    validate_port_types,
    validate_port_directions,
    has_duplicate_wire,
    has_self_loop,
    detect_cycle,
    validate_wire_connection,
    WireValidationError,
)


class TestPortTypeValidation:
    def test_exact_match(self):
        ok, err = validate_port_types("i32", "i32")
        assert ok is True
        assert err is None

    def test_compatible_types(self):
        ok, _ = validate_port_types("i32", "f64")
        assert ok is True

    def test_string_to_any(self):
        ok, _ = validate_port_types("String", "any")
        assert ok is True

    def test_incompatible_types(self):
        ok, err = validate_port_types("nonexistent", "i32")
        assert ok is False
        assert "Type mismatch" in err

    def test_strict_mode_rejects_compatible(self):
        ok, err = validate_port_types("i32", "f64", strict=True)
        assert ok is False
        assert "Strict type mismatch" in err

    def test_strict_mode_accepts_exact(self):
        ok, _ = validate_port_types("String", "String", strict=True)
        assert ok is True

    def test_any_source_accepts_all(self):
        ok, _ = validate_port_types("any", "i32")
        assert ok is True
        ok, _ = validate_port_types("any", "String")
        assert ok is True


class TestPortDirectionValidation:
    def test_valid_directions(self):
        from_ports = [{"name": "value", "direction": "out", "type": "i32"}]
        to_ports = [{"name": "set", "direction": "in", "type": "i32"}]
        ok, err = validate_port_directions(from_ports, to_ports, "value", "set")
        assert ok is True
        assert err is None

    def test_source_not_output(self):
        from_ports = [{"name": "set", "direction": "in", "type": "i32"}]
        to_ports = [{"name": "value", "direction": "out", "type": "i32"}]
        ok, err = validate_port_directions(from_ports, to_ports, "set", "value")
        assert ok is False
        assert "must be an output" in err

    def test_target_not_input(self):
        from_ports = [{"name": "value", "direction": "out", "type": "i32"}]
        to_ports = [{"name": "value", "direction": "out", "type": "i32"}]
        ok, err = validate_port_directions(from_ports, to_ports, "value", "value")
        assert ok is False
        assert "must be an input" in err

    def test_port_not_found(self):
        from_ports = [{"name": "value", "direction": "out", "type": "i32"}]
        to_ports = [{"name": "set", "direction": "in", "type": "i32"}]
        ok, err = validate_port_directions(from_ports, to_ports, "nonexistent", "set")
        assert ok is False
        assert "not found" in err


class TestDuplicateDetection:
    def test_no_duplicate(self):
        wires = [{"id": "w1", "fromId": "a", "fromPort": "out1", "toId": "b", "toPort": "in1"}]
        assert has_duplicate_wire(wires, "a", "out2", "b", "in1") is False

    def test_duplicate_found(self):
        wires = [{"id": "w1", "fromId": "a", "fromPort": "out1", "toId": "b", "toPort": "in1"}]
        assert has_duplicate_wire(wires, "a", "out1", "b", "in1") is True

    def test_exclude_id_skips_self(self):
        wires = [{"id": "w1", "fromId": "a", "fromPort": "out1", "toId": "b", "toPort": "in1"}]
        assert has_duplicate_wire(wires, "a", "out1", "b", "in1", exclude_id="w1") is False

    def test_empty_wires(self):
        assert has_duplicate_wire([], "a", "out1", "b", "in1") is False

    def test_self_loop(self):
        assert has_self_loop("a", "a") is True
        assert has_self_loop("a", "b") is False


class TestCycleDetection:
    def test_no_cycle_simple(self):
        wires: list[dict] = []
        assert detect_cycle(wires, "a", "b") is False

    def test_direct_cycle(self):
        """a → b already exists, adding b → a would create cycle"""
        wires = [{"id": "w1", "fromId": "a", "toId": "b"}]
        assert detect_cycle(wires, "b", "a") is True

    def test_indirect_cycle(self):
        """a → b → c exists, adding c → a would create cycle"""
        wires = [
            {"id": "w1", "fromId": "a", "toId": "b"},
            {"id": "w2", "fromId": "b", "toId": "c"},
        ]
        assert detect_cycle(wires, "c", "a") is True

    def test_no_cycle_different_branch(self):
        """a → b and a → c exist, adding c → d is fine"""
        wires = [
            {"id": "w1", "fromId": "a", "toId": "b"},
            {"id": "w2", "fromId": "a", "toId": "c"},
        ]
        assert detect_cycle(wires, "c", "d") is False

    def test_no_cycle_diamond(self):
        """a → b → d and a → c → d (diamond, no cycle)"""
        wires = [
            {"id": "w1", "fromId": "a", "toId": "b"},
            {"id": "w2", "fromId": "b", "toId": "d"},
            {"id": "w3", "fromId": "a", "toId": "c"},
            {"id": "w4", "fromId": "c", "toId": "d"},
        ]
        assert detect_cycle(wires, "d", "e") is False


class TestCombinedValidation:
    def _make_element(self, id: str, ports: list[dict]) -> dict:
        return {"id": id, "tag": "signal", "ports": ports}

    def test_valid_connection_passes(self):
        elements = [
            self._make_element("el-a", [{"name": "value", "direction": "out", "type": "i32"}]),
            self._make_element("el-b", [{"name": "set", "direction": "in", "type": "i32"}]),
        ]
        errors = validate_wire_connection(elements, [], "el-a", "value", "el-b", "set")
        assert errors == []

    def test_self_loop_blocked(self):
        elements = [
            self._make_element("el-a", [
                {"name": "value", "direction": "out", "type": "i32"},
                {"name": "set", "direction": "in", "type": "i32"},
            ]),
        ]
        errors = validate_wire_connection(elements, [], "el-a", "value", "el-a", "set")
        assert any("itself" in e for e in errors)

    def test_duplicate_blocked(self):
        elements = [
            self._make_element("el-a", [{"name": "value", "direction": "out", "type": "i32"}]),
            self._make_element("el-b", [{"name": "set", "direction": "in", "type": "i32"}]),
        ]
        wires = [{"id": "w1", "fromId": "el-a", "fromPort": "value", "toId": "el-b", "toPort": "set"}]
        errors = validate_wire_connection(elements, wires, "el-a", "value", "el-b", "set")
        assert any("Duplicate" in e for e in errors)

    def test_cycle_blocked(self):
        elements = [
            self._make_element("el-a", [
                {"name": "value", "direction": "out", "type": "i32"},
                {"name": "set", "direction": "in", "type": "i32"},
            ]),
            self._make_element("el-b", [
                {"name": "value", "direction": "out", "type": "i32"},
                {"name": "set", "direction": "in", "type": "i32"},
            ]),
        ]
        wires = [{"id": "w1", "fromId": "el-a", "fromPort": "value", "toId": "el-b", "toPort": "set"}]
        errors = validate_wire_connection(elements, wires, "el-b", "value", "el-a", "set")
        assert any("Cycle" in e for e in errors)

    def test_missing_element(self):
        elements = [
            self._make_element("el-a", [{"name": "value", "direction": "out", "type": "i32"}]),
        ]
        errors = validate_wire_connection(elements, [], "el-a", "value", "el-z", "set")
        assert any("not found" in e for e in errors)

    def test_multiple_errors(self):
        """Self-loop + missing target = multiple errors in one call."""
        elements = [
            self._make_element("el-a", [
                {"name": "value", "direction": "out", "type": "i32"},
                {"name": "set", "direction": "in", "type": "i32"},
            ]),
        ]
        wires = [{"id": "w1", "fromId": "el-a", "fromPort": "value", "toId": "el-a", "toPort": "set"}]
        errors = validate_wire_connection(elements, wires, "el-a", "value", "el-a", "set")
        assert len(errors) >= 2  # self-loop + duplicate
