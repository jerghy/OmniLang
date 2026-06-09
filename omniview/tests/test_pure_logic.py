"""Pure logic mode tests — OmniLang constructs without import omniview.

Validates that .omni files without UI components can be parsed
as pure OmniLang and routed to non-UI codegen.
"""

import pytest
from omnilang.lexer import create_parser
from omnilang.parser import OmniTransformer


def parse_logic(source: str):
    """Parse pure OmniLang source through unified grammar."""
    parser = create_parser()
    tree = parser.parse(source)
    return OmniTransformer().transform(tree)


class TestPureLogicFunctions:
    """Test basic function parsing in pure logic mode."""

    def test_simple_function(self):
        prog = parse_logic("fn add(a: i32, b: i32) -> i32 { return a + b; }")
        assert not prog.has_omniview
        assert len(prog.functions) == 1
        assert prog.functions[0].name == "add"

    def test_function_without_return_type(self):
        prog = parse_logic("fn greet(name: String) { print(name); }")
        assert len(prog.functions) == 1
        assert prog.functions[0].name == "greet"

    def test_void_function(self):
        prog = parse_logic("fn main() { }")
        assert len(prog.functions) == 1

    def test_multiple_functions(self):
        prog = parse_logic("""
            fn foo() -> i32 { return 1; }
            fn bar() -> i32 { return 2; }
        """)
        assert len(prog.functions) == 2


class TestPureLogicVariables:
    """Test variable declarations and assignments."""

    def test_let_declaration(self):
        prog = parse_logic("fn test() { let x: i32 = 42; return x; }")
        body = prog.functions[0].body
        assert body is not None
        assert len(body.stmts) >= 1

    def test_mutable_variable(self):
        prog = parse_logic("fn test() { let mut count: i32 = 0; count = 1; return count; }")
        assert prog.functions[0].body is not None

    def test_assign_with_op(self):
        prog = parse_logic("fn test() { let mut x: i32 = 1; x += 2; return x; }")
        assert prog.functions[0].body is not None


class TestPureLogicControlFlow:
    """Test if/while/for constructs."""

    def test_if_else(self):
        prog = parse_logic("""
            fn max(a: i32, b: i32) -> i32 {
                if a > b { return a; } else { return b; }
            }
        """)
        assert len(prog.functions) == 1

    def test_while_loop(self):
        prog = parse_logic("""
            fn countdown(n: i32) -> i32 {
                let mut x: i32 = n;
                while x > 0 { x = x - 1; }
                return x;
            }
        """)
        assert len(prog.functions) == 1

    def test_for_in(self):
        prog = parse_logic("""
            fn sum(arr: Array<i32>) -> i32 {
                let mut total: i32 = 0;
                for item in arr { total = total + item; }
                return total;
            }
        """)
        assert len(prog.functions) == 1


class TestPureLogicDataStructures:
    """Test array and map literals."""

    def test_array_literal(self):
        prog = parse_logic("fn test() { let arr = [1, 2, 3]; return arr; }")
        assert prog.functions[0].body is not None

    def test_map_literal(self):
        prog = parse_logic('fn test() { let m = {"a": 1, "b": 2}; return m; }')
        assert prog.functions[0].body is not None

    def test_array_index(self):
        prog = parse_logic("fn test(arr: Array<i32>) -> i32 { return arr[0]; }")
        assert prog.functions[0].body is not None


class TestPureLogicExpressions:
    """Test complex expressions."""

    def test_binary_ops(self):
        prog = parse_logic("fn calc(a: i32, b: i32) -> i32 { return a + b * 2 - 1; }")
        assert len(prog.functions) == 1

    def test_logical_ops(self):
        prog = parse_logic("fn check(a: i32, b: i32) -> bool { return a > 0 && b < 10; }")
        assert len(prog.functions) == 1

    def test_comparison_chain(self):
        prog = parse_logic("fn in_range(x: i32) -> bool { return x >= 0 && x <= 100; }")
        assert len(prog.functions) == 1

    def test_ternary_if_expr(self):
        prog = parse_logic("fn abs(x: i32) -> i32 { return if x >= 0 { x } else { -x }; }")
        assert len(prog.functions) == 1


class TestPureLogicNoOmniView:
    """Verify has_omniview is False without import."""

    def test_no_import_no_omniview(self):
        prog = parse_logic("fn main() { }")
        assert prog.has_omniview is False
        assert prog.lib_imports == []

    def test_with_import_has_omniview(self):
        prog = parse_logic("import omniview; fn main() { }")
        assert prog.has_omniview is True

    def test_other_import_no_omniview(self):
        prog = parse_logic("import something; fn main() { }")
        assert prog.has_omniview is False
        assert prog.lib_imports == ["something"]
