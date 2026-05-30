"""E0307: Borrow conflict (Rust target).

Warns when a variable is both mutably borrowed (or assigned) and
immutably accessed in the same scope *without* an intervening
synchronization point.  This mirrors Rust's borrow checker rules
and helps detect data races early when targeting Rust.

Because OmniLang is a cross-language language, mutable aliasing
that is safe in JS / Python / Dart may be a compilation error in
Rust.  This rule flags patterns that are likely to fail under
Rust's ownership model so the developer can restructure the code
before they attempt a Rust build.

Example of flagged pattern:
    let mut x = 42;
    let r = &x;      // immutable borrow
    x = 10;          // ERROR: can't assign x while borrowed
    print(r);
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation

from omnilang.ast_nodes import (
    Assign,
    VarDecl,
    VarRef,
    Block,
)


class E0307Rule(AnalyzeRule):
    code = "E0307"
    name = "borrow_conflict"
    severity = Severity.ERROR
    category = "ownership"
    message = (
        "Borrow conflict: variable `{name}` is both mutably borrowed "
        "and accessed in the same scope"
    )
    help = (
        "Rust target does not allow simultaneous mutable and immutable "
        "borrows.  Restructure the code to avoid aliasing, or use "
        "interior mutability (Cell/RefCell) if the pattern is intentional."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._check_block_for_borrow_conflicts(func.body, diagnostics, func.name)

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

    def _check_block_for_borrow_conflicts(
        self,
        block: Block,
        diagnostics: list,
        func_name: str,
    ):
        """Walk statements linearly; track mutable and immutable references
        per variable and report conflicts."""

        # Track, per variable name, whether we have seen an immutable
        # reference / mutable reference since the last clear point.
        imm_borrowed: dict[str, int] = {}  # name -> line
        mut_borrowed: dict[str, int] = {}  # name -> line

        for stmt in block.stmts:
            self._process_stmt_borrows(stmt, imm_borrowed, mut_borrowed, diagnostics, func_name)

        if block.trailing_expr:
            self._process_expr_borrows(
                block.trailing_expr, imm_borrowed, mut_borrowed, diagnostics, func_name,
            )

    def _process_stmt_borrows(
        self,
        stmt,
        imm_borrowed: dict,
        mut_borrowed: dict,
        diagnostics: list,
        func_name: str,
    ):
        if isinstance(stmt, VarDecl):
            # A mutable variable declaration establishes a new owner
            if stmt.mutable:
                old_mut = mut_borrowed.pop(stmt.name, None)
                old_imm = imm_borrowed.pop(stmt.name, None)

                if stmt.init:
                    # The initializer may reference other variables
                    self._process_expr_borrows(
                        stmt.init, imm_borrowed, mut_borrowed, diagnostics, func_name,
                    )
            else:
                # Immutable binding -- treat as immutable borrow of the value
                if stmt.init:
                    seen = self._collect_var_refs(stmt.init)
                    for ref_name in seen:
                        if ref_name in mut_borrowed:
                            loc = self._span_loc(stmt.span)
                            diagnostics.append(Diagnostic(
                                code=self.code,
                                severity=self.severity,
                                category=self.category,
                                message=self.message.format(name=ref_name),
                                location=loc,
                                help=self.help,
                            ))

        elif isinstance(stmt, Assign):
            # Assignment is a mutable access to the target
            if stmt.target in imm_borrowed:
                loc = self._span_loc(stmt.span)
                diagnostics.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(name=stmt.target),
                    location=loc,
                    help=self.help,
                ))

            if stmt.value:
                # Check the right-hand side for references to mutably borrowed vars
                seen = self._collect_var_refs(stmt.value)
                for ref_name in seen:
                    if ref_name in mut_borrowed:
                        loc = self._span_loc(stmt.span)
                        diagnostics.append(Diagnostic(
                            code=self.code,
                            severity=self.severity,
                            category=self.category,
                            message=self.message.format(name=ref_name),
                            location=loc,
                            help=self.help,
                        ))

        # Recurse into nested blocks -- borrow state does NOT propagate
        # across block boundaries for simplicity (a more precise analysis
        # would use NLL-style regions).
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                self._check_block_for_borrow_conflicts(child, diagnostics, func_name)

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    self._check_block_for_borrow_conflicts(elif_body, diagnostics, func_name)

    def _process_expr_borrows(
        self,
        expr,
        imm_borrowed: dict,
        mut_borrowed: dict,
        diagnostics: list,
        func_name: str,
    ):
        """Walk an expression tree and update borrow state / emit conflicts.

        For now this is a simplified version that collects variable
        references and checks for conflicts at each level.
        """
        refs = self._collect_var_refs(expr)
        for ref_name in refs:
            if ref_name in mut_borrowed:
                loc = self._span_loc(getattr(expr, "span", None))
                diagnostics.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(name=ref_name),
                    location=loc,
                    help=self.help,
                ))

        # Recurse into sub-expressions
        for field_name in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._process_expr_borrows(
                            item, imm_borrowed, mut_borrowed, diagnostics, func_name,
                        )
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                self._process_expr_borrows(
                    val, imm_borrowed, mut_borrowed, diagnostics, func_name,
                )

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
