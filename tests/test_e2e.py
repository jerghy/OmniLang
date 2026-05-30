"""End-to-end tests: compile OmniLang to all targets, run, compare outputs."""

import os
import pytest
from tests.conftest import FIXTURES_DIR

# All positive test cases: (filename, expected_output)
TEST_CASES = [
    ("01_hello_world.omni", "Hello, World!\nno newline"),
    ("02_variables.omni", "10\n3.14\n4.140000000000001\nhello\nhello\n"),
    ("03_arithmetic.omni", "13\n7\n30\n3\n1\n"),
    ("04_strings.omni", "Hello, World!\nhellohello\n"),
    ("05_arrays.omni", "3\nfalse\n1\n5\n5\n3\n5\n"),
    ("06_maps.omni", "2\nfalse\n1\ntrue\n2\ntwo\nthree\n"),
    ("07_control_flow.omni", "big\n012done\n"),
    ("08_functions.omni", "Hello, Alice\n6\n"),
    ("09_shadowing.omni", "1\n2\n1\n"),  # Python gives 1\n2\n2\n (no block scope — known limitation)
    ("10_ownership.omni", "hello\nhello\n2\n3\n"),
    ("11_for_loop.omni", "10\n20\n30\n0\n60\n1\n2\n3\n"),
    ("12_while_loop.omni", "0\n1\n3\n16\n"),
    ("13_if_expression.omni", "1\nyes\n"),
    ("14_type_system.omni", "42\n3.14\ntrue\ntest\n2\n1\n"),
    ("15_comprehensive.omni", "55\n1\n2\n3\n"),
    ("16_tail_return.omni", "7\nHello, World\nok\n"),
    ("17_ownership_complex.omni", "a\nb\n"),
    ("18_rust_temp_var.omni", "Hi, Hello, Bob\n"),
    ("19_map_key_variants.omni", "1\n2\n3\none\nfalse\n"),
    ("20_no_idiv.omni", "300\n100\n1000\nhelloworld\n"),
]

TARGETS = ["python", "js", "dart", "rust"]


@pytest.mark.parametrize("filename,expected", TEST_CASES)
@pytest.mark.parametrize("target", TARGETS)
def test_e2e(compile_and_run, filename, expected, target):
    path = os.path.join(FIXTURES_DIR, filename)
    if not os.path.exists(path):
        pytest.skip(f"Fixture not found: {filename}")

    output = compile_and_run(path, target)
    assert output == expected, f"[{target}] Expected {expected!r}, got {output!r}"


def test_cross_language_consistency(compile_and_run):
    """Verify all four targets produce identical output for all test cases."""
    for filename, _ in TEST_CASES:
        path = os.path.join(FIXTURES_DIR, filename)
        if not os.path.exists(path):
            continue
        outputs = {}
        for target in ["python", "js", "dart", "rust"]:
            outputs[target] = compile_and_run(path, target)

        baseline = outputs["python"]
        for target in ["js", "dart", "rust"]:
            assert outputs[target] == baseline, (
                f"Output mismatch for {filename}:\n"
                f"  PY: {baseline!r}\n"
                f"  {target.upper()}: {outputs[target]!r}"
            )
