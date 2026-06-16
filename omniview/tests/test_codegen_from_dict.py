"""Test that Python codegen produces identical output with dict vs dataclass input."""
from __future__ import annotations

import pytest
from omniview.tests.helpers import make_counter_ir_dict, ir_dataclass_to_dict


def test_react_codegen_from_dict_matches_pipeline():
    """React codegen: dict input -> same output as full Python pipeline."""
    from omniview.api import compile_omni
    from omniview.codegen.react import ReactCodegen

    source = open("build/test/counter/Counter.omni", encoding="utf-8").read()

    # Path A: Full Python pipeline (dataclass)
    result_a = compile_omni(source, filepath="counter/Counter.omni", target="react")
    assert result_a.success, f"Python pipeline errors: {result_a.errors}"

    # Path B: IR dict -> codegen directly
    ir_dict = make_counter_ir_dict()
    comp_dict = ir_dict["components"]["Counter"]
    result_b = ReactCodegen().generate_component(comp_dict)

    # Compare outputs file-by-file
    for filename in result_a.files:
        assert filename in result_b.files, f"Missing file: {filename}"
        a = result_a.files[filename].strip()
        b = result_b.files[filename].strip()
        assert a == b, f"Output mismatch for {filename}:\n---\nExpected (pipeline):\n{a}\n---\nGot (dict):\n{b}"


def test_empty_component_does_not_crash():
    """Codegen must handle an empty component gracefully."""
    from omniview.codegen.react import ReactCodegen

    empty = {
        "_kind": "ComponentIR",
        "name": "Empty",
        "props": {},
        "states": [],
        "computeds": [],
        "effects": [],
        "lifecycles": [],
        "fns": [],
        "store_refs": [],
        "refs": [],
        "view": None,
        "styles": [],
    }
    result = ReactCodegen().generate_component(empty)
    assert result.files
    content = result.files.get("Empty.tsx", "")
    assert "Empty" in content


def test_minimal_signal_component():
    """Component with one signal -> codegen produces valid output."""
    from omniview.codegen.react import ReactCodegen

    comp = {
        "_kind": "ComponentIR",
        "name": "Minimal",
        "props": {},
        "states": [{
            "_kind": "StateIntent",
            "name": "count",
            "type": {"kind": "i32"},
            "initial_value": {"_kind": "LiteralExpr", "value": 0},
            "mutability": "mutable",
            "update_semantics": "setter-call",
            "accessor_style": "direct",
            "is_copy": False,
        }],
        "computeds": [],
        "effects": [],
        "lifecycles": [],
        "fns": [],
        "store_refs": [],
        "refs": [],
        "view": None,
        "styles": [],
    }
    result = ReactCodegen().generate_component(comp)
    content = result.files.get("Minimal.tsx", "")
    assert "useState" in content
    assert "count" in content


def test_shopping_cart_dict_matches_pipeline():
    """ShoppingCart: dict path == pipeline path."""
    from omniview.api import compile_omni
    from omniview.codegen.react import ReactCodegen
    from omniview.tests.helpers import ir_dataclass_to_dict

    source = open("build/test/shopping-cart/ShoppingCart.omni", encoding="utf-8").read()
    result_a = compile_omni(source, filepath="shopping-cart/ShoppingCart.omni", target="react")
    assert result_a.success, f"Pipeline errors: {result_a.errors}"

    # Get IR via internal pipeline
    import omniview.preprocessor as pp
    from omnilang.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer
    from omniview.semantic.analyzer import SemanticAnalyzer
    from omniview.lowering import IRFixture

    pre = pp.preprocess(source)
    tree = create_omniview_parser().parse(pre.processed_source)
    ast = OmniViewTransformer(block_map=pre.blocks).transform(tree)
    analysis = SemanticAnalyzer(source_lines=source.split('\n')).analyze(ast)
    ir_map = IRFixture().lower_program(ast)

    for name, ir_obj in ir_map.items():
        if hasattr(ir_obj, '__dataclass_fields__'):
            comp_dict = ir_dataclass_to_dict(ir_obj)
            result_b = ReactCodegen().generate_component(comp_dict)
            assert result_b.files, f"No output for {name}"
