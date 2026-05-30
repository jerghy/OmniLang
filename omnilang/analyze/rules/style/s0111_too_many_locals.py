"""S0111: Too many local variables.

Warns when a function has more than 8 local variable declarations,
suggesting the function should be split into smaller functions.
Excessive local variables indicate a function doing too much
and reduce readability and maintainability.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, FunctionDef


# Maximum number of local variables before a warning is emitted
_MAX_LOCALS = 8


class S0111Rule(AnalyzeRule):
    code = "S0111"
    name = "too_many_locals"
    severity = Severity.STYLE
    category = "style"
    message = "Function `{name}` has {count} local variables (max suggested: {max_count})"
    help = (
        "Functions with many local variables are hard to read and maintain. "
        "Consider extracting related logic into smaller helper functions "
        "to reduce the local variable count."
    )

    max_locals: int = _MAX_LOCALS

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                count = self._count_locals_in_block(func.body)
                if count > self.max_locals:
                    diagnostics.append(
                        self._make_diagnostic(func, count)
                    )

        return diagnostics

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
        return SourceLocation(
            file=self._filename(), line=1, column=1, span=(1, 1, 1, 1),
        )

    def _make_diagnostic(self, func: FunctionDef, count: int) -> Diagnostic:
        span = func.span
        line = span.line if span else 1
        col = span.col if span else 1
        loc = SourceLocation(
            file=self._filename(),
            line=line, column=col,
            span=(
                line, col,
                span.end_line if span else 1,
                span.end_col if span else 1,
            ),
        )
        return Diagnostic(
            code=self.code,
            severity=self.severity,
            category=self.category,
            message=self.message.format(
                name=func.name, count=count, max_count=self.max_locals,
            ),
            location=loc,
            help=self.help,
        )

    def _count_locals_in_block(self, block) -> int:
        """Recursively count VarDecl statements in a block."""
        count = 0
        for stmt in getattr(block, "stmts", []):
            if isinstance(stmt, VarDecl):
                count += 1
            for child_attr in ("body", "then_body", "else_body"):
                child = getattr(stmt, child_attr, None)
                if child:
                    count += self._count_locals_in_block(child)
            if hasattr(stmt, "else_ifs"):
                for _, elif_body in getattr(stmt, "else_ifs", []):
                    if elif_body:
                        count += self._count_locals_in_block(elif_body)
        return count
