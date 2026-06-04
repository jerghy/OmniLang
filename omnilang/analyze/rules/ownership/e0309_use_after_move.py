"""E0309: Use after move (Rust target).

Detects when a variable is used after its value has been moved
(moved into a function call, returned, or reassigned) within the
same block.  This mirrors Rust's "use of moved value" error and
helps catch issues early before a Rust build.

The analysis is intra-procedural and works by:
  1. Tracking each variable's *ownership state* as we walk the AST.
  2. Recording a move when a variable is consumed:
       - Passed as a non-copy argument to a function call.
       - Returned from the function.
       - Assigned to another variable without `.clone()`.
  3. Flagging any subsequent `VarRef` to a moved variable.

Because OmniLang supports multiple targets, some moves that are
valid in JS/Python/Dart may not be valid in Rust.  This rule
targets Rust-specific move semantics.

Note: This is a best-effort intra-procedural analysis.  Precise
move tracking (e.g. partial moves, conditional moves) would require
a more sophisticated data-flow analysis.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

from omnilang.ast_nodes import (
    VarDecl,
    Assign,
    ReturnStmt,
    VarRef,
    Block,
    ExprStmt,
)


# Types that are *copy* types (like Rust's Copy trait) -- moving them
# does not invalidate the original.  This is a simplified built-in list.
_COPY_TYPES = frozenset({
    "i32", "i64", "f32", "f64", "bool", "char", "u8", "u16", "u32", "u64",
})


class E0309Rule(AnalyzeRule):
    code = "E0309"
    name = "use_after_move"
    severity = Severity.ERROR
    category = "ownership"
    message = "Use of moved value: `{name}` used after move"
    help = (
        "In Rust target, values are moved by default.  Use "
        "`.clone()` to preserve the original, or restructure "
        "the code to avoid using the value after the move."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._check_block_moves(func.body, set(), diagnostics)

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

    def _check_block_moves(
        self,
        block: Block,
        moved_vars: set[str],
        diagnostics: list,
    ):
        """Walk a block linearly tracking moved variables."""
        # Copy the set so callers' state is not mutated
        local_moved: set[str] = set(moved_vars)

        for stmt in block.stmts:
            self._check_stmt_moves(stmt, local_moved, diagnostics)

        if block.trailing_expr:
            self._check_expr_moves(block.trailing_expr, local_moved, diagnostics)

    def _check_stmt_moves(
        self,
        stmt,
        moved: set[str],
        diagnostics: list,
    ):
        if isinstance(stmt, VarDecl):
            # A variable declaration creates a fresh binding -- remove
            # it from the moved set if it was tracked (over-shadowing).
            moved.discard(stmt.name)

            if stmt.init:
                # The initializer may move other variables
                self._check_expr_moves(stmt.init, moved, diagnostics)

        elif isinstance(stmt, Assign):
            # Assignment to a non-copy type is a move of the RHS
            # (and also re-binds the LHS, so remove it from moved set).
            moved.discard(stmt.target)

            if stmt.value:
                rhs_refs = self._collect_var_refs(stmt.value)
                for ref_name in rhs_refs:
                    if ref_name in moved:
                        loc = self._span_loc(stmt.span)
                        diagnostics.append(Diagnostic(
                            code=self.code,
                            severity=self.severity,
                            category=self.category,
                            message=self.message.format(name=ref_name),
                            location=loc,
                            help=self.help,
                        ))

                # The RHS *moves* the referenced variables (unless copy-type)
                self._process_move_targets(stmt.value, moved)

        elif isinstance(stmt, ReturnStmt):
            if stmt.value:
                self._check_expr_moves(stmt.value, moved, diagnostics)
                # Returning moves all non-copy references
                self._process_move_targets(stmt.value, moved)

        elif isinstance(stmt, ExprStmt):
            # Expression statement -- check and record moves
            self._check_expr_moves(stmt.expr, moved, diagnostics)
            self._process_move_targets(stmt.expr, moved)

        else:
            # Generic handling: recurse into sub-blocks
            for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
                val = getattr(stmt, attr, None)
                if val is not None:
                    self._check_expr_moves(val, moved, diagnostics)

        # Recurse into nested blocks (with a *copy* of moved state,
        # since moves in one branch should not affect the other branch).
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                branch_moved = set(moved)
                self._check_block_moves(child, branch_moved, diagnostics)

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    branch_moved = set(moved)
                    self._check_block_moves(elif_body, branch_moved, diagnostics)

    def _check_expr_moves(
        self,
        expr,
        moved: set[str],
        diagnostics: list,
    ):
        """Recurse into expressions, checking VarRefs against moved set."""
        if isinstance(expr, VarRef):
            if expr.name in moved:
                loc = self._span_loc(expr.span)
                diagnostics.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(name=expr.name),
                    location=loc,
                    help=self.help,
                ))

        for field_name in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._check_expr_moves(item, moved, diagnostics)
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                self._check_expr_moves(val, moved, diagnostics)

    def _process_move_targets(self, expr, moved: set[str]):
        """Mark non-copy variables referenced in `expr` as moved.

        This is called after a consuming expression (function call
        argument, return value, assignment RHS).
        """
        refs = self._collect_var_refs(expr)
        for ref_name in refs:
            ty = self._get_var_type(ref_name)
            if ty not in _COPY_TYPES:
                moved.add(ref_name)

    def _collect_var_refs(self, expr) -> set[str]:
        """Return the set of variable names referenced in an expression."""
        names: set[str] = set()

        def _walk(e):
            if isinstance(e, VarRef):
                names.add(e.name)
            for fn in getattr(e, "__dataclass_fields__", {}):
                v = getattr(e, fn)
                if isinstance(v, list):
                    for item in v:
                        if hasattr(item, "__dataclass_fields__"):
                            _walk(item)
                elif hasattr(v, "__dataclass_fields__") and v is not None:
                    _walk(v)

        _walk(expr)
        return names

    def _get_var_type(self, name: str) -> str | None:
        """Look up the type of a variable from the type_map, if available."""
        if not hasattr(self._ctx, "type_map"):
            return None

        # Search through type_map entries matching variable declarations
        for key, ty in self._ctx.type_map.items():
            if isinstance(key, int):
                # key is an object id; we need to map name -> id
                pass

        # Fallback: check if name ends with known type pattern or
        # is in the type_map directly
        return self._ctx.type_map.get(name, None)
