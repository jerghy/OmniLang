"""P0105: Floating-point precision differences across targets.

Different target languages use different float precision internally:
  - JS: All numbers are IEEE 754 f64
  - Dart: double (f64), but VM may differ from compiled
  - Python: float is f64 (C double)
  - Rust: f32/f64 match IEEE 754

Exact float comparisons (`==`, `!=`) are flagged because they're
fragile across targets due to accumulated rounding differences.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import BinaryOp


class P0105Rule(AnalyzeRule):
    code = "P0105"
    name = "float_precision"
    severity = Severity.PORT
    category = "portability"
    message = "Floating-point comparison `{op}` may differ across targets due to precision"
    help = "Use an epsilon-based comparison: `abs(a - b) < epsilon` with an appropriate epsilon."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics)
        return diagnostics

    def _filename(self) -> str:
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block, diagnostics):
        for stmt in block.stmts:
            for attr in ['init', 'value', 'condition', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if val is not None and hasattr(val, '__dataclass_fields__'):
                    self._check_expr(val, diagnostics)
            for child_attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, child_attr, None)
                if hasattr(child, '__dataclass_fields__') or hasattr(child, 'stmts'):
                    self._check_block(child, diagnostics)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, diagnostics)

    def _check_expr(self, expr, diagnostics):
        if isinstance(expr, BinaryOp) and expr.op in ("==", "!="):
            left_ty = self._ctx.type_map.get(id(expr.left))
            right_ty = self._ctx.type_map.get(id(expr.right))
            left_name = getattr(left_ty, 'name', None)
            right_name = getattr(right_ty, 'name', None)
            if left_name in ("f64", "f32", "float") or right_name in ("f64", "f32", "float"):
                span = expr.span
                loc = SourceLocation(
                    file=self._filename(),
                    line=span.line if span else 1,
                    column=span.col if span else 1,
                    span=(span.line, span.col, span.end_line, span.end_col) if span else (1, 1, 1, 1),
                )
                diagnostics.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(op=expr.op),
                    location=loc, help=self.help,
                ))
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, diagnostics)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, diagnostics)
