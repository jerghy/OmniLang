"""F0107: Unnecessary clone in Rust target.

Detects .clone() calls where the original value is never used afterward,
meaning a simple move would suffice. In Rust target, unnecessary clones
waste CPU and memory and often indicate a misunderstanding of ownership.

Example:
    let x = compute();
    let y = x.clone();   // unnecessary -- x is never used again
    // 'let y = x;' would suffice (move)

Limitation: only reports when the cloned expression is a simple variable
reference. Complex expressions as clone source are not tracked.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import MethodCall, VarRef, Block


class F0107Rule(AnalyzeRule):
    code = "F0107"
    name = "unnecessary_clone"
    severity = Severity.PERF
    category = "performance"
    message = "Unnecessary `.clone()` on `{name}` -- value is never used afterward"
    help = (
        "The cloned variable is never referenced after this point. "
        "Use a move (`let y = x;`) instead of `.clone()` to avoid "
        "the allocation overhead."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._check_block_sequential(func.body)

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

    # -- Sequential block analysis --------------------------------------------

    def _check_block_sequential(self, block):
        """Process statements in order to detect unnecessary clones."""
        stmts = list(getattr(block, "stmts", []))
        trailing = block.trailing_expr

        for i, stmt in enumerate(stmts):
            self._check_clones_in_stmt(stmt, stmts[i + 1:], trailing)

            # Recurse into child blocks (to find nested clone calls)
            self._recurse_into_child_blocks(stmt)

    def _recurse_into_child_blocks(self, stmt):
        """Recurse into child blocks of a statement to continue scanning."""
        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                self._check_block_sequential(child)

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block):
                    self._check_block_sequential(elif_body)

    # -- Clone detection -------------------------------------------------------

    def _check_clones_in_stmt(self, stmt, remaining_stmts, trailing_expr):
        """Find all .clone() calls in a statement and check if they are necessary."""
        clones = self._find_clone_calls_in_stmt(stmt)

        for clone_expr, cloned_var in clones:
            if not self._is_var_used_after(
                cloned_var, remaining_stmts, trailing_expr,
            ):
                loc = self._span_loc(getattr(clone_expr, "span", None))
                self._diags.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(name=cloned_var),
                    location=loc,
                    help=self.help,
                ))

    def _find_clone_calls_in_stmt(self, stmt):
        """Return list of (MethodCall, cloned_var_name) for .clone() calls
        where the object is a simple variable reference."""
        results: list[tuple] = []
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None:
                results.extend(self._find_clone_calls_in_expr(val))
        return results

    def _find_clone_calls_in_expr(self, expr):
        """Recursively find .clone() calls in an expression tree."""
        results: list[tuple] = []
        if isinstance(expr, MethodCall) and expr.method == "clone":
            if isinstance(expr.object, VarRef):
                results.append((expr, expr.object.name))

        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        results.extend(self._find_clone_calls_in_expr(item))
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                results.extend(self._find_clone_calls_in_expr(val))

        return results

    # -- Variable-usage tracking -----------------------------------------------

    def _is_var_used_after(
        self, var_name: str,
        remaining_stmts: list,
        trailing_expr,
    ) -> bool:
        """Check if var_name is referenced anywhere in the remaining code."""
        for stmt in remaining_stmts:
            if self._stmt_references_var(stmt, var_name):
                return True
        if trailing_expr is not None and self._expr_references_var(trailing_expr, var_name):
            return True
        return False

    def _stmt_references_var(self, stmt, var_name: str) -> bool:
        """Check if a statement or any of its nested blocks references the variable."""
        for attr in ("init", "value", "condition", "iterable", "arg", "expr"):
            val = getattr(stmt, attr, None)
            if val is not None and self._expr_references_var(val, var_name):
                return True

        for child_attr in ("body", "then_body", "else_body"):
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                if self._block_references_var(child, var_name):
                    return True

        if hasattr(stmt, "else_ifs"):
            for _, elif_body in getattr(stmt, "else_ifs", []):
                if isinstance(elif_body, Block) and self._block_references_var(elif_body, var_name):
                    return True

        return False

    def _block_references_var(self, block, var_name: str) -> bool:
        """Check if a block (statements + trailing expr) references the variable."""
        for s in getattr(block, "stmts", []):
            if self._stmt_references_var(s, var_name):
                return True
        if block.trailing_expr and self._expr_references_var(block.trailing_expr, var_name):
            return True
        return False

    def _expr_references_var(self, expr, var_name: str) -> bool:
        """Check if an expression tree references the given variable."""
        if isinstance(expr, VarRef) and expr.name == var_name:
            return True
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__") and self._expr_references_var(item, var_name):
                        return True
            elif hasattr(val, "__dataclass_fields__") and val is not None:
                if self._expr_references_var(val, var_name):
                    return True
        return False
