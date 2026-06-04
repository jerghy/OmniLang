"""E0110: Unbounded generic type parameter.

Warns when a generic type parameter is declared without any trait
bounds.  In Rust-target code this may limit expressiveness (e.g. you
cannot call methods on `T` without an explicit bound).  In cross-target
code an unbounded parameter may behave differently across languages
(e.g. JS erases all generics at runtime).

The preferred style is to always provide at least one trait bound
(e.g. `T: Sized` or `T: Display`) to make the constraint explicit.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    GenericParam,
)


class E0110Rule(AnalyzeRule):
    code = "E0110"
    name = "unbounded_generic_param"
    severity = Severity.STYLE
    category = "type"
    message = "Generic type parameter `{name}` has no trait bounds"
    help = (
        "Add at least one trait bound (e.g. `T: Sized`) to make "
        "constraints explicit and improve cross-target portability. "
        "OmniLang convention: `T: Sized + Display` or similar."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            diagnostics.extend(
                self._check_params(func.generic_params, func.name, "function", func.span)
            )

        for struct in ctx.program.structs:
            diagnostics.extend(
                self._check_params(struct.generic_params, struct.name, "struct", struct.span)
            )

        for tagged in ctx.program.tagged_unions:
            diagnostics.extend(
                self._check_params(tagged.generic_params, tagged.name, "tagged union", tagged.span)
            )

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
        return SourceLocation(
            file=self._filename(), line=1, column=1, span=(1, 1, 1, 1),
        )

    def _check_params(
        self,
        params: list,
        owner_name: str,
        kind: str,
        span,
    ) -> list[Diagnostic]:
        """Emit a diagnostic for each generic parameter without bounds."""
        diagnostics: list[Diagnostic] = []

        for gp in params:
            if not isinstance(gp, GenericParam):
                continue
            if gp.bounds:
                continue  # has at least one bound -- ok

            loc = self._span_loc(gp.span or span)
            diagnostics.append(Diagnostic(
                code=self.code,
                severity=self.severity,
                category=self.category,
                message=self.message.format(name=gp.name),
                location=loc,
                help=self.help,
            ))

        return diagnostics
