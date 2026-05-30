"""Diagnostic suppression system for OmniLang.

Supports:
- Inline comments: #[allow(P0101)], #[deny(P0102)]
- Block-level: #[allow(P0101)] at start of block
- File-level: #[allow(P0101)] at top of file
- Config-level: .omnilang.toml [analyze.rules] overrides
"""

import re
from dataclasses import dataclass, field
from omnilang.analyze.diagnostic import Diagnostic


@dataclass
class SuppressionConfig:
    """Per-scope suppression configuration."""
    allow: set[str] = field(default_factory=set)   # suppressed codes
    deny: set[str] = field(default_factory=set)    # force-enable codes
    level: str = "port"  # minimum severity level


def parse_inline_suppress(line: str) -> SuppressionConfig | None:
    """Parse #[allow(X)] or #[deny(X)] from a source line.

    Returns SuppressionConfig or None if no directive found.
    """
    cfg = SuppressionConfig()

    # Match #[allow(CODE, CODE, ...)]
    allow_match = re.findall(r'#\[allow\(([^)]+)\)\]', line)
    for m in allow_match:
        for code in m.split(','):
            cfg.allow.add(code.strip().upper())

    # Match #[deny(CODE, CODE, ...)]
    deny_match = re.findall(r'#\[deny\(([^)]+)\)\]', line)
    for m in deny_match:
        for code in m.split(','):
            cfg.deny.add(code.strip().upper())

    if cfg.allow or cfg.deny:
        return cfg
    return None


def apply_suppression(
    diagnostics: list[Diagnostic],
    source_lines: dict[str, list[str]],
    config_rules: dict[str, str] | None = None,
) -> list[Diagnostic]:
    """Filter diagnostics based on suppression directives.

    Args:
        diagnostics: Raw diagnostics from rule checks.
        source_lines: Source code keyed by filename.
        config_rules: Rule overrides from .omnilang.toml (code → allow/deny/warn).

    Returns:
        Filtered list of diagnostics.
    """
    if config_rules is None:
        config_rules = {}

    # Build file-level allow/deny sets from config
    config_allow: set[str] = set()
    config_deny: set[str] = set()
    for code, level in config_rules.items():
        if level.lower() == "allow":
            config_allow.add(code.upper())
        elif level.lower() == "deny":
            config_deny.add(code.upper())

    filtered = []
    for diag in diagnostics:
        code = diag.code.upper()

        # Config-level allow suppresses
        if code in config_allow:
            continue

        # Check file-level inline suppression
        filename = diag.location.file
        if filename in source_lines:
            file_suppressed = False
            for line_no in range(len(source_lines[filename])):
                line = source_lines[filename][line_no]
                cfg = parse_inline_suppress(line)
                if cfg and code in cfg.allow:
                    file_suppressed = True
                    break
            if file_suppressed:
                continue

        filtered.append(diag)

    return filtered


def filter_by_severity(
    diagnostics: list[Diagnostic],
    min_level: str = "port",
) -> list[Diagnostic]:
    """Filter diagnostics below a severity threshold.

    Severity ordering: error > warning > port > style > perf > hint > sec
    """
    level_order = {
        "error": 0, "warning": 1, "port": 2,
        "style": 3, "perf": 4, "hint": 5, "sec": 6,
    }
    min_idx = level_order.get(min_level, 2)
    return [d for d in diagnostics
            if level_order.get(d.severity.value, 2) <= min_idx]
