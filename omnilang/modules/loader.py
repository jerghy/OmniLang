"""Multi-file loader for OmniLang projects.

Recursively resolves imports from an entry file, parses each module,
and returns a list of (path, AST) pairs ready for compilation.
"""

from pathlib import Path
from omnilang.parser import parse_file
from omnilang.modules.resolver import ModuleResolver
from omnilang.modules.graph import ModuleGraph


class ModuleLoader:
    """Loads an OmniLang project by recursively resolving imports."""

    def __init__(self, project_root: str | Path, aliases: dict[str, str] | None = None):
        self.resolver = ModuleResolver(project_root, aliases=aliases)
        self.graph = ModuleGraph()
        self._loaded: dict[str, object] = {}  # path → Program AST

    def load_project(self, entry_file: str | Path) -> list[tuple[str, object]]:
        """Load all modules starting from the entry file.

        Returns list of (path, AST) pairs in compilation order.
        Raises ValueError if cycles are detected.
        """
        entry_path = str(Path(entry_file).resolve())
        self._load_recursive(entry_path)

        # Check for cycles
        cycles = self.graph.detect_cycles()
        if cycles:
            cycle_strs = [" → ".join(c + [c[0]]) for c in cycles]
            raise ValueError(f"Circular dependency detected (E0501): {'; '.join(cycle_strs)}")

        # Return in compilation order
        order = self.graph.compilation_order()
        return [(path, self._loaded[path]) for path in order if path in self._loaded]

    def _load_recursive(self, file_path: str):
        """Recursively load a module and its imports."""
        if file_path in self._loaded:
            return

        # Parse the file
        prog = parse_file(file_path)
        self._loaded[file_path] = prog

        # Extract imports (simplified: scan AST for import information)
        imports = self._extract_imports(prog, file_path)
        exports = self._extract_exports(prog)

        self.graph.add_module(file_path, imports, exports)

        # Recurse into imports
        for imp_path in imports:
            if imp_path not in self._loaded:
                self._load_recursive(imp_path)

    def _extract_imports(self, prog, current_file: str) -> list[str]:
        """Extract imported module paths from an AST.

        Scans for ImportStmt nodes (to be added in Phase 5 AST).
        For now: use a simple placeholder that returns empty list.
        """
        imports = []
        for item in getattr(prog, 'functions', []):
            pass  # imports are separate top-level items
        return imports

    def _extract_exports(self, prog) -> list[str]:
        """Extract exported symbol names from an AST."""
        exports = []
        for item in getattr(prog, 'functions', []):
            if getattr(item, 'is_exported', False):
                exports.append(item.name)
        return exports
