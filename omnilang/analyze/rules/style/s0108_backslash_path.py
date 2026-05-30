"""S0108: Backslash in import path.

OmniLang uses forward-slash (`/`) for import paths on all platforms.
Using backslash (`\\`) in import paths will fail on Unix/macOS.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import ImportStmt


class S0108Rule(AnalyzeRule):
    code = "S0108"
    name = "backslash_in_path"
    severity = Severity.WARNING
    category = "style"
    message = "Import path contains backslash `\\`: use `/` for cross-platform compatibility"
    help = "Replace `\\` with `/` in import paths. OmniLang normalizes paths internally."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []
        for imp in ctx.program.imports:
            path = imp.path or ""
            if "\\" in path:
                span = imp.span
                loc = SourceLocation(
                    file=self._filename(ctx),
                    line=span.line if span else 1,
                    column=span.col if span else 1,
                    span=(span.line, span.col, span.end_line, span.end_col) if span else (1, 1, 1, 1),
                )
                diagnostics.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category, message=self.message,
                    location=loc, help=self.help,
                ))
        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"
