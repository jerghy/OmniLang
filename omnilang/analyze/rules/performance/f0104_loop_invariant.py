"""F0104: Loop invariant computation.

Suggests hoisting constant computations out of loops when an expression's
result does not change across iterations. Detects patterns like calling
.length/.size() on an unchanged collection, or recomputing constant
arithmetic inside a loop body.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    ForInStmt, WhileStmt, BinaryOp, MethodCall, VarRef,
    Literal, FuncCall, Block,
)

_COMPUTED_METHODS = {"len", "length", "size", "count", "is_empty"}


class F0104Rule(AnalyzeRule):
    code = "F0104"
    name = "loop_invariant"
    severity = Severity.PERF
    category = "performance"
    message = "Loop-invariant computation `{desc}` should be hoisted outside the loop"
    help = "Move invariant expressions before the loop to avoid recomputing the same value on every iteration."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._walk_block(func.body)

        return self._diags

    def _filename(self) -> str:
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _span_loc(self, span) -> SourceLocation:
        if span:
            return SourceLocation(
                file=self._filename(), line=span.line, column=span.col,
                span=(span.line, span.col, span.end_line, span.end_col),
            )
        return SourceLocation(file=self._filename(), line=1, column=1, span=(1, 1, 1, 1))

    def _walk_block(self, block):
        for stmt in getattr(block, "stmts", []):
            if isinstance(stmt, ForInStmt):
                self._check_loop_body(stmt.body, stmt.var_name)
            elif isinstance(stmt, WhileStmt):
                # Check if condition itself has invariant computations
                self._walk_expr(stmt.condition)
                self._check_loop_body(stmt.body, None)

            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child and not isinstance(stmt, (ForInStmt, WhileStmt)):
                    self._walk_block(child)

        if block.trailing_expr:
            self._walk_expr(block.trailing_expr)

    def _check_loop_body(self, body: Block, loop_var: str | None):
        """Check a loop body for invariant expressions."""
        for stmt in getattr(body, "stmts", []):
            for attr in ["init", "value", "condition", "iterable", "arg", "expr"]:
                val = getattr(stmt, attr, None)
                if val:
                    self._check_expr_invariant(val, loop_var, stmt)
            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child:
                    self._check_loop_body(child, loop_var)

    def _check_expr_invariant(self, expr, loop_var: str | None, parent_stmt):
        """Check if an expression is loop-invariant (doesn't depend on loop variable)."""
        # Is it a method call like .len(), .size() on a non-mutated reference?
        if isinstance(expr, MethodCall) and expr.method in _COMPUTED_METHODS:
            if isinstance(expr.object, VarRef) and expr.object.name != loop_var:
                loc = self._span_loc(getattr(expr, "span", None))
                self._diags.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(
                        desc=f"`{expr.object.name}.{expr.method}()`"
                    ),
                    location=loc, help=self.help,
                ))

        # Is it a constant binary expression (both sides are literals)?
        if isinstance(expr, BinaryOp):
            if self._is_literal(expr.left) and self._is_literal(expr.right):
                self._diags.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(
                        desc="constant arithmetic expression"
                    ),
                    location=self._span_loc(getattr(expr, "span", None)),
                    help=self.help,
                ))

        # Recurse into children
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._check_expr_invariant(val, loop_var, parent_stmt)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._check_expr_invariant(item, loop_var, parent_stmt)

    def _is_literal(self, expr) -> bool:
        if isinstance(expr, Literal):
            return True
        if isinstance(expr, VarRef):
            return False
        return False

    def _walk_expr(self, expr):
        # Recurse into child expressions for non-loop context
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)
