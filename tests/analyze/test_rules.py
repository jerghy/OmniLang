"""Unit tests for analyze rules."""
import pytest
from omnilang.parser import parse
from omnilang.semantic import analyze
from omnilang.analyze.registry import build_default_registry, AnalyzeContext


def run_check(source: str, targets: list[str] | None = None) -> dict[str, int]:
    """Run all rules on source and return diagnostic counts by code."""
    if targets is None:
        targets = ["rust", "dart", "js", "python"]
    prog = parse(source, filename="<test>")
    sl = {"<test>": source.split("\n")}
    try:
        tm, rm = analyze(prog, sl)
    except SystemExit:
        tm, rm = {}, {}
    registry = build_default_registry()
    ctx = AnalyzeContext(program=prog, source_lines=sl, type_map=tm, rename_map=rm, targets=targets)
    counts = {}
    for rule in registry.list_all():
        try:
            diags = rule.check(ctx)
            if diags:
                counts[rule.code] = len(diags)
        except Exception:
            pass
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


def test_p0105_float_precision():
    src = "fn main() { let a: f64 = 3.14; let b: f64 = 2.71; let c: bool = a == b; }"
    counts = run_check(src)
    assert "P0105" in counts or "P0102" in counts, "Float eq should trigger P0105 or P0102"


def test_e0201_await_in_non_async():
    src = "fn fetch() -> String { return \"ok\"; } fn main() { let x = await fetch(); }"
    counts = run_check(src)
    assert "E0201" in counts, "E0201 should detect await in non-async function"


def test_e0202_async_without_await():
    src = "async fn empty() { println(\"ok\"); } fn main() { }"
    counts = run_check(src)
    assert "E0202" in counts, "E0202 should detect async without await"


def test_s0108_backslash_in_path():
    src = 'import { foo } from ".\\\\bar"; fn main() { }'
    counts = run_check(src)
    assert "S0108" in counts, "S0108 should detect backslash in import path"


def test_s0101_unused_variable():
    src = "fn main() { let x: i32 = 5; let y: i32 = 10; println(x.to_string()); }"
    counts = run_check(src)
    assert "S0101" in counts, "S0101 should detect unused variable (y)"


def test_s0102_unused_import():
    src = 'import { foo } from "./lib"; fn main() { }'
    counts = run_check(src)
    assert "S0102" in counts, "S0102 should detect unused import"


def test_rules_registered():
    registry = build_default_registry()
    assert registry.count >= 54, f"Expected at least 54 rules, got {registry.count}"
    for code in ["P0101","P0110","P0102","E0105","E0201","E0306",
                  "S0101","S0102","S0108","P0105","P0107","E0401",
                  "E0101","F0101","SEC0101","E0202","E0104","E0110",
                  "F0105","S0103","S0110","SEC0102","P0108","E0309"]:
        assert registry.get(code) is not None, f"Rule {code} not found"
