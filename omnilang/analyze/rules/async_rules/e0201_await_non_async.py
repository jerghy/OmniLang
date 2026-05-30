"""E0201: await used outside async function."""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import AwaitExpr

class E0201Rule(AnalyzeRule):
    code = "E0201"; name = "await_in_non_async"; severity = Severity.ERROR; category = "async"
    message = "`await` can only be used inside `async fn`"
    help = "Mark the containing function as `async fn` or remove the `await`."
    def check(self, ctx):
        diags = []
        self._ctx = ctx
        for func in ctx.program.functions:
            if not func.is_async and func.body:
                self._check_block(func.body, diags)
        return diags
    def _filename(self): return next(iter(self._ctx.source_lines),"<input>") if self._ctx.source_lines else "<input>"
    def _check_block(self, block, diags):
        for stmt in getattr(block,'stmts',[]):
            for attr in ['init','value','condition','iterable','arg','expr']:
                v = getattr(stmt,attr,None)
                if v: self._walk(v, diags)
            for attr in ['body','then_body','else_body']:
                c = getattr(stmt,attr,None)
                if c: self._check_block(c, diags)
    def _walk(self, expr, diags):
        if isinstance(expr, AwaitExpr):
            s = getattr(expr,'span',None)
            loc = SourceLocation(file=self._filename(), line=s.line if s else 1, column=s.col if s else 1, span=(s.line,s.col,s.end_line,s.end_col) if s else (1,1,1,1))
            diags.append(Diagnostic(code=self.code, severity=self.severity, category=self.category, message=self.message, location=loc, help=self.help))
        for fn in getattr(expr,'__dataclass_fields__',{}):
            val = getattr(expr,fn)
            if hasattr(val,'__dataclass_fields__') and val is not None: self._walk(val, diags)
            elif isinstance(val,list):
                for item in val:
                    if hasattr(item,'__dataclass_fields__'): self._walk(item, diags)
