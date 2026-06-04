"""E0106: Unreachable match arm (dead pattern).

A match arm can never be reached because a previous arm already matches all its cases.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import MatchExpr


class E0106Rule(AnalyzeRule):
    code = "E0106"
    name = "dead_match_arm"
    severity = Severity.WARNING
    category = "type"
    message = "Unreachable match arm: previous wildcard `_` or superset pattern already matches"
    help = "Remove the dead arm or reorder match arms so the dead arm is checked first."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics)
        return diagnostics

    def _filename(self):
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block, diagnostics):
        for stmt in block.stmts:
            for attr in ['init', 'value', 'condition', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if isinstance(val, MatchExpr):
                    self._check_match(val, diagnostics)
                elif hasattr(val, '__dataclass_fields__'):
                    self._check_expr(val, diagnostics)
            for attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, attr, None)
                if hasattr(child, 'stmts'):
                    self._check_block(child, diagnostics)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, diagnostics)

    def _check_expr(self, expr, diagnostics):
        if isinstance(expr, MatchExpr):
            self._check_match(expr, diagnostics)
        for fn in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, diagnostics)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, diagnostics)

    def _check_match(self, match: MatchExpr, diagnostics):
        _has_wildcard = False
        for i, arm in enumerate(match.arms):
            pattern = arm.pattern
            pname = getattr(pattern, 'name', getattr(pattern, 'value', None))
            pcls = type(pattern).__name__
            if pcls == 'IgnorePattern' or pname == '_':
                _has_wildcard = True
                if i + 1 < len(match.arms):
                    span = match.arms[i + 1].span or match.span
                    loc = SourceLocation(file=self._filename(), line=span.line if span else 1,
                        column=span.col if span else 1,
                        span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1))
                    diagnostics.append(Diagnostic(code=self.code, severity=self.severity,
                        category=self.category, message=self.message, location=loc, help=self.help))
                    break
