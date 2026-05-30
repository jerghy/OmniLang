"""S0109: Redundant type annotation.

Flags type annotations on variable declarations where the annotation
exactly matches the type that can be trivially inferred from the
initialiser expression.  Removing redundant annotations improves
readability and reduces maintenance burden.

Examples:
  let x: i32 = 42       # redundant — literal 42 is obviously i32
  let name: String = "hello"  # redundant
  let x: f64 = 3.14     # redundant
  let items: Array<i32> = []   # NOT redundant (inferred type may differ)
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, Literal, Block, TypeAnnotation, Expr


# Mapping from literal kind strings to the corresponding type annotation name.
_LITERAL_KIND_TO_TYPE = {
    "i32": "i32",
    "i64": "i64",
    "f32": "f32",
    "f64": "f64",
    "bool": "bool",
    "String": "String",
    "str": "String",
    "char": "String",
}


class S0109Rule(AnalyzeRule):
    code = "S0109"
    name = "redundant_annotation"
    severity = Severity.HINT
    category = "style"
    message = "Redundant type annotation `{ann}` on `{name}` — type is trivially inferred"
    help = "Remove the `: {ann}` annotation and let type inference determine the type."

    _ctx: AnalyzeContext | None = None

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, diagnostics)
        self._ctx = None
        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _check_block(self, block: Block, diagnostics: list[Diagnostic]) -> None:
        for stmt in block.stmts:
            if isinstance(stmt, VarDecl):
                self._check_var_decl(stmt, diagnostics)

            for child_attr in ('body', 'then_body', 'else_body'):
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._check_block(child, diagnostics)

    def _check_var_decl(
        self, decl: VarDecl, diagnostics: list[Diagnostic],
    ) -> None:
        """Check whether *decl* has a redundant type annotation."""
        if decl.type_ann is None or decl.init is None:
            return

        inferred_type = self._infer_literal_type(decl.init)
        if inferred_type is None:
            return  # cannot trivially infer — skip

        ann_name = self._annotation_name(decl.type_ann)
        if ann_name is None:
            return

        if ann_name == inferred_type:
            self._report(decl, ann_name, diagnostics)

    @staticmethod
    def _infer_literal_type(expr: Expr) -> str | None:
        """Try to infer the type of a literal expression.

        Returns a type name (e.g. ``"i32"``) or ``None`` if the expression
        is not a simple literal whose type can be trivially determined.
        """
        if isinstance(expr, Literal):
            return _LITERAL_KIND_TO_TYPE.get(expr.kind)
        # String templates: `Hello, ${name}` — always yields a String
        from omnilang.ast_nodes import TemplateString
        if isinstance(expr, TemplateString):
            return "String"
        return None

    @staticmethod
    def _annotation_name(ann: TypeAnnotation) -> str | None:
        """Extract the simple name from a type annotation.

        Returns ``None`` for parameterised types like ``Array<i32>``
        since those are never trivially redundant.
        """
        if ann.type_args:
            return None  # generic with args — keep annotation
        return ann.name

    def _report(self, decl: VarDecl, ann_name: str, diagnostics: list[Diagnostic]) -> None:
        span = decl.span if decl.span else None
        line = span.line if span else 1
        col = span.col if span else 1
        loc = SourceLocation(
            file=self._filename(self._ctx),
            line=line, column=col,
            span=(line, col, span.end_line if span else 1, span.end_col if span else 1),
        )
        diagnostics.append(Diagnostic(
            code=self.code, severity=self.severity,
            category=self.category,
            message=self.message.format(ann=ann_name, name=decl.name),
            location=loc,
            help=self.help.format(ann=ann_name),
        ))
