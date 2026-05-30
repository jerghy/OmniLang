"""E0204: Await in tight loop without a yield point.

Detects `await` expressions inside `for` or `while` loop bodies that
do not contain a cooperative yield point (another `await`, `yield`,
or `task_delay` call).  A tight loop with only blocking awaits can
starve other tasks on the same executor, especially in single-threaded
targets like Python asyncio or JS.

To fix, insert a `task_delay(0)` or `await sleep(0)` at the top of
the loop body, or restructure the loop to include at least one yield
point per iteration.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

from omnilang.ast_nodes import (
    AwaitExpr,
    FuncCall,
    VarRef,
    ForInStmt,
    WhileStmt,
    Block,
)


# Set of function names that act as yield points (cooperative scheduling)
_YIELD_FUNCS = frozenset({
    "task_delay",
    "sleep",
    "yield",
    "asyncio_sleep",
    "next_tick",
    "set_timeout",
    "delay",
})

# Set of function names that are known *blocking* awaits which do NOT
# yield to the scheduler (e.g. waiting on a future that is immediately
# resolved or doing IO that hogs the event loop).
_BLOCKING_AWAIT_FUNCS = frozenset({
    "read_file",
    "write_file",
    "connect",
    "receive",
})


class E0204Rule(AnalyzeRule):
    code = "E0204"
    name = "await_in_tight_loop"
    severity = Severity.WARNING
    category = "async"
    message = "`await` inside a tight loop without a yield point may starve the event loop"
    help = (
        "Add a `task_delay(0)` or `await sleep(0)` at the top of the "
        "loop body to allow cooperative scheduling between iterations."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._check_block_for_loops(func.body, in_loop=False, yield_seen=False, diagnostics=diagnostics)

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

    def _check_block_for_loops(
        self,
        block: Block,
        in_loop: bool,
        yield_seen: bool,
        diagnostics: list,
    ):
        """Recurse through statements, tracking loop context and yield points."""
        for stmt in block.stmts:
            self._check_stmt_in_loop(stmt, in_loop, yield_seen, diagnostics)

        if block.trailing_expr:
            self._check_expr_in_loop(block.trailing_expr, in_loop, yield_seen, diagnostics)

    def _check_stmt_in_loop(
        self,
        stmt,
        in_loop: bool,
        yield_seen: bool,
        diagnostics: list,
    ):
        if isinstance(stmt, (ForInStmt, WhileStmt)):
            # Entering a loop -- track yield state separately
            loop_yield_seen = False
            body = stmt.body
            if body:
                for child_stmt in body.stmts:
                    self._check_stmt_in_loop(child_stmt, in_loop=True, yield_seen=loop_yield_seen, diagnostics=diagnostics)
                    loop_yield_seen = self._has_yield_point(child_stmt) or loop_yield_seen
                if body.trailing_expr:
                    self._check_expr_in_loop(body.trailing_expr, in_loop=True, yield_seen=loop_yield_seen, diagnostics=diagnostics)
                    loop_yield_seen = self._is_yield_expr(body.trailing_expr) or loop_yield_seen

            # Report if there are awaits in the body but no yield point
            if in_loop and not loop_yield_seen:
                self._report_loop_awaits(body, diagnostics)

            return

        # Recurse into nested blocks
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                self._check_block_for_loops(child, in_loop, yield_seen, diagnostics)

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    self._check_block_for_loops(elif_body, in_loop, yield_seen, diagnostics)

        # Check expressions within the statement
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None:
                self._check_expr_in_loop(val, in_loop, yield_seen, diagnostics)

    def _check_expr_in_loop(
        self,
        expr,
        in_loop: bool,
        yield_seen: bool,
        diagnostics: list,
    ):
        """Recurse into expressions looking for awaits in loop context."""
        if isinstance(expr, AwaitExpr) and in_loop and not yield_seen:
            loc = self._span_loc(expr.span)
            diagnostics.append(Diagnostic(
                code=self.code,
                severity=self.severity,
                category=self.category,
                message=self.message,
                location=loc,
                help=self.help,
            ))

        for field_name in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._check_expr_in_loop(item, in_loop, yield_seen, diagnostics)
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                self._check_expr_in_loop(val, in_loop, yield_seen, diagnostics)

    def _has_yield_point(self, stmt) -> bool:
        """Check if a statement contains a yield point."""
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None and self._is_yield_expr(val):
                return True
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                for s in child.stmts:
                    if self._has_yield_point(s):
                        return True
                if child.trailing_expr and self._is_yield_expr(child.trailing_expr):
                    return True
        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    for s in elif_body.stmts:
                        if self._has_yield_point(s):
                            return True
        return False

    def _is_yield_expr(self, expr) -> bool:
        """Return True if `expr` is a yield point (await or yield call)."""
        if isinstance(expr, AwaitExpr):
            return True
        if isinstance(expr, FuncCall):
            if isinstance(expr.func, VarRef) and expr.func.name in _YIELD_FUNCS:
                return True
        return False

    def _report_loop_awaits(self, body: Block, diagnostics: list):
        """Walk the loop body and report each await that lacks a yield."""
        for stmt in body.stmts:
            self._report_awaits_in_stmt(stmt, diagnostics)
        if body.trailing_expr:
            self._report_awaits_in_expr(body.trailing_expr, diagnostics)

    def _report_awaits_in_stmt(self, stmt, diagnostics: list):
        """Recursively report all awaits in a statement."""
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None:
                self._report_awaits_in_expr(val, diagnostics)
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                self._report_loop_awaits(child, diagnostics)
        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    self._report_loop_awaits(elif_body, diagnostics)

    def _report_awaits_in_expr(self, expr, diagnostics: list):
        if isinstance(expr, AwaitExpr):
            loc = self._span_loc(expr.span)
            diagnostics.append(Diagnostic(
                code=self.code,
                severity=self.severity,
                category=self.category,
                message=self.message,
                location=loc,
                help=self.help,
            ))
        for field_name in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._report_awaits_in_expr(item, diagnostics)
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                self._report_awaits_in_expr(val, diagnostics)
