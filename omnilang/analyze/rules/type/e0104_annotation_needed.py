"""E0104: Type annotation needed for ambiguous inference.

When a variable's type cannot be reliably inferred, an explicit annotation is required.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, Block


class E0104Rule(AnalyzeRule):
    code = "E0104"
    name = "annotation_needed"
    severity = Severity.ERROR
    category = "type"
    message = "Type annotation needed for `{name}`: cannot infer type from context"
    help = "Add an explicit type annotation, e.g. `let {name}: Type = ...`."

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

    def _check_block(self, block: Block, diagnostics):
        for stmt in block.stmts:
            if isinstance(stmt, VarDecl) and not stmt.type_ann and not stmt.init:
                span = stmt.span
                loc = SourceLocation(file=self._filename(), line=span.line if span else 1,
                    column=span.col if span else 1,
                    span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1))
                diagnostics.append(Diagnostic(code=self.code, severity=self.severity,
                    category=self.category, message=self.message.format(name=stmt.name),
                    location=loc, help=self.help.format(name=stmt.name)))
            for attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, attr, None)
                if isinstance(child, Block):
                    self._check_block(child, diagnostics)
