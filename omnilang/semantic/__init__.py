"""Semantic analysis passes for OmniLang."""

from omnilang.ast_nodes import Program
from omnilang.utils import ErrorCollector


def analyze(program: Program, source_lines: dict[str, list[str]] | None = None):
    """Run all semantic analysis passes. Returns (type_map, rename_map) or exits on error."""
    errors = ErrorCollector()

    from omnilang.semantic import resolver
    scope_info = resolver.resolve(program, errors)

    from omnilang.semantic import type_checker
    type_map = type_checker.check(program, scope_info, errors)

    from omnilang.semantic import shadow_checker
    rename_map = shadow_checker.check(program, scope_info, errors)

    from omnilang.semantic import restrict_checker
    restrict_checker.check(program, scope_info, type_map, errors)

    from omnilang.semantic import main_checker
    main_checker.check(program, errors)

    if errors.has_errors():
        errors.report_and_exit(source_lines)

    return type_map, rename_map
