"""P0112: Float to int precision loss.

Implicit float-to-int conversions may truncate differently across targets
(Rust panics on overflow in debug, JS uses ToInt32, Python truncates toward
negative infinity, Dart rounds toward zero). This rule flags implicit
conversions and explicit casts where precision loss is likely.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    VarDecl, Assign, ReturnStmt, FuncCall, VarRef,
)


_INT_TYPES = {"i32", "i64", "i16", "i8", "u32", "u64", "u16", "u8", "int", "Int"}
_FLOAT_TYPES = {"f32", "f64", "float", "Float", "Float32", "Float64"}


class P0112Rule(AnalyzeRule):
    code = "P0112"
    name = "float_to_int_loss"
    severity = Severity.PORT
    category = "portability"
    message = "Implicit float-to-int conversion may truncate — behavior differs across targets"
    help = "Use an explicit cast (e.g., `x.to_int()` or `as i32`) to make truncation semantics clear."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []

        for func in ctx.program.functions:
            if func.body:
                self._walk_block(func.body)
            # Check function parameter types: float param passed to int context
            for param in func.params:
                if param.type_ann and param.type_ann.name in _FLOAT_TYPES:
                    if func.return_type and func.return_type.name in _INT_TYPES:
                        loc = self._span_loc(func.span)
                        self._diags.append(Diagnostic(
                            code=self.code, severity=self.severity,
                            category=self.category,
                            message=f"Function `{func.name}` takes float parameter `{param.name}` but returns int — value may be truncated",
                            location=loc, help=self.help,
                        ))

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

    def _type_name(self, t) -> str:
        if t is None:
            return "?"
        if hasattr(t, "name"):
            return t.name
        return str(t)

    def _walk_block(self, block):
        for stmt in getattr(block, "stmts", []):
            if isinstance(stmt, VarDecl):
                if stmt.type_ann and stmt.init:
                    declared = stmt.type_ann.name
                    actual_ty = self._ctx.type_map.get(id(stmt.init))
                    if actual_ty:
                        actual = self._type_name(actual_ty)
                        if actual in _FLOAT_TYPES and declared in _INT_TYPES:
                            loc = self._span_loc(stmt.span)
                            self._diags.append(Diagnostic(
                                code=self.code, severity=self.severity,
                                category=self.category,
                                message=f"Implicit float-to-int conversion: initializing `{declared}` with `{actual}` value",
                                location=loc, help=self.help,
                            ))

            elif isinstance(stmt, ReturnStmt):
                if stmt.value and hasattr(stmt, "func") and stmt.func:
                    ret_ty = self._ctx.type_map.get(id(stmt.func))
                    if ret_ty:
                        ret_name = self._type_name(ret_ty)
                        if ret_name in _INT_TYPES:
                            val_ty = self._ctx.type_map.get(id(stmt.value))
                            if val_ty and self._type_name(val_ty) in _FLOAT_TYPES:
                                loc = self._span_loc(stmt.span)
                                self._diags.append(Diagnostic(
                                    code=self.code, severity=self.severity,
                                    category=self.category,
                                    message=f"Returning float value where `{ret_name}` is expected — implicit truncation",
                                    location=loc, help=self.help,
                                ))

            elif isinstance(stmt, Assign):
                if stmt.value:
                    # Check if assigning a float-typed expression to an int-typed target
                    val_ty = self._ctx.type_map.get(id(stmt.value))
                    target_ty = self._ctx.type_map.get(id(stmt))
                    if target_ty and val_ty:
                        tname = self._type_name(target_ty)
                        vname = self._type_name(val_ty)
                        if vname in _FLOAT_TYPES and tname in _INT_TYPES:
                            loc = self._span_loc(stmt.span)
                            self._diags.append(Diagnostic(
                                code=self.code, severity=self.severity,
                                category=self.category,
                                message=f"Assigning float value to `{tname}` — implicit truncation",
                                location=loc, help=self.help,
                            ))

            for attr in ["init", "value", "condition", "iterable", "arg", "expr"]:
                val = getattr(stmt, attr, None)
                if val:
                    self._walk_expr(val, stmt)
            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child:
                    self._walk_block(child)

        if block.trailing_expr:
            self._walk_expr(block.trailing_expr, None)

    def _walk_expr(self, expr, parent_stmt):
        # Check function call arguments for float-to-int narrowing
        if isinstance(expr, FuncCall):
            if isinstance(expr.func, VarRef):
                for arg in expr.args:
                    arg_ty = self._ctx.type_map.get(id(arg))
                    if arg_ty and self._type_name(arg_ty) in _FLOAT_TYPES:
                        loc = self._span_loc(getattr(arg, "span", None))
                        self._diags.append(Diagnostic(
                            code=self.code, severity=self.severity,
                            category=self.category,
                            message="Float argument passed to function — may be truncated to int depending on signature",
                            location=loc, help=self.help,
                        ))

        # Recurse into child expressions
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val, parent_stmt)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item, parent_stmt)
