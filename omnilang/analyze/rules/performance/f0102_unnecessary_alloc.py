"""F0102: Unnecessary allocation.

Detects patterns where a temporary collection is allocated but only
used for iteration, where a range or iterator adapter would suffice.
Examples: constructing a list literal for a single for-in pass,
or calling .to_array()/.collect() then iterating immediately.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import ArrayLiteral, MapLiteral, ForInStmt, MethodCall


class F0102Rule(AnalyzeRule):
    code = "F0102"
    name = "unnecessary_alloc"
    severity = Severity.PERF
    category = "performance"
    message = "Temporary collection allocated but only used for iteration — use a range or iterator adapter instead"
    help = "Replace `[a, b, c].iter()` with an iterator adapter to avoid heap allocation."

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
            # Check for for-in over an array/map literal
            if isinstance(stmt, ForInStmt) and isinstance(stmt.iterable, ArrayLiteral):
                elements = stmt.iterable.elements
                if elements:
                    loc = self._span_loc(stmt.span)
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message="Iterating over an array literal creates a temporary allocation",
                        location=loc,
                        help="Use a range expression `0..n` or an iterator adapter instead of allocating an array.",
                    ))

            if isinstance(stmt, ForInStmt) and isinstance(stmt.iterable, MapLiteral):
                if stmt.iterable.entries:
                    loc = self._span_loc(stmt.span)
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message="Iterating over a map literal creates a temporary allocation",
                        location=loc,
                        help="Use an iterator adapter instead of allocating a map literal for iteration.",
                    ))

            # Check for .to_array()/.collect() followed by iteration in same expression
            if isinstance(stmt, ForInStmt) and isinstance(stmt.iterable, MethodCall):
                if stmt.iterable.method in ("to_array", "to_vec", "collect", "to_list"):
                    loc = self._span_loc(stmt.span)
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=f"`.{stmt.iterable.method}()` creates an unnecessary allocation before iteration",
                        location=loc,
                        help="Iterate directly over the original collection or iterator without calling `.to_array()` first.",
                    ))

            # Walk child statements for nested loops
            for attr in ["init", "value", "condition", "iterable", "arg", "expr"]:
                val = getattr(stmt, attr, None)
                if val:
                    self._walk_expr(val)
            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child:
                    self._walk_block(child)

        if block.trailing_expr:
            self._walk_expr(block.trailing_expr)

    def _walk_expr(self, expr):
        if isinstance(expr, MethodCall) and expr.method in ("to_array", "to_vec", "collect", "to_list"):
            loc = self._span_loc(getattr(expr, "span", None))
            self._diags.append(Diagnostic(
                code=self.code, severity=self.severity,
                category=self.category,
                message=f"Unnecessary `.{expr.method}()` call — prefer lazy iteration",
                location=loc,
                help="Remove `.to_array()`/`.collect()` and iterate over the original collection directly.",
            ))

        # Recurse into child expressions
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)
