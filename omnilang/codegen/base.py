"""Base class for code generators with shared utilities."""

from omnilang.ast_nodes import (
    Program, FunctionDef, Block,
)


class CodeGenerator:
    """Abstract base class for target language code generators."""

    def __init__(self):
        self.output: list[str] = []
        self.indent_level = 0
        self.type_map: dict = {}
        self.rename_map: dict = {}

    def generate(self, program: Program, rename_map: dict, type_map: dict) -> str:
        """Generate target code. Main entry point."""
        self.output = []
        self.indent_level = 0
        self.rename_map = rename_map
        self.type_map = type_map
        self.program = program  # store for function lookup
        self._gen_program(program)
        return "\n".join(self.output) + "\n"

    def _emit(self, line: str = ""):
        indent = "    " * self.indent_level
        self.output.append(f"{indent}{line}")

    def _indent(self):
        self.indent_level += 1

    def _dedent(self):
        self.indent_level = max(0, self.indent_level - 1)

    # ── Abstract / overridable ───────────────────

    def _gen_program(self, prog: Program):
        raise NotImplementedError

    def _gen_function(self, func: FunctionDef):
        raise NotImplementedError

    def _gen_block(self, block: Block):
        raise NotImplementedError

    def _gen_stmt(self, stmt):
        raise NotImplementedError

    def _gen_expr(self, expr, ctx: str = "default") -> str:
        """ctx: 'default' | 'assign_target' """
        raise NotImplementedError
