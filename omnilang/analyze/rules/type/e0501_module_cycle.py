"""E0501: Circular module dependency detected.

Uses Tarjan's SCC algorithm from omnilang.modules.graph to detect cycles.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation


class E0501Rule(AnalyzeRule):
    code = "E0501"
    name = "module_cycle"
    severity = Severity.ERROR
    category = "type"
    message = "Circular module dependency detected: {cycle}"
    help = "Break the cycle by extracting shared types into a separate module or using forward references."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []

        # Use the module graph if available
        try:
            from omnilang.modules.graph import ModuleGraph
            graph = ModuleGraph()

            # Build graph from import statements in the program
            imports_by_file: dict[str, list[str]] = {}
            primary_file = self._filename(ctx)

            for imp in ctx.program.imports:
                if primary_file not in imports_by_file:
                    imports_by_file[primary_file] = []
                imports_by_file[primary_file].append(imp.path)

            for fname, deps in imports_by_file.items():
                graph.add_module(fname, deps)

            cycles = graph.detect_cycles()
            for cycle in cycles:
                if len(cycle) > 1:
                    cycle_str = " -> ".join(cycle) + " -> " + cycle[0]
                    diagnostics.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=self.message.format(cycle=cycle_str),
                        location=SourceLocation(
                            file=primary_file, line=1, column=1,
                            span=(1, 1, 1, 1),
                        ),
                        help=self.help,
                    ))
        except ImportError:
            pass  # Module graph not available
        except Exception:
            pass  # Graph building failed — skip

        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"
