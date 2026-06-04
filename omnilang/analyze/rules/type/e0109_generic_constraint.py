"""E0109: Generic constraint violation.

Checks that type arguments supplied to generic functions, structs,
or tagged unions satisfy declared trait bounds.  For example, if a
generic parameter `T: Display + Clone` is used and a type argument
is provided that does not implement all required traits, this rule
emits a diagnostic.

In a cross-language context this also covers implicit constraints
that targets enforce (e.g. Rust's `Sized` or JS's `PlainObject`).
"""

from omnilang.analyze.registry import AnalyzeRule, AnalyzeContext
from omnilang.analyze.diagnostic import Diagnostic, Severity, SourceLocation
from omnilang.ast_nodes import (
    GenericParam,
)


class E0109Rule(AnalyzeRule):
    code = "E0109"
    name = "generic_constraint_violation"
    severity = Severity.ERROR
    category = "type"
    message = (
        "Type argument violates generic constraint: "
        "expected `{param}` to satisfy `{bounds}`"
    )
    help = (
        "Ensure the type argument implements all required traits. "
        "Add `impl {bounds}` on the argument type or widen the bounds."
    )

    def check(self, ctx: AnalyzeContext) -> list[Diagnostic]:
        self._ctx = ctx
        diagnostics: list[Diagnostic] = []

        # Check generic functions
        for func in ctx.program.functions:
            diags = self._check_generic_params(func.generic_params, func.name, func.span)
            diagnostics.extend(diags)
            if func.body:
                self._check_type_args_in_body(func.body, func.generic_params, diagnostics)

        # Check generic structs
        for struct in ctx.program.structs:
            diags = self._check_generic_params(struct.generic_params, struct.name, struct.span)
            diagnostics.extend(diags)

        # Check generic tagged unions
        for tagged in ctx.program.tagged_unions:
            diags = self._check_generic_params(tagged.generic_params, tagged.name, tagged.span)
            diagnostics.extend(diags)

        return diagnostics

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
        return SourceLocation(
            file=self._filename(), line=1, column=1, span=(1, 1, 1, 1),
        )

    def _check_generic_params(
        self,
        params: list,
        owner_name: str,
        span,
    ) -> list[Diagnostic]:
        """Verify type arguments in the type_map satisfy declared bounds."""
        diagnostics: list[Diagnostic] = []

        for gp in params:
            if not isinstance(gp, GenericParam):
                continue
            if not gp.bounds:
                continue  # no constraints to violate

            # Look up the concrete type argument assigned to this parameter
            type_key = f"{owner_name}::{gp.name}"
            type_arg = self._ctx.type_map.get(type_key)

            if type_arg is None:
                # May not have been resolved yet -- skip
                continue

            arg_traits = self._get_implemented_traits(type_arg)
            for bound in gp.bounds:
                if bound not in arg_traits:
                    loc = self._span_loc(gp.span)
                    diagnostics.append(Diagnostic(
                        code=self.code,
                        severity=self.severity,
                        category=self.category,
                        message=self.message.format(
                            param=gp.name,
                            bounds=" + ".join(gp.bounds),
                        ),
                        location=loc,
                        help=self.help,
                    ))
                    break  # one diagnostic per parameter is enough

        return diagnostics

    def _check_type_args_in_body(self, block, generic_params, diagnostics):
        """Walk expressions in a function body looking for type-argument
        usage that may violate constraints."""
        # For practical purposes the type_map already captures type
        # assignments; the main check happens in _check_generic_params
        # above.  This hook exists for future constraint checking at
        # call-sites (e.g. checking that concrete type args at a call
        # site satisfy the callee's bounds).
        pass

    def _get_implemented_traits(self, type_ann) -> set[str]:
        """Return the set of trait/interface names a type implements.

        Uses the type_map's precomputed trait information where
        available, falling back to a minimal built-in set.
        """
        if type_ann is None:
            return set()

        # Check for precomputed trait data in type_map
        tid = id(type_ann)
        if tid in getattr(self._ctx, "trait_map", {}):
            return self._ctx.trait_map[tid]

        # Built-in trait inference for primitive / known types
        name = getattr(type_ann, "name", None) or str(type_ann)
        builtin_traits: dict[str, set[str]] = {
            "i32":    {"Copy", "Clone", "Eq", "Ord", "Display", "Debug", "Sized"},
            "i64":    {"Copy", "Clone", "Eq", "Ord", "Display", "Debug", "Sized"},
            "f64":    {"Copy", "Clone", "PartialEq", "Display", "Debug", "Sized"},
            "f32":    {"Copy", "Clone", "PartialEq", "Display", "Debug", "Sized"},
            "bool":   {"Copy", "Clone", "Eq", "Display", "Debug", "Sized"},
            "String": {"Clone", "Eq", "Ord", "Display", "Debug", "Sized"},
            "void":   {"Sized"},
        }
        return builtin_traits.get(name, {"Sized"})  # assume Sized by default
