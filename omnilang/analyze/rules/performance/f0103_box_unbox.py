"""F0103: Box/unbox overhead.

Warns about unnecessary boxing/unboxing that forces values onto the heap.
Common patterns: wrapping a value in Box<T> only to immediately dereference it,
or chaining .box()/.unbox() calls that cancel out.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, Assign, MethodCall, FuncCall, VarRef, UnaryOp


class F0103Rule(AnalyzeRule):
    code = "F0103"
    name = "box_unbox"
    severity = Severity.PERF
    category = "performance"
    message = "Unnecessary box/unbox overhead: value is wrapped then immediately unwrapped"
    help = "Avoid boxing values that don't need heap allocation. Use `&` references or stack allocation instead."

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
            # VarDecl with boxed init
            if isinstance(stmt, VarDecl) and stmt.init:
                if self._is_boxing_call(stmt.init) or self._is_boxed_type(stmt):
                    loc = self._span_loc(stmt.span)
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=f"Variable `{stmt.name}` is boxed but may not need heap allocation",
                        location=loc, help=self.help,
                    ))

            # Assign with boxing chain
            if isinstance(stmt, Assign) and stmt.value:
                self._walk_expr(stmt.value)

            for attr in ["init", "value", "condition", "iterable", "arg", "expr"]:
                val = getattr(stmt, attr, None)
                if val:
                    self._walk_expr(val)
            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child:
                    self._walk_block(child)

        if block.trailing_expr:
            self._walk_expr(block.trailing_expr)

    def _is_boxing_call(self, expr) -> bool:
        """Check if an expression is a boxing operation (box(), Box::new(), into_box())."""
        if isinstance(expr, FuncCall) and isinstance(expr.func, VarRef):
            if expr.func.name in ("box", "Box.new", "into_box", "Box::new"):
                return True
        if isinstance(expr, MethodCall) and expr.method in ("box", "into_box", "boxed"):
            return True
        return False

    def _is_unboxing_call(self, expr) -> bool:
        """Check if an expression is an unboxing operation (unbox(), *, deref())."""
        if isinstance(expr, FuncCall) and isinstance(expr.func, VarRef):
            if expr.func.name in ("unbox", "deref"):
                return True
        if isinstance(expr, MethodCall) and expr.method in ("unbox", "into_inner"):
            return True
        if isinstance(expr, UnaryOp) and expr.op == "*":
            return True
        return False

    def _is_boxed_type(self, decl: VarDecl) -> bool:
        """Check if the declared type is a Box type."""
        if decl.type_ann and decl.type_ann.name in ("Box", "box"):
            return True
        return False

    def _walk_expr(self, expr):
        # Detect boxing followed by unboxing pattern
        if isinstance(expr, MethodCall):
            if expr.method in ("unbox", "into_inner", "unwrapping"):
                if self._is_boxing_call(expr.object):
                    loc = self._span_loc(getattr(expr, "span", None))
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message="Value is boxed then immediately unboxed — redundant heap allocation",
                        location=loc,
                        help="Remove the intermediate box()/unbox() and use the value directly.",
                    ))

        if isinstance(expr, UnaryOp) and expr.op == "*":
            if self._is_boxing_call(expr.operand):
                loc = self._span_loc(getattr(expr, "span", None))
                self._diags.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category,
                    message="Value is boxed then immediately dereferenced — redundant heap allocation",
                    location=loc,
                    help="Remove Box::new() and use the value directly on the stack.",
                ))

        # Recurse into child expressions
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)
