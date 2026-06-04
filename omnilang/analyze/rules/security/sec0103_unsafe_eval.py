"""SEC0103: Unsafe eval/exec usage.

Warns about calls to dynamic code execution functions such as ``eval``,
``exec``, ``Function``, ``setTimeout`` / ``setInterval`` with a string
argument, and similar patterns.  Dynamic code execution can lead to
arbitrary code injection and is almost always avoidable.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    FuncCall, MethodCall, VarRef, Literal, TemplateString, Block,
)


# Functions / methods known to perform dynamic code evaluation.
# Lower-case set — comparisons are case-insensitive.
_EVAL_LIKE_NAMES = frozenset({
    # Python / general
    "eval", "exec", "compile",
    # JavaScript
    "settimeout", "setinterval",  # when called with a string
})


class SEC0103Rule(AnalyzeRule):
    code = "SEC0103"
    name = "unsafe_eval"
    severity = Severity.SEC
    category = "security"
    message = "Use of `{name}` with a string argument enables arbitrary code execution"
    help = "Replace dynamic code execution with a safer alternative such as a lookup table or a sandboxed parser."

    _ctx: AnalyzeContext | None = None

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
            # Check statements that hold expressions
            for attr in ('init', 'value', 'condition', 'iterable', 'arg', 'expr'):
                val = getattr(stmt, attr, None)
                if val is not None and hasattr(val, '__dataclass_fields__'):
                    self._check_expr(val, diagnostics)

            for child_attr in ('body', 'then_body', 'else_body'):
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._check_block(child, diagnostics)

        if block.trailing_expr:
            self._check_expr(block.trailing_expr, diagnostics)

    def _check_expr(self, expr, diagnostics: list[Diagnostic]) -> None:
        """Recursively walk an expression tree looking for eval-like calls."""
        if isinstance(expr, FuncCall):
            self._check_func_call(expr, diagnostics)
        elif isinstance(expr, MethodCall):
            self._check_method_call(expr, diagnostics)

        # Recurse into sub-expressions
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name, None)
            if val is None:
                continue
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, diagnostics)
            elif hasattr(val, '__dataclass_fields__'):
                self._check_expr(val, diagnostics)

    def _check_func_call(
        self, call: FuncCall, diagnostics: list[Diagnostic],
    ) -> None:
        """Check a FuncCall for eval-like patterns."""
        fn_name = self._get_call_name(call.func)
        if fn_name is None:
            return

        if fn_name not in _EVAL_LIKE_NAMES:
            return

        # Check: at least one argument is a string literal or template
        if not self._has_string_arg(call.args):
            return

        self._report(call, fn_name, diagnostics)

    def _check_method_call(
        self, call: MethodCall, diagnostics: list[Diagnostic],
    ) -> None:
        """Check a MethodCall — e.g. ``obj.eval(...)``."""
        if call.method.lower() not in _EVAL_LIKE_NAMES:
            return
        if not self._has_string_arg(call.args):
            return
        self._report(call, call.method, diagnostics)

    @staticmethod
    def _get_call_name(func_expr) -> str | None:
        """Extract the function name from a callable expression."""
        if isinstance(func_expr, VarRef):
            return func_expr.name.lower()
        # Could be a dotted name resolved via MethodCall later
        return None

    @staticmethod
    def _has_string_arg(args: list) -> bool:
        """Check whether at least one argument is a string literal or
        a string-valued template."""

        for arg in args:
            if isinstance(arg, Literal) and arg.kind in ("String", "str", "char"):
                return True
            if isinstance(arg, TemplateString):
                # Templates with only literal parts are still string values
                return True
        return False

    def _report(self, node, name: str, diagnostics: list[Diagnostic]) -> None:
        span = node.span if node.span else None
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
            message=self.message.format(name=name),
            location=loc,
            help=self.help,
        ))
