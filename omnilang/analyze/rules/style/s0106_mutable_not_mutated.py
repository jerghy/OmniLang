"""S0106: Mutable variable never mutated.

Suggests removing the `mut` keyword from variables that are declared
as mutable but never reassigned.  Unnecessary mutability makes it
harder for humans and compilers to reason about the code.
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import VarDecl, Assign, Block, Destructure


class S0106Rule(AnalyzeRule):
    code = "S0106"
    name = "mutable_not_mutated"
    severity = Severity.STYLE
    category = "style"
    message = "Mutable variable `{name}` is never mutated"
    help = "Remove the `mut` keyword: `let {name}` instead of `let mut {name}`."

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        diagnostics = []

        for func in ctx.program.functions:
            if func.body:
                # Per-function analysis to respect shadowing
                mutable_decls: dict[str, VarDecl] = {}
                mutated: set[str] = set()

                self._analyze_block(
                    func.body, mutable_decls, mutated,
                    outer_mutable=mutable_decls, outer_mutated=mutated,
                )

                # Report mutable variables that were never assigned
                for name, decl in mutable_decls.items():
                    if name not in mutated:
                        diagnostics.append(self._make_diagnostic(decl, ctx))

        return diagnostics

    @staticmethod
    def _filename(ctx: AnalyzeContext) -> str:
        if ctx.source_lines:
            return next(iter(ctx.source_lines.keys()), "<input>")
        return "<input>"

    def _make_diagnostic(self, decl: VarDecl, ctx: AnalyzeContext) -> Diagnostic:
        span = decl.span
        line = span.line if span else 1
        col = span.col if span else 1
        loc = SourceLocation(
            file=self._filename(ctx),
            line=line, column=col,
            span=(line, col, span.end_line if span else 1, span.end_col if span else 1),
        )
        return Diagnostic(
            code=self.code, severity=self.severity,
            category=self.category,
            message=self.message.format(name=decl.name),
            location=loc,
            help=self.help.format(name=decl.name),
        )

    def _analyze_block(
        self,
        block: Block,
        mutable_decls: dict[str, VarDecl],
        mutated: set[str],
        outer_mutable: dict[str, VarDecl],
        outer_mutated: set[str],
    ) -> None:
        """Walk a block collecting mutable declarations and mutations.

        Parameters are mutated *in-place* — they are the accumulation
        dictionaries for the enclosing scope.
        """
        for stmt in block.stmts:
            if isinstance(stmt, VarDecl):
                if stmt.mutable:
                    mutable_decls[stmt.name] = stmt
            elif isinstance(stmt, Destructure):
                if stmt.mutable:
                    # For destructured patterns the variable names are not
                    # directly on the stmt; we handle this conservatively.
                    pass
            elif isinstance(stmt, Assign):
                mutated.add(stmt.target)

            # Recurse into child blocks — pass the *same* accumulators so
            # the child sees the parent's scope.
            for child_attr in ('body', 'then_body', 'else_body'):
                child = getattr(stmt, child_attr, None)
                if isinstance(child, Block):
                    self._analyze_block(
                        child, mutable_decls, mutated,
                        outer_mutable, outer_mutated,
                    )

        # Trailing expressions cannot mutate variables by themselves;
        # mutations happen through Assign statements inside the block.
