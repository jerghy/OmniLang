# tests/annotation/test_roundtrip.py
"""End-to-end round-trip tests: parse -> generate -> parse -> validate."""

import pytest
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.generator import generate_annotations
from editor.server.annotation.validator import validate_annotations

COMPLETE_SOURCE = """// @code-tab type=logic
import omniview;

// ═══════════════════════════════════════════════════════════
// @chain inc
//   @node n1  signal  name=count  type=i32  pos=100,50
//   @node n2  getVariable  name=count  pos=100,150
//   @node n3  add                       pos=300,50
//   @node n4  setVariable  name=count   pos=500,50
//   @node n5  return                    pos=650,50
//   @wire w1  n1.value -> n2.value
//   @wire w2  n2.value -> n3.a
//   @wire w3  n3.result -> n4.value
//   @wire w4  n4 -> n5.value
// ═══════════════════════════════════════════════════════════
fn inc() {
    let tmp = count + 1;
    count = tmp;
}

// ═══════════════════════════════════════════════════════════
// @chain reset
//   @node n6  signal  name=count  type=i32  pos=100,50
//   @node n7  literal  value=0  type=i32   pos=100,150
//   @node n8  setVariable  name=count      pos=300,50
//   @node n9  return                       pos=450,50
//   @wire w5  n7.value -> n8.value
//   @wire w6  n8 -> n9.value
// ═══════════════════════════════════════════════════════════
fn reset() {
    count = 0;
}

component Counter {
    signal count: i32 = 0;

    view {
        // @edit
        <div id="box" class: box>
            // @edit
            <h3 id="title" class: title>counter demo</h3>
            // @edit
            <p id="big" class: big-number>{count}</p>
            // @edit
            <div id="row" class: row>
                // @edit
                <button id="btn-inc" class: btn @onPress={inc}>+</button>
                // @edit
                <button id="btn-reset" class: btn-reset @onPress={reset}>reset</button>
            </div>
        </div>
    }
    style {
        .box { background: white; }
    }
}
"""


class TestFullRoundTrip:
    def test_complete_file_roundtrip(self):
        """Parse a full file, validate, generate, re-parse, and verify equality."""
        ann1 = parse_annotations(COMPLETE_SOURCE, "Counter.omni")
        errors1 = validate_annotations(ann1)
        assert len(errors1) == 0, f"Validation errors in original: {errors1}"

        generated = generate_annotations(ann1, COMPLETE_SOURCE)
        ann2 = parse_annotations(generated, "Counter.omni")
        errors2 = validate_annotations(ann2)
        assert len(errors2) == 0, f"Validation errors after round-trip: {errors2}"

        assert ann1.code_tab_type == ann2.code_tab_type
        assert len(ann1.view_markers) == len(ann2.view_markers)
        assert len(ann1.chains) == len(ann2.chains)

        for c1, c2 in zip(ann1.chains, ann2.chains):
            assert c1.name == c2.name
            assert len(c1.nodes) == len(c2.nodes)
            assert len(c1.wires) == len(c2.wires)
            for n1, n2 in zip(c1.nodes, c2.nodes):
                assert n1.id == n2.id
                assert n1.tag == n2.tag
                assert n1.attrs == n2.attrs
                assert n1.pos_x == n2.pos_x
                assert n1.pos_y == n2.pos_y
            for w1, w2 in zip(c1.wires, c2.wires):
                assert w1.id == w2.id
                assert w1.from_id == w2.from_id
                assert w1.from_port == w2.from_port
                assert w1.to_id == w2.to_id
                assert w1.to_port == w2.to_port

    def test_generated_source_is_valid_omni(self):
        """Generated source should still be parsable by the Omni compiler."""
        ann = parse_annotations(COMPLETE_SOURCE, "Counter.omni")
        generated = generate_annotations(ann, COMPLETE_SOURCE)

        assert "fn inc()" in generated
        assert "fn reset()" in generated
        assert "component Counter" in generated
        assert "signal count" in generated
        assert "view {" in generated

    def test_multiple_roundtrips_are_stable(self):
        """Three round-trips should produce identical output (idempotent)."""
        ann = parse_annotations(COMPLETE_SOURCE, "test.omni")
        gen1 = generate_annotations(ann, COMPLETE_SOURCE)
        ann2 = parse_annotations(gen1, "test.omni")
        gen2 = generate_annotations(ann2, gen1)

        ann3 = parse_annotations(gen2, "test.omni")
        assert len(ann2.chains) == len(ann3.chains)
        assert len(ann2.view_markers) == len(ann3.view_markers)
