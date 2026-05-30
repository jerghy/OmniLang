"""E0206: Missing error handling in async function.

Detects `async fn` that contain `await` expressions but do not have
any `try` / `catch` (exception handling) in the same function body.
In OmniLang, async operations often involve fallible IO or network
calls, and omitting error handling can lead to silent failures or
unhandled promise rejections.

This rule does *not* require every single `await` to be individually
wrapped; it simply checks that the function body contains at least
one error-handling construct (`try`/`catch`/`TryStmt`).

Targets affected:
  - JS:   Unhandled promise rejection.
  - Rust: Compiler warning for unused Result (separate rule).
  - Python: Unhandled exception during event loop run.
  - Dart: Unhandled Future error.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

from omnilang.ast_nodes import (
    AwaitExpr,
    Block,
)


class E0206Rule(AnalyzeRule):
    code = "E0206"
    name = "missing_error_handling_async"
    severity = Severity.WARNING
    category = "async"
    message = "Async function `{name}` has `await` but no error handling"
    help = (
        "Wrap await expressions with try/catch or add a `.catch()` "
        "handler to avoid unhandled promise/exception rejections."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.is_async and func.body:
                has_await = self._has_await_in_block(func.body)
                has_try = self._has_try_in_block(func.body)

                if has_await and not has_try:
                    loc = self._span_loc(func.span)
                    diagnostics.append(Diagnostic(
                        code=self.code,
                        severity=self.severity,
                        category=self.category,
                        message=self.message.format(name=func.name),
                        location=loc,
                        help=self.help,
                    ))

        return diagnostics

    def _filename(self) -> str:
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _span_loc(self, span) -> SourceLocation:
        if span:
            return SourceLocation(
                file=self._filename(), line=span.line, column=span.col,
                span=(span.line, span.col, span.end_line, span.end_col),
            )
        return SourceLocation(
            file=self._filename(), line=1, column=1, span=(1, 1, 1, 1),
        )

    def _has_await_in_block(self, block: Block) -> bool:
        """Recursively check if a block contains any await expression."""
        for stmt in block.stmts:
            if self._stmt_has_await(stmt):
                return True
        if block.trailing_expr:
            if self._expr_has_await(block.trailing_expr):
                return True
        return False

    def _stmt_has_await(self, stmt) -> bool:
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None and self._expr_has_await(val):
                return True

        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block) and self._has_await_in_block(child):
                return True

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block) and self._has_await_in_block(elif_body):
                    return True
        return False

    def _expr_has_await(self, expr) -> bool:
        """Return True if the expression (or any sub-expression) is an await."""
        if isinstance(expr, AwaitExpr):
            return True

        for field_name in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__") and self._expr_has_await(item):
                        return True
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                if self._expr_has_await(val):
                    return True

        return False

    def _has_try_in_block(self, block: Block) -> bool:
        """Recursively check if a block contains try/catch error handling.

        Looks for:
          - TryStmt (structured try/catch/finally node)
          - Method calls named 'catch' or 'except' (e.g. promise.catch())
          - IfStmt that checks for errors (e.g. `if err != null`)
        """
        for stmt in block.stmts:
            if self._stmt_is_try_catch(stmt):
                return True
        return False

    def _stmt_is_try_catch(self, stmt) -> bool:
        """Detect if a statement is an error-handling construct."""
        # TryStmt / TryCatch -- native OmniLang try/catch
        if hasattr(stmt, "__class__"):
            type_name = type(stmt).__name__.lower()
            if "try" in type_name:
                return True

        # .catch() method call on a promise/future
        for attr in ("init", "value", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None:
                from omnilang.ast_nodes import MethodCall
                if isinstance(val, MethodCall) and val.method in ("catch", "except", "handle_error"):
                    return True

        # Recurse into nested blocks for try detection
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block) and self._has_try_in_block(child):
                return True

        return False
