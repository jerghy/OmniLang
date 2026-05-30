"""E0402: &dyn Trait overhead (Rust target only).

When polymorphic class usage requires Rust trait objects (&dyn Trait),
the compiler inserts a vtable indirection. This warns about implicit
trait-object patterns visible at the OmniLang AST level.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, ClassDef, MethodCall, VarRef


class E0402Rule(AnalyzeRule):
    code = "E0402"
    name = "dyn_trait_overhead"
    severity = Severity.WARNING
    category = "type"
    message = "Polymorphic class usage requires `&dyn Trait` object in Rust, adding vtable indirection"
    help = "Prefer generics (monomorphization) over trait objects where performance is critical."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []

        # Only relevant when Rust is a target
        if "rust" not in ctx.targets:
            return []

        # Build a set of class / interface names that serve as trait-object boundaries
        self._interface_names: set[str] = set()
        for cls in ctx.program.classes:
            if cls.implements:
                self._interface_names.update(cls.implements)

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
            if isinstance(stmt, VarDecl):
                # Variable typed as an interface rather than a concrete class
                if stmt.type_ann and stmt.type_ann.name in self._interface_names:
                    loc = self._span_loc(stmt.span)
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=self.message,
                        location=loc, help=self.help,
                    ))
                if stmt.init:
                    self._walk_expr(stmt.init)
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

    def _walk_expr(self, expr):
        if isinstance(expr, MethodCall):
            obj = expr.object
            if isinstance(obj, VarRef):
                obj_ty = self._ctx.type_map.get(id(obj))
                if obj_ty and getattr(obj_ty, "name", "") in self._interface_names:
                    loc = self._span_loc(getattr(expr, "span", None))
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=f"Method call on trait object `{obj_ty.name}` requires `&dyn {obj_ty.name}` in Rust",
                        location=loc, help=self.help,
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
