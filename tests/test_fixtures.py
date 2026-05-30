"""Auto-discover and test all .omni fixture files.

Each fixture is parsed and checked for syntax validity.
Analyze fixtures also run through the rule pipeline.
"""

import os
import pytest
from pathlib import Path

FIXTURES_DIR = Path(__file__).parent / "fixtures"


# Error fixtures are intentionally invalid — skip parse test for these
SKIP_PARSE_DIRS = {"errors"}

def discover_fixtures(include_errors: bool = False):
    """Find all .omni fixture files and categorize them."""
    fixtures = []
    for root, dirs, files in os.walk(FIXTURES_DIR):
        for f in sorted(files):
            if f.endswith(".omni"):
                full = os.path.join(root, f)
                rel = os.path.relpath(full, FIXTURES_DIR)
                cat = rel.split(os.sep)[0] if os.sep in rel else "root"
                if not include_errors and cat in SKIP_PARSE_DIRS:
                    continue
                fixtures.append(pytest.param(full, rel, cat, id=rel.replace(os.sep, "_").replace(".omni", "")))
    return fixtures


FIXTURE_PARAMS = discover_fixtures()


@pytest.mark.parametrize("path,rel_path,category", FIXTURE_PARAMS)
def test_fixture_parses(path, rel_path, category):
    """Every .omni fixture must parse successfully."""
    from omnilang.parser import parse
    with open(path, "r", encoding="utf-8") as f:
        source = f.read()
    try:
        prog = parse(source, filename=path)
        assert prog is not None, f"Parse returned None for {rel_path}"
    except Exception as e:
        pytest.fail(f"Parse failed for {rel_path}: {e}")


# Only run analyze check on fixtures in analyze/ and portability/ dirs
ANALYZE_DIRS = {"analyze", "portability", "interface"}

ANALYZE_PARAMS = [p for p in FIXTURE_PARAMS if p.values[2] in ANALYZE_DIRS]


@pytest.mark.parametrize("path,rel_path,category", ANALYZE_PARAMS)
def test_fixture_analyzes(path, rel_path, category):
    """Analyze fixtures should run through the rule pipeline without crashes."""
    from omnilang.parser import parse
    from omnilang.semantic import analyze
    from omnilang.analyze.registry import build_default_registry, AnalyzeContext

    with open(path, "r", encoding="utf-8") as f:
        source = f.read()

    prog = parse(source, filename=path)
    sl = {path: source.split("\n")}

    try:
        tm, rm = analyze(prog, sl)
    except SystemExit:
        tm, rm = {}, {}

    registry = build_default_registry()
    ctx = AnalyzeContext(program=prog, source_lines=sl, type_map=tm, rename_map=rm)

    for rule in registry.list_all():
        try:
            diagnostics = rule.check(ctx)
            # Just verify no exceptions — diagnostics may or may not exist
        except Exception as e:
            pytest.fail(f"Rule {rule.code} crashed on {rel_path}: {e}")


def test_fixture_count():
    """Ensure we have a reasonable number of test fixtures."""
    count = len(FIXTURE_PARAMS)
    assert count >= 80, f"Expected at least 80 fixtures, got {count}"
    print(f"\nTotal fixtures discovered: {count}")
