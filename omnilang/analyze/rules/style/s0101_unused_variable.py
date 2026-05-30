"""S0101: Unused variable.

Detects variables that are declared but never read.
These clutter the code and may indicate a bug (e.g., misspelled later usage).
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, VarRef, Assign, Block


class S0101Rule(AnalyzeRule):
    code = "S0101"
    name = "unused_variable"
    severity = Severity.WARNING
    category = "style"
    message = "Variable `{name}` is declared but never used"
    help = "Remove the unused variable declaration or use it in an expression."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []
        for func in ctx.program.functions:
            if func.body:
                declared: set[str] = set()
                used: set[str] = set()
                self._collect_decls_and_uses(func.body, declared, used)
                unused = declared - used - {'_', 'self', 'this'}
                for name in sorted(unused):
                    loc = self._guess_loc(func.body, name, ctx)
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

    def _guess_loc(self, block: Block, name: str, ctx: AnalyzeContext) -> SourceLocation:
        for stmt in block.stmts:
            if isinstance(stmt, VarDecl) and stmt.name == name and stmt.span:
                span = stmt.span
                return SourceLocation(
                    file=self._filename(ctx), line=span.line, column=span.col,
                    span=(span.line, span.col, span.end_line, span.end_col),
                )
        return SourceLocation(file=self._filename(ctx), line=1, column=1, span=(1,1,1,1))

    def _collect_decls_and_uses(self, block: Block, declared: set, used: set):
        for stmt in block.stmts:
            if isinstance(stmt, VarDecl):
                declared.add(stmt.name)
                if stmt.init:
                    self._collect_uses(stmt.init, used)
            elif isinstance(stmt, Assign):
                self._collect_uses(stmt.target, used)
                if stmt.value:
                    self._collect_uses(stmt.value, used)
            else:
                for attr in ['init', 'value', 'condition', 'iterable', 'arg', 'expr']:
                    val = getattr(stmt, attr, None)
                    if val is not None and hasattr(val, '__dataclass_fields__'):
                        self._collect_uses(val, used)

            for child_attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._collect_decls_and_uses(child, declared, used)

        if block.trailing_expr:
            self._collect_uses(block.trailing_expr, used)

    def _collect_uses(self, expr, used: set):
        if isinstance(expr, VarRef) and expr.name:
            used.add(expr.name)
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._collect_uses(item, used)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._collect_uses(val, used)
