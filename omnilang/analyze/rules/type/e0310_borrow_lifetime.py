"""E0310: Borrow lifetime mismatch (Rust target).

Warns when a reference may outlive the value it refers to, which would
cause a compile error in Rust target. Checks for variables declared in
nested (child) blocks that are referenced in outer scopes where the
inner variable has already been dropped.

Example of flagged pattern:
    let r;
    {
        let items = [1, 2, 3];
        r = &items[0];   // borrow from `items`
    }
    print(r);            // ERROR: items is dropped, r may dangle

The analysis is block-scoped: after a nested block is processed, its
local variables are marked as "dropped". Any subsequent reference to
those variables in the enclosing scope is flagged as a potential
lifetime mismatch.  Shadowing is handled -- a variable re-declared in
the enclosing scope is not flagged.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, VarRef, Block


class E0310Rule(AnalyzeRule):
    code = "E0310"
    name = "borrow_lifetime"
    severity = Severity.ERROR
    category = "type"
    message = "Reference to `{name}` may outlive the referenced value"
    help = (
        "The variable `{name}` is declared in a nested scope and may be "
        "dropped before this reference is used. Consider extending its "
        "lifetime, moving the declaration up, or restructuring the code "
        "to avoid the dangling reference."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, self._diags)

        return self._diags

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

    # -- Block-level lifetime analysis ----------------------------------------

    def _check_block(self, block, diagnostics) -> set:
        """Walk a block and check for lifetime issues.

        For each child block we recurse into, its local variable names are
        added to *closed_vars* -- variables that have gone out of scope.
        Every reference in subsequent sibling code is checked against
        closed_vars.

        Returns the set of variable names declared directly in THIS block
        (i.e. that go out of scope when this block ends).
        """
        local_vars: set[str] = set()
        closed_vars: set[str] = set()  # vars from child blocks that have ended

        for stmt in getattr(block, "stmts", []):
            # Check expressions in this statement against vars from
            # already-closed child blocks
            self._check_stmt_refs(stmt, closed_vars, local_vars, diagnostics)

            # Record variable declarations in this block
            if isinstance(stmt, VarDecl):
                local_vars.add(stmt.name)

            # Recurse into child blocks -- their locals become "closed"
            self._collect_from_child_blocks(stmt, closed_vars, diagnostics)

        # Check the trailing expression against closed vars from child blocks
        if block.trailing_expr:
            self._check_expr_refs(
                block.trailing_expr, closed_vars, local_vars, diagnostics,
            )

        return local_vars

    def _collect_from_child_blocks(self, stmt, closed_vars, diagnostics):
        """Recurse into nested blocks and add their locals to *closed_vars*."""
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                child_locals = self._check_block(child, diagnostics)
                closed_vars |= child_locals

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    child_locals = self._check_block(elif_body, diagnostics)
                    closed_vars |= child_locals

    # -- Reference checking ---------------------------------------------------

    def _check_stmt_refs(self, stmt, closed_vars, local_vars, diagnostics):
        """Check all top-level expressions in a statement for references
        to closed variables."""
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None:
                self._check_expr_refs(val, closed_vars, local_vars, diagnostics)

    def _check_expr_refs(self, expr, closed_vars, local_vars, diagnostics):
        """Recursively check an expression for VarRef nodes pointing to
        closed variables, respecting current-block shadowing."""
        if isinstance(expr, VarRef):
            if expr.name in closed_vars and expr.name not in local_vars:
                loc = self._span_loc(expr.span)
                diagnostics.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(name=expr.name),
                    location=loc,
                    help=self.help,
                ))

        # Recurse into sub-expressions
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._check_expr_refs(
                            item, closed_vars, local_vars, diagnostics,
                        )
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                self._check_expr_refs(
                    val, closed_vars, local_vars, diagnostics,
                )
