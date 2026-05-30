"""S0102: Unused import.

Detects import statements that bring in symbols which are never used
in the file. These add unnecessary dependencies and slow compilation.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import ImportStmt, VarRef, Block


class S0102Rule(AnalyzeRule):
    code = "S0102"
    name = "unused_import"
    severity = Severity.WARNING
    category = "style"
    message = "Import `{name}` is never used"
    help = "Remove the unused import to clean up dependencies."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []

        # Collect all imported names
        imports: dict[str, ImportStmt] = {}
        used_names: set[str] = set()

        for imp in ctx.program.imports:
            for name in imp.names:
                imports[name] = imp

        # Collect all variable references across the program
        for func in ctx.program.functions:
            if func.body:
                self._collect_uses(func.body, used_names)

        for name, imp in imports.items():
            if name not in used_names:
                span = imp.span
                loc = SourceLocation(
                    file=self._filename(ctx),
                    line=span.line if span else 1,
                    column=span.col if span else 1,
                    span=(span.line, span.col, span.end_line, span.end_col) if span else (1, 1, 1, 1),
                )
                diagnostics.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(name=name),
                    location=loc, help=self.help,
                ))

        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _collect_uses(self, block: Block, used: set):
        for stmt in block.stmts:
            for attr in ['init', 'value', 'condition', 'iterable', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if val is not None and hasattr(val, '__dataclass_fields__'):
                    self._collect_uses_expr(val, used)
            for child_attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._collect_uses(child, used)
        if block.trailing_expr:
            self._collect_uses_expr(block.trailing_expr, used)

    def _collect_uses_expr(self, expr, used: set):
        if isinstance(expr, VarRef) and expr.name:
            used.add(expr.name)
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._collect_uses_expr(item, used)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._collect_uses_expr(val, used)
