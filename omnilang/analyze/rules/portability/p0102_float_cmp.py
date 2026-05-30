"""P0102: Cross-language float comparison precision difference."""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import BinaryOp


class P0102Rule(AnalyzeRule):
    code = "P0102"
    name = "float_comparison"
    severity = Severity.PORT
    category = "portability"
    message = "Floating-point equality comparison may differ across platforms"
    help = "Float ==/!= can differ due to IEEE 754 implementation variance. Use epsilon comparison."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diags = []
        self._ctx = ctx
        for func in ctx.program.functions:
            if func.body:
                self._walk_block(func.body, diags)
        return diags

    def _filename(self):
        return next(iter(self._ctx.source_lines.keys()), "<input>") if self._ctx.source_lines else "<input>"

    def _walk_block(self, block, diags):
        for stmt in getattr(block, 'stmts', []):
            for attr in ['init', 'value', 'condition', 'iterable', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if val:
                    self._walk_expr(val, diags)
            for attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, attr, None)
                if child:
                    self._walk_block(child, diags)

    def _walk_expr(self, expr, diags):
        if isinstance(expr, BinaryOp) and expr.op in ("==", "!="):
            left_ty = self._ctx.type_map.get(id(expr.left))
            if left_ty and getattr(left_ty, 'name', None) == "f64":
                span = getattr(expr, 'span', None)
                loc = SourceLocation(file=self._filename(), line=span.line if span else 1,
                                     column=span.col if span else 1,
                                     span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1))
                diags.append(Diagnostic(code=self.code, severity=self.severity, category=self.category,
                                         message=self.message, location=loc, help=self.help))
        for fn in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, fn)
            if hasattr(val, '__dataclass_fields__') and val is not None:
                self._walk_expr(val, diags)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._walk_expr(item, diags)
