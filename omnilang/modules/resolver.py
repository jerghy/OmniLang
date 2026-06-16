"""Module path resolver for OmniLang.

Resolves import paths to file system paths following these rules:
    "./foo.omni"   → relative to importing file
    "../lib/bar.omni" → relative parent traversal
    "@alias/pkg.omni" → path alias from .omnilang.toml config

The .omni extension is MANDATORY — bare imports without it are rejected.
All paths use forward slash `/` internally; pathlib handles platform conversion.
"""

from pathlib import Path
from dataclasses import dataclass


@dataclass
class ModulePath:
    """Resolved module path information."""
    path: Path           # Absolute file path (.omni)
    display: str         # Original import string
    is_std: bool = False # True if from standard library


class ModuleResolver:
    """Resolves OmniLang import paths to .omni file locations.

    The resolver enforces that all import paths carry a .omni extension.
    The sole exception is the bare string 'omniview', which is recognised
    as a built-in library marker and returns None (the caller handles it).
    """

    def __init__(self, project_root: str | Path, aliases: dict[str, str] | None = None,
                 std_path: str | Path | None = None):
        self.project_root = Path(project_root).resolve()
        self.aliases = aliases or {}
        self.std_path = Path(std_path) if std_path else Path(__file__).parent.parent.parent / "stdlib"

    def resolve(self, import_path: str, from_file: str | Path | None = None) -> ModulePath | None:
        """Resolve an import path to a ModulePath.

        The import path MUST have a .omni extension.  This is enforced here.
        Returns None if the file cannot be found or the path is invalid.

        Args:
            import_path: The import string (e.g., "./foo.omni", "../lib/bar.omni")
            from_file: The file containing the import (for relative resolution)
        """
        # Validate: .omni extension is mandatory
        if not import_path.endswith(".omni"):
            # Allow bare 'omniview' as a library marker (not a file import)
            if import_path == "omniview":
                return None  # handled by caller
            return None  # invalid — missing .omni extension

        # Path aliases: @alias/pkg.omni
        if import_path.startswith("@"):
            parts = import_path.split("/", 1)
            alias_key = parts[0]
            if len(parts) == 1:
                return None
            subpath = parts[1]

            if alias_key not in self.aliases:
                return None  # alias not found

            base = self.aliases[alias_key]
            resolved = Path(base) / subpath
            return ModulePath(
                path=resolved.resolve(),
                display=import_path,
                is_std=(alias_key == "@std"),
            )

        # Relative paths: ./foo.omni or ../foo.omni
        if import_path.startswith("./") or import_path.startswith("../"):
            base = Path(from_file).parent if from_file else self.project_root
            resolved = (base / import_path).resolve()
            return ModulePath(
                path=resolved,
                display=import_path,
                is_std=False,
            )

        # Project-relative (fallback)
        resolved = (self.project_root / import_path).resolve()
        return ModulePath(
            path=resolved,
            display=import_path,
            is_std=False,
        )

    def add_alias(self, name: str, target: str):
        """Register a path alias (from .omnilang.toml [aliases] section)."""
        self.aliases[name] = target

    def add_aliases(self, aliases: dict[str, str]):
        """Register multiple path aliases at once."""
        self.aliases.update(aliases)
