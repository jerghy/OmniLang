"""F0106: Range check elision hint.

Suggests bounds checks that could be eliminated when the compiler or
runtime can prove an index is within bounds. This hints the user to
add explicit range constraints so the generated code (especially in
Rust and Dart) can skip bounds-checking overhead.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    IndexGet, VarRef, Literal, BinaryOp, ForInStmt,
    WhileStmt, VarDecl,
)


class F0106Rule(AnalyzeRule):
    code = "F0106"
    name = "range_check_elision"
    severity = Severity.PERF
    category = "performance"
    message = "Bounds check on `{desc}` could be elided with a range hint"
    help = "Add an explicit range constraint (e.g., `0..len` loop) or use `.get_unchecked()` to bypass unnecessary bounds checks."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._walk_block(func.body)

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
        return SourceLocation(file=self._filename(), line=1, column=1, span=(1, 1, 1, 1))

    def _walk_block(self, block):
        for stmt in getattr(block, "stmts", []):
            for attr in ["init", "value", "condition", "iterable", "arg", "expr"]:
                val = getattr(stmt, attr, None)
                if val:
                    self._walk_expr(val)
            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child:
                    self._walk_block(child)
            if hasattr(stmt, "else_ifs"):
                for _, elif_body in getattr(stmt, "else_ifs", []):
                    if elif_body:
                        self._walk_block(elif_body)

        if block.trailing_expr:
            self._walk_expr(block.trailing_expr)

    def _walk_expr(self, expr):
        if isinstance(expr, IndexGet):
            self._check_index_bounds(expr)

        # Recurse into child expressions
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)

    def _check_index_bounds(self, index_get: IndexGet):
        """Analyze an index expression for elision hints."""
        index = index_get.index
        obj = index_get.object

        # Case 1: index is a loop variable iterating 0..len — hint is already safe
        # (no diagnostic needed — the bounds check can be elided naturally)

        # Case 2: index is a compile-time constant
        if isinstance(index, Literal) and isinstance(index.value, int):
            loc = self._span_loc(getattr(index_get, "span", None))
            self._diags.append(Diagnostic(
                code=self.code, severity=self.severity,
                category=self.category,
                message=self.message.format(desc=f"constant index {index.value}"),
                location=loc,
                help="For constant small indices, the bounds check is known at compile time — "
                     "consider using `.get_unchecked()` for hot paths.",
            ))

        # Case 3: index is a variable not obviously bounded
        if isinstance(index, VarRef):
            obj_desc = ""
            if isinstance(obj, VarRef):
                obj_desc = f"{obj.name}[{index.name}]"
            elif hasattr(obj, "__dataclass_fields__"):
                obj_desc = f"<expr>[{index.name}]"
            else:
                obj_desc = f"<collection>[{index.name}]"

            loc = self._span_loc(getattr(index_get, "span", None))
            self._diags.append(Diagnostic(
                code=self.code, severity=self.severity,
                category=self.category,
                message=self.message.format(desc=obj_desc),
                location=loc,
                help="If the index is guaranteed to be in-bounds (e.g., loop from 0 to len), "
                     "add an explicit range assertion or use `.get_unchecked()` to skip the bounds check.",
            ))

        # Case 4: index is computed from an expression that may overflow
        if isinstance(index, BinaryOp) and index.op in ("+", "-", "*"):
            loc = self._span_loc(getattr(index_get, "span", None))
            self._diags.append(Diagnostic(
                code=self.code, severity=self.severity,
                category=self.category,
                message=self.message.format(desc="computed index expression"),
                location=loc,
                help="A bounds check is required for computed indices. "
                     "If the computation cannot overflow, prefix with `unchecked` or use `.get_unchecked()`.",
            ))
