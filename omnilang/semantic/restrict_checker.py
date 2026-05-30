"""Restriction checker — enforces all "forbidden" semantic rules."""

from omnilang.ast_nodes import (
    Program, FunctionDef, Block, VarDecl, Assign, IfStmt, WhileStmt, ForInStmt,
    BreakStmt, ContinueStmt, ReturnStmt, PrintStmt, ExprStmt,
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    TypeAnnotation, Stmt,
)
from omnilang.utils import ErrorCollector
from omnilang.semantic.resolver import ScopeInfo


def check(program: Program, scope_info: ScopeInfo, type_map: dict, errors: ErrorCollector):
    """Run all restriction checks."""
    checker = _RestrictChecker(scope_info, type_map, errors)
    checker.check_program(program)


class _RestrictChecker:
    def __init__(self, scope_info: ScopeInfo, type_map: dict, errors: ErrorCollector):
        self.scope_info = scope_info
        self.type_map = type_map
        self.errors = errors
        self.in_loop = 0
        self.in_function = 0
        self.in_for = 0
        self.for_iterable: Expr | None = None
        self.current_func: FunctionDef | None = None

    def check_program(self, program: Program):
        for func in program.functions:
            if func.body:
                self.check_function(func)

    def check_function(self, func: FunctionDef):
        self.current_func = func
        self.in_function += 1
        self.check_block(func.body)
        self.in_function -= 1
        self.current_func = None

    def check_block(self, block: Block):
        for stmt in block.stmts:
            self.check_stmt(stmt)
        if block.trailing_expr:
            # Check trailing expr rules
            if self.current_func:
                func_return = self.current_func.return_type
                is_void = func_return is None or func_return.name == "void"
                if is_void:
                    self.errors.add(
                        "void function cannot have a trailing expression return value",
                        block.trailing_expr.span
                    )

    def check_stmt(self, stmt: Stmt):
        if isinstance(stmt, VarDecl):
            self._check_var_decl(stmt)

        elif isinstance(stmt, Assign):
            self._check_assign(stmt)

        elif isinstance(stmt, BreakStmt):
            if self.in_loop == 0:
                self.errors.add("break outside of loop", stmt.span)

        elif isinstance(stmt, ContinueStmt):
            if self.in_loop == 0:
                self.errors.add("continue outside of loop", stmt.span)

        elif isinstance(stmt, ReturnStmt):
            if self.in_function == 0:
                self.errors.add("return outside of function", stmt.span)
            # Check main rules
            if self.current_func and self.current_func.name == "main":
                if stmt.value is not None:
                    self.errors.add("main function cannot return a value", stmt.span)

        elif isinstance(stmt, IfStmt):
            self.check_block(stmt.then_body)
            for _, blk in stmt.else_ifs:
                self.check_block(blk)
            if stmt.else_body:
                self.check_block(stmt.else_body)

        elif isinstance(stmt, WhileStmt):
            self.in_loop += 1
            self.check_block(stmt.body)
            self.in_loop -= 1

        elif isinstance(stmt, ForInStmt):
            old_in_for = self.in_for
            old_iterable = self.for_iterable
            old_var_name = getattr(self, '_for_var_name', None)
            self.in_loop += 1
            self.in_for += 1
            self.for_iterable = stmt.iterable
            self._for_var_name = stmt.var_name
            self.check_block(stmt.body)
            self.in_loop -= 1
            self.in_for = old_in_for
            self.for_iterable = old_iterable
            self._for_var_name = old_var_name

        elif isinstance(stmt, ExprStmt):
            self._check_expr_for_forbidden(stmt.expr)

        elif isinstance(stmt, PrintStmt):
            pass  # already checked by type checker

    def _check_var_decl(self, stmt: VarDecl):
        if stmt.name.startswith("_"):
            self.errors.add(f"Identifier '{stmt.name}' starts with '_' (reserved)", stmt.span)
        self._check_expr_for_forbidden(stmt.init)

    def _check_assign(self, stmt: Assign):
        if self.in_for:
            # Check if modifying the for-in iterable container
            if isinstance(stmt.target, str) and self.for_iterable:
                if isinstance(self.for_iterable, VarRef) and self.for_iterable.name == stmt.target:
                    self.errors.add(
                        "Cannot modify the iterated container inside a for-in loop",
                        stmt.span
                    )
            # Check if modifying the loop variable itself
            if stmt.index is None:
                # Check if the target is a for-in loop variable
                if hasattr(self, '_for_var_name') and stmt.target == self._for_var_name:
                    self.errors.add(
                        "Cannot modify for-in loop variable",
                        stmt.span
                    )

        # Check for float division in compound assignment (f64)
        if stmt.op in ("/=", "%="):
            expr_ty = self.type_map.get(id(stmt.value))
            if expr_ty and expr_ty.name == "f64":
                self.errors.add("Float division/modulo is forbidden in compound assignment", stmt.span)

    def _check_expr_for_forbidden(self, expr):
        """Recursively check expression for forbidden operations."""
        if expr is None:
            return

        if isinstance(expr, BinaryOp):
            if expr.op in ("/", "%"):
                left_ty = self.type_map.get(id(expr.left))
                if left_ty and left_ty.name == "f64":
                    self.errors.add("Float division is forbidden", expr.span)
                right_ty = self.type_map.get(id(expr.right))
                if right_ty and right_ty.name == "f64":
                    self.errors.add("Float modulo is forbidden", expr.span)
            self._check_expr_for_forbidden(expr.left)
            self._check_expr_for_forbidden(expr.right)

        elif isinstance(expr, UnaryOp):
            self._check_expr_for_forbidden(expr.operand)

        elif isinstance(expr, MethodCall):
            self._check_expr_for_forbidden(expr.object)
            for arg in expr.args:
                self._check_expr_for_forbidden(arg)
            # Check if this is a mutation on the for-in iterable
            if self.in_for and isinstance(expr.object, VarRef) and self.for_iterable:
                if isinstance(self.for_iterable, VarRef) and self.for_iterable.name == expr.object.name:
                    mutating = {"push", "pop", "insert"}
                    if expr.method in mutating:
                        self.errors.add(
                            f"Cannot call '{expr.method}' on iterated container inside for-in loop",
                            expr.span
                        )

        elif isinstance(expr, FuncCall):
            for arg in expr.args:
                self._check_expr_for_forbidden(arg)

        elif isinstance(expr, IndexGet):
            self._check_expr_for_forbidden(expr.object)
            self._check_expr_for_forbidden(expr.index)

        elif isinstance(expr, ArrayLiteral):
            for elem in expr.elements:
                self._check_expr_for_forbidden(elem)

        elif isinstance(expr, MapLiteral):
            for key, val in expr.entries:
                self._check_expr_for_forbidden(key)
                self._check_expr_for_forbidden(val)

        elif isinstance(expr, IfExpr):
            self._check_expr_for_forbidden(expr.condition)
            self._check_expr_for_forbidden(expr.then_expr)
            self._check_expr_for_forbidden(expr.else_expr)
