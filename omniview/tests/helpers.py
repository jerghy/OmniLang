"""Shared test utilities for IR tests."""
from __future__ import annotations

import sys
from pathlib import Path


def ir_dataclass_to_dict(ir_obj) -> dict | list | str | int | float | bool | None:
    """Convert a Python IR dataclass tree to a JSON-serializable dict with _kind markers."""
    if ir_obj is None:
        return None
    if isinstance(ir_obj, (str, int, float, bool)):
        return ir_obj
    if isinstance(ir_obj, list):
        return [ir_dataclass_to_dict(v) for v in ir_obj]
    if isinstance(ir_obj, dict):
        return {k: ir_dataclass_to_dict(v) for k, v in ir_obj.items()}
    if hasattr(ir_obj, '__dataclass_fields__'):
        result = {"_kind": type(ir_obj).__name__}
        for field in ir_obj.__dataclass_fields__:
            val = getattr(ir_obj, field)
            result[field] = ir_dataclass_to_dict(val)
        return result
    # Handle enum values
    if hasattr(ir_obj, 'value'):
        return ir_obj.value
    return str(ir_obj)


def make_counter_ir_dict():
    """Run Python pipeline on Counter.omni and return IR as dict."""
    import omniview.preprocessor as pp
    from omnilang.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer
    from omniview.semantic.analyzer import SemanticAnalyzer
    from omniview.lowering import IRFixture

    counter_path = Path("build/test/counter/Counter.omni")
    if not counter_path.exists():
        # Try from repo root relative paths
        counter_path = Path(__file__).parent.parent.parent / "build" / "test" / "counter" / "Counter.omni"

    source = counter_path.read_text(encoding="utf-8")
    pre = pp.preprocess(source)
    tree = create_omniview_parser().parse(pre.processed_source)
    ast = OmniViewTransformer(block_map=pre.blocks).transform(tree)
    analysis = SemanticAnalyzer(source_lines=source.split('\n')).analyze(ast)
    ir_map = IRFixture().lower_program(ast)

    return {
        "version": "1.0",
        "components": {
            "Counter": ir_dataclass_to_dict(ir_map["Counter"])
        },
        "stores": {},
        "source_map": {},
    }
