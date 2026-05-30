"""Diagnostic data structures for the OmniLang static analysis system.

Mirrors rustc's Diagnostic / SubDiagnostic / CodeSuggestion pattern.
All types are frozen dataclasses for immutability and hashability.
"""

from dataclasses import dataclass, field
from enum import Enum


class Severity(Enum):
    """Diagnostic severity levels, modeled after rustc's Level enum.

    Ordered by severity: Error is most severe, Hint least.
    """
    ERROR = "error"       # Compilation must fail (e.g., type mismatch)
    WARNING = "warning"   # Likely a bug, but compilation may succeed
    PORT = "port"         # Cross-language portability issue (OmniLang unique)
    STYLE = "style"       # Code style / convention violation
    PERF = "perf"         # Performance concern
    SEC = "sec"           # Security vulnerability
    HINT = "hint"         # Informational suggestion

    def __lt__(self, other: "Severity") -> bool:
        order = {Severity.ERROR: 0, Severity.WARNING: 1, Severity.PORT: 2,
                 Severity.STYLE: 3, Severity.PERF: 4, Severity.SEC: 5, Severity.HINT: 6}
        return order.get(self, 99) < order.get(other, 99)

    def short(self) -> str:
        """Single-char representation for compact output."""
        return {
            Severity.ERROR: "E",
            Severity.WARNING: "W",
            Severity.PORT: "P",
            Severity.STYLE: "S",
            Severity.PERF: "F",
            Severity.SEC: "SEC",
            Severity.HINT: "H",
        }.get(self, "?")

    def color(self) -> str:
        """ANSI color name for terminal rendering."""
        return {
            Severity.ERROR: "red",
            Severity.WARNING: "yellow",
            Severity.PORT: "cyan",
            Severity.STYLE: "magenta",
            Severity.PERF: "blue",
            Severity.SEC: "red",
            Severity.HINT: "green",
        }.get(self, "white")


@dataclass(frozen=True)
class SourceLocation:
    """Precise source location with span information.

    Fields match the standard compiler span model:
        file   — Source file path
        line   — 1-based line number
        column — 1-based column number
        span   — (start_line, start_col, end_line, end_col) for underline rendering
    """
    file: str
    line: int
    column: int
    span: tuple[int, int, int, int]  # (start_line, start_col, end_line, end_col)

    def __str__(self) -> str:
        return f"{self.file}:{self.line}:{self.column}"


@dataclass(frozen=True)
class CodeAction:
    """A suggested fix for a diagnostic.

    message — Human-readable description of the fix
    edit    — Source replacement (old_text → new_text)
    """
    message: str
    edit: tuple[str, str]  # (old_text, new_text)
    applicable: bool = True  # False if the fix is uncertain


@dataclass(frozen=True)
class Diagnostic:
    """A single diagnostic message produced by an analyze rule.

    Mirrors rustc's Diagnostic with:
        code        — Rule code (e.g., "P0101", "E0105")
        severity    — Error / Warning / Port / Style / Perf / Sec / Hint
        category    — "type" / "async" / "ownership" / "portability" / ...
        message     — Human-readable summary
        location    — Source location
        help        — Optional help text (from explain/ docs)
        suggestions — Auto-fix options (CodeAction list)
        related     — Related diagnostics (sub-diagnostics, notes)
    """
    code: str
    severity: Severity
    category: str
    message: str
    location: SourceLocation
    help: str | None = None
    suggestions: list[CodeAction] = field(default_factory=list)
    related: list["Diagnostic"] = field(default_factory=list)

    def __str__(self) -> str:
        sev = self.severity.short()
        return f"{sev} {self.code}: {self.message}"


@dataclass
class DiagnosticSummary:
    """Aggregated diagnostic counts by severity.

    Used for exit code determination and summary output.
    """
    errors: int = 0
    warnings: int = 0
    ports: int = 0
    styles: int = 0
    perfs: int = 0
    secs: int = 0
    hints: int = 0

    @classmethod
    def from_diagnostics(cls, diagnostics: list[Diagnostic]) -> "DiagnosticSummary":
        summary = cls()
        for d in diagnostics:
            match d.severity:
                case Severity.ERROR:
                    summary.errors += 1
                case Severity.WARNING:
                    summary.warnings += 1
                case Severity.PORT:
                    summary.ports += 1
                case Severity.STYLE:
                    summary.styles += 1
                case Severity.PERF:
                    summary.perfs += 1
                case Severity.SEC:
                    summary.secs += 1
                case Severity.HINT:
                    summary.hints += 1
        return summary

    @property
    def total(self) -> int:
        return (self.errors + self.warnings + self.ports +
                self.styles + self.perfs + self.secs + self.hints)

    @property
    def has_errors(self) -> bool:
        return self.errors > 0

    def exit_code(self) -> int:
        """Determine exit code:
        0 = no issues (or only hints)
        1 = errors found
        2 = warnings/ports without errors
        """
        if self.errors > 0:
            return 1
        if self.warnings > 0 or self.ports > 0 or self.styles > 0 or self.perfs > 0 or self.secs > 0:
            return 2
        return 0
