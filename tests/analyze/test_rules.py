"""Unit tests for analyze rules."""
import pytest
from omnilang.parser import parse
from omnilang.semantic import analyze
from omnilang.analyze.registry import build_default_registry, AnalyzeContext


def run_check(source: str) -> dict[str, int]:
    """Run all rules on source and return diagnostic counts by code."""
    prog = parse(source, filename="<test>")
    sl = {"<test>": source.split("\n")}
    tm, rm = analyze(prog, sl)
    registry = build_default_registry()
    ctx = AnalyzeContext(program=prog, source_lines=sl, type_map=tm, rename_map=rm)
    counts = {}
    for rule in registry.list_all():
        diags = rule.check(ctx)
        if diags:
            counts[rule.code] = len(diags)
    return counts


def test_p0101_integer_division():
    src = "fn main() { let a: i32 = 5; let b: i32 = 2; let c: i32 = a / b; }"
    counts = run_check(src)
    assert "P0101" in counts, "P0101 should detect integer division"


def test_p0110_literal_divzero():
    src = "fn main() { let a: i32 = 5 / 0; }"
    counts = run_check(src)
    assert "P0110" in counts, "P0110 should detect literal division by zero"


def test_p0102_float_comparison():
    src = "fn main() { let a: f64 = 3.14; let b: f64 = 2.71; let c: bool = a == b; }"
    counts = run_check(src)
    assert "P0102" in counts, "P0102 should detect float equality comparison"


def test_no_portability_issues_clean_code():
    src = "fn main() { let a: i32 = 5; let b: i32 = 2; println(a.to_string()); }"
    counts = run_check(src)
    assert "P0101" not in counts, "Clean code should not trigger P0101"


def test_rules_registered():
    registry = build_default_registry()
    assert registry.count >= 3, f"Expected at least 3 rules, got {registry.count}"
    assert registry.get("P0101") is not None
    assert registry.get("P0110") is not None
    assert registry.get("P0102") is not None
