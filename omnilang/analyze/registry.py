"""Rule registry for the OmniLang static analysis system.

Provides a cargo-check-like rule registration mechanism where each
diagnostic rule is registered by code and can be looked up, enabled,
or disabled through configuration.
"""

import re
from dataclasses import dataclass, field
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation


class AnalyzeRule:
    """Base class for all static analysis rules.

    Each rule has a unique code (e.g., "P0101"), a default severity,
    and a check() method that walks the AST to find issues.
    """

    code: str = ""
    name: str = ""
    severity: Severity = Severity.WARNING
    category: str = ""
    message: str = ""
    help: str = ""

    def check(self, ctx: "AnalyzeContext") -> list[Diagnostic]:
        """Walk the AST/program and return a list of diagnostics.

        Subclasses must implement this method.
        """
        raise NotImplementedError


@dataclass
class AnalyzeContext:
    """Context passed to each rule's check() method.

    Contains the program AST, source lines, target configuration,
    and any pre-computed analysis results.
    """
    program: object  # Program AST
    source_lines: dict[str, list[str]]
    targets: list[str] = field(default_factory=lambda: ["rust", "dart", "js", "python"])
    type_map: dict = field(default_factory=dict)
    rename_map: dict = field(default_factory=dict)
    capture_map: dict = field(default_factory=dict)


class RuleRegistry:
    """Registry for all diagnostic rules, similar to rustc's LintStore.

    Usage:
        registry = RuleRegistry()
        registry.register(P0101_Rule())
        rule = registry.get("P0101")
        for rule in registry.list_enabled(config):
            diags = rule.check(ctx)
    """

    def __init__(self):
        self._rules: dict[str, AnalyzeRule] = {}

    def register(self, rule: AnalyzeRule):
        """Register a rule instance."""
        self._rules[rule.code] = rule

    def get(self, code: str) -> AnalyzeRule | None:
        """Get a specific rule by code."""
        return self._rules.get(code)

    def list_enabled(self, config: object = None) -> list[AnalyzeRule]:
        """Return rules enabled under the given config.

        Args:
            config: OmniConfig or similar with analyze.level and analyze.rules dict.

        Returns:
            List of enabled rules in code order.
        """
        level_order = {
            "error": 0,
            "warning": 1,
            "port": 2,
            "style": 3,
        }
        min_level = "port"
        rule_overrides: dict[str, str] = {}

        if config and hasattr(config, 'analyze'):
            min_level = getattr(config.analyze, 'level', 'port')
            rule_overrides = dict(getattr(config.analyze, 'rules', {}))

        enabled = []
        for rule in sorted(self._rules.values(), key=lambda r: r.code):
            effective = rule_overrides.get(rule.code, "warn")
            if effective == "allow":
                continue
            if effective == "deny":
                enabled.append(rule)
                continue
            # Check severity threshold
            rule_level = rule.severity.value
            if level_order.get(rule_level, 99) <= level_order.get(min_level, 99):
                enabled.append(rule)
        return enabled

    def list_all(self) -> list[AnalyzeRule]:
        """Return all registered rules."""
        return sorted(self._rules.values(), key=lambda r: r.code)

    @property
    def count(self) -> int:
        return len(self._rules)


def build_default_registry() -> RuleRegistry:
    """Build the default rule registry with all built-in rules.

    This is the main entry point for the analyze pipeline.
    Rules are added as they are implemented; unimplemented rules
    are registered as stubs that return empty diagnostics.
    """
    registry = RuleRegistry()

    # P01xx portability rules
    from omnilang.analyze.rules.portability import (
        p0101_idiv, p0102_float_cmp, p0103_nan, p0104_string_enc,
        p0105_float_precision, p0106_closure_capture, p0107_async_scheduling,
        p0109_overflow, p0110_divzero, p0111_generics_js,
    )
    from omnilang.analyze.rules.type import (
        e0101_type_mismatch, e0102_missing_field, e0103_extra_field,
        e0105_match_exhaust,
    )
    from omnilang.analyze.rules.async_rules import (
        e0201_await_non_async, e0202_async_without_await, e0203_unhandled_await,
    )
    from omnilang.analyze.rules.ownership import e0306_closure_escape
    from omnilang.analyze.rules.style import s0101_unused_variable, s0102_unused_import, s0107_structural, s0108_backslash_path
    from omnilang.analyze.rules.performance import f0101_large_copy
    from omnilang.analyze.rules.security import sec0101_sql_inject

    for rule_cls in [
        # Type errors
        e0101_type_mismatch.E0101Rule, e0102_missing_field.E0102Rule,
        e0103_extra_field.E0103Rule, e0105_match_exhaust.E0105Rule,
        # Async rules
        e0201_await_non_async.E0201Rule, e0202_async_without_await.E0202Rule,
        e0203_unhandled_await.E0203Rule,
        # Ownership
        e0306_closure_escape.E0306Rule,
        # Portability
        p0101_idiv.P0101Rule, p0102_float_cmp.P0102Rule, p0103_nan.P0103Rule,
        p0104_string_enc.P0104Rule, p0105_float_precision.P0105Rule,
        p0106_closure_capture.P0106Rule, p0107_async_scheduling.P0107Rule,
        p0109_overflow.P0109Rule, p0110_divzero.P0110Rule, p0111_generics_js.P0111Rule,
        # Performance
        f0101_large_copy.F0101Rule,
        # Style
        s0101_unused_variable.S0101Rule, s0102_unused_import.S0102Rule,
        s0107_structural.S0107Rule, s0108_backslash_path.S0108Rule,
        # Security
        sec0101_sql_inject.SEC0101Rule,
    ]:
        registry.register(rule_cls())

    return registry
