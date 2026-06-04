"""Auto-fix engine for OmniLang diagnostics.

Provides automatic source code fixes for diagnostic rules.
Target: 80% of diagnostics should have auto-fix suggestions.
"""

from omnilang.analyze.diagnostic import Diagnostic


class FixEngine:
    """Applies auto-fixes to source code based on diagnostics."""

    def __init__(self):
        self._fixers: dict[str, callable] = {}

    def register(self, code: str, fixer: callable):
        """Register a fixer function for a diagnostic code."""
        self._fixers[code] = fixer

    def has_fix(self, diagnostic: Diagnostic) -> bool:
        """Check if a diagnostic has an available auto-fix."""
        return diagnostic.code in self._fixers or bool(diagnostic.suggestions)

    def apply(self, diagnostic: Diagnostic, source: str) -> str | None:
        """Apply auto-fix for a diagnostic. Returns modified source or None."""
        if diagnostic.suggestions:
            action = diagnostic.suggestions[0]
            if action.applicable:
                old, new = action.edit
                return source.replace(old, new, 1)

        if diagnostic.code in self._fixers:
            return self._fixers[diagnostic.code](diagnostic, source)

        return None

    def apply_all(self, diagnostics: list[Diagnostic], source: str,
                  interactive: bool = False) -> str:
        """Apply all available fixes. With interactive=True, previews each change."""
        modified = source
        for diag in diagnostics:
            fix = self.apply(diag, modified)
            if fix and fix != modified:
                if interactive:
                    print(f"Fix {diag.code}: {diag.message}")
                    print(f"  {diag.suggestions[0].message if diag.suggestions else 'auto-fix'}")
                modified = fix
        return modified


# Built-in fixers
def _fix_P0101_replace_with_idiv(diag: Diagnostic, source: str) -> str | None:
    """Replace a / b with _idiv(a, b) for P0101."""
    lines = source.split("\n")
    line_no = diag.location.line - 1
    if 0 <= line_no < len(lines):
        col = diag.location.column - 1
        span_end = diag.location.span[3] - 1
        # Simple replacement: wrap in _idiv()
        before = lines[line_no][:col]
        expr = lines[line_no][col:span_end]
        after = lines[line_no][span_end:]
        lines[line_no] = f"{before}_idiv({expr}){after}"
        return "\n".join(lines)
    return None


def build_default_fix_engine() -> FixEngine:
    """Create a FixEngine with built-in fixers."""
    engine = FixEngine()
    engine.register("P0101", _fix_P0101_replace_with_idiv)
    return engine
