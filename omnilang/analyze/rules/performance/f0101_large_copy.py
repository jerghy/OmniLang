"""F0101: Unnecessary large object copy."""
from omnilang.analyze.registry import AnalyzeRule
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, VarRef

class F0101Rule(AnalyzeRule):
    code = "F0101"
    name = "large_copy"
    severity = Severity.PERF
    category = "performance"
    message = "Cloning a large collection may be expensive — consider passing by reference"
    help = "Use & reference instead of .clone() for large arrays/maps."
    def check(self, ctx):
        diags = []
        for func in ctx.program.functions:
            if func.body:
                self._check(func.body, diags, ctx)
        return diags
    def _check(self, block, diags, ctx):
        for stmt in getattr(block,'stmts',[]):
            if isinstance(stmt, VarDecl) and isinstance(stmt.init, VarRef):
                vty = ctx.type_map.get(id(stmt.init))
                if vty and getattr(vty,'name','') in ('Array','Map','String'):
                    s = stmt.span
                    if s:
                        loc = SourceLocation(file=next(iter(ctx.source_lines),"<input>"), line=s.line, column=s.col, span=(s.line,s.col,s.end_line,s.end_col))
                        diags.append(Diagnostic(code=self.code, severity=self.severity, category=self.category, message=self.message, location=loc, help=self.help))
            for attr in ['body','then_body','else_body']:
                c = getattr(stmt,attr,None)
                if c:
                    self._check(c, diags, ctx)
