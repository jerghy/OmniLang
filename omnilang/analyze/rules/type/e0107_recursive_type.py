"""E0107: Recursive type without indirection.

A type definition that directly or indirectly refers to itself without
using a pointer/box/reference causes infinite size at compile time.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation


class E0107Rule(AnalyzeRule):
    code = "E0107"
    name = "recursive_type"
    severity = Severity.ERROR
    category = "type"
    message = "Recursive type `{name}` must use indirection (Box/Ref/&) to avoid infinite size"
    help = "Wrap the recursive field in `Box<T>` or use `&T` reference for heap allocation."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        for struct in ctx.program.structs:
            for field in struct.fields:
                tname = getattr(field.type_ann, 'name', '')
                if tname == struct.name:
                    span = field.span or struct.span
                    loc = SourceLocation(file=self._filename(),
                        line=span.line if span else 1, column=span.col if span else 1,
                        span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1))
                    diagnostics.append(Diagnostic(code=self.code, severity=self.severity,
                        category=self.category, message=self.message.format(name=struct.name),
                        location=loc, help=self.help))
        return diagnostics

    def _filename(self):
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"
