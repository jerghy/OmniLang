"""S0110: Too many function parameters.

Suggests grouping parameters into a struct/configuration object when a
function has more than a configurable threshold (default: 5) parameters.
Functions with many parameters are hard to call, test, and maintain.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import FunctionDef


class S0110Rule(AnalyzeRule):
    code = "S0110"
    name = "too_many_params"
    severity = Severity.STYLE
    category = "style"
    message = "Function `{name}` has {count} parameters (max suggested: {max_count})"
    help = "Consider grouping related parameters into a struct or configuration object."

    # Maximum number of parameters allowed before a warning is emitted
    max_params: int = 5

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []
        for func in ctx.program.functions:
            count = len(func.params)
            if count > self.max_params:
                diagnostics.append(self._make_diagnostic(func, count, ctx))
        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _make_diagnostic(
        self, func: FunctionDef, count: int, ctx: AnalyzeContext,
    ) -> Diagnostic:
        span = func.span
        line = span.line if span else 1
        col = span.col if span else 1
        loc = SourceLocation(
            file=self._filename(ctx),
            line=line, column=col,
            span=(line, col, span.end_line if span else 1, span.end_col if span else 1),
        )
        return Diagnostic(
            code=self.code, severity=self.severity,
            category=self.category,
            message=self.message.format(
                name=func.name, count=count, max_count=self.max_params,
            ),
            location=loc,
            help=self.help,
        )
