"""E0103: Unknown field in struct literal.

When constructing a struct literal, all field names must match
the struct definition. Unknown fields are rejected.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import StructLiteral, StructDef


class E0103Rule(AnalyzeRule):
    code = "E0103"
    name = "extra_field"
    severity = Severity.ERROR
    category = "type"
    message = "Unknown field `{field}` in struct literal `{struct_name}`"
    help = "Remove the unknown field or add it to the struct definition."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []

        struct_defs: dict[str, StructDef] = {}
        for s in ctx.program.structs:
            struct_defs[s.name] = {f.name for f in s.fields}

        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics, struct_defs)

        return diagnostics

    def _filename(self) -> str:
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block, diagnostics, struct_defs: dict):
        for stmt in block.stmts:
            for attr in ['init', 'value', 'condition', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if val is not None and hasattr(val, '__dataclass_fields__'):
                    self._check_expr(val, diagnostics, struct_defs)
            for child_attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, child_attr, None)
                if hasattr(child, 'stmts'):
                    self._check_block(child, diagnostics, struct_defs)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, diagnostics, struct_defs)

    def _check_expr(self, expr, diagnostics, struct_defs: dict):
        if isinstance(expr, StructLiteral):
            known = struct_defs.get(expr.type_name)
            if known is not None:
                for field_name in expr.fields:
                    if field_name not in known:
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
                            message=self.message.format(field=field_name, struct_name=expr.type_name),
                            location=loc, help=self.help,
                        ))

        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, diagnostics, struct_defs)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, diagnostics, struct_defs)
