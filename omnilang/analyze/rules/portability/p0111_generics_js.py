"""P0111: Generic type erasure in JS plain mode."""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

class P0111Rule(AnalyzeRule):
    code = "P0111"; name = "generics_erasure_js"; severity = Severity.PORT; category = "portability"
    message = "Generic types are erased in JS plain mode — no runtime type checking"
    help = "Use TypeScript mode for generic type preservation, or add runtime type guards."
    def check(self, ctx):
        diags = []
        for func in ctx.program.functions:
            if func.generic_params and "js" in ctx.targets:
                s = func.span
                if s:
                    loc = SourceLocation(file=next(iter(ctx.source_lines),"<input>"), line=s.line, column=s.col, span=(s.line,s.col,s.end_line,s.end_col))
                    diags.append(Diagnostic(code=self.code, severity=self.severity, category=self.category, message=f"Generic function '{func.name}' will lose type info in JS", location=loc, help=self.help))
        return diags
