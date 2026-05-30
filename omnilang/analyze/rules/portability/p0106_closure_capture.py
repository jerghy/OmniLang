"""P0106: Closure capture semantics differ across targets."""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import ClosureExpr
from omnilang.semantic.capture_analyzer import analyze_captures, CaptureKind

class P0106Rule(AnalyzeRule):
    code = "P0106"; name = "closure_capture_semantics"; severity = Severity.PORT; category = "portability"
    message = "Closure capture semantics differ: JS/Dart/Python use reference capture; Rust uses move+Cell/RefCell"
    help = "For mutable captures in cross-target code, avoid relying on shared mutable state in closures."
    def check(self, ctx):
        diags = []
        captures = analyze_captures(ctx.program)
        for ce_id, caps in captures.items():
            mutable_caps = [c for c in caps if c.kind == CaptureKind.MUTABLE]
            if mutable_caps and "rust" in ctx.targets:
                # Find the ClosureExpr by id
                for func in ctx.program.functions:
                    for found in self._find_closures(func.body, ce_id):
                        s = found.span
                        if s:
                            loc = SourceLocation(file=next(iter(ctx.source_lines),"<input>"), line=s.line, column=s.col, span=(s.line,s.col,s.end_line,s.end_col))
                            diags.append(Diagnostic(code=self.code, severity=self.severity, category=self.category, message=self.message, location=loc, help=self.help))
        return diags
    def _find_closures(self, block, target_id):
        results = []
        if block is None: return results
        for stmt in getattr(block,'stmts',[]):
            for attr in ['init','value','condition','iterable','arg','expr']:
                v = getattr(stmt,attr,None)
                if isinstance(v, ClosureExpr) and id(v) == target_id: results.append(v)
            for attr in ['body','then_body','else_body']:
                c = getattr(stmt,attr,None)
                if c: results.extend(self._find_closures(c, target_id))
        return results
