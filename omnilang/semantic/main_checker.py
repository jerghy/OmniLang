"""Main function checker — validates the program entry point."""

from omnilang.ast_nodes import Program, ReturnStmt, Block
from omnilang.utils import ErrorCollector


def check(program: Program, errors: ErrorCollector):
    """Verify that the program has a valid main function."""
    main_func = None
    for func in program.functions:
        if func.name == "main":
            main_func = func
            break

    # 1. Must exist
    if main_func is None:
        errors.add("Program must contain a fn main() entry point")
        return

    # 2. Must have no parameters
    if main_func.params:
        errors.add("main function must have no parameters", main_func.span)

    # 3. Must not declare a return type (or only void)
    if main_func.return_type is not None and main_func.return_type.name != "void":
        errors.add("main function cannot declare a return type", main_func.return_type.span)

    # 4. Must not have a trailing expression
    if main_func.body and main_func.body.trailing_expr:
        errors.add("main function cannot have a return value (trailing expression)",
                   main_func.body.trailing_expr.span)

    # 5. Must not have return expr; (only bare return; allowed)
    if main_func.body:
        _check_no_return_value(main_func.body, errors)


def _check_no_return_value(block: Block, errors: ErrorCollector):
    for stmt in block.stmts:
        if isinstance(stmt, ReturnStmt):
            if stmt.value is not None:
                errors.add("main function cannot return a value", stmt.span)
        # Check nested blocks
        if hasattr(stmt, 'then_body'):
            _check_no_return_value(stmt.then_body, errors)
        if hasattr(stmt, 'else_ifs'):
            for _, blk in stmt.else_ifs:
                _check_no_return_value(blk, errors)
        if hasattr(stmt, 'else_body') and stmt.else_body:
            _check_no_return_value(stmt.else_body, errors)
        if hasattr(stmt, 'body') and isinstance(stmt.body, Block):
            _check_no_return_value(stmt.body, errors)
