"""Tests for IRReader — validates Rust-generated IR JSON input.

All tests use Python-pipeline-generated IR dicts (no Rust dependency).
"""
from __future__ import annotations

import json

import pytest
from omniview.codegen.ir_reader import IRReader
from omniview.tests.helpers import make_counter_ir_dict


# ── Fixture ────────────────────────────────────────────

@pytest.fixture
def counter_ir_json():
    """IR dict from Python pipeline (simulates Rust output)."""
    return make_counter_ir_dict()


# ── Basic read tests ───────────────────────────────────

def test_read_valid_ir(counter_ir_json):
    reader = IRReader(counter_ir_json)
    assert reader.is_valid
    assert reader.errors == []


def test_read_version(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comps = reader.components()
    assert "Counter" in comps


def test_read_component_name(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comp = reader.components()["Counter"]
    assert comp["name"] == "Counter"


def test_read_states(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comp = reader.components()["Counter"]
    assert len(comp["states"]) >= 1
    assert comp["states"][0]["name"] == "count"


def test_read_view(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comp = reader.components()["Counter"]
    assert comp["view"] is not None
    kind = comp["view"].get("_kind", "")
    assert kind in ("ElementIR", "ConditionalIntent", "ListRenderingIntent")


# ── Source type tests (JSON string vs dict) ────────────

def test_accept_json_string(counter_ir_json):
    json_str = json.dumps(counter_ir_json)
    reader = IRReader(json_str)
    assert reader.is_valid


def test_accept_dict(counter_ir_json):
    reader = IRReader(counter_ir_json)
    assert reader.is_valid


# ── Error handling tests ───────────────────────────────

def test_invalid_version():
    reader = IRReader({"version": "0.5", "components": {"A": {}}})
    assert not reader.is_valid
    assert any("version" in e.lower() for e in reader.errors)


def test_empty_ir():
    reader = IRReader({})
    assert not reader.is_valid
    assert len(reader.errors) > 0


def test_missing_components_and_stores():
    reader = IRReader({"version": "1.0"})
    assert not reader.is_valid


def test_invalid_json_string():
    reader = IRReader("not valid json")
    assert not reader.is_valid
    assert any("json" in e.lower() for e in reader.errors)


# ── Stores tests ───────────────────────────────────────

def test_stores_empty(counter_ir_json):
    reader = IRReader(counter_ir_json)
    assert reader.stores() == {}


def test_stores_with_data():
    ir = {
        "version": "1.0",
        "components": {},
        "stores": {
            "CartStore": {
                "_kind": "StoreIR",
                "name": "CartStore",
                "signals": {"items": {"kind": "Array", "type_args": [{"kind": "String"}]}},
                "signal_defaults": {"items": []},
                "getters": {},
                "actions": [],
            }
        },
        "source_map": {},
    }
    reader = IRReader(ir)
    assert reader.is_valid
    stores = reader.stores()
    assert "CartStore" in stores
    assert stores["CartStore"]["name"] == "CartStore"


# ── Source Map test ───────────────────────────────────

def test_source_map(counter_ir_json):
    reader = IRReader(counter_ir_json)
    sm = reader.source_map()
    assert isinstance(sm, dict)


# ── ir_kind static method tests ────────────────────────

def test_ir_kind():
    assert IRReader.ir_kind({"_kind": "ElementIR"}) == "ElementIR"
    assert IRReader.ir_kind({"_kind": "StateIntent"}) == "StateIntent"
    assert IRReader.ir_kind({}) == "unknown"
    assert IRReader.ir_kind("not a dict") == "unknown"


def test_ir_kind_on_real_counter(counter_ir_json):
    """ir_kind works on Counter component dict."""
    reader = IRReader(counter_ir_json)
    comp = reader.components()["Counter"]
    assert IRReader.ir_kind(comp) == "ComponentIR"
