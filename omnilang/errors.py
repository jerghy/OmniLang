"""Structured error types for the OmniLang CLI."""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal


@dataclass
class OmniError:
    """A single diagnostic from compilation or analysis."""
    severity: Literal["error", "warning", "info"]
    code: str
    message: str
    file: str | None = None
    line: int | None = None
    col: int | None = None
    suggestion: str | None = None

    def format(self) -> str:
        loc = ""
        if self.file:
            loc = f"{self.file}"
            if self.line is not None:
                loc += f":{self.line}"
                if self.col is not None:
                    loc += f":{self.col}"
        sev = self.severity.capitalize()
        msg = f"{sev}: {self.message}"
        if loc:
            msg += f"\n  -> {loc} ({self.code})"
        if self.suggestion:
            msg += f"\n  Suggestion: {self.suggestion}"
        return msg


class OmniLangError(Exception):
    """Base exception for all omnic CLI errors."""
    exit_code: int = 1

    def __init__(self, errors: list[OmniError] | None = None):
        self.errors = errors or []
        super().__init__("")

    def format(self) -> str:
        if not self.errors:
            return "An unknown error occurred."
        lines = [e.format() for e in self.errors]
        error_count = sum(1 for e in self.errors if e.severity == "error")
        warn_count = sum(1 for e in self.errors if e.severity == "warning")
        summary_parts = []
        if error_count:
            summary_parts.append(f"{error_count} error(s)")
        if warn_count:
            summary_parts.append(f"{warn_count} warning(s)")
        if summary_parts:
            lines.append("")
            lines.append(", ".join(summary_parts))
        return "\n".join(lines)


class CompilerUnavailable(OmniLangError):
    """Rust compiler DLL is not found."""
    exit_code = 2

    def __init__(self):
        super().__init__([OmniError(
            severity="error",
            code="COMPILER_UNAVAILABLE",
            message="Rust compiler DLL (omni_compiler.dll) not found. "
                    "Build it with: cd omni-compiler && cargo build --release",
        )])


class CompileError(OmniLangError):
    """Compilation failed."""
    exit_code = 1


class AnalyseError(OmniLangError):
    """Static analysis found issues."""
    exit_code = 1


class ConfigError(OmniLangError):
    """Project configuration issue."""
    exit_code = 3

    @classmethod
    def not_found(cls) -> "ConfigError":
        return cls([OmniError(
            severity="error",
            code="CONFIG_NOT_FOUND",
            message="No .omnilang.toml found. Run 'omnic create <name>' to create a project.",
        )])


class InputError(OmniLangError):
    """Bad CLI arguments or file not found."""
    exit_code = 4

    @classmethod
    def file_not_found(cls, filepath: str) -> "InputError":
        return cls([OmniError(
            severity="error",
            code="FILE_NOT_FOUND",
            message=f"File not found: {filepath}",
            file=filepath,
        )])
