"""SEC0102: Hardcoded credentials.

Detects string literals and variable assignments that appear to contain
hardcoded secrets (passwords, tokens, API keys, etc.).  Hardcoded
secrets are a security risk since they can be leaked through version
control, logs, or binary inspection.

Detection strategy:
  - Variable names matching patterns like ``password``, ``secret``,
    ``token``, ``api_key``, ``credential`` whose initialiser is a string
    literal are flagged.
  - String literal values that look like high-entropy secrets (long
    alphanumeric sequences, base64-ish tokens) are flagged even without
    a suspicious variable name.
"""

import re
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, Assign, Literal, Block, Expr


# ── Patterns for secret-suggesting variable names ─────────
_SECRET_NAME_RE = re.compile(
    r'^('
    r'password|passwd|pwd|secret|token|api[_-]?key|apikey|'
    r'auth[_-]?token|access[_-]?token|refresh[_-]?token|'
    r'credential|credentials|cred|'
    r'private[_-]?key|secret[_-]?key|'
    r'session[_-]?secret|cookie[_-]?secret|'
    r'db[_-]?pass|db[_-]?password|'
    r'jwt[_-]?secret|encryption[_-]?key|'
    r'master[_-]?key|root[_-]?pass'
    r')$',
    re.IGNORECASE,
)

# Value heuristics for detecting high-entropy / machine-generated strings.
_MIN_SECRET_LENGTH = 16
_HIGH_ENTROPY_RE = re.compile(
    r'^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[^a-zA-Z0-9\s]).{12,}$'
)
_BASE64_LIKE_RE = re.compile(r'^[A-Za-z0-9+/]{20,}={0,2}$')
_HEX_LIKE_RE = re.compile(r'^[0-9a-fA-F]{20,}$')

# Well-known test / dummy values that should not trigger the rule.
_FALSE_POSITIVES = frozenset({
    "password", "password123", "secret", "changeme", "admin", "guest",
    "password1", "passw0rd", "letmein", "123456", "qwerty", "abc123",
    "supersecret", "default", "test", "testing", "example",
})


class SEC0102Rule(AnalyzeRule):
    code = "SEC0102"
    name = "hardcoded_credential"
    severity = Severity.SEC
    category = "security"
    message = "Hardcoded `{kind}` detected — value looks like a secret"
    help = "Move the secret to an environment variable or a secrets manager and load it at runtime."

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
            elif isinstance(stmt, Assign):
                self._check_assign(stmt, diagnostics)

            for child_attr in ('body', 'then_body', 'else_body'):
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._check_block(child, diagnostics)

    def _check_var_decl(
        self, decl: VarDecl, diagnostics: list[Diagnostic],
    ) -> None:
        """Check if a variable declaration looks like a hardcoded credential."""
        if decl.init is None:
            return

        val = self._extract_string_value(decl.init)
        if val is None:
            return

        match = _SECRET_NAME_RE.match(decl.name)
        if match:
            if val.lower() in _FALSE_POSITIVES:
                return
            self._report(decl, match.group(1).lower(), diagnostics)
            return

        # Value-based heuristic: high-entropy strings
        if self._looks_like_secret(val):
            self._report(decl, "value", diagnostics)

    def _check_assign(
        self, assign: Assign, diagnostics: list[Diagnostic],
    ) -> None:
        """Check if an assignment looks like a hardcoded credential."""
        val = self._extract_string_value(assign.value)
        if val is None:
            return

        if self._looks_like_secret(val):
            self._report(assign, "value", diagnostics)

    @staticmethod
    def _extract_string_value(expr: Expr) -> str | None:
        """Extract the string value from an expression if it is a string
        literal or a simple template containing only literal parts."""
        from omnilang.ast_nodes import TemplateString

        if isinstance(expr, Literal) and expr.kind in ("String", "str", "char"):
            if isinstance(expr.value, str):
                return expr.value
        elif isinstance(expr, TemplateString):
            parts = []
            for part in expr.parts:
                if part.kind == "literal" and isinstance(part.value, str):
                    parts.append(part.value)
                else:
                    return None  # interpolated expression — not a simple literal
            return "".join(parts) if parts else None
        return None

    @staticmethod
    def _looks_like_secret(val: str) -> bool:
        """Heuristic: is *val* likely a generated secret rather than a
        human-readable string?"""
        if len(val) < _MIN_SECRET_LENGTH:
            return False
        if val.lower() in _FALSE_POSITIVES:
            return False
        if _BASE64_LIKE_RE.match(val):
            return True
        if _HEX_LIKE_RE.match(val):
            return True
        if _HIGH_ENTROPY_RE.match(val):
            return True
        return False

    def _report(self, node, kind: str, diagnostics: list[Diagnostic]) -> None:
        span = node.span if node.span else None
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
            message=self.message.format(kind=kind),
            location=loc,
            help=self.help,
        ))
