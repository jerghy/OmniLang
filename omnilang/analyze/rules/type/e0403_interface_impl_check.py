"""E0403: Class does not fully implement its interfaces.

Checks that every class implementing an interface provides all required methods.
"""
from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import ClassDef, InterfaceDef


class E0403Rule(AnalyzeRule):
    code = "E0403"
    name = "interface_not_implemented"
    severity = Severity.ERROR
    category = "type"
    message = "Class `{class_name}` does not implement interface method `{method}` from `{iface}`"
    help = "Add the missing method implementation to the class, or remove the interface from the implements clause."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics = []

        # Collect all interfaces (from tagged unions that serve as interfaces, and explicit InterfaceDef)
        interfaces: dict[str, InterfaceDef] = {}
        for item in ctx.program.tagged_unions:
            # Tagged unions with variants that have no fields act like interfaces
            pass

        # Collect class method names
        for cls in ctx.program.classes:
            class_methods = set()
            for member in cls.members:
                if member.func_def:
                    class_methods.add(member.func_def.name)
                elif member.name:
                    class_methods.add(member.name)

            # Check implements clause
            for iface_name in cls.implements:
                # Look for interface definition
                iface = self._find_interface(iface_name, ctx)
                if iface:
                    for im in iface.members:
                        if im.name not in class_methods:
                            span = cls.span
                            loc = SourceLocation(
                                file=self._filename(),
                                line=span.line if span else 1,
                                column=span.col if span else 1,
                                span=(span.line, span.col, span.end_line, span.end_col) if span else (1, 1, 1, 1),
                            )
                            diagnostics.append(Diagnostic(
                                code=self.code, severity=self.severity,
                                category=self.category,
                                message=self.message.format(
                                    class_name=cls.name,
                                    method=im.name,
                                    iface=iface_name,
                                ),
                                location=loc,
                                help=self.help,
                            ))

        return diagnostics

    def _filename(self):
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _find_interface(self, name: str, ctx: AnalyzeContext) -> InterfaceDef | None:
        """Find an interface by name from the program."""
        # Check Program's classes/interfaces — stored in tagged_unions for now
        # or look through the AST
        for func in ctx.program.functions:
            for stmt in (func.body.stmts if func.body else []):
                if isinstance(stmt, InterfaceDef) and stmt.name == name:
                    return stmt

        # Also look in program-level — interface definitions are parsed
        # but stored differently. Check if there's an interfaces list
        if hasattr(ctx.program, 'interfaces'):
            for iface in ctx.program.interfaces:
                if iface.name == name:
                    return iface

        # Check tagged unions that might serve as interfaces
        for tu in ctx.program.tagged_unions:
            if tu.name == name:
                # Convert tagged union to interface-like check
                class FakeInterfaceMember:
                    def __init__(self, n):
                        self.name = n
                        self.is_method = False
                iface = InterfaceDef(name=name, members=[])
                for v in tu.variants:
                    iface.members.append(FakeInterfaceMember(v.name))
                return iface

        return None
