"""Configuration system for OmniLang projects.

Priority: CLI arguments > .omnilang.toml (project-level) > defaults.

Handles TOML parsing with Python 3.10+ compatibility (tomllib / tomli fallback).
"""

from dataclasses import dataclass, field
from pathlib import Path


# ── Target-specific configs ───────────────────────────

@dataclass
class TargetJsConfig:
    """JavaScript/TypeScript target configuration."""
    mode: str = "plain"          # "plain" | "typescript"
    module: str = "esm"          # "esm" | "commonjs"
    private_fields: bool = True  # Use #private fields in class output


@dataclass
class TargetRustConfig:
    """Rust target configuration."""
    async_runtime: str = "tokio"  # "tokio" | "async-std"


@dataclass
class TargetPythonConfig:
    """Python target configuration.

    min_version drives code generation branching:
        "3.10" (default) — union types with `str | int`, match/case
        "3.8" or "3.9"  — downgrade to Union[str, int], if/elif/else (v0.3)
    """
    min_version: str = "3.10"


@dataclass
class TargetDartConfig:
    """Dart target configuration."""
    pass  # Reserved for future Dart-specific options


# ── Analyze config ────────────────────────────────────

@dataclass
class AnalyzeConfig:
    """Static analysis configuration.

    level — Minimum severity to report:
        "error"   — Only errors
        "warning" — Errors + warnings
        "port"    — Errors + warnings + portability (default)
        "style"   — All diagnostics including style/performance

    rules — Per-rule severity override:
        { "P0101": "allow", "E0101": "deny" }
    """
    level: str = "port"         # "error" | "warning" | "port" | "style"
    rules: dict[str, str] = field(default_factory=dict)


# ── Source / output sub-configs ───────────────────────

@dataclass
class SourceConfig:
    """[source] section — where source files live."""
    dir: str = "src"


@dataclass
class OutputConfig:
    """[output] section — where compiled output goes."""
    dir: str = "build"


# ── Project config ────────────────────────────────────

@dataclass
class OmniConfig:
    """Root configuration for an OmniLang project.

    Loaded from .omnilang.toml at project root.
    """
    project_name: str = ""
    project_version: str = "0.1.0"

    # Target selections
    targets: list[str] = field(default_factory=lambda: ["rust", "dart", "js", "python"])
    ui_target: str = ""

    # Alias mapping: "@alias" → "./path"
    aliases: dict[str, str] = field(default_factory=dict)

    # Target-specific configs
    js: TargetJsConfig = field(default_factory=TargetJsConfig)
    rust: TargetRustConfig = field(default_factory=TargetRustConfig)
    python: TargetPythonConfig = field(default_factory=TargetPythonConfig)
    dart: TargetDartConfig = field(default_factory=TargetDartConfig)

    # Analyze
    analyze: AnalyzeConfig = field(default_factory=AnalyzeConfig)

    # Source / output
    source: SourceConfig = field(default_factory=SourceConfig)
    output: OutputConfig = field(default_factory=OutputConfig)

    # Legacy top-level fields (deprecated — use source/output sub-configs)
    source_dir: str = ""
    output_dir: str = ""


# ── Default .omnilang.toml template ───────────────────

DEFAULT_CONFIG_TEMPLATE = """\
[project]
name = "{name}"
version = "0.1.0"
# targets = ["rust", "dart", "js", "python"]
# ui_target = "react"        # "react" | "vue" | "dioxus" | "flutter" | "omniview"

# ── Path aliases ──────────────────────────────────────

[aliases]
# "@components" = "./src/components"
# "@lib" = "./src/lib"

# ── Source / output ───────────────────────────────────

[source]
dir = "src"

[output]
dir = "build"

# ── Target-specific options ───────────────────────────

[target.js]
mode = "plain"           # "plain" | "typescript"
module = "esm"           # "esm" | "commonjs"
private_fields = true    # Use #private in class output

[target.rust]
async_runtime = "tokio"  # "tokio" | "async-std"

[target.python]
min_version = "3.10"     # Affects match/case and | type syntax generation

[target.dart]
# No Dart-specific options yet

# ── Static analysis ───────────────────────────────────

[analyze]
# Minimum severity to report: "error" | "warning" | "port" | "style"
level = "port"

[analyze.rules]
# Uncomment to allow or deny specific rules:
# P0101 = "allow"   # cross_lang_div_behavior
# P0110 = "deny"     # potential_divzero
# E0101 = "warn"     # type_mismatch downgrade
"""


# ── TOML parsing ──────────────────────────────────────

def _load_toml(path: Path) -> dict:
    """Parse a TOML file, with Python 3.10 compatibility.

    Uses tomllib (Python 3.11+ stdlib) with tomli fallback.
    """
    try:
        import tomllib
    except ImportError:
        try:
            import tomli as tomllib
        except ImportError:
            raise ImportError(
                "TOML parsing requires tomli package on Python < 3.11. "
                "Install with: pip install tomli"
            ) from None

    with open(path, "rb") as f:
        return tomllib.load(f)


def _parse_target_js(data: dict | None) -> TargetJsConfig:
    """Parse JS target config from TOML dict."""
    if data is None:
        return TargetJsConfig()
    return TargetJsConfig(
        mode=data.get("mode", "plain"),
        module=data.get("module", "esm"),
        private_fields=data.get("private_fields", True),
    )


def _parse_target_rust(data: dict | None) -> TargetRustConfig:
    """Parse Rust target config from TOML dict."""
    if data is None:
        return TargetRustConfig()
    return TargetRustConfig(
        async_runtime=data.get("async_runtime", "tokio"),
    )


def _parse_target_python(data: dict | None) -> TargetPythonConfig:
    """Parse Python target config from TOML dict."""
    if data is None:
        return TargetPythonConfig()
    return TargetPythonConfig(
        min_version=data.get("min_version", "3.10"),
    )


def _parse_target_dart(data: dict | None) -> TargetDartConfig:
    """Parse Dart target config from TOML dict."""
    return TargetDartConfig()


def _parse_analyze(data: dict | None) -> AnalyzeConfig:
    """Parse analyze config from TOML dict."""
    if data is None:
        return AnalyzeConfig()
    return AnalyzeConfig(
        level=data.get("level", "port"),
        rules=data.get("rules", {}),
    )


def _parse_source(data: dict | None) -> SourceConfig:
    """Parse [source] section from TOML dict."""
    if data is None:
        return SourceConfig()
    return SourceConfig(
        dir=data.get("dir", "src"),
    )


def _parse_output(data: dict | None) -> OutputConfig:
    """Parse [output] section from TOML dict."""
    if data is None:
        return OutputConfig()
    return OutputConfig(
        dir=data.get("dir", "build"),
    )


def load_config(path: str | Path | None = None) -> OmniConfig:
    """Load OmniLang project configuration.

    Looks for .omnilang.toml:
        1. At the given path, or
        2. Walking up from the current directory, or
        3. Returns default config if not found.

    Returns:
        OmniConfig with all sections populated.
    """
    config_path = _find_config(path)

    if config_path is None:
        return OmniConfig()

    raw = _load_toml(config_path)

    project = raw.get("project", {})
    target = raw.get("target", {})

    # Parse aliases from [aliases] section
    aliases = dict(raw.get("aliases", {}))

    # Parse source / output sections
    src_cfg = _parse_source(raw.get("source"))
    out_cfg = _parse_output(raw.get("output"))

    config = OmniConfig(
        project_name=project.get("name", ""),
        project_version=project.get("version", "0.1.0"),
        targets=project.get("targets", ["rust", "dart", "js", "python"]),
        ui_target=project.get("ui_target", ""),
        aliases=aliases,
        js=_parse_target_js(target.get("js")),
        rust=_parse_target_rust(target.get("rust")),
        python=_parse_target_python(target.get("python")),
        dart=_parse_target_dart(target.get("dart")),
        analyze=_parse_analyze(raw.get("analyze")),
        source=src_cfg,
        output=out_cfg,
        # Legacy fields (fall back to sub-config sections)
        source_dir=project.get("source_dir", src_cfg.dir),
        output_dir=project.get("output_dir", out_cfg.dir),
    )

    return config


def find_config_dir(start: str | None = None) -> Path | None:
    """Walk upward from start (or cwd) to find a directory containing .omnilang.toml."""
    current = Path(start).resolve() if start else Path.cwd()
    for parent in [current, *current.parents]:
        if (parent / ".omnilang.toml").exists():
            return parent
    return None


def _find_config(path: str | Path | None = None) -> Path | None:
    """Find .omnilang.toml by walking up from path or cwd."""
    if path:
        p = Path(path)
        if p.is_file():
            return p
        if p.is_dir():
            candidate = p / ".omnilang.toml"
            if candidate.exists():
                return candidate
            return None

    # Walk up from current directory
    current = Path.cwd()
    for parent in [current, *current.parents]:
        candidate = parent / ".omnilang.toml"
        if candidate.exists():
            return candidate

    return None


def generate_config(path: str | Path, project_name: str | None = None) -> Path:
    """Generate a default .omnilang.toml at the given directory.

    Args:
        path: Directory path to create the config in.
        project_name: Project name (defaults to directory name).

    Returns:
        Path to the created config file.
    """
    p = Path(path)
    if not p.exists():
        p.mkdir(parents=True, exist_ok=True)

    name = project_name or p.resolve().name
    content = DEFAULT_CONFIG_TEMPLATE.format(name=name)

    config_file = p / ".omnilang.toml"
    with open(config_file, "w", encoding="utf-8") as f:
        f.write(content)

    return config_file


def validate_config(config: OmniConfig) -> list[str]:
    """Validate an OmniConfig and return a list of error messages.

    Returns an empty list if the config is valid.
    """
    errors = []

    # Validate targets
    valid_targets = {"rust", "dart", "js", "python"}
    for t in config.targets:
        if t not in valid_targets:
            errors.append(f"Invalid target '{t}'. Valid: {', '.join(sorted(valid_targets))}")

    # Validate ui_target
    valid_ui_targets = {"", "react", "vue", "dioxus", "flutter", "omniview"}
    if config.ui_target not in valid_ui_targets:
        errors.append(
            f"Invalid ui_target '{config.ui_target}'. "
            f"Valid: {', '.join(sorted(t for t in valid_ui_targets if t))}"
        )

    # Validate analyze level
    valid_levels = {"error", "warning", "port", "style"}
    if config.analyze.level not in valid_levels:
        errors.append(
            f"Invalid analyze.level '{config.analyze.level}'. "
            f"Valid: {', '.join(sorted(valid_levels))}"
        )

    # Validate target-specific settings
    if config.js.mode not in ("plain", "typescript"):
        errors.append(f"Invalid target.js.mode '{config.js.mode}'. Valid: plain, typescript")

    if config.js.module not in ("esm", "commonjs"):
        errors.append(f"Invalid target.js.module '{config.js.module}'. Valid: esm, commonjs")

    if config.rust.async_runtime not in ("tokio", "async-std"):
        errors.append(
            f"Invalid target.rust.async_runtime '{config.rust.async_runtime}'. "
            f"Valid: tokio, async-std"
        )

    # Validate python min_version format
    py_ver = config.python.min_version
    if not (py_ver.startswith("3.") and len(py_ver) >= 4):
        errors.append(f"Invalid target.python.min_version '{py_ver}'. Expected format: '3.10'")

    # Validate rule overrides
    for code, level in config.analyze.rules.items():
        if level not in ("allow", "warn", "deny"):
            errors.append(
                f"Invalid rule override '{code} = \"{level}\"'. "
                f"Valid: allow, warn, deny"
            )

    return errors
