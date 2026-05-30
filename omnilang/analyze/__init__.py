"""OmniLang static analysis system (omnic check).

Provides a cargo-check-equivalent diagnostic pipeline for cross-language
portability analysis, type checking, and linting.

Architecture:
    1. Parse .omni source → AST
    2. Walk AST with registered rules → Diagnostic list
    3. Render diagnostics (terminal / JSON / GitHub Actions)
    4. Optionally auto-fix (fix_engine)

Usage:
    from omnilang.analyze import AnalyzePipeline

    pipeline = AnalyzePipeline(config)
    diagnostics = pipeline.run(program, source_lines)
    if diagnostics:
        pipeline.render(diagnostics)
"""

from omnilang.analyze.diagnostic import (
    Severity,
    SourceLocation,
    CodeAction,
    Diagnostic,
    DiagnosticSummary,
)

__all__ = [
    "Severity",
    "SourceLocation",
    "CodeAction",
    "Diagnostic",
    "DiagnosticSummary",
]
