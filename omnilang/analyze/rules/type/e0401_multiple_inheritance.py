"""E0401: Multiple inheritance rejected.

OmniLang does not support multiple class inheritance.
Classes may extend at most one base class; interface-like contracts
should use traits (via the `nominal struct` + `impl` pattern) instead.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import ClassDef


class E0401Rule(AnalyzeRule):
    code = "E0401"
    name = "multiple_inheritance"
    severity = Severity.ERROR
    category = "type"
    message = "Multiple inheritance is not supported: class `{name}` extends `{extends}` and implements {implements}"
    help = "Use a single base class with trait composition. Convert extra parents to traits via `nominal struct` + `impl`."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []

        for cls in ctx.program.classes:
            self._check_class(cls, diagnostics)

        return diagnostics

    def _filename(self) -> str:
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _span_loc(self, span) -> SourceLocation:
        if span:
            return SourceLocation(
                file=self._filename(), line=span.line, column=span.col,
                span=(span.line, span.col, span.end_line, span.end_col),
            )
        return SourceLocation(file=self._filename(), line=1, column=1, span=(1, 1, 1, 1))

    def _check_class(self, cls: ClassDef, diagnostics: list[Diagnostic]):
        # Multiple inheritance: extends is set AND implements has entries
        if cls.extends and cls.implements:
            implements_str = ", ".join(cls.implements)
            loc = self._span_loc(cls.span)
            diagnostics.append(Diagnostic(
                code=self.code,
                severity=self.severity,
                category=self.category,
                message=self.message.format(
                    name=cls.name,
                    extends=cls.extends,
                    implements=implements_str,
                ),
                location=loc,
                help=self.help,
            ))
        # Also check if extends itself refers to a comma-separated list
        elif cls.extends and "," in cls.extends:
            loc = self._span_loc(cls.span)
            diagnostics.append(Diagnostic(
                code=self.code,
                severity=self.severity,
                category=self.category,
                message=self.message.format(
                    name=cls.name,
                    extends=cls.extends,
                    implements="<none>",
                ),
                location=loc,
                help=self.help,
            ))
