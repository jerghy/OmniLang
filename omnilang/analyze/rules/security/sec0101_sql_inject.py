"""SEC0101: Potential string-based injection vulnerability."""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import BinaryOp, TemplateString, FuncCall

class SEC0101Rule(AnalyzeRule):
    code = "SEC0101"; name = "potential_injection"; severity = Severity.SEC; category = "security"
    message = "String concatenation or template with user input may be vulnerable to injection"
    help = "Use parameterized queries or sanitize user input before string interpolation."
    def check(self, ctx):
        diags = []
        for func in ctx.program.functions:
            if func.body: self._check(func.body, diags)
        return diags
    def _check(self, block, diags):
        for stmt in getattr(block,'stmts',[]):
            for attr in ['init','value','condition','iterable','arg','expr']:
                v = getattr(stmt,attr,None)
                if isinstance(v, TemplateString):
                    for p in v.parts:
                        if p.kind == "expr" and hasattr(p.value, 'name'):
                            s = getattr(v,'span',None)
                            if s:
                                loc = SourceLocation(file="<input>", line=s.line, column=s.col, span=(s.line,s.col,s.end_line,s.end_col))
                                diags.append(Diagnostic(code=self.code, severity=Severity.WARNING, category=self.category, message=self.message, location=loc, help=self.help))
                                break
            for attr in ['body','then_body','else_body']:
                c = getattr(stmt,attr,None)
                if c: self._check(c, diags)
