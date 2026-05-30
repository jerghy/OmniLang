"""Error reporting and utility functions for the OmniLang compiler."""

import sys
from dataclasses import dataclass


@dataclass(frozen=True)
class Span:
    line: int
    col: int
    end_line: int
    end_col: int

    def __str__(self) -> str:
        return f"{self.line}:{self.col}"


class CompilerError(Exception):
    def __init__(self, message: str, span: Span | None = None, filename: str | None = None):
        self.message = message
        self.span = span
        self.filename = filename
        super().__init__(message)


class ErrorCollector:
    def __init__(self):
        self.errors: list[CompilerError] = []

    def add(self, message: str, span: Span | None = None, filename: str | None = None):
        self.errors.append(CompilerError(message, span, filename))

    def has_errors(self) -> bool:
        return len(self.errors) > 0

    def report_and_exit(self, source_lines: dict[str, list[str]] | None = None):
        if not self.errors:
            return
        for err in self.errors:
            print(format_error(err, source_lines), file=sys.stderr)
        sys.exit(1)


def format_error(err: CompilerError, source_lines: dict[str, list[str]] | None = None) -> str:
    if err.span and err.filename:
        lines = source_lines or {}
        file_lines = lines.get(err.filename, [])
        if file_lines and err.span.line <= len(file_lines):
            src_line = file_lines[err.span.line - 1].rstrip()
            indicator = " " * (err.span.col - 1) + "^" * max(err.span.end_col - err.span.col, 1)
            return (
                f"Error: {err.filename}:{err.span}: {err.message}\n"
                f"  --> {err.filename}:{err.span}\n"
                f"   |\n"
                f"{err.span.line:>3} | {src_line}\n"
                f"   | {indicator}"
            )
    elif err.span and err.filename:
        return f"Error: {err.filename}:{err.span}: {err.message}"
    elif err.filename:
        return f"Error: {err.filename}: {err.message}"
    return f"Error: {err.message}"
