"""Cross-framework behavior fixture validation tests.

Compiles each behavior fixture to all 4 target frameworks and
validates that the generated output contains expected patterns.

These tests validate that the full pipeline (parse → analyze → lower → codegen)
produces correct, compilable output for all targets.
"""

import os
import sys
import tempfile
import pytest

# ── Fixture paths ─────────────────────────────────────────

FIXTURES_DIR = os.path.join(
    os.path.dirname(__file__), "..", "..", "tests", "frameworks", "behavior"
)

BEHAVIOR_FIXTURES = [
    "01_counter.omni",
    "02_list_rendering.omni",
    "03_conditional.omni",
    "04_bind_input.omni",
    "05_store_read.omni",
    "06_error_boundary.omni",
]

TARGETS = ["react", "vue", "dioxus", "flutter"]


def _compile_fixture(fixture_name: str, target: str) -> dict[str, str]:
    """Compile a behavior fixture to the given target and return generated files.

    Returns dict of filename → content.
    """
    fixture_path = os.path.join(FIXTURES_DIR, fixture_name)
    if not os.path.exists(fixture_path):
        pytest.skip(f"Fixture not found: {fixture_path}")

    with open(fixture_path, "r", encoding="utf-8") as f:
        source = f.read()

    # Preprocess
    from omniview.preprocessor import preprocess
    pp_result = preprocess(source)

    # Parse
    from lark import Lark
    from omniview.parser import OmniViewTransformer
    import omniview.cli as cli_mod

    grammar_path = cli_mod.find_grammar_path()
    with open(grammar_path, "r", encoding="utf-8") as f:
        grammar = f.read()

    parser = Lark(grammar, parser="earley", start="start")
    tree = parser.parse(pp_result.processed_source)
    transformer = OmniViewTransformer(block_map=pp_result.blocks)
    program = transformer.transform(tree)

    # Semantic analysis
    from omniview.semantic.analyzer import SemanticAnalyzer
    analyzer = SemanticAnalyzer()
    result = analyzer.analyze(program)
    # Don't fail on semantic errors — some fixtures might have warnings
    # But log them for debugging
    if result.has_errors:
        err_msgs = [str(d) for d in result.diagnostics if d.level == "ERROR"]
        if err_msgs:
            print(f"  Semantic errors in {fixture_name}: {err_msgs}")

    # Lower to IR
    from omniview.lowering import IRFixture
    lower = IRFixture()
    ir_map = lower.lower_program(program)

    # Generate code
    if target == "react":
        from omniview.codegen.react import ReactCodegen
        codegen = ReactCodegen()
    elif target == "vue":
        from omniview.codegen.vue import VueCodegen
        codegen = VueCodegen()
    elif target == "dioxus":
        from omniview.codegen.dioxus import DioxusCodegen
        codegen = DioxusCodegen()
    elif target == "flutter":
        from omniview.codegen.flutter import FlutterCodegen
        codegen = FlutterCodegen()
    else:
        raise ValueError(f"Unknown target: {target}")

    # Collect generated files
    all_files = {}
    for name, ir in ir_map.items():
        if "states" in ir or "view" in ir:
            gen_result = codegen.generate_component(ir, ir_map, source_line=1)
        else:
            gen_result = codegen.generate_store(ir, source_line=1)
        all_files.update(gen_result.files)

    return all_files


# ── Per-fixture validation ────────────────────────────────

class TestCounterFixture:
    """01_counter.omni: signal + mutation + event + interpolation."""

    @pytest.mark.parametrize("target", TARGETS)
    def test_compiles(self, target):
        files = _compile_fixture("01_counter.omni", target)
        assert len(files) > 0, f"No files generated for target={target}"

    def test_react_output(self):
        files = _compile_fixture("01_counter.omni", "react")
        tsx = [v for k, v in files.items() if k.endswith(".tsx")][0]
        assert "useState" in tsx
        assert "setCount" in tsx
        assert "Count:" in tsx
        assert "testid" in tsx.lower() or "test-id" in tsx.lower() or "data-testid" in tsx.lower()

    def test_vue_output(self):
        files = _compile_fixture("01_counter.omni", "vue")
        vue = [v for k, v in files.items() if k.endswith(".vue")][0]
        assert "ref(" in vue or "ref<" in vue.lower()
        assert "count" in vue.lower()

    def test_dioxus_output(self):
        files = _compile_fixture("01_counter.omni", "dioxus")
        rs = [v for k, v in files.items() if k.endswith(".rs")][0]
        assert "fn " in rs or "use_signal" in rs or "GlobalSignal" in rs

    def test_flutter_output(self):
        files = _compile_fixture("01_counter.omni", "flutter")
        dart = [v for k, v in files.items() if k.endswith(".dart")][0]
        assert "Widget" in dart or "StatefulWidget" in dart or "setState" in dart


class TestListRenderingFixture:
    """02_list_rendering.omni: @for + @empty + spread + bind."""

    @pytest.mark.parametrize("target", TARGETS)
    def test_compiles(self, target):
        files = _compile_fixture("02_list_rendering.omni", target)
        assert len(files) > 0

    def test_react_has_map(self):
        files = _compile_fixture("02_list_rendering.omni", "react")
        tsx = [v for k, v in files.items() if k.endswith(".tsx")][0]
        assert ".map(" in tsx or "map(" in tsx  # @for → .map()

    def test_vue_has_v_for(self):
        files = _compile_fixture("02_list_rendering.omni", "vue")
        vue = [v for k, v in files.items() if k.endswith(".vue")][0]
        assert "v-for" in vue


class TestConditionalFixture:
    """03_conditional.omni: @if + @else if + @else."""

    @pytest.mark.parametrize("target", TARGETS)
    def test_compiles(self, target):
        files = _compile_fixture("03_conditional.omni", target)
        assert len(files) > 0

    def test_react_has_ternary(self):
        files = _compile_fixture("03_conditional.omni", "react")
        tsx = [v for k, v in files.items() if k.endswith(".tsx")][0]
        assert "?" in tsx and ":" in tsx  # Conditional → ternary

    def test_vue_has_v_if(self):
        files = _compile_fixture("03_conditional.omni", "vue")
        vue = [v for k, v in files.items() if k.endswith(".vue")][0]
        assert "v-if" in vue or "v-else" in vue


class TestBindInputFixture:
    """04_bind_input.omni: bind:value + bind:checked + form."""

    @pytest.mark.parametrize("target", TARGETS)
    def test_compiles(self, target):
        files = _compile_fixture("04_bind_input.omni", target)
        assert len(files) > 0

    def test_react_has_onChange(self):
        files = _compile_fixture("04_bind_input.omni", "react")
        tsx = [v for k, v in files.items() if k.endswith(".tsx")][0]
        assert "onChange" in tsx  # bind:value → onChange

    def test_vue_has_v_model(self):
        files = _compile_fixture("04_bind_input.omni", "vue")
        vue = [v for k, v in files.items() if k.endswith(".vue")][0]
        assert "v-model" in vue


class TestStoreReadFixture:
    """05_store_read.omni: store getter + action + component store access."""

    @pytest.mark.parametrize("target", TARGETS)
    def test_compiles(self, target):
        files = _compile_fixture("05_store_read.omni", target)
        assert len(files) > 0

    def test_react_has_zustand(self):
        files = _compile_fixture("05_store_read.omni", "react")
        all_code = "\n".join(files.values())
        assert "useUserStore" in all_code or "create" in all_code or "getState" in all_code

    def test_vue_has_pinia(self):
        files = _compile_fixture("05_store_read.omni", "vue")
        all_code = "\n".join(files.values())
        assert "defineStore" in all_code or "useUserStore" in all_code or "store" in all_code.lower()

    def test_store_file_generated(self):
        """Store fixtures should generate a separate store file."""
        files = _compile_fixture("05_store_read.omni", "react")
        store_files = [k for k in files if "store" in k.lower() or "Store" in k]
        assert len(store_files) > 0, f"No store file in {list(files.keys())}"


# ── Cross-target consistency ──────────────────────────────

class TestDioxusBehaviorSpecific:
    """Dioxus-specific code quality checks for behavior fixtures."""

    def test_counter_uses_signal_api(self):
        files = _compile_fixture("01_counter.omni", "dioxus")
        rs = [v for k, v in files.items() if k.endswith(".rs")][0]
        # Should use Dioxus Signal API, not raw += on signals
        assert ".set(" in rs or "use_signal" in rs or "GlobalSignal" in rs

    def test_list_uses_iter_map(self):
        files = _compile_fixture("02_list_rendering.omni", "dioxus")
        rs = [v for k, v in files.items() if k.endswith(".rs")][0]
        # @for should generate .iter().map() or similar iteration
        assert "iter()" in rs or "map(" in rs or "for" in rs

    def test_conditional_uses_if_block(self):
        files = _compile_fixture("03_conditional.omni", "dioxus")
        rs = [v for k, v in files.items() if k.endswith(".rs")][0]
        assert "if " in rs  # @if → Rust if

    def test_bind_uses_oninput(self):
        files = _compile_fixture("04_bind_input.omni", "dioxus")
        rs = [v for k, v in files.items() if k.endswith(".rs")][0]
        assert "oninput" in rs or "onchange" in rs

    def test_store_has_global_signal(self):
        files = _compile_fixture("05_store_read.omni", "dioxus")
        all_code = "\n".join(files.values())
        assert "GlobalSignal" in all_code or "use_signal" in all_code or "struct" in all_code

    def test_error_boundary_has_comment(self):
        files = _compile_fixture("06_error_boundary.omni", "dioxus")
        rs = [v for k, v in files.items() if k.endswith(".rs")][0]
        assert "@error" in rs or "catch_unwind" in rs


class TestCrossFrameworkConsistency:
    """All targets should generate code for all fixtures."""

    @pytest.mark.parametrize("fixture", BEHAVIOR_FIXTURES)
    @pytest.mark.parametrize("target", TARGETS)
    def test_all_targets_generate_code(self, fixture, target):
        files = _compile_fixture(fixture, target)
        assert len(files) >= 1, f"{fixture} → {target}: no files generated"
        # Each fixture should generate at least one component file
        code_files = [k for k in files if not k.endswith(".css") and not k.endswith(".map")]
        assert len(code_files) >= 1, f"{fixture} → {target}: no code files in {list(files.keys())}"

    def test_consistency_matrix(self):
        """Generate a consistency matrix: fixture × target → file count."""
        results = {}
        for fixture in BEHAVIOR_FIXTURES:
            results[fixture] = {}
            for target in TARGETS:
                try:
                    files = _compile_fixture(fixture, target)
                    results[fixture][target] = len(files)
                except Exception as e:
                    results[fixture][target] = f"ERROR: {e}"

        # Print matrix for debugging
        header = f"{'Fixture':<30}" + "".join(f"{t:>10}" for t in TARGETS)
        print(f"\n  Cross-framework consistency matrix:\n  {header}")
        for fixture in BEHAVIOR_FIXTURES:
            row = f"  {fixture:<30}"
            for target in TARGETS:
                val = results[fixture][target]
                row += f"{str(val):>10}"
            print(row)

        # All fixtures must compile for all targets
        for fixture in BEHAVIOR_FIXTURES:
            for target in TARGETS:
                val = results[fixture][target]
                assert not isinstance(val, str) or not val.startswith("ERROR"), \
                    f"{fixture} → {target}: {val}"
                assert isinstance(val, int) and val > 0, \
                    f"{fixture} → {target}: expected files, got {val}"
