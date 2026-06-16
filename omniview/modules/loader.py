"""OmniView-aware module loader.

Loads and parses .omni files through the full OmniView pipeline:
preprocess → parse → transform → semantic analysis → lower to IR.

Handles recursive import resolution and multi-file dependency tracking.
Detects and reports circular imports with clear diagnostic messages.
"""

from pathlib import Path
from dataclasses import dataclass, field
import re

from omnilang.modules.resolver import ModuleResolver
from omnilang.modules.graph import ModuleGraph


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

            # Parse through OmniView pipeline
            from omniview.preprocessor import preprocess
            from omnilang.lexer import create_omniview_parser
            from omniview.parser import OmniViewTransformer

            pp = preprocess(source)
            parser = create_omniview_parser()
            tree = parser.parse(pp.processed_source)
            transformer = OmniViewTransformer(block_map=pp.blocks)
            program = transformer.transform(tree)

            # Extract imports and exports
            resolved_imports = []
            import_lines = {}  # import_path → source line number for diagnostics
            exported_names = []

            for imp in getattr(program, 'imports', []):
                imp_path = getattr(imp, 'path', '')
                resolved = self.resolver.resolve(imp_path, from_file=file_path)
                if resolved:
                    resolved_path = str(resolved.path)
                    resolved_imports.append(resolved_path)
                    # Record source line for diagnostics
                    imp_span = getattr(imp, 'span', None)
                    if imp_span:
                        import_lines[resolved_path] = imp_span.line
                    # Recursively load dependency (cycle detection via _import_stack)
                    self._load_recursive(resolved_path)

            # Collect exported names (components + stores are auto-exported)
            for comp in getattr(program, 'components', []):
                exported_names.append(getattr(comp, 'name', ''))
            for store in getattr(program, 'stores', []):
                exported_names.append(getattr(store, 'name', ''))

            # Build loaded module
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
