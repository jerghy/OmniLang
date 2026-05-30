"""S0103: Dead code after return/break/continue.

Detects statements that are unreachable because they follow a return,
break, continue, or an unconditional exit (e.g., exit(), panic!()).
Unreachable code is misleading and should be removed.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    Block, ReturnStmt, BreakStmt, ContinueStmt, IfStmt, WhileStmt, ForInStmt,
    FuncCall, VarRef, ExprStmt, Stmt,
)


class S0103Rule(AnalyzeRule):
    code = "S0103"
    name = "dead_code"
    severity = Severity.WARNING
    category = "style"
    message = "Unreachable statement after `{kind}`"
    help = "Remove the dead code or move it before the `{kind}` statement."

    # Names of functions considered "never returns" (always exit/panic)
    _EXIT_FUNCS = frozenset({"exit", "panic", "unreachable", "todo", "unimplemented"})

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics, in_loop=False)
        self._ctx = None
        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(
        self, block: Block, diagnostics: list[Diagnostic],
        in_loop: bool,
    ) -> None:
        """Walk a block's sequential statements, marking dead code after
        any unconditional control-flow transfer."""
        dead = False
        for stmt in block.stmts:
            if dead:
                self._emit_dead(stmt, diagnostics)
                continue  # keep scanning so all subsequent dead stmts are flagged

            if self._is_unconditional_exit(stmt, in_loop):
                dead = True
                continue  # the exit statement itself is not dead

            # Recurse into child blocks (the exit doesn't affect sibling blocks)
            self._recurse_child_blocks(stmt, diagnostics, in_loop)

        # Trailing expression after an unconditional exit is also dead.
        if dead and block.trailing_expr:
            self._emit_dead_expr(block.trailing_expr, diagnostics)

    def _is_unconditional_exit(
        self, stmt: Stmt, in_loop: bool,
    ) -> bool:
        """Return True if *stmt* unconditionally transfers control away."""
        if isinstance(stmt, ReturnStmt):
            return True
        if in_loop and isinstance(stmt, (BreakStmt, ContinueStmt)):
            return True
        # Detect fn calls known to abort: exit(), panic(), unreachable(), etc.
        if isinstance(stmt, ExprStmt) and isinstance(stmt.expr, FuncCall):
            fn = stmt.expr.func
            if isinstance(fn, VarRef) and fn.name in self._EXIT_FUNCS:
                return True
        return False

    def _recurse_child_blocks(
        self, stmt: Stmt, diagnostics: list[Diagnostic],
        in_loop: bool,
    ) -> None:
        """Descend into control-flow structures that contain blocks."""
        if isinstance(stmt, IfStmt):
            self._check_block(stmt.then_body, diagnostics, in_loop)
            for _, elif_block in stmt.else_ifs:
                self._check_block(elif_block, diagnostics, in_loop)
            if stmt.else_body:
                self._check_block(stmt.else_body, diagnostics, in_loop)
        elif isinstance(stmt, WhileStmt):
            self._check_block(stmt.body, diagnostics, in_loop=True)
        elif isinstance(stmt, ForInStmt):
            self._check_block(stmt.body, diagnostics, in_loop=True)

    def _emit_dead(self, stmt: Stmt, diagnostics: list[Diagnostic]) -> None:
        span = stmt.span if stmt.span else None
        line = span.line if span else 1
        col = span.col if span else 1
        kind = type(stmt).__name__.replace("Stmt", "").lower()
        loc = SourceLocation(
            file=self._filename(self._get_ctx()),
            line=line, column=col,
            span=(line, col, span.end_line if span else 1, span.end_col if span else 1),
        )
        diagnostics.append(Diagnostic(
            code=self.code, severity=self.severity,
            category=self.category,
            message=self.message.format(kind=kind),
            location=loc, help=self.help.format(kind=kind),
        ))

    def _emit_dead_expr(self, expr, diagnostics: list[Diagnostic]) -> None:
        span = expr.span if expr.span else None
        line = span.line if span else 1
        col = span.col if span else 1
        loc = SourceLocation(
            file=self._filename(self._get_ctx()),
            line=line, column=col,
            span=(line, col, span.end_line if span else 1, span.end_col if span else 1),
        )
        diagnostics.append(Diagnostic(
            code=self.code, severity=self.severity,
            category=self.category,
            message=self.message.format(kind="trailing expression"),
            location=loc, help=self.help.format(kind="trailing expression"),
        ))

    # ── Context lookup helper ─────────────────────────────
    # Stored by the check() entry-point so _emit_dead can reach the ctx.
    _ctx: AnalyzeContext | None = None

    def _get_ctx(self) -> AnalyzeContext:
        # This is set by check(); in normal flow it's always present.
        return self._ctx
