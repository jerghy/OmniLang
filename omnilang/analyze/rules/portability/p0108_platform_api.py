"""P0108: Platform-specific API usage.

Certain APIs only exist on specific target platforms (e.g., Windows vs Unix,
browser vs Node.js). This rule warns when such an API is used without a
platform guard, which may cause compilation or runtime failures on other targets.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import FuncCall, MethodCall, VarRef

# Known platform-specific API groups
_PLATFORM_APIS: dict[str, dict[str, list[str]]] = {
    "windows": {
        "methods": ["set_console_title", "get_registry_value", "set_registry_value"],
    },
    "unix": {
        "methods": ["chmod", "chown", "set_umask", "get_umask"],
    },
    "posix": {
        "methods": ["fork", "exec", "pipe", "kill", "signal"],
    },
    "browser": {
        "methods": ["alert", "confirm", "prompt", "localStorage", "sessionStorage",
                     "addEventListener", "removeEventListener"],
    },
    "node": {
        "methods": ["require", "__dirname", "__filename", "process.exit",
                     "process.cwd", "process.env"],
    },
}


class P0108Rule(AnalyzeRule):
    code = "P0108"
    name = "platform_api"
    severity = Severity.PORT
    category = "portability"
    message = "Platform-specific API `{api}` is restricted to `{platform}` targets"
    help = "Guard platform-specific calls behind `#[cfg(target=...)]` or use the omni/platform abstraction layer."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        self._diags: list[Diagnostic] = []

        # Build a reverse lookup: method_name -> list of platforms
        self._method_to_platforms: dict[str, list[str]] = {}
        for platform, apis in _PLATFORM_APIS.items():
            for method in apis.get("methods", []):
                self._method_to_platforms.setdefault(method, []).append(platform)

        for func in ctx.program.functions:
            if func.body:
                self._walk_block(func.body)

        return self._diags

    def _filename(self) -> str:
        if self._ctx.source_lines:
            return next(iter(self._ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _span_loc(self, span) -> SourceLocation:
        if span:
            return SourceLocation(
                file=self._filename(), line=span.line, column=span.col,
                span=(span.line, span.col, span.end_line, span.end_col),
            )
        return SourceLocation(file=self._filename(), line=1, column=1, span=(1, 1, 1, 1))

    def _walk_block(self, block):
        for stmt in getattr(block, "stmts", []):
            for attr in ["init", "value", "condition", "iterable", "arg", "expr"]:
                val = getattr(stmt, attr, None)
                if val:
                    self._walk_expr(val)
            for child_attr in ["body", "then_body", "else_body"]:
                child = getattr(stmt, child_attr, None)
                if child:
                    self._walk_block(child)
            if hasattr(stmt, "else_ifs"):
                for _, elif_body in getattr(stmt, "else_ifs", []):
                    if elif_body:
                        self._walk_block(elif_body)

        if block.trailing_expr:
            self._walk_expr(block.trailing_expr)

    def _walk_expr(self, expr):
        if isinstance(expr, MethodCall):
            platforms = self._method_to_platforms.get(expr.method)
            if platforms and expr.method not in self._ctx.targets:
                loc = self._span_loc(getattr(expr, "span", None))
                platforms_str = "/".join(platforms)
                self._diags.append(Diagnostic(
                    code=self.code, severity=self.severity,
                    category=self.category,
                    message=self.message.format(api=expr.method, platform=platforms_str),
                    location=loc, help=self.help,
                ))
        elif isinstance(expr, FuncCall):
            if isinstance(expr.func, VarRef):
                platforms = self._method_to_platforms.get(expr.func.name)
                if platforms and expr.func.name not in self._ctx.targets:
                    loc = self._span_loc(getattr(expr, "span", None))
                    platforms_str = "/".join(platforms)
                    self._diags.append(Diagnostic(
                        code=self.code, severity=self.severity,
                        category=self.category,
                        message=self.message.format(api=expr.func.name, platform=platforms_str),
                        location=loc, help=self.help,
                    ))

        # Recurse into child expressions
        for fn in getattr(expr, "__dataclass_fields__", {}):
            val = getattr(expr, fn)
            if hasattr(val, "__dataclass_fields__") and val is not None:
                self._walk_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if hasattr(item, "__dataclass_fields__"):
                        self._walk_expr(item)
