"""F0105: Repeated string concatenation.

Detects repeated string concatenation within loops (`s += fragment` or
`s = s + fragment`). In many targets (Rust, JS, Dart), repeated += on
strings creates a new allocation per iteration. A StringBuilder or
collecting into a Vec/Array then joining is preferred.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    Assign, BinaryOp, VarRef, ForInStmt, WhileStmt,
    Block,
)


class F0105Rule(AnalyzeRule):
    code = "F0105"
    name = "string_concat"
    severity = Severity.PERF
    category = "performance"
    message = "Repeated string concatenation in loop `{op}` creates O(n^2) allocations"
    help = "Use StringBuilder or collect into an array then `.join()` to avoid repeated allocations."

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
            # Check if this stmt is inside a loop (passed via _check_loop_body)
            # Top-level: recurse through control flow and detect loops
            if isinstance(stmt, (ForInStmt, WhileStmt)):
                body = stmt.body
                self._check_loop_body(body)

            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child and not isinstance(stmt, (ForInStmt, WhileStmt)):
                    self._walk_block(child)

        if block.trailing_expr:
            self._walk_expr(block.trailing_expr)

    def _check_loop_body(self, body: Block):
        """Check a loop body for string concatenation patterns."""
        for stmt in getattr(body, "stmts", []):
            if isinstance(stmt, Assign):
                self._check_string_concat_in_assign(stmt)

            # Recurse into nested loops / control flow inside the loop
            if isinstance(stmt, (ForInStmt, WhileStmt)):
                self._check_loop_body(stmt.body)

            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child and not isinstance(stmt, (ForInStmt, WhileStmt)):
                    self._check_loop_body(child)

    def _check_string_concat_in_assign(self, stmt: Assign):
        """Check a single assignment for string concatenation."""
        if stmt.op in ("+=", "="):
            # s += expr  (op = "+=")
            if stmt.op == "+=":
                loc = self._span_loc(stmt.span)
                self._diags.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(op=f"{stmt.target} += ..."),
                    location=loc, help=self.help,
                ))
            # s = s + expr  (op = "=", value is BinaryOp with "+")
            elif stmt.op == "=" and stmt.value:
                if isinstance(stmt.value, BinaryOp) and stmt.value.op == "+":
                    left = stmt.value.left
                    if isinstance(left, VarRef) and left.name == stmt.target:
                        loc = self._span_loc(stmt.span)
                        self._diags.append(Diagnostic(
                            code=self.code, severity=self.severity,
                            category=self.category,
                            message=self.message.format(
                                op=f"{stmt.target} = {stmt.target} + ..."
                            ),
                            location=loc, help=self.help,
                        ))

    def _walk_expr(self, expr):
        # Recurse into expression trees outside loop context
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)
