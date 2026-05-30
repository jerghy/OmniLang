"""E0205: Blocking call in async function.

Detects calls to known blocking functions (I/O operations, synchronous
sleeps, CPU-heavy computations, etc.) inside `async fn` bodies.  Blocking
the event-loop thread defeats the purpose of async code and can lead to
starvation of other tasks, especially in single-threaded targets like
Python asyncio and JavaScript.

Targets affected:
  - JS:              Blocks the entire event loop.
  - Python asyncio:  Blocks the event loop until the call returns.
  - Dart:            Blocks the isolate.
  - Rust (tokio):    Blocks the current task thread (less severe).

Fix: Wrap the call in `spawn_blocking`, use `await`-based alternatives,
     or move the work to a dedicated thread pool.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

from omnilang.ast_nodes import (
    FuncCall,
    MethodCall,
    VarRef,
    Block,
    FunctionDef,
)


# Known blocking function names (case-insensitive matching).
_BLOCKING_FUNCS = frozenset({
    "sleep",
    "time_sleep",
    "thread_sleep",
    "delay_sync",
    "read_file_sync",
    "write_file_sync",
    "read_file",
    "write_file",
    "fs_read_sync",
    "fs_write_sync",
    "fs_read",
    "fs_write",
    "connect_sync",
    "accept_sync",
    "recv_sync",
    "send_sync",
    "stdin_read_line",
    "stdin_read",
    "http_get_sync",
    "http_post_sync",
    "wait",
    "join_thread",
    "lock_acquire",
    "mutex_lock",
    "condvar_wait",
    "barrier_wait",
    "read_dir_sync",
    "remove_file_sync",
    "create_dir_sync",
    "copy_file_sync",
})

# Known blocking method names when called on common receiver types.
_BLOCKING_METHODS = frozenset({
    "read",
    "write",
    "read_line",
    "read_to_string",
    "flush",
    "recv",
    "send",
    "join",
    "lock",
    "unwrap",
    "expect",
    "result",
})


class E0205Rule(AnalyzeRule):
    code = "E0205"
    name = "blocking_in_async"
    severity = Severity.ERROR
    category = "async"
    message = "Blocking call `{name}` detected inside `async fn {func}`"
    help = (
        "Use await-based alternatives or wrap the call in "
        "`spawn_blocking()` / `asyncio.to_thread()` to avoid "
        "blocking the event loop."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.is_async and func.body:
                self._check_block_async(func.body, func.name, diagnostics)

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

    def _check_block_async(self, block: Block, func_name: str, diagnostics: list):
        for stmt in block.stmts:
            self._check_stmt_async(stmt, func_name, diagnostics)
        if block.trailing_expr:
            self._check_expr_async(block.trailing_expr, func_name, diagnostics)

    def _check_stmt_async(self, stmt, func_name: str, diagnostics: list):
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None:
                self._check_expr_async(val, func_name, diagnostics)

        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                self._check_block_async(child, func_name, diagnostics)

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    self._check_block_async(elif_body, func_name, diagnostics)

    def _check_expr_async(self, expr, func_name: str, diagnostics: list):
        if isinstance(expr, FuncCall):
            if isinstance(expr.func, VarRef):
                call_name = expr.func.name.lower()
                if call_name in _BLOCKING_FUNCS:
                    loc = self._span_loc(expr.span)
                    diagnostics.append(Diagnostic(
                        code=self.code,
                        severity=self.severity,
                        category=self.category,
                        message=self.message.format(
                            name=expr.func.name,
                            func=func_name,
                        ),
                        location=loc,
                        help=self.help,
                    ))

        if isinstance(expr, MethodCall):
            method_name = expr.method.lower()
            if method_name in _BLOCKING_METHODS:
                loc = self._span_loc(expr.span)
                diagnostics.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(
                        name=f"{expr.method}()",
                        func=func_name,
                    ),
                    location=loc,
                    help=self.help,
                ))

        # Recurse into sub-expressions
        for field_name in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._check_expr_async(item, func_name, diagnostics)
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                self._check_expr_async(val, func_name, diagnostics)
