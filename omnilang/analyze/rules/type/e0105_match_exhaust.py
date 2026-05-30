"""E0105: Match exhaustiveness checking.

Ensures all variants of a tagged union are covered in match arms.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import MatchExpr, IgnorePattern, TaggedUnionDef


class E0105Rule(AnalyzeRule):
    code = "E0105"
    name = "match_exhaustiveness"
    severity = Severity.ERROR
    category = "type"
    message = "Non-exhaustive match: missing variant(s)"
    help = "Add match arms for all variants, or add a wildcard `_ => ...` arm."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diags = []
        self._ctx = ctx

        # Collect tagged union definitions
        unions: dict[str, TaggedUnionDef] = {}
        for tu in getattr(ctx.program, 'tagged_unions', []):
            unions[tu.name] = tu

        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, unions, diags)
        return diags

    def _filename(self):
        return next(iter(self._ctx.source_lines.keys()), "<input>") if self._ctx.source_lines else "<input>"

    def _check_block(self, block, unions, diags):
        for stmt in getattr(block, 'stmts', []):
            for attr in ['init', 'value', 'condition', 'iterable', 'arg', 'expr']:
                val = getattr(stmt, attr, None)
                if isinstance(val, MatchExpr):
                    self._check_match(val, unions, diags)
                elif val:
                    self._walk_expr(val, unions, diags)
            for attr in ['body', 'then_body', 'else_body']:
                child = getattr(stmt, attr, None)
                if child:
                    self._check_block(child, unions, diags)

    def _walk_expr(self, expr, unions, diags):
        if isinstance(expr, MatchExpr):
            self._check_match(expr, unions, diags)
        for fn in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._walk_expr(item, unions, diags)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._walk_expr(val, unions, diags)

    def _check_match(self, me: MatchExpr, unions, diags):
        # Check if any arm has a wildcard — if so, it's exhaustive
        has_wildcard = any(isinstance(a.pattern, IgnorePattern) for a in me.arms)
        if has_wildcard:
            return

        # Try to determine scrutinee type
        scrutinee_ty = self._ctx.type_map.get(id(me.scrutinee))
        if scrutinee_ty is None:
            return
        type_name = getattr(scrutinee_ty, 'name', None)
        if type_name and type_name in unions:
            union_def = unions[type_name]
            covered = set()
            for arm in me.arms:
                if hasattr(arm.pattern, 'name'):
                    covered.add(arm.pattern.name)
            missing = [v.name for v in union_def.variants if v.name not in covered]
            if missing:
                span = getattr(me, 'span', None)
                loc = SourceLocation(file=self._filename(), line=span.line if span else 1,
                                     column=span.col if span else 1,
                                     span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1))
                diags.append(Diagnostic(code=self.code, severity=self.severity, category=self.category,
                                        message=f"{self.message}: {', '.join(missing)}",
                                        location=loc, help=self.help))
