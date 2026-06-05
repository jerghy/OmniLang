#!/usr/bin/env python3
"""验证 OmniLang parse_inline_expr 能处理 OmniView TodoMVC 中的所有表达式模式。

用法: python tools/verify_parse_inline_expr.py
"""

import sys
import os
import io

# Fix Windows encoding issues
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

# Add parent dir to path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from omnilang.ast_nodes import (
    Expr, Literal, VarRef, BinaryOp, UnaryOp, FuncCall, MethodCall,
    ArrayLiteral, MapLiteral, IfExpr, SpreadExpr,
    Block, ExprStmt, Assign, ReturnStmt, VarDecl, IfStmt,
)
from omnilang.parser_helpers import parse_inline_expr


# ── Expression Test Cases from TodoMVC/Shop/Dashboard ──

EXPRESSION_TESTS = [
    # ── Literals ──
    ("0", "integer literal", lambda r: isinstance(r, Literal) and r.value == 0),
    ('""', "empty string literal", lambda r: isinstance(r, Literal) and r.value == ""),
    ('"all"', "string literal", lambda r: isinstance(r, Literal) and r.value == "all"),
    ("true", "boolean literal true", lambda r: isinstance(r, Literal) and r.value is True),
    ("false", "boolean literal false", lambda r: isinstance(r, Literal) and r.value is False),

    # ── Variable references ──
    ("filter", "bare identifier", lambda r: isinstance(r, VarRef) and r.name == "filter"),
    ("input_text", "signal identifier", lambda r: isinstance(r, VarRef) and r.name == "input_text"),
    ("sidebar_open", "boolean signal identifier", lambda r: isinstance(r, VarRef) and r.name == "sidebar_open"),

    # ── Dot path / field access ──
    ("store.display_count", "2-segment dot path",
     lambda r: isinstance(r, MethodCall) and isinstance(r.object, VarRef)
               and r.object.name == "store" and r.method == "display_count" and r.args == []),
    ("store.name", "dot path to signal",
     lambda r: isinstance(r, MethodCall) and isinstance(r.object, VarRef)
               and r.object.name == "store" and r.method == "name"),
    ("store.logged_in", "dot path to boolean signal",
     lambda r: isinstance(r, MethodCall) and isinstance(r.object, VarRef)
               and r.object.name == "store" and r.method == "logged_in"),

    # ── Binary operators ──
    ('filter == "all"', "equality comparison",
     lambda r: isinstance(r, BinaryOp) and r.op == "=="
               and isinstance(r.left, VarRef) and r.left.name == "filter"
               and isinstance(r.right, Literal) and r.right.value == "all"),
    ("task_count + 1", "addition",
     lambda r: isinstance(r, BinaryOp) and r.op == "+"
               and isinstance(r.left, VarRef) and r.left.name == "task_count"
               and isinstance(r.right, Literal) and r.right.value == 1),
    ("count * 2", "multiplication",
     lambda r: isinstance(r, BinaryOp) and r.op == "*"),

    # ── Unary operators ──
    ("!sidebar_open", "logical not",
     lambda r: isinstance(r, UnaryOp) and r.op == "!"
               and isinstance(r.operand, VarRef) and r.operand.name == "sidebar_open"),

    # ── Method calls (postfix expr: VarRef.store → MethodCall) ──
    ("store.add_item()", "method call empty args",
     lambda r: isinstance(r, MethodCall) and isinstance(r.object, VarRef)
               and r.object.name == "store" and r.method == "add_item" and r.args == []),
    ("store.logout()", "method call empty args 2",
     lambda r: isinstance(r, MethodCall) and isinstance(r.object, VarRef)
               and r.object.name == "store" and r.method == "logout" and r.args == []),
    ('store.login("Admin")', "method call with string arg",
     lambda r: isinstance(r, MethodCall) and isinstance(r.object, VarRef)
               and r.object.name == "store" and r.method == "login"
               and len(r.args) == 1 and isinstance(r.args[0], Literal)
               and r.args[0].value == "Admin"),

    # ── Array literals ──
    ("[1, 2, 3]", "array literal",
     lambda r: isinstance(r, ArrayLiteral) and len(r.elements) == 3),
    ("[...items, item]", "spread in array",
     lambda r: isinstance(r, ArrayLiteral)
               and isinstance(r.elements[0], SpreadExpr)),

    # ── OmniLang if-expr (NOT C-style ?:) ──
    # Note: C-style ternary `a ? b : c` is NOT supported in OmniLang.
    # OmniView uses `if a { b } else { c }` syntax for conditional expressions.
    # See docs/OMNILANG_OMNIVIEW_API_CONTRACT.md §已知差异.

    # ── Logical operators ──
    ('a && b', "logical and",
     lambda r: isinstance(r, BinaryOp) and r.op == "&&"),
    ('a || b', "logical or",
     lambda r: isinstance(r, BinaryOp) and r.op == "||"),
]

# ── Block-level expressions (simulated via parse_inline_block wrapper) ──

BLOCK_TESTS = [
    # Assignments
    ("input_text = \"\";", "assign string literal"),
    ('filter = "all";', "assign string literal 2"),
    ("task_count = task_count + 1;", "assign with binary op"),
    ("item_count = 0;", "assign integer literal"),
    ("sidebar_open = true;", "assign boolean literal"),
    ("name = \"Guest\"; logged_in = false;", "multi-statement block"),
    ("name = user; logged_in = true;", "multi-statement with var ref"),
]


def test_parse_inline_expr():
    """Test parse_inline_expr against all TodoMVC expression patterns."""
    passed = 0
    failed = 0
    errors = []

    print("=" * 60)
    print("Testing parse_inline_expr with TodoMVC expression set")
    print("=" * 60)

    for expr_text, description, validator in EXPRESSION_TESTS:
        try:
            result = parse_inline_expr(expr_text)
            if validator(result):
                passed += 1
                print(f"  ✅ {description}: '{expr_text}' → {type(result).__name__}")
            else:
                failed += 1
                msg = f"  ❌ {description}: '{expr_text}' → {type(result).__name__} (validator returned False)"
                print(msg)
                errors.append(msg)
        except Exception as e:
            failed += 1
            msg = f"  ❌ {description}: '{expr_text}' → EXCEPTION: {e}"
            print(msg)
            errors.append(msg)

    print(f"\n  Expression tests: {passed}/{passed + failed} passed")

    return passed, failed, errors


def test_parse_inline_block():
    """Test parse_inline_block (simulated) against block-level patterns."""
    from omnilang.lexer import create_parser
    from omnilang.parser import OmniTransformer

    parser = create_parser()
    transformer = OmniTransformer()

    passed = 0
    failed = 0
    errors = []

    print("\n" + "=" * 60)
    print("Testing parse_inline_block with block-level patterns")
    print("=" * 60)

    for block_text, description in BLOCK_TESTS:
        try:
            source = f"fn __tpl() {{ {block_text} }}"
            tree = parser.parse(source)
            prog = transformer.transform(tree)
            if prog.functions and prog.functions[0].body:
                body = prog.functions[0].body
                stmt_count = len(body.stmts) if body.stmts else 0
                passed += 1
                print(f"  ✅ {description}: '{block_text}' → Block with {stmt_count} stmt(s)")
            else:
                failed += 1
                msg = f"  ❌ {description}: '{block_text}' → no body in result"
                print(msg)
                errors.append(msg)
        except Exception as e:
            failed += 1
            msg = f"  ❌ {description}: '{block_text}' → EXCEPTION: {e}"
            print(msg)
            errors.append(msg)

    print(f"\n  Block tests: {passed}/{passed + failed} passed")

    return passed, failed, errors


def test_store_action_calls():
    """Test recognition of store.action() patterns.

    This is critical because previous implementation lost args in lowering.
    """
    from omnilang.lexer import create_parser
    from omnilang.parser import OmniTransformer

    parser = create_parser()
    transformer = OmniTransformer()

    passed = 0
    failed = 0
    errors = []

    print("\n" + "=" * 60)
    print("Testing store.action() pattern recognition")
    print("=" * 60)

    store_action_tests = [
        ("store.increment();", "store.increment()", 1, 0),
        ("store.login(\"Admin\");", "store.login(\"Admin\")", 1, 1),
        ("store.add_item();", "store.add_item()", 1, 0),
        ("store.clear_cart();", "store.clear_cart()", 1, 0),
        ("store.logout();", "store.logout()", 1, 0),
    ]

    for source_expr, desc, expected_stmts, expected_args in store_action_tests:
        try:
            source = f"fn __tpl() {{ {source_expr} }}"
            tree = parser.parse(source)
            prog = transformer.transform(tree)
            body = prog.functions[0].body
            stmt = body.stmts[0]

            if isinstance(stmt, ExprStmt):
                expr = stmt.expr
                # With optional () in method_call grammar,
                # store.action() is MethodCall directly (postfix chain)
                if isinstance(expr, MethodCall):
                    mc = expr
                    if (isinstance(mc.object, VarRef) and mc.object.name == "store"
                            and len(mc.args) == expected_args):
                        passed += 1
                        print(f"  ✅ {desc}: store.{mc.method}({len(mc.args)} args)")
                    else:
                        failed += 1
                        msg = f"  ❌ {desc}: unexpected MethodCall structure"
                        print(msg)
                        errors.append(msg)
                else:
                    failed += 1
                    msg = f"  ❌ {desc}: not MethodCall → {type(expr)}"
                    print(msg)
                    errors.append(msg)
            else:
                failed += 1
                msg = f"  ❌ {desc}: not ExprStmt → {type(stmt)}"
                print(msg)
                errors.append(msg)
        except Exception as e:
            failed += 1
            msg = f"  ❌ {desc}: EXCEPTION: {e}"
            print(msg)
            errors.append(msg)

    print(f"\n  Store action tests: {passed}/{passed + failed} passed")

    return passed, failed, errors


if __name__ == "__main__":
    total_passed = 0
    total_failed = 0
    all_errors = []

    # Run all test suites
    for test_fn in [test_parse_inline_expr, test_parse_inline_block, test_store_action_calls]:
        p, f, errs = test_fn()
        total_passed += p
        total_failed += f
        all_errors.extend(errs)

    print("\n" + "=" * 60)
    print(f"OVERALL: {total_passed}/{total_passed + total_failed} passed")
    print("=" * 60)

    if all_errors:
        print("\n❌ FAILURES:")
        for err in all_errors:
            print(f"  {err}")
        sys.exit(1)
    else:
        print("\n✅ ALL TESTS PASSED — parse_inline_expr can handle TodoMVC expression set")
        sys.exit(0)
