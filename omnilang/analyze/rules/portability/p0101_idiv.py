"""P0101: Cross-language integer division behavior.

In OmniLang, integer division (`/`) on `i32` types has different
semantics across target languages:
    - JS: Returns float (5 / 2 = 2.5)
    - Dart/Rust: Truncates toward zero (5 / 2 = 2, -5 / 2 = -2)
    - Python: Floors toward negative infinity (5 / 2 = 2, -5 / 2 = -3)

This rule warns when integer division is detected in code targeting
JS or Python, where the behavior diverges from Dart/Rust.

Fix: Use `_idiv(a, b)` from `omni/math` for portable integer division.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import BinaryOp


class P0101Rule(AnalyzeRule):
    code = "P0101"
    name = "cross_lang_div_behavior"
    severity = Severity.PORT
    category = "portability"
    message = "Integer division `/` has different semantics across target languages"
    help = (
        "JS: returns float. Dart/Rust: truncates toward zero. "
        "Python: floors toward negative infinity. "
        "Use `_idiv(a, b)` from 'omni/math' for portable integer division."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []
        self._ctx = ctx
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics)
        return diagnostics

    def _filename(self) -> str:
        """Get the primary source filename from the context."""
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block, diagnostics):
        for stmt in block.stmts:
            self._check_stmt(stmt, diagnostics)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, diagnostics)

    def _check_stmt(self, stmt, diagnostics):
        for attr in ['init', 'value', 'condition', 'iterable', 'arg', 'expr']:
            if hasattr(stmt, attr):
                val = getattr(stmt, attr)
                if val is not None:
                    self._check_expr(val, diagnostics)
        for attr in ['body', 'then_body', 'else_body']:
            child = getattr(stmt, attr, None)
            if child:
                self._check_block(child, diagnostics)
        if hasattr(stmt, 'else_ifs'):
            for _, elif_body in getattr(stmt, 'else_ifs', []):
                if elif_body:
                    self._check_block(elif_body, diagnostics)

    def _check_expr(self, expr, diagnostics):
        if isinstance(expr, BinaryOp) and expr.op in ("/", "%"):
            left_ty = self._ctx.type_map.get(id(expr.left))
            if left_ty and getattr(left_ty, 'name', None) == "i32":
                span = expr.span
                loc = SourceLocation(
                    file=self._filename(),
                    line=span.line if span else 1,
                    column=span.col if span else 1,
                    span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1),
                )
                diagnostics.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category, message=self.message,
                    location=loc, help=self.help,
                ))
        # Recurse
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, diagnostics)
                    elif isinstance(item, tuple):
                        for sub in item:
                            if hasattr(sub, '__dataclass_fields__'):
                                self._check_expr(sub, diagnostics)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, diagnostics)
