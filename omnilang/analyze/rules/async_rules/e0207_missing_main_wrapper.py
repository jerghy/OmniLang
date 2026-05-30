"""E0207: Async main without runtime wrapper.

Detects `async fn main()` that lacks a runtime wrapper call. In Rust target,
async fn requires a #[tokio::main] attribute or explicit Runtime::block_on()
call. In Python target, it needs asyncio.run(). Without a wrapper, the
program may fail to execute the async body.

Example of flagged code:
    async fn main() {
        // ... async code but no runtime wrapper
    }

Expected patterns that suppress the diagnostic:
    - asyncio.run(main())        # Python
    - tokio::main                # Rust (attribute)
    - Runtime::block_on(...)     # Rust (explicit)
    - executor.block_on(...)     # Generic
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import FunctionDef, FuncCall, VarRef, MethodCall, Block


# Wrapper-call method names that indicate a runtime is present
_WRAPPER_METHODS = {"run", "block_on", "run_until_complete"}

# Wrapper-call function names (direct VarRef names)
_WRAPPER_FUNC_NAMES = {
    "asyncio_run", "tokio_run", "block_on",
    "run_async", "start", "execute",
}


class E0207Rule(AnalyzeRule):
    code = "E0207"
    name = "missing_main_wrapper"
    severity = Severity.WARNING
    category = "async"
    message = "Async `main()` function has no runtime wrapper -- may not execute correctly"
    help = (
        "Wrap the async main call with a runtime executor. In Rust use "
        "#[tokio::main] or Runtime::block_on(); in Python use "
        "asyncio.run()."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.name == "main" and func.is_async:
                if not self._has_runtime_wrapper(func.body):
                    loc = self._span_loc(func.span)
                    diagnostics.append(Diagnostic(
                        code=self.code,
                        severity=self.severity,
                        category=self.category,
                        message=self.message,
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

    def _has_runtime_wrapper(self, body) -> bool:
        """Check if the function body contains any runtime wrapper call."""
        if not body:
            return False

        for stmt in getattr(body, "stmts", []):
            if self._stmt_has_wrapper(stmt):
                return True
            # Recurse into child blocks
            for child_attr in ("body", "then_body", "else_body"):
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block) and self._has_runtime_wrapper(child):
                    return True
            if hasattr(stmt, "else_ifs"):
                for _, elif_body in getattr(stmt, "else_ifs", []):
                    if isinstance(elif_body, Block) and self._has_runtime_wrapper(elif_body):
                        return True

        # Check trailing expression
        if body.trailing_expr and self._expr_is_wrapper(body.trailing_expr):
            return True

        return False

    def _stmt_has_wrapper(self, stmt) -> bool:
        """Check if any expression in a statement is a runtime wrapper call."""
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None and self._expr_is_wrapper(val):
                return True
        return False

    def _expr_is_wrapper(self, expr) -> bool:
        """Check if an expression represents a runtime wrapper call.

        Recognizes patterns like:
          - asyncio.run(...)         (FuncCall with dotted or plain name)
          - tokio::main              (via VarRef)
          - runtime.block_on(...)    (MethodCall)
          - executor.run(...)        (MethodCall)
        """
        # FuncCall: a function being called
        if isinstance(expr, FuncCall):
            func = expr.func
            if isinstance(func, VarRef) and func.name in _WRAPPER_FUNC_NAMES:
                return True
            # Handle dotted function names like "asyncio.run" as MethodCall
            # (already handled by MethodCall check below)

        # MethodCall: obj.run(...), obj.block_on(...)
        if isinstance(expr, MethodCall) and expr.method in _WRAPPER_METHODS:
            return True

        # Recurse into child expressions (e.g., nested in a binary op)
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__") and self._expr_is_wrapper(item):
                        return True
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                if self._expr_is_wrapper(val):
                    return True

        return False
