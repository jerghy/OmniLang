"""P0107: Async task scheduling order differs across runtimes.

Different async runtimes schedule tasks differently:
  - JS: Microtask queue (Promise callbacks run before next tick)
  - Dart: Microtask queue, similar to JS but with zones
  - Python (asyncio): Tasks yield at await points, order depends on event loop
  - Rust (tokio): Work-stealing scheduler, no ordering guarantee

Code that depends on specific task ordering is non-portable.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import AwaitExpr, Block


class P0107Rule(AnalyzeRule):
    code = "P0107"
    name = "async_scheduling_order"
    severity = Severity.PORT
    category = "portability"
    message = "Multiple consecutive `await` calls: task scheduling order differs across async runtimes"
    help = (
        "JS/Dart use microtask queues, Python uses asyncio event loop, "
        "Rust uses work-stealing. Do not depend on specific ordering of "
        "concurrently scheduled tasks. Use `all()` or `race()` for explicit coordination."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
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
        # Look for consecutive await expressions in a block (fragile ordering pattern)
        for i, stmt in enumerate(block.stmts):
            await_count = self._count_awaits(stmt)
            if await_count >= 2 and hasattr(stmt, 'span'):
                span = stmt.span
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

            for child_attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._check_block(child, diagnostics, ctx)

        if block.trailing_expr and hasattr(block.trailing_expr, '__dataclass_fields__'):
            if self._count_awaits(block.trailing_expr) >= 2 and hasattr(block.trailing_expr, 'span'):
                span = block.trailing_expr.span
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

    def _count_awaits(self, expr) -> int:
        if isinstance(expr, AwaitExpr):
            return 1
        total = 0
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        total += self._count_awaits(item)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                total += self._count_awaits(val)
        return total
