"""E0306: Closure that captures variables escapes its defining scope.

When a closure is returned from or stored outside its parent function,
captured mutable variables may lead to use-after-free or data races.
In Rust, this requires Cell/RefCell or move semantics.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import ClosureExpr, ReturnStmt, VarDecl, Assign, Block, FunctionDef


class E0306Rule(AnalyzeRule):
    code = "E0306"
    name = "closure_escape"
    severity = Severity.WARNING
    category = "ownership"
    message = "Closure capturing `{captures}` escapes its defining function"
    help = (
        "Captured variables may need Cell/RefCell wrapping in Rust. "
        "Consider using `move` closures or refactoring to avoid escaping."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []

        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics)

        return diagnostics

    def _filename(self) -> str:
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block: Block, diagnostics):
        for stmt in block.stmts:
            self._check_stmt(stmt, diagnostics)
        if block.trailing_expr:
            self._check_expr_escape(stmt if False else None, diagnostics)  # not applicable
            self._check_expr_recursive(block.trailing_expr, diagnostics)

    def _check_stmt(self, stmt, diagnostics):
        if isinstance(stmt, ReturnStmt) and stmt.value:
            closures = self._find_closure_returns(stmt.value)
            for ce, captures in closures:
                if captures:
                    cap_names = ", ".join(captures)
                    span = ce.span
                    loc = SourceLocation(
                        file=self._filename(),
                        line=span.line if span else 1,
                        column=span.col if span else 1,
                        span=(span.line, span.col, span.end_line, span.end_col) if span else (1, 1, 1, 1),
                    )
                    diagnostics.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=self.message.format(captures=cap_names),
                        location=loc, help=self.help,
                    ))

        for child_attr in ['body', 'then_body', 'else_body']:
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                self._check_block(child, diagnostics)

    def _check_expr_escape(self, expr, diagnostics):
        """Check if an expression is a closure that captures variables in an escaping context."""
        pass  # handled by _check_stmt ReturnStmt logic

    def _check_expr_recursive(self, expr, diagnostics):
        """Recurse into expressions to find closures."""
        if isinstance(expr, ClosureExpr):
            # Check if this closure captures free variables
            captures = self._get_free_vars(expr)
            pass  # Escaping detection is done in ReturnStmt traversal

        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr_recursive(item, diagnostics)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr_recursive(val, diagnostics)

    def _find_closure_returns(self, expr) -> list[tuple]:
        """Find closures that are being returned (escaping context)."""
        results = []
        if isinstance(expr, ClosureExpr):
            results.append((expr, self._get_free_vars(expr)))
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        results.extend(self._find_closure_returns(item))
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                results.extend(self._find_closure_returns(val))
        return results

    def _get_free_vars(self, ce: ClosureExpr) -> list[str]:
        """Extract free variables referenced inside the closure body."""
        params = {p.name for p in ce.params}
        free = set()

        def walk(expr):
            for fn in getattr(expr, '__dataclass_fields__', {}):
                v = getattr(expr, fn)
                if isinstance(v, list):
                    for item in v:
                        if hasattr(item, '__dataclass_fields__'):
                            walk(item)
                elif hasattr(v, '__dataclass_fields__') and v is not None:
                    walk(v)

            # Check for VarRef (variable references)
            if hasattr(expr, 'name') and hasattr(expr, '__dataclass_fields__') and len(getattr(expr, '__dataclass_fields__', {})) <= 3:
                name = getattr(expr, 'name', None)
                if name and isinstance(name, str) and name not in params and not name.startswith('_'):
                    free.add(name)

        if ce.body:
            walk(ce.body)
        return sorted(free)
