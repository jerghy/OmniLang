# tests/annotation/test_validator.py
"""Tests for annotation validator -- all P, I, C, S, F rules."""

import pytest
from editor.server.annotation.parser import parse_annotations
from editor.server.annotation.validator import validate_annotations
from editor.server.annotation.types import AnnotationErrorCode

VALID_SOURCE = """// @code-tab type=logic
import omniview;

component Test {
    view {
        // @edit
        <div id="root">
            // @edit
            <p id="msg">Hello</p>
        </div>
    }
}
"""


class TestPositionRules:
    def test_P1_code_tab_must_be_first(self):
        # Parser rejects code before @code-tab; construct AnnotationFile manually
        from editor.server.annotation.types import AnnotationFile, CodeTabType
        file = AnnotationFile(
            filepath="test.omni",
            code_tab_type=CodeTabType.LOGIC,
            code_tab_line=2,
            raw_source="import omniview;\n// @code-tab type=logic\n",
        )
        errors = validate_annotations(file)
        assert any(e.code == AnnotationErrorCode.CODE_TAB_NOT_FIRST for e in errors)

    def test_P6_nodes_must_come_before_wires(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @wire w1  n1.value -> n2.a
//   @node n1  getVariable  name=x  pos=50,50
//   @node n2  add                   pos=200,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.NODE_AFTER_WIRE for e in errors)


class TestIdentityRules:
    def test_I1_duplicate_element_ids(self):
        source = """// @code-tab type=logic
component Test {
    view {
        // @edit
        <div id="dup">
            // @edit
            <p id="dup">text</p>
        </div>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.DUPLICATE_ELEMENT_ID for e in errors)

    def test_I2_duplicate_node_ids(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  getVariable  name=x  pos=50,50
//   @node n1  add                   pos=200,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.DUPLICATE_NODE_ID for e in errors)

    def test_I4_chain_name_mismatch_fn(self):
        source = """// @code-tab type=logic
// ══════
// @chain wrongName
//   @node n1  add  pos=50,50
// ══════
fn correctName() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.CHAIN_NAME_MISMATCH for e in errors)


class TestCompletenessRules:
    def test_C1_missing_required_attr(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  signal  name=count  pos=50,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        # signal requires 'type'
        assert any(e.code == AnnotationErrorCode.MISSING_REQUIRED_ATTR for e in errors)

    def test_C4_missing_edit_for_element(self):
        source = """// @code-tab type=logic
component Test {
    view {
        // @edit
        <div id="root">
            <p id="msg">text</p>
        </div>
    }
}
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.EXTRA_EDIT for e in errors)


class TestConsistencyRules:
    def test_S1_wire_references_nonexistent_node(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  getVariable  name=x  pos=50,50
//   @wire w1  n99.value -> n1.value
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.WIRE_TARGET_NOT_FOUND for e in errors)

    def test_S4_unknown_node_tag(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  unknownTag  pos=50,50
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.UNKNOWN_NODE_TAG for e in errors)

    def test_S2_wire_port_invalid(self):
        source = """// @code-tab type=logic
// ══════
// @chain test
//   @node n1  getVariable  name=x  pos=50,50
//   @node n2  return                pos=200,50
//   @wire w1  n1.invalid -> n2.value
// ══════
fn test() { }
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert any(e.code == AnnotationErrorCode.WIRE_PORT_INVALID for e in errors)


class TestValidationPasses:
    def test_valid_source_passes(self):
        result = parse_annotations(VALID_SOURCE, "test.omni")
        errors = validate_annotations(result)
        assert len(errors) == 0

    def test_full_chain_passes(self):
        source = """// @code-tab type=logic
// ══════
// @chain validate
//   @node n1  getVariable  name=value  pos=50,50
//   @node n2  literal  value=""  type=String  pos=50,150
//   @node n3  compare  op="=="  pos=250,50
//   @node n4  return  value="error"  pos=450,50
//   @wire w1  n1.value -> n3.a
//   @wire w2  n2.value -> n3.b
//   @wire w3  n3.result -> n4.value
// ══════
pub fn validate(value: String): String {
    return "ok";
}
"""
        result = parse_annotations(source, "test.omni")
        errors = validate_annotations(result)
        assert len(errors) == 0
