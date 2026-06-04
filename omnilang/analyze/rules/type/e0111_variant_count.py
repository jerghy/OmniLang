"""E0111: Variant count mismatch between tagged union and match arms."""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import MatchExpr


class E0111Rule(AnalyzeRule):
    code = "E0111"
    name = "variant_count_mismatch"
    severity = Severity.WARNING
    category = "type"
    message = "Match has {arms} arms but type `{name}` has {variants} variants"
    help = "Verify all variants are handled or add missing arms."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        union_variants = {u.name: len(u.variants) for u in ctx.program.tagged_unions}
        for func in ctx.program.functions:
            if func.body:
                self._walk(func.body, diagnostics, union_variants)
        return diagnostics

    def _filename(self):
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _walk(self, block, diagnostics, uv):
        for stmt in block.stmts:
            for attr in ['init', 'value', 'expr']:
                val = getattr(stmt, attr, None)
                if isinstance(val, MatchExpr):
                    self._check(val, diagnostics, uv)
                elif hasattr(val, '__dataclass_fields__'):
                    self._recurse_expr(val, diagnostics, uv)
            for attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, attr, None)
                if hasattr(child, 'stmts'):
                    self._walk(child, diagnostics, uv)
        if block.trailing_expr:
            self._recurse_expr(block.trailing_expr, diagnostics, uv)

    def _check(self, m: MatchExpr, diagnostics, uv):
        scrutinee_ty = self._ctx.type_map.get(id(m.scrutinee))
        if scrutinee_ty:
            tname = getattr(scrutinee_ty, 'name', '')
            if tname in uv and uv[tname] != len(m.arms):
                span = m.span
                loc = SourceLocation(file=self._filename(), line=span.line if span else 1,
                    column=span.col if span else 1,
                    span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1))
                diagnostics.append(Diagnostic(code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(arms=len(m.arms), name=tname, variants=uv[tname]),
                    location=loc, help=self.help))

    def _recurse_expr(self, expr, diagnostics, uv):
        if isinstance(expr, MatchExpr):
            self._check(expr, diagnostics, uv)
        for fn in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._recurse_expr(item, diagnostics, uv)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._recurse_expr(val, diagnostics, uv)
