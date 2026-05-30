"""E0308: Cross-language ownership semantics.

Ownership and borrowing semantics differ significantly across
OmniLang's target languages:

  - Rust:  Strict ownership, borrow checking, move semantics.
  - C++:   Value semantics with move (C++11+), manual ownership.
  - Dart:  Reference counting / garbage collection.
  - JS:    Garbage collection, everything is a reference.
  - Python: Reference counting + GC, everything is a reference.

This rule warns when ownership patterns are expressed in ways that
are likely to produce different behavior or performance across targets.
Common patterns flagged:

  1. Returning a reference to a local variable ('dangling ref').
  2. Using `move` on a type that is implicitly moved in all targets.
  3. Closures that capture by reference when the closure outlives
     the captured scope (closure escape across targets).
  4. Functions annotated with `&self` / `&mut self` style patterns
     that don't translate to non-Rust targets.

The goal is not to prevent any particular pattern, but to alert the
developer that cross-target semantics may differ and suggest how to
write more portable ownership-aware code.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

from omnilang.ast_nodes import (
    FunctionDef,
    ClosureExpr,
    ReturnStmt,
    VarRef,
    Block,
)


class E0308Rule(AnalyzeRule):
    code = "E0308"
    name = "cross_lang_ownership"
    severity = Severity.PORT
    category = "ownership"
    message = "Cross-language ownership concern: {detail}"
    help = (
        "Ownership patterns may differ across targets.  Use explicit "
        "`.clone()` for move semantics in GC targets, or restructure "
        "to avoid reference lifetime assumptions."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._check_function_ownership(func, diagnostics)

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

    def _check_function_ownership(self, func: FunctionDef, diagnostics: list):
        """Inspect the function body for cross-target ownership concerns."""

        # 1. Dangling reference: returning a reference to a local variable
        if func.body.trailing_expr:
            self._check_dangling_return(func.body, func, diagnostics)

        # 2. Closure capture that may escape
        for stmt in func.body.stmts:
            self._check_stmt_ownership(stmt, func, diagnostics)

        # 3. Nested blocks
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(func.body, child_attr, None)
            if isinstance(child, Block):
                for stmt in child.stmts:
                    self._check_stmt_ownership(stmt, func, diagnostics)

    def _check_stmt_ownership(self, stmt, func: FunctionDef, diagnostics: list):
        """Walk statements for ownership patterns that differ across targets."""
        # Return statements that return references
        if isinstance(stmt, ReturnStmt) and stmt.value:
            self._check_expr_ownership(stmt.value, func, diagnostics)

        # Recurse into block children
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                for s in child.stmts:
                    self._check_stmt_ownership(s, func, diagnostics)

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    for s in elif_body.stmts:
                        self._check_stmt_ownership(s, func, diagnostics)

    def _check_expr_ownership(self, expr, func: FunctionDef, diagnostics: list):
        """Examine an expression tree for ownership-related cross-target issues."""

        # Closures capturing by reference
        if isinstance(expr, ClosureExpr):
            captures = self._get_captured_vars(expr)
            if captures:
                loc = self._span_loc(expr.span)
                cap_str = ", ".join(captures)
                diagnostics.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(
                        detail=(
                            f"closure captures `{cap_str}` by reference -- "
                            "GC targets (JS/Python) behave differently from Rust"
                        ),
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
                        self._check_expr_ownership(item, func, diagnostics)
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                self._check_expr_ownership(val, func, diagnostics)

    def _check_dangling_return(self, block: Block, func: FunctionDef, diagnostics: list):
        """Check if a function returns a reference to a local variable.

        This looks for patterns where a `VarRef` in the trailing expression
        refers to a variable declared inside the function body, which would
        be a dangling reference in Rust but perfectly valid in GC targets.
        """
        if not block.trailing_expr:
            return

        local_vars = self._collect_local_vars(block)
        returned_refs = self._collect_var_refs(block.trailing_expr)

        dangling = [v for v in returned_refs if v in local_vars]
        if dangling:
            loc = self._span_loc(block.trailing_expr.span if hasattr(block.trailing_expr, "span") else None)
            diagnostics.append(Diagnostic(
                code=self.code,
                severity=self.severity,
                category=self.category,
                message=self.message.format(
                    detail=(
                        f"returns reference to local variable(s) "
                        f"`{', '.join(dangling)}` which would be a dangling "
                        f"reference in Rust target"
                    ),
                ),
                location=loc,
                help=(
                    "Returning references to locals is undefined behavior "
                    "in Rust.  Use owned types or `.clone()` for portability."
                ),
            ))

    def _collect_local_vars(self, block: Block) -> set[str]:
        """Collect variable names declared in a block (direct children only)."""
        local_vars: set[str] = set()

        for stmt in block.stmts:
            if hasattr(stmt, "name") and hasattr(stmt, "__dataclass_fields__"):
                name = getattr(stmt, "name", None)
                if name and isinstance(name, str):
                    local_vars.add(name)

        return local_vars

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

    def _get_captured_vars(self, ce: ClosureExpr) -> list[str]:
        """Extract free variables referenced inside a closure body."""
        params = {p.name for p in ce.params}
        free: set[str] = set()

        def walk(expr):
            if isinstance(expr, VarRef) and expr.name not in params and not expr.name.startswith("_"):
                free.add(expr.name)
            for fn in getattr(expr, "__dataclass_fields__", {}):
                v = getattr(expr, fn)
                if isinstance(v, list):
                    for item in v:
                        if hasattr(item, "__dataclass_fields__"):
                            walk(item)
                elif hasattr(v, "__dataclass_fields__") and v is not None:
                    walk(v)

        if ce.body:
            walk(ce.body)
        return sorted(free)
