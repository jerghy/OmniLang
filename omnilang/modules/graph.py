"""Module dependency graph with cycle detection (E0501).

Uses Tarjan's SCC algorithm to find circular dependencies.
Topological sort determines compilation order.
"""

from dataclasses import dataclass, field


@dataclass
class ModuleNode:
    """A node in the dependency graph representing one .omni module."""
    path: str        # Absolute file path
    imports: list[str] = field(default_factory=list)  # Imported module paths
    exported: list[str] = field(default_factory=list)  # Exported symbol names


class ModuleGraph:
    """Dependency graph for OmniLang modules."""

    def __init__(self):
        self._nodes: dict[str, ModuleNode] = {}

    def add_module(self, path: str, imports: list[str], exports: list[str] | None = None):
        """Add or update a module node."""
        node = self._nodes.get(path)
        if node:
            node.imports = imports
            if exports:
                node.exported = exports
        else:
            self._nodes[path] = ModuleNode(path=path, imports=imports, exported=exports or [])

    def detect_cycles(self) -> list[list[str]]:
        """Find circular dependencies using Tarjan's algorithm.

        Returns list of cycles, each cycle is a list of module paths.
        Empty list means no cycles.
        """
        index_counter = [0]
        indices: dict[str, int] = {}
        lowlinks: dict[str, int] = {}
        on_stack: set[str] = set()
        stack: list[str] = []
        cycles: list[list[str]] = []

        def strongconnect(v: str):
            indices[v] = index_counter[0]
            lowlinks[v] = index_counter[0]
            index_counter[0] += 1
            stack.append(v)
            on_stack.add(v)

            node = self._nodes.get(v)
            if node:
                for w in node.imports:
                    if w not in indices:
                        strongconnect(w)
                        lowlinks[v] = min(lowlinks[v], lowlinks[w])
                    elif w in on_stack:
                        lowlinks[v] = min(lowlinks[v], indices[w])

            if lowlinks[v] == indices[v]:
                scc = []
                while True:
                    w = stack.pop()
                    on_stack.discard(w)
                    scc.append(w)
                    if w == v:
                        break
                if len(scc) > 1:
                    cycles.append(scc)

        for node_path in self._nodes:
            if node_path not in indices:
                strongconnect(node_path)

        return cycles

    def compilation_order(self) -> list[str]:
        """Return modules in topological order (dependencies first)."""
        visited: set[str] = set()
        order: list[str] = []

        def visit(path: str):
            if path in visited:
                return
            visited.add(path)
            node = self._nodes.get(path)
            if node:
                for dep in node.imports:
                    if dep in self._nodes:
                        visit(dep)
            order.append(path)

        for path in self._nodes:
            visit(path)

        return order
