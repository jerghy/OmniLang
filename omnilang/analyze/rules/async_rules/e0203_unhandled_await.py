"""E0203: Unhandled await in async function (Rust target).

In Rust, every `await` on a Result-returning future must have its error
handled (via `?`, `.unwrap()`, or explicit `match`). Unhandled results
cause the future to silently drop errors.

This rule fires when targeting Rust and finding await expressions without
error handling in the enclosing context.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import AwaitExpr, Block


class E0203Rule(AnalyzeRule):
    code = "E0203"
    name = "unhandled_await_rust"
    severity = Severity.WARNING
    category = "async"
    message = "Await expression may have unhandled error (Rust requires explicit error handling)"
    help = (
        "In Rust, use `expr.await?` to propagate errors, `.unwrap()` to panic on error, "
        "or `match` to handle both Ok and Err variants explicitly."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        # This rule is only relevant for Rust target
        targets = [t.lower() for t in ctx.targets]
        if "rust" not in targets:
            return []

        diagnostics = []
        for func in ctx.program.functions:
            if func.is_async and func.body:
                self._check_block(func.body, diagnostics, ctx)

        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block: Block, diagnostics, ctx: AnalyzeContext):
        for stmt in block.stmts:
            for attr in ['init', 'value', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if val is not None and hasattr(val, '__dataclass_fields__'):
                    self._check_expr(val, diagnostics, ctx)
            for child_attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._check_block(child, diagnostics, ctx)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, diagnostics, ctx)

    def _check_expr(self, expr, diagnostics, ctx: AnalyzeContext):
        if isinstance(expr, AwaitExpr):
            # Check if this await is inside error-handling context
            # Simplified: flag all awaits in Rust-targeted async functions
            span = expr.span
            loc = SourceLocation(
                file=self._filename(ctx),
                line=span.line if span else 1,
                column=span.col if span else 1,
                span=(span.line, span.col, span.end_line, span.end_col) if span else (1, 1, 1, 1),
            )
            diagnostics.append(Diagnostic(
                code=self.code, severity=self.severity,
                category=self.category, message=self.message,
                location=loc, help=self.help,
            ))

        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, diagnostics, ctx)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, diagnostics, ctx)
