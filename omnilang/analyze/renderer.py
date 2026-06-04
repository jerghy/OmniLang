"""Diagnostic renderers for omnic check output.

Supports three output formats:
    - terminal: Colored output with source context (rustc-style)
    - json: Machine-readable JSON output
    - github: GitHub Actions workflow annotations
"""

from omnilang.analyze.diagnostic import Diagnostic, DiagnosticSummary


def render_terminal(diagnostics: list[Diagnostic], source_lines: dict[str, list[str]]):
    """Render diagnostics to terminal with colors and source context."""
    for diag in diagnostics:
        sev_name = diag.severity.name
        print(f"\033[1m{sev_name}: {diag.code}\033[0m: {diag.message}")
        print(f"  \033[34m-->\033[0m {diag.location.file}:{diag.location.line}:{diag.location.column}")
        print("   |")

        # Show source line with underline
        file_lines = source_lines.get(diag.location.file, [])
        line_no = diag.location.line
        if file_lines and line_no <= len(file_lines):
            src_line = file_lines[line_no - 1].rstrip()
            col = diag.location.column
            # Calculate indicator width
            span_end = diag.location.span[3]
            width = max(span_end - col, 1)
            indicator = " " * (col - 1) + "^" * width

            print(f"  \033[34m{line_no:>3}\033[0m | {src_line}")
            print(f"   | \033[31m{indicator}\033[0m")

        if diag.help:
            print(f"   = \033[36mhelp:\033[0m {diag.help}")
        for suggestion in diag.suggestions:
            print(f"   = \033[32msuggestion:\033[0m {suggestion.message}")
        print()


def render_terminal_plain(diagnostics: list[Diagnostic], source_lines: dict[str, list[str]]):
    """Render diagnostics without ANSI escape codes (for non-TTY output)."""
    for diag in diagnostics:
        print(f"{diag.severity.name}: {diag.code}: {diag.message}")
        print(f"  --> {diag.location.file}:{diag.location.line}:{diag.location.column}")
        print("   |")

        file_lines = source_lines.get(diag.location.file, [])
        line_no = diag.location.line
        if file_lines and line_no <= len(file_lines):
            src_line = file_lines[line_no - 1].rstrip()
            col = diag.location.column
            span_end = diag.location.span[3]
            width = max(span_end - col, 1)
            indicator = " " * (col - 1) + "^" * width

            print(f"  {line_no:>3} | {src_line}")
            print(f"   | {indicator}")

        if diag.help:
            print(f"   = help: {diag.help}")
        print()


def render_json(diagnostics: list[Diagnostic]):
    """Render diagnostics as JSON."""
    import json

    summary = DiagnosticSummary.from_diagnostics(diagnostics)
    output = {
        "diagnostics": [
            {
                "code": d.code,
                "severity": d.severity.value,
                "category": d.category,
                "message": d.message,
                "location": {
                    "file": d.location.file,
                    "line": d.location.line,
                    "column": d.location.column,
                },
                "help": d.help,
                "suggestions": [
                    {"message": s.message, "applicable": s.applicable}
                    for s in d.suggestions
                ],
            }
            for d in diagnostics
        ],
        "summary": {
            "errors": summary.errors,
            "warnings": summary.warnings,
            "ports": summary.ports,
            "total": summary.total,
        },
    }
    print(json.dumps(output, indent=2))


def render_github(diagnostics: list[Diagnostic]):
    """Render diagnostics as GitHub Actions workflow commands.

    Format: ::error file=main.omni,line=5,col=10::E0101: type mismatch
    """
    sev_map = {
        "error": "error",
        "warning": "warning",
        "port": "warning",
        "style": "notice",
        "perf": "warning",
        "sec": "error",
        "hint": "notice",
    }
    for diag in diagnostics:
        gh_sev = sev_map.get(diag.severity.value, "warning")
        print(
            f"::{gh_sev} "
            f"file={diag.location.file},"
            f"line={diag.location.line},"
            f"col={diag.location.column}"
            f"::{diag.code}: {diag.message}"
        )


def render_summary(summary: DiagnosticSummary):
    """Render a one-line summary of diagnostic counts."""
    parts = []
    if summary.errors:
        parts.append(f"\033[31m{summary.errors} error(s)\033[0m")
    if summary.warnings:
        parts.append(f"\033[33m{summary.warnings} warning(s)\033[0m")
    if summary.ports:
        parts.append(f"\033[36m{summary.ports} portability issue(s)\033[0m")
    if summary.styles:
        parts.append(f"{summary.styles} style(s)")
    if summary.perfs:
        parts.append(f"{summary.perfs} perf(s)")
    if summary.hints:
        parts.append(f"{summary.hints} hint(s)")

    if not parts:
        print("No issues found.")
    else:
        print(", ".join(parts))
