"""E0202: Async function without any await expression.

An async function that contains no `await` call serves no purpose
and may indicate a logic error (forgot to await something).
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import AwaitExpr, Block


class E0202Rule(AnalyzeRule):
    code = "E0202"
    name = "async_without_await"
    severity = Severity.WARNING
    category = "async"
    message = "async function `{name}` contains no await expression"
    help = "Either add an await call or remove the `async` modifier."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []
        for func in ctx.program.functions:
            if func.is_async and func.body:
                if not self._has_await(func.body):
                    span = func.span
                    loc = SourceLocation(
                        file=self._filename(ctx),
                        line=span.line if span else 1,
                        column=span.col if span else 1,
                        span=(span.line, span.col, span.end_line, span.end_col) if span else (1, 1, 1, 1),
                    )
                    diagnostics.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=self.message.format(name=func.name),
                        location=loc, help=self.help,
                    ))
        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _has_await(self, block: Block) -> bool:
        for stmt in block.stmts:
            if self._stmt_has_await(stmt):
                return True
        if block.trailing_expr and self._expr_has_await(block.trailing_expr):
            return True
        return False

    def _stmt_has_await(self, stmt) -> bool:
        for attr in ['init', 'value', 'condition', 'arg', 'expr']:
            val = getattr(stmt, attr, None)
            if val is not None and hasattr(val, '__dataclass_fields__'):
                if self._expr_has_await(val):
                    return True
        for child_attr in ['body', 'then_body', 'else_body']:
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                if self._has_await(child):
                    return True
        return False

    def _expr_has_await(self, expr) -> bool:
        if isinstance(expr, AwaitExpr):
            return True
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        if self._expr_has_await(item):
                            return True
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                if self._expr_has_await(val):
                    return True
        return False
