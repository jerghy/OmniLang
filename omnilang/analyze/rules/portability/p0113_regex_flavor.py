"""P0113: Regex flavor differences across targets.

Detects regex patterns that behave differently in JS/Dart/Python/Rust
regex engines. Flags features like backreferences, lookahead/lookbehind,
and atomic groups that have varying support across targets.

Known differences:
  - Backreferences (\\1, \\2): Supported in JS, Python; NOT in Rust `regex` crate
  - Lookahead groups: Not supported in Dart before 3.x
  - Lookbehind groups: Not supported in Dart before 3.x
  - Unicode property escapes (\\p{...}): Behavior differs across engines
  - Atomic groups (?>...): Not supported in JS
  - Possessive quantifiers (++, *+, ?+): Not supported in JS
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import Literal, MethodCall, FuncCall, VarRef


# Regex-related method names
_REGEX_METHODS = {
    "match", "replace", "replace_all", "split", "find", "find_all",
    "test", "search", "exec", "matches", "is_match", "captures",
}

# Regex constructor/function names
_REGEX_TYPES = {"Regex", "RegExp", "regex", "Pattern", "re"}


def _has_backreference(s: str) -> bool:
    """Check for backreferences like \\1, \\2, etc. in a regex string."""
    i = 0
    while True:
        i = s.find("\\", i)
        if i < 0 or i >= len(s) - 1:
            return False
        if s[i + 1].isdigit():
            return True
        i += 1


def _has_possessive_quantifier(s: str) -> bool:
    """Check for possessive quantifiers: ++, *+, ?+, }+."""
    for q in ("*+", "?+", "++", "}+"):
        if q in s:
            return True
    return False


# Each feature: (name, check_fn, detail_msg, affected_targets)
_REGEX_FEATURES = [
    (
        "backreference",
        _has_backreference,
        "Backreferences are not supported by Rust's regex crate",
        {"rust"},
    ),
    (
        "lookahead",
        lambda s: "(?=" in s,
        "Lookahead groups are not supported in Dart (before 3.x)",
        {"dart"},
    ),
    (
        "negative lookahead",
        lambda s: "(?!" in s,
        "Negative lookahead groups are not supported in Dart (before 3.x)",
        {"dart"},
    ),
    (
        "lookbehind",
        lambda s: "(?<=" in s,
        "Lookbehind groups are not supported in Dart (before 3.x)",
        {"dart"},
    ),
    (
        "negative lookbehind",
        lambda s: "(?<!" in s,
        "Negative lookbehind groups are not supported in Dart (before 3.x)",
        {"dart"},
    ),
    (
        "unicode property escape",
        lambda s: "\\p{" in s or "\\P{" in s,
        "Unicode property escapes behave differently across JS and Dart",
        {"js", "dart"},
    ),
    (
        "atomic group",
        lambda s: "(?>" in s,
        "Atomic groups are not supported by JavaScript",
        {"js"},
    ),
    (
        "possessive quantifier",
        _has_possessive_quantifier,
        "Possessive quantifiers are not supported by JavaScript",
        {"js"},
    ),
]


class P0113Rule(AnalyzeRule):
    code = "P0113"
    name = "regex_flavor"
    severity = Severity.PORT
    category = "portability"
    message = "Regex pattern contains `{feature}` -- behavior differs across target languages"
    help = (
        "Check regex compatibility across JS/Python/Rust/Dart. "
        "Consider using a compatible sub-pattern or target-conditional code."
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
        # Check method calls for regex usage: str.match(...)
        if isinstance(expr, MethodCall) and expr.method in _REGEX_METHODS:
            for arg in expr.args:
                self._check_regex_string(arg)

        # Check constructor calls: Regex("..."), re.compile("...")
        if isinstance(expr, FuncCall):
            if isinstance(expr.func, VarRef) and expr.func.name in _REGEX_TYPES:
                if expr.args:
                    self._check_regex_string(expr.args[0])

        # Recurse into children
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)

    def _check_regex_string(self, expr):
        """Check a string literal for regex flavor differences."""
        if not isinstance(expr, Literal) or expr.kind != "String":
            return
        regex_str = str(expr.value)

        # Quick check: must have regex metacharacters
        if not any(c in regex_str for c in "\\^$.*+?()[]{}|"):
            return

        for feature_name, check_fn, detail, targets in _REGEX_FEATURES:
            if check_fn(regex_str):
                loc = self._span_loc(expr.span)
                targets_str = ", ".join(sorted(targets))
                self._diags.append(Diagnostic(
                    code=self.code,
                    severity=self.severity,
                    category=self.category,
                    message=self.message.format(feature=feature_name),
                    location=loc,
                    help=f"{detail} (affected: {targets_str}). {self.help}",
                ))
