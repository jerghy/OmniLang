"""Python code generator for OmniLang."""

from omnilang.codegen.base import CodeGenerator
from omnilang.ast_nodes import (
    Program, FunctionDef, Param, Block,
    VarDecl, Assign, Destructure, IfStmt, WhileStmt, ForInStmt,
    BreakStmt, ContinueStmt, ReturnStmt, PrintStmt, ExprStmt,
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    TemplateString, TemplatePart, NullCoalesce, OptionalChain, ChainLink,
    ClosureExpr, SpreadExpr, StructLiteral, StructDef,
    MatchExpr, MatchArm, OrPattern, StructPattern,
    ArrayPattern, ObjectPattern, VarPattern, IgnorePattern, RestPattern,
    TypeAnnotation, UnionType, AwaitExpr,
)


class PythonGenerator(CodeGenerator):

    def __init__(self):
        super().__init__()
        self.needs_idiv = False
        self.needs_irem = False
        self.needs_try_catch = False

    # ── Top level ─────────────────────────────────

    def _gen_program(self, prog: Program):
        for struct in getattr(prog, 'structs', []):
            self._gen_struct_def(struct)
        for tu in getattr(prog, 'tagged_unions', []):
            self._gen_tagged_union(tu)
        for cls in getattr(prog, 'classes', []):
            self._gen_class_def(cls)
        for func in prog.functions:
            self._gen_function(func)
        # Entry point - detect async main
        main_func = next((f for f in prog.functions if f.name == "main"), None)
        if main_func and main_func.is_async:
            self._emit()
            self._emit("import asyncio")
            self._emit('if __name__ == "__main__":')
            self._indent()
            self._emit("asyncio.run(main())")
            self._dedent()
        else:
            self._emit()
            self._emit('if __name__ == "__main__":')
            self._indent()
            self._emit("main()")
            self._dedent()

        # Prepend helpers if needed
        if self.needs_idiv or self.needs_irem or self.needs_try_catch:
            helpers = self._gen_helpers()
            self.output = helpers + self.output

    def _gen_helpers(self) -> list[str]:
        lines = []
        if self.needs_idiv:
            lines.append("def _idiv(a, b):")
            lines.append("    q = abs(a) // abs(b)")
            lines.append("    return q if (a < 0) == (b < 0) else -q")
            lines.append("")
        if self.needs_irem:
            lines.append("def _irem(a, b):")
            lines.append("    return a - _idiv(a, b) * b")
            lines.append("")
        if self.needs_try_catch:
            lines.append("async def _omni_try_catch(__omni_future):")
            lines.append("    try:")
            lines.append("        __omni_val = await __omni_future")
            lines.append('        return {"_tag": "Ok", "value": __omni_val}')
            lines.append("    except BaseException as __omni_err:")
            lines.append('        return {"_tag": "Err", "error": __omni_err}')
            lines.append("")
        return lines

    def _gen_function(self, func: FunctionDef):
        if func.generic_params:
            for gp in func.generic_params:
                self._emit(f"from typing import TypeVar")
                bounds_str = f", {', '.join(gp.bounds)}" if gp.bounds else ""
                self._emit(f"_{gp.name} = TypeVar('_{gp.name}'{bounds_str})")

        prefix = "async " if func.is_async else ""
        params_str = ", ".join(p.name for p in func.params)
        self._emit(f"{prefix}def {func.name}({params_str}):")
        self._indent()
        if func.body:
            self._gen_block(func.body)
        self._dedent()
        self._emit()

    # ── Block ─────────────────────────────────────

    def _gen_block(self, block: Block):
        for stmt in block.stmts:
            self._gen_stmt(stmt)
        if block.trailing_expr:
            expr_code = self._gen_expr(block.trailing_expr)
            self._emit(f"return {expr_code}")

    # ── Statements ────────────────────────────────

    def _rename(self, name: str) -> str:
        """Look up rename for the current scope level and ancestors."""
        level = len(self.scope_stack) - 1 if hasattr(self, 'scope_stack') else 0
        return self._lookup_rename_at(level, name)

    def _lookup_rename_at(self, level: int, name: str) -> str:
        """Walk up scope levels to find a rename for name."""
        for lvl in range(level, -1, -1):
            if lvl < len(self.rename_map):
                scope_renames = self.rename_map[lvl]
                if isinstance(scope_renames, dict) and name in scope_renames:
                    return scope_renames[name]
        return name

    def _enter_scope(self):
        if not hasattr(self, 'scope_stack'):
            self.scope_stack = [0]
        self.scope_stack.append(len(self.scope_stack))

    def _leave_scope(self):
        if hasattr(self, 'scope_stack') and len(self.scope_stack) > 1:
            self.scope_stack.pop()

    def _gen_stmt(self, stmt):
        if isinstance(stmt, VarDecl):
            init = self._gen_expr(stmt.init)
            # Container clone from variable: use list()/dict()
            if isinstance(stmt.init, VarRef) and stmt.type_ann:
                if stmt.type_ann.name == "Array":
                    init = f"list({init})"
                elif stmt.type_ann.name == "Map":
                    init = f"dict({init})"
            name = self._rename(stmt.name)
            self._emit(f"{name} = {init}")

        elif isinstance(stmt, Assign):
            target_name = self._rename(stmt.target)
            value = self._gen_expr(stmt.value)
            if stmt.op != "=":
                if stmt.op in ("/=", "%="):
                    self.needs_idiv = True
                    self.needs_irem = True
                    op_func = "_idiv" if stmt.op == "/=" else "_irem"
                    if stmt.index:
                        sub = self._gen_expr(stmt.index)
                        self._emit(f"{target_name}[{sub}] = {op_func}({target_name}[{sub}], {value})")
                    else:
                        self._emit(f"{target_name} = {op_func}({target_name}, {value})")
                else:
                    if stmt.index:
                        sub = self._gen_expr(stmt.index)
                        self._emit(f"{target_name}[{sub}] {stmt.op} {value}")
                    else:
                        self._emit(f"{target_name} {stmt.op} {value}")
            else:
                if stmt.index:
                    sub = self._gen_expr(stmt.index)
                    self._emit(f"{target_name}[{sub}] = {value}")
                else:
                    self._emit(f"{target_name} = {value}")

        elif isinstance(stmt, IfStmt):
            cond = self._gen_expr(stmt.condition)
            self._emit(f"if {cond}:")
            self._indent()
            self._enter_scope()
            self._gen_block(stmt.then_body)
            self._leave_scope()
            self._dedent()
            for elif_cond, elif_body in stmt.else_ifs:
                c = self._gen_expr(elif_cond)
                self._emit(f"elif {c}:")
                self._indent()
                self._enter_scope()
                self._gen_block(elif_body)
                self._leave_scope()
                self._dedent()
            if stmt.else_body:
                self._emit("else:")
                self._indent()
                self._enter_scope()
                self._gen_block(stmt.else_body)
                self._leave_scope()
                self._dedent()

        elif isinstance(stmt, WhileStmt):
            cond = self._gen_expr(stmt.condition)
            self._emit(f"while {cond}:")
            self._indent()
            self._enter_scope()
            self._gen_block(stmt.body)
            self._leave_scope()
            self._dedent()

        elif isinstance(stmt, ForInStmt):
            iterable = self._gen_expr(stmt.iterable)
            var_name = self._rename(stmt.var_name)
            self._emit(f"for {var_name} in {iterable}:")
            self._indent()
            self._enter_scope()
            self._gen_block(stmt.body)
            self._leave_scope()
            self._dedent()

        elif isinstance(stmt, BreakStmt):
            self._emit("break")

        elif isinstance(stmt, ContinueStmt):
            self._emit("continue")

        elif isinstance(stmt, ReturnStmt):
            if stmt.value:
                val = self._gen_expr(stmt.value)
                self._emit(f"return {val}")
            else:
                self._emit("return")

        elif isinstance(stmt, PrintStmt):
            arg = self._gen_expr(stmt.arg)
            if stmt.newline:
                self._emit(f"print({arg})")
            else:
                self._emit(f"print({arg}, end='')")

        elif isinstance(stmt, ExprStmt):
            self._gen_expr(stmt.expr)

        elif isinstance(stmt, Destructure):
            pattern = self._gen_pattern(stmt.pattern)
            init = self._gen_expr(stmt.init)
            self._emit(f"{pattern} = {init}")

    # ── Expressions ───────────────────────────────

    def _gen_expr(self, expr, ctx: str = "default") -> str:
        if expr is None:
            return "None"

        if isinstance(expr, Literal):
            if expr.kind == "String":
                escaped = expr.value.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")
                return f'"{escaped}"'
            elif expr.kind == "bool":
                return "True" if expr.value else "False"
            return str(expr.value)

        elif isinstance(expr, VarRef):
            return self._rename(expr.name)

        elif isinstance(expr, BinaryOp):
            left = self._gen_expr(expr.left)
            right = self._gen_expr(expr.right)
            if expr.op in ("/", "%"):
                left_ty = self.type_map.get(id(expr.left))
                if left_ty and left_ty.name == "i32":
                    self.needs_idiv = True if expr.op == "/" else self.needs_idiv
                    self.needs_irem = True if expr.op == "%" else self.needs_irem
                    func = "_idiv" if expr.op == "/" else "_irem"
                    return f"{func}({left}, {right})"
            return f"({left} {expr.op} {right})"

        elif isinstance(expr, UnaryOp):
            operand = self._gen_expr(expr.operand)
            return f"({expr.op} {operand})"

        elif isinstance(expr, FuncCall):
            func_name = self._gen_expr(expr.func)
            raw_name = expr.func.name if isinstance(expr.func, VarRef) else None

            # ── Async built-in functions ─────────────
            if raw_name == "all":
                if expr.args and isinstance(expr.args[0], ArrayLiteral):
                    items = ", ".join(self._gen_expr(e) for e in expr.args[0].elements)
                    return f"asyncio.gather({items})"
                inner = self._gen_expr(expr.args[0])
                return f"asyncio.gather(*{inner})"
            if raw_name == "race":
                if expr.args and isinstance(expr.args[0], ArrayLiteral):
                    items = ", ".join(self._gen_expr(e) for e in expr.args[0].elements)
                    return f"asyncio.wait([{items}], return_when=asyncio.FIRST_COMPLETED)"
                inner = self._gen_expr(expr.args[0])
                return f"asyncio.wait({inner}, return_when=asyncio.FIRST_COMPLETED)"
            if raw_name == "tryCatch":
                self.needs_try_catch = True
                future = self._gen_expr(expr.args[0])
                return f"_omni_try_catch({future})"

            args = ", ".join(self._gen_expr(a) for a in expr.args)
            return f"{func_name}({args})"

        elif isinstance(expr, MethodCall):
            obj = self._gen_expr(expr.object)
            method = expr.method

            # Map method mappings
            if method == "len":
                return f"len({obj})"
            elif method == "is_empty":
                return f"len({obj}) == 0"
            elif method == "get":
                args = ", ".join(self._gen_expr(a) for a in expr.args)
                return f"{obj}[{args}]"
            elif method == "push":
                args = ", ".join(self._gen_expr(a) for a in expr.args)
                self._emit(f"{obj}.append({args})")
                return "None"
            elif method == "pop":
                return f"{obj}.pop()"
            elif method == "contains":
                args = ", ".join(self._gen_expr(a) for a in expr.args)
                return f"({args} in {obj})"
            elif method == "insert":
                k, v = (self._gen_expr(a) for a in expr.args)
                self._emit(f"{obj}[{k}] = {v}")
                return "None"
            elif method == "to_string":
                obj_ty = self.type_map.get(id(expr.object))
                if obj_ty and obj_ty.name == "bool":
                    return f"str({obj}).lower()"
                return f"str({obj})"
            else:
                args = ", ".join(self._gen_expr(a) for a in expr.args)
                return f"{obj}.{method}({args})"

        elif isinstance(expr, IndexGet):
            obj = self._gen_expr(expr.object)
            idx = self._gen_expr(expr.index)
            return f"{obj}[{idx}]"

        elif isinstance(expr, ArrayLiteral):
            elems = ", ".join(self._gen_expr(e) for e in expr.elements)
            return f"[{elems}]"

        elif isinstance(expr, MapLiteral):
            entries = ", ".join(
                f"{self._gen_expr(k)}: {self._gen_expr(v)}"
                for k, v in expr.entries
            )
            return f"{{{entries}}}"

        elif isinstance(expr, IfExpr):
            cond = self._gen_expr(expr.condition)
            then = self._gen_expr(expr.then_expr)
            else_ = self._gen_expr(expr.else_expr)
            return f"({then} if {cond} else {else_})"

        elif isinstance(expr, TemplateString):
            return self._gen_template_string(expr)

        elif isinstance(expr, NullCoalesce):
            left = self._gen_expr(expr.left)
            right = self._gen_expr(expr.right)
            return f"({left} if {left} is not None else {right})"

        elif isinstance(expr, OptionalChain):
            return self._gen_optional_chain(expr)

        elif isinstance(expr, SpreadExpr):
            inner = self._gen_expr(expr.expr)
            return f"*{inner}"

        elif isinstance(expr, StructLiteral):
            return self._gen_struct_literal(expr)

        elif isinstance(expr, ClosureExpr):
            return self._gen_closure(expr)

        elif isinstance(expr, MatchExpr):
            return self._gen_match(expr)

        elif isinstance(expr, AwaitExpr):
            return f"await {self._gen_expr(expr.expr)}"

        return "None"

    # ── v0.2 helpers ──────────────────────────────

    def _gen_match(self, me: MatchExpr) -> str:
        """Generate Python match/case for match expression (Python 3.10+)."""
        scrutinee = self._gen_expr(me.scrutinee)
        tmp = f"_m{self.temp_counter}"
        self.temp_counter = getattr(self, 'temp_counter', 0) + 1

        self._emit(f"{tmp} = {scrutinee}")
        self._emit(f"match {tmp}:")
        self._indent()
        for arm in me.arms:
            pat_str = self._gen_match_pattern(arm.pattern)
            body_str = self._gen_arm_body(arm)
            self._emit(f"case {pat_str}:")
            self._indent()
            self._emit(f"_result = {body_str}")
            self._dedent()
        self._dedent()
        return "_result"

    def _gen_match_pattern(self, pat) -> str:
        """Generate Python match pattern."""
        if isinstance(pat, IgnorePattern):
            return "_"
        elif isinstance(pat, Literal):
            return self._gen_expr(pat)
        elif isinstance(pat, VarPattern):
            return pat.name
        elif isinstance(pat, OrPattern):
            return " | ".join(self._gen_match_pattern(p) for p in pat.patterns)
        elif isinstance(pat, StructPattern):
            if pat.fields:
                fields = ", ".join(
                    f"{name}={self._gen_match_pattern(p)}"
                    for name, p in pat.fields
                )
                return f"{pat.name}({fields})"
            return pat.name
        return "_"

    def _gen_arm_body(self, arm: MatchArm) -> str:
        if isinstance(arm.body, Block):
            # Inline block: generate statements then return
            old = self.output
            self.output = []
            for stmt in arm.body.stmts:
                self._gen_stmt(stmt)
            if arm.body.trailing_expr:
                self._emit(f"return {self._gen_expr(arm.body.trailing_expr)}")
            result = "; ".join(l.strip() for l in self.output if l.strip())
            self.output = old
            return result
        return self._gen_expr(arm.body)

    def _gen_closure(self, ce: ClosureExpr) -> str:
        """Generate Python lambda or nested def from ClosureExpr."""
        params_str = ", ".join(p.name for p in ce.params)
        if isinstance(ce.body, Block):
            # Multi-statement: nested def with nonlocal
            name = f"__closure_{ce.span.line}_{ce.span.col}" if ce.span else "__closure_anon"
            name = name.replace(":", "_").replace("-", "_")
            self._emit(f"def {name}({params_str}):")
            self._indent()
            # Simple nonlocal: add for any assignment targets
            self._gen_block(ce.body)
            self._dedent()
            return name
        else:
            body_code = self._gen_expr(ce.body)
            return f"lambda {params_str}: {body_code}"

    def _gen_template_string(self, ts: TemplateString) -> str:
        """Generate Python f-string from TemplateString AST."""
        parts = []
        for p in ts.parts:
            if p.kind == "literal":
                parts.append(str(p.value).replace("{", "{{").replace("}", "}}"))
            elif p.kind == "expr":
                parts.append(f"{{{self._gen_expr(p.value)}}}")
        return "f\"" + "".join(parts) + "\""

    def _gen_optional_chain(self, oc: OptionalChain) -> str:
        """Generate Python safe navigation (manual if/else chain)."""
        obj = self._gen_expr(oc.object)
        # Python doesn't have optional chaining — generate if/else
        # For simplicity in Phase 1, use getattr with default
        result = obj
        for link in oc.chain:
            if link.kind == "field":
                result = f"({result}.{link.name} if {result} is not None else None)"
            elif link.kind == "index":
                idx = self._gen_expr(link.index)
                result = f"({result}[{idx}] if {result} is not None else None)"
            elif link.kind == "call":
                args = ", ".join(self._gen_expr(a) for a in link.args)
                result = f"({result}.{link.name}({args}) if {result} is not None else None)"
        return result

    def _gen_struct_literal(self, sl: StructLiteral) -> str:
        """Generate Python constructor call from struct literal."""
        fields = ", ".join(
            f"{name}={self._gen_expr(value)}"
            for name, value in sl.fields
        )
        return f"{sl.type_name}({fields})"

    def _gen_pattern(self, pat) -> str:
        """Generate Python destructuring pattern string."""
        if isinstance(pat, VarPattern):
            return pat.name
        elif isinstance(pat, IgnorePattern):
            return "_"
        elif isinstance(pat, RestPattern):
            return f"*{pat.name}"
        elif isinstance(pat, ArrayPattern):
            elems = [self._gen_pattern(e) for e in pat.elements]
            if pat.rest:
                elems.append(self._gen_pattern(pat.rest))
            return ", ".join(elems)
        elif isinstance(pat, ObjectPattern):
            return ", ".join(pat.fields)
        return "_"

    def _gen_tagged_union(self, tu):
        """Python: tagged union → dataclass variants + Union type alias."""
        self._emit("from dataclasses import dataclass")
        self._emit("from typing import Union")
        self._emit()
        for v in tu.variants:
            self._emit("@dataclass")
            self._emit(f"class {v.name}:")
            self._indent()
            if v.fields:
                for f in v.fields:
                    self._emit(f"{f.name}: {self._py_type(f.type_ann)}")
            else:
                self._emit("pass")
            self._dedent()
            self._emit()
        variant_names = ", ".join(v.name for v in tu.variants)
        self._emit(f"{tu.name} = Union[{variant_names}]")
        self._emit()

    def _gen_class_def(self, cls):
        """Python: class → standard Python class."""
        base = f"({cls.extends})" if cls.extends else ""
        self._emit(f"class {cls.name}{base}:")
        self._indent()
        if not cls.members:
            self._emit("pass")
        else:
            for m in cls.members:
                if m.func_def:
                    self._gen_function(m.func_def)
                elif m.name:
                    default = f" = {self._gen_expr(m.init)}" if m.init else ""
                    self._emit(f"{m.name}: {self._py_type(m.type_ann)}{default}")
        self._dedent()
        self._emit()

    def _gen_struct_def(self, sd: StructDef):
        """Generate Python @dataclass from OmniLang struct definition."""
        self._emit("from dataclasses import dataclass")
        self._emit()
        self._emit("@dataclass")
        if sd.is_nominal:
            self._emit("@dataclass(frozen=True)")
        self._emit(f"class {sd.name}:")
        self._indent()
        if sd.fields:
            for f in sd.fields:
                py_type = self._py_type(f.type_ann)
                if f.default:
                    default_val = self._gen_expr(f.default)
                    self._emit(f"{f.name}: {py_type} = {default_val}")
                else:
                    self._emit(f"{f.name}: {py_type}")
        else:
            self._emit("pass")
        self._dedent()
        self._emit()

    def _py_type(self, ty) -> str:
        """Convert OmniLang type annotation to Python type hint."""
        if ty is None:
            return "None"
        name = ty.name
        mapping = {
            "i32": "int", "f64": "float", "bool": "bool",
            "String": "str", "void": "None",
        }
        if name in mapping:
            return mapping[name]
        if name == "Array" and ty.type_args:
            inner = self._py_type(ty.type_args[0])
            return f"list[{inner}]"
        if name == "Map" and len(ty.type_args) >= 2:
            k = self._py_type(ty.type_args[0])
            v = self._py_type(ty.type_args[1])
            return f"dict[{k}, {v}]"
        return name

    def _type_of(self, node) -> TypeAnnotation | None:
        for decl in [VarDecl]:  # simplified
            pass
        return self.type_map.get(id(node))
