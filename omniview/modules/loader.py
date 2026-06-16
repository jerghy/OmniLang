"""OmniView-aware module loader.

Loads .omni files and resolves imports recursively.
Note: Python AST parser has been removed; import resolution uses regex.
All compilation now goes through the Rust compiler.
"""

from pathlib import Path
from dataclasses import dataclass, field
import re as _re

from omnilang.modules.resolver import ModuleResolver
from omnilang.modules.graph import ModuleGraph


# Minimal placeholder for the old Program object (Python AST removed).
class _MinimalProgram:
    imports = []
    components = []
    stores = []
    lib_imports = []


@dataclass
class LoadedModule:
    """A loaded and parsed OmniView module."""
    path: str                    # Absolute file path
    source: str                     # Raw source text
    program: object                 # Parsed OmniView Program AST
    ir_map: dict | None = None     # Lowered IR (name → IR dict)
    imports: list[str] = field(default_factory=list)   # Resolved import paths
    exported: list[str] = field(default_factory=list)  # Exported component/store names
    import_lines: dict[str, int] = field(default_factory=dict)  # resolved_path → source line


class CircularImportError(Exception):
    """Raised when circular imports are detected between .omni modules.

    Provides a human-readable chain showing the import path cycle with
    source line numbers for each import statement in the cycle.
    """

    def __init__(self, chain: list[str], graph: ModuleGraph | None = None):
        self.chain = chain
        short_names = [Path(p).name for p in chain]
        # Clean cycle: don't duplicate closing node if chain already loops
        if len(short_names) >= 2 and short_names[-1] == short_names[0]:
            cycle_str = " → ".join(short_names)
        else:
            cycle_str = " → ".join(short_names) + f" → {short_names[0]}"

        # Build per-file details with line numbers where available
        details = []
        for i, file_path in enumerate(chain):
            next_file = chain[(i + 1) % len(chain)]
            if i == len(chain) - 1 and next_file == chain[0]:
                break  # skip self-closing line
            fname = Path(file_path).name
            line_info = ""
            if graph:
                node = graph._nodes.get(file_path)
                if node and next_file in node.import_lines:
                    line_info = f":{node.import_lines[next_file]}"
            details.append(f"    {fname}{line_info} imports {Path(next_file).name}")

        super().__init__(
            f"Circular import detected:\n"
            f"  {cycle_str}\n"
            + "\n".join(details) +
            f"\n  Break the cycle by:\n"
            f"  1. Extract shared types/stores into a third module\n"
            f"  2. Restructure to a linear dependency chain\n"
            f"  3. Use forward references via store interfaces"
        )


class ModuleLoader:
    """Loads and resolves OmniView .omni files with dependency tracking."""

    def __init__(self, project_root: str | Path):
        self.project_root = Path(project_root).resolve()
        self.resolver = ModuleResolver(self.project_root)
        self.graph = ModuleGraph()
        self._loaded: dict[str, LoadedModule] = {}
        self._loading: set[str] = set()  # tracks files currently being processed
        self._import_stack: list[str] = []  # tracks import chain [entry, ..., current]

    def load(self, entry_file: str | Path) -> list[LoadedModule]:
        """Load an entry file and all its dependencies recursively.

        Args:
            entry_file: Path to the main .omni file

        Returns:
            List of LoadedModule in compilation order (dependencies first).

        Raises:
            CircularImportError: If circular dependencies are detected.
            FileNotFoundError: If an imported module cannot be found.
        """
        entry_path = str(Path(entry_file).resolve())
        self._load_recursive(entry_path)

        # Post-load cycle detection via Tarjan's SCC
        cycles = self.graph.detect_cycles()
        if cycles:
            for cycle in cycles:
                # cycle is a list of paths forming a strongly connected component
                if len(cycle) >= 2:
                    raise CircularImportError(cycle, self.graph)

        # Return in compilation order (topological sort)
        order = self.graph.compilation_order()
        return [self._loaded[p] for p in order if p in self._loaded]

    def build_source_map(self, entry_file: str | Path) -> tuple[dict[str, str], dict[str, str]]:
        """Load all transitive dependencies and return (source_map, path_mapping).

        source_map: {resolved_absolute_path: source_text}
        path_mapping: {original_import_path: resolved_absolute_path}
        """
        self.load(entry_file)  # populate self._loaded

        source_map: dict[str, str] = {}
        path_mapping: dict[str, str] = {}

        for file_path, module in self._loaded.items():
            abs_path = str(Path(file_path).resolve())
            source_map[abs_path] = module.source

            # Use original import paths from the AST to build the path mapping
            for imp in getattr(module.program, 'imports', []):
                imp_path = getattr(imp, 'path', '')
                if imp_path:
                    resolved = self.resolver.resolve(imp_path, from_file=file_path)
                    if resolved:
                        path_mapping[imp_path] = str(resolved.path.resolve())

        return source_map, path_mapping

    def _is_view_file(self, source: str) -> bool:
        """Check if a .omni source file is a view (UI) file.

        A file is a view file if it contains `import omniview;`.
        """
        return 'import omniview' in source

    def _check_boundary(self, from_file: str, to_file: str, to_source: str):
        """Enforce: logic files cannot import view files.

        Raises ValueError if a logic file tries to import a view file.
        """
        # The importer may not be in _loaded yet (still being processed),
        # so read its source directly from disk.
        from_source = Path(from_file).read_text(encoding="utf-8")

        from_is_view = self._is_view_file(from_source)
        to_is_view = self._is_view_file(to_source)

        if not from_is_view and to_is_view:
            raise ValueError(
                f"logic file '{Path(from_file).name}' cannot import view file "
                f"'{Path(to_file).name}'. "
                f"View files can import logic files, but not the reverse."
            )

    def _load_recursive(self, file_path: str) -> LoadedModule:
        """Load a single file and recursively load its imports.

        Uses _loading set + _import_stack to detect cycles during recursive
        descent before infinite recursion occurs.
        """
        # Already loaded — return cached
        if file_path in self._loaded:
            return self._loaded[file_path]

        # Currently being loaded elsewhere in the stack — circular dependency!
        if file_path in self._loading:
            # Build the cycle chain from the import stack
            # Find where file_path appears in the stack to know the cycle start
            try:
                cycle_start = self._import_stack.index(file_path)
            except ValueError:
                cycle_start = 0
            chain = self._import_stack[cycle_start:] + [file_path]
            raise CircularImportError(chain, self.graph)

        if not Path(file_path).exists():
            raise FileNotFoundError(f"Module not found: {file_path}")

        # Mark as in-progress and push onto import stack
        self._loading.add(file_path)
        self._import_stack.append(file_path)

        try:
            # Read source
            with open(file_path, "r", encoding="utf-8") as f:
                source = f.read()

            # Note: Python AST parser has been removed. All compilation
            # goes through the Rust compiler. For module loading, we extract
            # imports via regex as a lightweight alternative.
            # Extract imports via regex
            resolved_imports = []
            import_lines = {}
            exported_names = []

            # Match: import { Name } from "./path";
            import_pattern = _re.compile(r'import\s+\{(?:[^}]+)\}\s+from\s+"([^"]+)"')
            for m in import_pattern.finditer(source):
                imp_path = m.group(1)
                resolved = self.resolver.resolve(imp_path, from_file=file_path)
                if resolved:
                    resolved_path = str(resolved.path)
                    resolved_imports.append(resolved_path)
                    import_lines[resolved_path] = source[:m.start()].count('\n') + 1
                    dep_source = Path(resolved_path).read_text(encoding="utf-8")
                    self._check_boundary(file_path, resolved_path, dep_source)
                    self._load_recursive(resolved_path)

            # Match: export fn/component/store names
            export_pattern = _re.compile(r'(?:export\s+)?(?:component|store|fn)\s+(\w+)')
            for m in export_pattern.finditer(source):
                name = m.group(1)
                if name not in exported_names:
                    exported_names.append(name)

            # Build loaded module with a minimal program placeholder
            program = _MinimalProgram()
            module = LoadedModule(
                path=file_path,
                source=source,
                program=program,
                imports=resolved_imports,
                exported=exported_names,
                import_lines=import_lines,
            )
            self._loaded[file_path] = module

            # Register in dependency graph for post-load cycle detection
            self.graph.add_module(file_path, resolved_imports, exported_names, import_lines)

            return module

        finally:
            # Always remove from tracking structures, even on error
            self._loading.discard(file_path)
            if self._import_stack and self._import_stack[-1] == file_path:
                self._import_stack.pop()

    def get_export_map(self) -> dict[str, str]:
        """Build a map of exported symbol → source file path.

        Returns:
            Dict mapping component/store names to their file paths.
        """
        export_map = {}
        for mod in self._loaded.values():
            for name in mod.exported:
                export_map[name] = mod.path
        return export_map

    def validate_no_cycles(self) -> None:
        """Explicitly validate for circular imports and raise if found.

        Useful as a standalone check before compilation.
        Raises CircularImportError if any cycles are detected.
        """
        cycles = self.graph.detect_cycles()
        if cycles:
            for cycle in cycles:
                if len(cycle) >= 2:
                    raise CircularImportError(cycle, self.graph)
