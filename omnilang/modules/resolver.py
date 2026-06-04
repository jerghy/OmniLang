"""Module path resolver for OmniLang.

Resolves import paths to file system paths following these rules:
    "./foo"      → relative to importing file
    "../lib/foo" → relative parent traversal
    "std/fs"     → standard library path
    "@alias/pkg" → path alias from .omnilang.toml config

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
    """Resolves OmniLang import paths to .omni file locations."""

    def __init__(self, project_root: str | Path, aliases: dict[str, str] | None = None,
                 std_path: str | Path | None = None):
        self.project_root = Path(project_root).resolve()
        self.aliases = aliases or {}
        self.std_path = Path(std_path) if std_path else Path(__file__).parent.parent.parent / "stdlib"

    def resolve(self, import_path: str, from_file: str | Path | None = None) -> ModulePath | None:
        """Resolve an import path to an absolute .omni file path.

        Args:
            import_path: The import string (e.g., "./foo", "../lib/bar", "std/fs")
            from_file: The file containing the import (for relative resolution)

        Returns:
            ModulePath with absolute file path, or None if not found.
        """
        # 1. Path aliases
        if import_path.startswith("@"):
            alias_key, _, rest = import_path[1:].partition("/")
            # Look up with @ prefix
            full_key = f"@{alias_key}"
            if full_key in self.aliases:
                base = Path(self.aliases[full_key]).resolve()
                resolved = (base / rest).resolve() if rest else base
                return ModulePath(path=self._ensure_omni(resolved), display=import_path)
            return None

        # 2. Relative paths
        if import_path.startswith("./") or import_path.startswith("../"):
            base = Path(from_file).parent if from_file else self.project_root
            resolved = (base / import_path).resolve()
            return ModulePath(path=self._ensure_omni(resolved), display=import_path)

        # 3. Standard library
        resolved = self.std_path / import_path
        omni_path = self._ensure_omni(resolved)
        if omni_path.exists():
            return ModulePath(path=omni_path, display=import_path, is_std=True)

        # 4. Project-relative
        resolved = (self.project_root / import_path).resolve()
        omni_path = self._ensure_omni(resolved)
        return ModulePath(path=omni_path, display=import_path) if omni_path.exists() else ModulePath(path=omni_path, display=import_path)

    def _ensure_omni(self, path: Path) -> Path:
        """Ensure the path has .omni extension."""
        if path.suffix == ".omni":
            return path
        return path.with_suffix(".omni")
