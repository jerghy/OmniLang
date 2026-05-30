"""E0102: Missing required field in struct literal.

When constructing a struct literal (TypeName { field: value, ... }),
all required fields (those without defaults) must be provided.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import StructLiteral, StructDef


class E0102Rule(AnalyzeRule):
    code = "E0102"
    name = "missing_field"
    severity = Severity.ERROR
    category = "type"
    message = "Missing required field `{field}` in struct literal `{struct_name}`"
    help = "Provide a value for the required field, or add a default value in the struct definition."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []

        # Build struct definition lookup
        struct_defs: dict[str, StructDef] = {}
        for s in ctx.program.structs:
            struct_defs[s.name] = s

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
            defn = struct_defs.get(expr.type_name)
            if defn:
                provided = set(expr.fields.keys())
                for field in defn.fields:
                    if field.default is None and field.name not in provided:
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
                            message=self.message.format(field=field.name, struct_name=expr.type_name),
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
