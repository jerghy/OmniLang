"""S0105: Non-idiomatic naming.

Checks function and variable names against the naming conventions of
the target language(s).  OmniLang compiles to multiple backends; a name
that is idiomatic in one language may be jarring in another.

Convention table:
  Language    Functions        Variables/Fields   Constants
  ───────     ─────────        ────────────────   ─────────
  Rust        snake_case       snake_case         SCREAMING_SNAKE
  Python      snake_case       snake_case         SCREAMING_SNAKE
  JavaScript  camelCase        camelCase          SCREAMING_SNAKE / ALL_CAPS
  Dart        lowerCamelCase   lowerCamelCase     lowerCamelCase / ALL_CAPS
  Go          camelCase        camelCase / mixed  ALL_CAPS for exported
"""

import re
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, FunctionDef, Block


# ── Regex patterns ────────────────────────────────────────

_SNAKE_CASE_RE = re.compile(r'^[a-z][a-z0-9]*(_[a-z0-9]+)*$')
_UPPER_SNAKE_RE = re.compile(r'^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$')
_CAMEL_CASE_RE = re.compile(r'^[a-z][a-zA-Z0-9]*$')
_PASCAL_CASE_RE = re.compile(r'^[A-Z][a-zA-Z0-9]*$')

# Built-in / magic names that are exempt from conventions in all languages
_EXEMPT_NAMES = frozenset({
    '_', 'self', 'this', '__init__', '__str__', '__repr__', '__call__',
    '__getitem__', '__setitem__', '__iter__', '__next__', '__len__',
    '__enter__', '__exit__', '__eq__', '__ne__', '__lt__', '__le__',
    '__gt__', '__ge__', '__hash__', '__add__', '__sub__', '__mul__',
    '__truediv__', '__floordiv__', '__mod__', '__pow__',
    '__and__', '__or__', '__xor__', '__lshift__', '__rshift__',
    '__neg__', '__pos__', '__invert__', '__contains__',
    '__aenter__', '__aexit__', '__aiter__', '__anext__',
})


class S0105Rule(AnalyzeRule):
    code = "S0105"
    name = "non_idiomatic_name"
    severity = Severity.STYLE
    category = "style"
    message = "`{name}` does not follow {lang} naming conventions ({expected})"
    help = "Rename to `{suggestion}` to follow {lang} conventions."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []
        languages = ctx.targets or ["rust", "python", "js", "dart"]

        for func in ctx.program.functions:
            for lang in languages:
                diag = self._check_function_name(func.name, lang)
                if diag:
                    diagnostics.append(self._locate(func.name, diag, func.span))
                    break  # first language that complains is enough
            if func.body:
                self._check_block_vars(func.body, languages, diagnostics)

        self._ctx = None
        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    # ── Per-language validators ───────────────────────────

    @staticmethod
    def _is_snake_case(name: str) -> bool:
        return bool(_SNAKE_CASE_RE.match(name))

    @staticmethod
    def _is_camel_case(name: str) -> bool:
        return bool(_CAMEL_CASE_RE.match(name))

    @staticmethod
    def _is_pascal_case(name: str) -> bool:
        return bool(_PASCAL_CASE_RE.match(name))

    @staticmethod
    def _is_upper_snake(name: str) -> bool:
        return bool(_UPPER_SNAKE_RE.match(name))

    @staticmethod
    def _to_snake(name: str) -> str:
        """Convert a camelCase or PascalCase name to snake_case heuristically."""
        s = re.sub(r'(.)([A-Z][a-z]+)', r'\1_\2', name)
        s = re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', s)
        return s.lower()

    @staticmethod
    def _to_camel(name: str) -> str:
        """Convert a snake_case name to lowerCamelCase."""
        parts = name.split('_')
        return parts[0] + ''.join(p.capitalize() for p in parts[1:])

    def _check_function_name(self, name: str, lang: str) -> str | None:
        """Return an error message fragment if *name* is not idiomatic for *lang*,
        or None if it is fine."""
        if _is_magic_or_exempt(name):
            return None

        if lang in ("rust", "python"):
            if not self._is_snake_case(name):
                return f"{lang}, expected snake_case"
        elif lang in ("js", "dart", "typescript"):
            if not self._is_camel_case(name):
                return f"{lang}, expected camelCase"
        elif lang in ("go", "golang"):
            # Go: exported = PascalCase, unexported = camelCase
            if not (self._is_camel_case(name) or self._is_pascal_case(name)):
                return f"{lang}, expected camelCase or PascalCase"

        return None

    def _check_var_name(self, name: str, lang: str) -> str | None:
        if _is_magic_or_exempt(name):
            return None

        if lang in ("rust", "python"):
            if not (self._is_snake_case(name) or self._is_upper_snake(name)):
                return f"{lang}, expected snake_case"
        elif lang in ("js", "dart", "typescript"):
            if not self._is_camel_case(name):
                return f"{lang}, expected lowerCamelCase"
        elif lang in ("go", "golang"):
            if not (self._is_camel_case(name) or self._is_upper_snake(name)):
                return f"{lang}, expected camelCase"

        return None

    # ── Block walking ─────────────────────────────────────

    def _check_block_vars(
        self, block: Block, languages: list[str],
        diagnostics: list[Diagnostic],
    ) -> None:
        for stmt in block.stmts:
            if isinstance(stmt, VarDecl):
                for lang in languages:
                    diag = self._check_var_name(stmt.name, lang)
                    if diag:
                        diagnostics.append(
                            self._locate(stmt.name, diag, stmt.span)
                        )
                        break  # first language complaint suffices

            # Recurse
            for child_attr in ('body', 'then_body', 'else_body'):
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._check_block_vars(child, languages, diagnostics)

        if block.trailing_expr is not None:
            pass  # expressions don't declare names — nothing to check

    # ── Location helpers ──────────────────────────────────

    def _locate(self, name: str, msg_fragment: str, span) -> Diagnostic:
        line = span.line if span else 1
        col = span.col if span else 1
        # Parse the fragment to extract the expected pattern
        if ", expected " in msg_fragment:
            parts = msg_fragment.split(", expected ", 1)
            lang_part = parts[0]
            expected = parts[1]
        else:
            lang_part = "unknown"
            expected = msg_fragment

        # Build a slightly more actionable message
        full_msg = self.message.format(name=name, lang=lang_part, expected=expected)
        return Diagnostic(
            code=self.code, severity=self.severity,
            category=self.category,
            message=full_msg,
            location=SourceLocation(
                file=self._filename(self._get_ctx()),
                line=line, column=col,
                span=(line, col, span.end_line if span else 1, span.end_col if span else 1),
            ),
            help=self.help.format(suggestion=expected, lang=lang_part),
        )

    _ctx: AnalyzeContext | None = None

    def _get_ctx(self) -> AnalyzeContext:
        return self._ctx


# ── Module-level helpers ──────────────────────────────────

def _is_magic_or_exempt(name: str) -> bool:
    if name in _EXEMPT_NAMES:
        return True
    # Dunder methods
    if name.startswith('__') and name.endswith('__') and len(name) > 4:
        return True
    return False
