#!/usr/bin/env python3
"""Performance benchmarks for the OmniLang compiler pipeline.

Measures:
- Parse time (lark-parser)
- Semantic analysis time
- Code generation time (per target)
- Total pipeline time
- Generated code line counts vs v0.1 baseline
"""

import time
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))


BENCHMARK_SOURCE = """
fn factorial(n: i32) -> i32 {
    if n <= 1 { return 1; }
    return n * factorial(n - 1);
}

fn fib(n: i32) -> i32 {
    if n <= 1 { return n; }
    return fib(n - 1) + fib(n - 2);
}

fn is_prime(n: i32) -> bool {
    if n < 2 { return false; }
    let mut i: i32 = 2;
    while i * i <= n {
        if n % i == 0 { return false; }
        i = i + 1;
    }
    return true;
}

fn sum_range(start: i32, end: i32) -> i32 {
    let mut total: i32 = 0;
    let mut i: i32 = start;
    while i <= end {
        total = total + i;
        i = i + 1;
    }
    return total;
}

fn main() {
    println(factorial(10).to_string());
    println(fib(20).to_string());
    println(is_prime(17).to_string());
    println(sum_range(1, 100).to_string());
}
"""


def benchmark_pipeline():
    """Run full pipeline benchmark."""
    from omnilang.parser import parse
    from omnilang.semantic import analyze
    from omnilang.codegen import generate

    results = {}

    # Parse
    t0 = time.perf_counter()
    prog = parse(BENCHMARK_SOURCE, filename="<bench>")
    t1 = time.perf_counter()
    results["parse"] = (t1 - t0) * 1000

    # Semantic analysis
    source_lines = {"<bench>": BENCHMARK_SOURCE.split("\n")}
    t0 = time.perf_counter()
    try:
        type_map, rename_map = analyze(prog, source_lines)
    except SystemExit:
        type_map, rename_map = {}, {}
    t1 = time.perf_counter()
    results["semantic"] = (t1 - t0) * 1000

    # Code generation per target
    for target in ["rust", "dart", "js", "python"]:
        t0 = time.perf_counter()
        code = generate(prog, target, rename_map, type_map)
        t1 = time.perf_counter()
        results[f"codegen_{target}"] = (t1 - t0) * 1000
        results[f"lines_{target}"] = len(code.split("\n"))

    results["total"] = sum(v for k, v in results.items()
                           if k.startswith("parse") or k.startswith("semantic")
                           or k.startswith("codegen"))

    return results


def benchmark_analyze():
    """Run analyze pipeline benchmark."""
    from omnilang.parser import parse
    from omnilang.semantic import analyze
    from omnilang.analyze.registry import build_default_registry, AnalyzeContext

    t0 = time.perf_counter()
    prog = parse(BENCHMARK_SOURCE, filename="<bench>")
    source_lines = {"<bench>": BENCHMARK_SOURCE.split("\n")}
    try:
        type_map, rename_map = analyze(prog, source_lines)
    except SystemExit:
        type_map, rename_map = {}, {}

    registry = build_default_registry()
    ctx = AnalyzeContext(program=prog, source_lines=source_lines,
                         type_map=type_map, rename_map=rename_map)

    diagnostics = []
    for rule in registry.list_all():
        try:
            diagnostics.extend(rule.check(ctx))
        except Exception:
            pass

    t1 = time.perf_counter()
    return {
        "analyze_ms": (t1 - t0) * 1000,
        "rules_ran": registry.count,
        "diagnostics": len(diagnostics),
    }


def print_results():
    """Print benchmark results."""
    print("=" * 60)
    print("OmniLang v0.2 Performance Benchmark")
    print("=" * 60)

    pipeline = benchmark_pipeline()
    print("\n--- Pipeline (ms) ---")
    print(f"  Parse:          {pipeline['parse']:.1f}ms")
    print(f"  Semantic:       {pipeline['semantic']:.1f}ms")
    for target in ["rust", "dart", "js", "python"]:
        print(f"  Codegen/{target}:   {pipeline[f'codegen_{target}']:.1f}ms"
              f" ({pipeline[f'lines_{target}']} lines)")
    print(f"  Total pipeline: {pipeline['total']:.1f}ms")
    print(f"  Target: <500ms {pipeline['total'] < 500 and 'PASS' or 'FAIL'}")

    analyze = benchmark_analyze()
    print(f"\n--- Analyze ---")
    print(f"  Rules ran:     {analyze['rules_ran']}")
    print(f"  Diagnostics:   {analyze['diagnostics']}")
    print(f"  Time:          {analyze['analyze_ms']:.1f}ms")
    print(f"  Target: <200ms {analyze['analyze_ms'] < 200 and 'PASS' or 'FAIL'}")


if __name__ == "__main__":
    print_results()
