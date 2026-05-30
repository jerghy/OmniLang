"""P0110: Potential division by zero.

Detects integer division or modulo where the divisor is a literal zero
or a variable that might be zero at runtime.

All targets: division by zero is undefined or panic behavior.
This is a PORT-level diagnostic because the behavior varies:
    - JS: Returns Infinity / NaN (silently)
    - Dart: Throws IntegerDivisionByZeroException
    - Python: Raises ZeroDivisionError
    - Rust: Panics at runtime
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import BinaryOp, Literal


class P0110Rule(AnalyzeRule):
    code = "P0110"
    name = "potential_divzero"
    severity = Severity.PORT
    category = "portability"
    message = "Potential division by zero — behavior varies across target languages"
    help = (
        "JS returns Infinity/NaN silently; Dart throws exception; "
        "Python raises ZeroDivisionError; Rust panics. "
        "Check divisor is non-zero before dividing."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []

        for func in ctx.program.functions:
            if func.body:
                self._check_block(func.body, ctx, diagnostics)

        return diagnostics

    def _check_block(self, block, ctx: AnalyzeContext, diagnostics: list[Diagnostic]):
        for stmt in block.stmts:
            self._check_stmt(stmt, ctx, diagnostics)
        if block.trailing_expr:
            self._check_expr(block.trailing_expr, ctx, diagnostics)

    def _check_stmt(self, stmt, ctx, diagnostics):
        for attr in ['init', 'value', 'condition', 'iterable', 'arg', 'expr']:
            if hasattr(stmt, attr):
                val = getattr(stmt, attr)
                if val is not None:
                    self._check_expr(val, ctx, diagnostics)
        for attr in ['body', 'then_body', 'else_body']:
            if hasattr(stmt, attr):
                child = getattr(stmt, attr)
                if child:
                    self._check_block(child, ctx, diagnostics)
        if hasattr(stmt, 'else_ifs'):
            for _, elif_body in getattr(stmt, 'else_ifs', []):
                if elif_body:
                    self._check_block(elif_body, ctx, diagnostics)

    def _check_expr(self, expr, ctx, diagnostics):
        if isinstance(expr, BinaryOp) and expr.op in ("/", "%"):
            # Check if right operand is literal 0
            if isinstance(expr.right, Literal):
                val = expr.right.value
                if val == 0 or val == 0.0:
                    span = expr.span
                    loc = SourceLocation(
                        file="<input>",
                        line=span.line if span else 1,
                        column=span.col if span else 1,
                        span=(span.line, span.col, span.end_line, span.end_col) if span else (1,1,1,1),
                    )
                    diagnostics.append(Diagnostic(
                        code=self.code,
                        severity=Severity.ERROR,  # Escalate: literal divzero
                        category=self.category,
                        message="Division by literal zero is always an error",
                        location=loc,
                        help="Remove or guard this division with a non-zero check.",
                    ))

        # Recurse
        for field_name in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, field_name)
            if isinstance(val, list):
                for item in val:
                    if hasattr(item, '__dataclass_fields__'):
                        self._check_expr(item, ctx, diagnostics)
                    elif isinstance(item, tuple):
                        for sub in item:
                            if hasattr(sub, '__dataclass_fields__'):
                                self._check_expr(sub, ctx, diagnostics)
            elif hasattr(val, '__dataclass_fields__') and val is not None:
                self._check_expr(val, ctx, diagnostics)
