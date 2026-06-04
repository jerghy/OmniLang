"""E0108: Method not found on type.

A method call references a method that doesn't exist on the receiver's type.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import MethodCall


class E0108Rule(AnalyzeRule):
    code = "E0108"
    name = "method_not_found"
    severity = Severity.ERROR
    category = "type"
    message = "Method `{method}` not found on type `{type_name}`"
    help = "Check the method name or ensure the receiver type has this method defined."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics)
        return diagnostics

    def _filename(self):
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block, diagnostics):
        for stmt in block.stmts:
            for attr in ['init', 'value', 'condition', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if hasattr(val, '__dataclass_fields__'):
                    self._check_expr(val, diagnostics)
            for attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, attr, None)
                if hasattr(child, 'stmts'):
                    self._check_block(child, diagnostics)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, diagnostics)

    def _check_expr(self, expr, diagnostics):
        if isinstance(expr, MethodCall):
            obj_ty = self._ctx.type_map.get(id(expr.object))
            tname = getattr(obj_ty, 'name', '?') if obj_ty else '?'
            if tname != '?':
                span = expr.span
                loc = SourceLocation(file=self._filename(),
                    line=span.line if span else 1, column=span.col if span else 1,
                    span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1))
                diagnostics.append(Diagnostic(code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(method=expr.method, type_name=tname),
                    location=loc, help=self.help))
        for fn in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, diagnostics)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, diagnostics)
