"""S0104: Over-complex expression.

Flags deeply nested expressions that exceed a configurable depth
threshold (default: 4 levels). Deeply nested code is hard to read
and often signals that the logic should be broken into intermediate
variables or helper functions.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    Block,
)


class S0104Rule(AnalyzeRule):
    code = "S0104"
    name = "complex_expression"
    severity = Severity.STYLE
    category = "style"
    message = "Expression exceeds {max_depth} nesting levels (depth={depth})"
    help = "Extract sub-expressions into named intermediate variables to improve readability."

    max_depth: int = 4

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics)
        self._ctx = None
        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block: Block, diagnostics: list[Diagnostic]) -> None:
        for stmt in block.stmts:
            # Check any expression-holding attribute on the statement
            for attr in ('init', 'value', 'condition', 'iterable', 'arg', 'expr'):
                val = getattr(stmt, attr, None)
                if val is not None and hasattr(val, '__dataclass_fields__'):
                    depth = self._expr_depth(val)
                    if depth > self.max_depth:
                        self._report(val, depth, diagnostics)

            # Recurse into child blocks
            for child_attr in ('body', 'then_body', 'else_body'):
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._check_block(child, diagnostics)

        if block.trailing_expr:
            depth = self._expr_depth(block.trailing_expr)
            if depth > self.max_depth:
                self._report(block.trailing_expr, depth, diagnostics)

    def _expr_depth(self, expr, _depth: int = 0) -> int:
        """Compute the maximum nesting depth of an expression tree.

        ``a + (b + (c + d))`` has a depth of 3 (the innermost ``c + d``
        is at nesting level 3 counting from the outermost operator).
        """
        if not hasattr(expr, '__dataclass_fields__'):
            return _depth

        # Fields known to hold child expressions directly
        _direct_fields = frozenset({
            "left", "right", "operand", "func", "object", "index",
            "condition", "then_expr", "else_expr", "expr",
        })

        max_child_depth = _depth

        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name, None)
            if val is None:
                continue
            # Treat list children (e.g. FuncCall.args, MatchExpr.arms)
            # as one nesting level deeper
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        # Arms, parts, entries, chain links count +1
                        child_depth = self._expr_depth(item, _depth + 1)
                        if child_depth > max_child_depth:
                            max_child_depth = child_depth
            elif hasattr(val, '__dataclass_fields__'):
                # Direct child expression — count +1
                child_depth = self._expr_depth(val, _depth + 1)
                if child_depth > max_child_depth:
                    max_child_depth = child_depth

        return max_child_depth

    def _report(self, expr, depth: int, diagnostics: list[Diagnostic]) -> None:
        span = expr.span if expr.span else None
        line = span.line if span else 1
        col = span.col if span else 1
        loc = SourceLocation(
            file=self._filename(self._ctx),
            line=line, column=col,
            span=(line, col, span.end_line if span else 1, span.end_col if span else 1),
        )
        diagnostics.append(Diagnostic(
            code=self.code, severity=self.severity,
            category=self.category,
            message=self.message.format(max_depth=self.max_depth, depth=depth),
            location=loc, help=self.help,
        ))
