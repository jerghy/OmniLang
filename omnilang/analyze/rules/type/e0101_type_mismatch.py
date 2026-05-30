"""E0101: Type mismatch in assignment, return, or call argument.

Detects when a value of one type is used where a different type is expected.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    VarDecl, Assign, ReturnStmt, FunctionDef, FnType,
    TypeAnnotation, UnionType, Block, FuncCall,
)


class E0101Rule(AnalyzeRule):
    code = "E0101"
    name = "type_mismatch"
    severity = Severity.ERROR
    category = "type"
    message = "Type mismatch: expected `{expected}`, got `{actual}`"
    help = "Ensure the expression type matches the declared or inferred type."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []
        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, func)
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
        return SourceLocation(file=self._filename(), line=1, column=1, span=(1,1,1,1))

    def _type_name(self, t) -> str:
        if t is None:
            return "?"
        if hasattr(t, 'name'):
            return t.name
        if isinstance(t, UnionType):
            return " | ".join(self._type_name(m) for m in t.types)
        return str(t)

    def _check_block(self, block: Block, func: FunctionDef):
        for stmt in block.stmts:
            self._check_stmt(stmt, func)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, func)

    def _check_stmt(self, stmt, func: FunctionDef):
        if isinstance(stmt, VarDecl):
            if stmt.type_ann and stmt.init:
                declared = self._type_name(stmt.type_ann)
                actual_ty = self._ctx.type_map.get(id(stmt.init))
                if actual_ty:
                    actual = self._type_name(actual_ty)
                    if declared != actual and declared != "?" and actual != "?":
                        self._diags.append(Diagnostic(
                            code=self.code, severity=self.severity,
                            category=self.category,
                            message=self.message.format(expected=declared, actual=actual),
                            location=self._span_loc(stmt.span),
                            help=self.help,
                        ))
        elif isinstance(stmt, ReturnStmt):
            if stmt.value and func.return_type:
                expected = self._type_name(func.return_type)
                actual_ty = self._ctx.type_map.get(id(stmt.value))
                if actual_ty:
                    actual = self._type_name(actual_ty)
                    if expected != actual and expected != "?" and actual != "?":
                        self._diags.append(Diagnostic(
                            code=self.code, severity=self.severity,
                            category=self.category,
                            message=self.message.format(expected=expected, actual=actual),
                            location=self._span_loc(stmt.span),
                            help=self.help,
                        ))
        for child_attr in ['body', 'then_body', 'else_body']:
            child = getattr(stmt, child_attr, None)
            if isinstance(child, Block):
                self._check_block(child, func)
        if hasattr(stmt, 'else_ifs'):
            for _, elif_body in getattr(stmt, 'else_ifs', []):
                if isinstance(elif_body, Block):
                    self._check_block(elif_body, func)

    def _check_expr(self, expr, func: FunctionDef):
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, func)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, func)
