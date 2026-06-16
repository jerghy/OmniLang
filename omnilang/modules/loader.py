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

        # Extract imports and exports from the AST
        imports = self._extract_imports(prog)
        export_map = self._extract_exports(prog)

        self.graph.add_module(file_path, imports, list(export_map.keys()))

        # Recurse into imports
        for imp_path in imports:
            if imp_path not in self._loaded:
                self._load_recursive(imp_path)

    def _extract_imports(self, program) -> list[str]:
        """Extract all file paths referenced in import statements.

        Returns list of paths as written in source (before resolution).
        """
        paths: list[str] = []

        # Named imports: import { A } from "./path.omni"
        for imp in getattr(program, 'imports', []):
            paths.append(imp.path)

        # Type-only imports: import type { A } from "./path.omni"
        for imp in getattr(program, 'type_imports', []):
            paths.append(imp.path)

        # Namespace imports: import "./path.omni" as name
        for imp in getattr(program, 'namespace_imports', []):
            paths.append(imp.path)

        # Export-from: export { A } from "./path.omni" — also a dependency
        for exp in getattr(program, 'export_froms', []):
            paths.append(exp.path)

        return paths

    def _extract_exports(self, program) -> dict[str, str]:
        """Build map of exported name -> source file path."""
        exports: dict[str, str] = {}

        # Functions with is_pub=True are exportable from this file
        for func in getattr(program, 'functions', []):
            if getattr(func, 'is_pub', False) or getattr(func, 'is_exported', False):
                exports[func.name] = "(this file)"

        # Structs with is_pub=True
        for struct in getattr(program, 'structs', []):
            if getattr(struct, 'is_pub', False):
                exports[struct.name] = "(this file)"

        # Re-exports: export { Name } from "./other.omni"
        for exp_from in getattr(program, 'export_froms', []):
            for name in exp_from.names:
                exports[name] = exp_from.path

        return exports
