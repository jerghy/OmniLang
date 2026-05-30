"""P0114: Number literal precision across targets.

Warns about large integer literals (>2^53) that lose precision when targeting
JavaScript. JavaScript's Number type is IEEE-754 double-precision, which can
only represent integers up to 2^53 - 1 exactly. Beyond that, some integers
cannot be represented precisely.

Also flags very large literals that may not fit in specific target integer
types (e.g., Rust i64, Dart int limits).
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import Literal


# 2^53 - 1: maximum safe integer in JS Number
MAX_JS_SAFE_INT = 9007199254740991

# 2^53: first integer that loses precision in JS
FIRST_UNSAFE_INT = 9007199254740992

# Target-specific type limits for additional warnings
_TARGET_LIMITS = {
    "i32": (2147483647, "Value exceeds i32 range (max 2147483647)"),
    "u32": (4294967295, "Value exceeds u32 range (max 4294967295)"),
    "i64": (9223372036854775807, "Value approaches i64 upper limit (max 9223372036854775807)"),
    "u64": (18446744073709551615, "Value exceeds u64 range"),
}


class P0114Rule(AnalyzeRule):
    code = "P0114"
    name = "number_literal_precision"
    severity = Severity.PORT
    category = "portability"
    message = "Integer literal {value} exceeds the exact-precision range of JavaScript Number type"
    help = (
        "JavaScript represents all numbers as IEEE-754 doubles. Integers above "
        "2^53 may lose precision. Use BigInt or a numeric library for exact "
        "large-integer arithmetic."
    )

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
        return SourceLocation(
            file=self._filename(), line=1, column=1, span=(1, 1, 1, 1),
        )

    def _walk_block(self, block):
        for stmt in getattr(block, "stmts", []):
            for attr in ["init", "value", "condition", "iterable", "arg", "expr"]:
                val = getattr(stmt, attr, None)
                if val is not None:
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
        if isinstance(expr, Literal) and isinstance(expr.value, int):
            val = expr.value

            # Check for values exceeding JS safe integer range
            if val > FIRST_UNSAFE_INT or val < -FIRST_UNSAFE_INT:
                loc = self._span_loc(expr.span)
                self._diags.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(value=val),
                    location=loc,
                    help=self.help,
                ))

            # Check target-specific type limits
            self._check_target_limits(val, expr)

        # Recurse into children
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)

    def _check_target_limits(self, val: int, expr):
        """Check if the value exceeds type limits for active targets."""
        for target, (limit, msg) in _TARGET_LIMITS.items():
            if val > limit:
                loc = self._span_loc(expr.span)
                self._diags.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=f"Integer literal {val} may exceed {target} range",
                    location=loc,
                    help=msg,
                ))
