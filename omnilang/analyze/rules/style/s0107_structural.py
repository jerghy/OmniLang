"""S0107: Structural type coincidence warning."""
from omnilang.analyze.registry import AnalyzeRule
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

class S0107Rule(AnalyzeRule):
    code = "S0107"
    name = "structural_coincidence"
    severity = Severity.HINT
    category = "style"
    message = "Two structs have identical shapes — possible coincidence vs intentional structural typing"
    help = "If this is intentional structural typing, suppress with #[allow(S0107)]. If nominal, add `nominal` keyword."
    def check(self, ctx):
        diags = []
        structs = getattr(ctx.program,'structs',[])
        for i, s1 in enumerate(structs):
            for s2 in structs[i+1:]:
                if self._same_shape(s1, s2):
                    s = s1.span
                    if s:
                        loc = SourceLocation(file=next(iter(ctx.source_lines),"<input>"), line=s.line, column=s.col, span=(s.line,s.col,s.end_line,s.end_col))
                        diags.append(Diagnostic(code=self.code, severity=self.severity, category=self.category, message=f"Structs '{s1.name}' and '{s2.name}' have identical shapes", location=loc, help=self.help))
        return diags
    def _same_shape(self, a, b):
        if len(a.fields) != len(b.fields):
            return False
        for fa, fb in zip(a.fields, b.fields):
            if fa.name != fb.name:
                return False
            if fa.type_ann and fb.type_ann and fa.type_ann.name != fb.type_ann.name:
                return False
        return True
