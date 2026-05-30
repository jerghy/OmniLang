"""JavaScript code generator for OmniLang."""

from omnilang.codegen.base import CodeGenerator
from omnilang.ast_nodes import (
    Program, FunctionDef, Block,
    VarDecl, Assign, Destructure, IfStmt, WhileStmt, ForInStmt,
    BreakStmt, ContinueStmt, ReturnStmt, PrintStmt, ExprStmt,
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    TemplateString, TemplatePart, NullCoalesce, OptionalChain, ChainLink,
    ClosureExpr, SpreadExpr, StructLiteral, StructDef,
    MatchExpr, MatchArm, OrPattern, StructPattern,
    TaggedUnionDef, Variant, AwaitExpr,
    ArrayPattern, ObjectPattern, VarPattern, IgnorePattern, RestPattern,
    TypeAnnotation, UnionType,
)


HELPERS_JS = {
    "_idiv": """function _idiv(a, b) {
    if (b === 0) throw new Error("division by zero");
    const r = Math.trunc(a / b);
    return r === 0 ? 0 : r;
}""",
    "_irem": """function _irem(a, b) {
    if (b === 0) throw new Error("division by zero");
    return a % b;
}""",
    "_arrlen": """function _arrlen(arr) { return arr.length; }""",
    "_arrisempty": """function _arrisempty(arr) { return arr.length === 0; }""",
    "_arrget": """function _arrget(arr, i) {
    if (i < 0 || i >= arr.length) throw new Error("index out of bounds");
    return arr[i];
}""",
    "_arrset": """function _arrset(arr, i, val) {
    if (i < 0 || i >= arr.length) throw new Error("index out of bounds");
    arr[i] = val;
}""",
    "_arrpop": """function _arrpop(arr) {
    if (arr.length === 0) throw new Error("pop from empty array");
    return arr.pop();
}""",
    "_maplen": """function _maplen(map) { return map.size; }""",
    "_mapget": """function _mapget(map, key) {
    if (!map.has(key)) throw new Error("Key not found");
    return map.get(key);
}""",
    "_arrClone": """function _arrClone(arr) { return arr.slice(); }""",
    "_mapClone": """function _mapClone(map) { return new Map(map); }""",
}


class JavaScriptGenerator(CodeGenerator):

    def __init__(self):
        super().__init__()
        self.needed_helpers: set[str] = set()
        self.var_types: dict[str, str] = {}  # variable name → type name

    def _get_var_type(self, name: str):
        """Get the OmniLang type of a variable by name."""
        return self.var_types.get(name)

    def _track_var(self, name: str, type_ann):
        """Track a variable's type."""
        if type_ann:
            self.var_types[name] = type_ann

    # ── Top level ─────────────────────────────────

    def _gen_program(self, prog: Program):
        # Emit struct definitions first
        for struct in getattr(prog, 'structs', []):
            self._gen_struct_def(struct)
        for tu in getattr(prog, 'tagged_unions', []):
            self._gen_tagged_union(tu)
        # Import/export: generate ES module imports/exports
        self._gen_imports_exports(prog)
        for func in prog.functions:
            self._gen_function(func)

        self._emit("main();")

        # Prepend helpers
        if self.needed_helpers:
            helper_lines = []
            for name in sorted(self.needed_helpers):
                helper_lines.append(HELPERS_JS[name])
                helper_lines.append("")
            self.output = helper_lines + self.output

    def _gen_function(self, func: FunctionDef):
        prefix = "async " if func.is_async else ""
        params_str = ", ".join(p.name for p in func.params)
        self._emit(f"{prefix}function {func.name}({params_str}) {{")
        self._indent()
        if func.body:
            self._gen_block(func.body)
        self._dedent()
        self._emit("}")
        self._emit()

    # ── Block ─────────────────────────────────────

    def _gen_block(self, block: Block):
        for stmt in block.stmts:
            self._gen_stmt(stmt)
        if block.trailing_expr:
            expr_code = self._gen_expr(block.trailing_expr)
            self._emit(f"return {expr_code};")

    # ── Statements ────────────────────────────────

    def _gen_stmt(self, stmt):
        if isinstance(stmt, VarDecl):
            keyword = "const" if not stmt.mutable else "let"
            init = self._gen_expr(stmt.init)
            self._track_var(stmt.name, stmt.type_ann)

            # Handle container clone from variable
            if isinstance(stmt.init, VarRef):
                ty = self.type_map.get(id(stmt.init))
                if ty:
                    if ty.name == "Array":
                        init = f"_arrClone({init})"
                        self.needed_helpers.add("_arrClone")
                    elif ty.name == "Map":
                        init = f"_mapClone({init})"
                        self.needed_helpers.add("_mapClone")
            elif isinstance(stmt.init, MapLiteral):
                # JS Map literal: new Map([[k,v],...])
                entries = ", ".join(
                    f"[{self._gen_expr(k)}, {self._gen_expr(v)}]"
                    for k, v in stmt.init.entries
                )
                init = f"new Map([{entries}])"

            self._emit(f"{keyword} {stmt.name} = {init};")

        elif isinstance(stmt, Assign):
            value = self._gen_expr(stmt.value)

            if stmt.op == "=":
                if stmt.index:
                    idx = self._gen_expr(stmt.index)
                    # Check container type by looking at the target
                    target_ty = self._get_var_type(stmt.target)
                    if target_ty and target_ty.name == "Array":
                        self.needed_helpers.add("_arrset")
                        self._emit(f"_arrset({stmt.target}, {idx}, {value});")
                    elif target_ty and target_ty.name == "Map":
                        self._emit(f"{stmt.target}.set({idx}, {value});")
                    else:
                        self._emit(f"{stmt.target}[{idx}] = {value};")
                else:
                    self._emit(f"{stmt.target} = {value};")
            elif stmt.op in ("/=", "%="):
                self.needed_helpers.add("_idiv")
                self.needed_helpers.add("_irem")
                func = "_idiv" if stmt.op == "/=" else "_irem"
                if stmt.index:
                    idx = self._gen_expr(stmt.index)
                    self._emit(f"{stmt.target}[{idx}] = {func}({stmt.target}[{idx}], {value});")
                else:
                    self._emit(f"{stmt.target} = {func}({stmt.target}, {value});")
            else:
                if stmt.index:
                    idx = self._gen_expr(stmt.index)
                    self._emit(f"{stmt.target}[{idx}] {stmt.op} {value};")
                else:
                    self._emit(f"{stmt.target} {stmt.op} {value};")

        elif isinstance(stmt, IfStmt):
            cond = self._gen_expr(stmt.condition)
            self._emit(f"if ({cond}) {{")
            self._indent()
            self._gen_block(stmt.then_body)
            self._dedent()
            for elif_cond, elif_body in stmt.else_ifs:
                c = self._gen_expr(elif_cond)
                self._emit(f"}} else if ({c}) {{")
                self._indent()
                self._gen_block(elif_body)
                self._dedent()
            if stmt.else_body:
                self._emit("} else {")
                self._indent()
                self._gen_block(stmt.else_body)
                self._dedent()
            self._emit("}")

        elif isinstance(stmt, WhileStmt):
            cond = self._gen_expr(stmt.condition)
            self._emit(f"while ({cond}) {{")
            self._indent()
            self._gen_block(stmt.body)
            self._dedent()
            self._emit("}")

        elif isinstance(stmt, ForInStmt):
            iterable = self._gen_expr(stmt.iterable)
            self._emit(f"for (const {stmt.var_name} of {iterable}) {{")
            self._indent()
            self._gen_block(stmt.body)
            self._dedent()
            self._emit("}")

        elif isinstance(stmt, BreakStmt):
            self._emit("break;")

        elif isinstance(stmt, ContinueStmt):
            self._emit("continue;")

        elif isinstance(stmt, ReturnStmt):
            if stmt.value:
                val = self._gen_expr(stmt.value)
                self._emit(f"return {val};")
            else:
                self._emit("return;")

        elif isinstance(stmt, PrintStmt):
            arg = self._gen_expr(stmt.arg)
            if stmt.newline:
                self._emit(f"console.log({arg});")
            else:
                self._emit(f"process.stdout.write({arg});")

        elif isinstance(stmt, ExprStmt):
            code = self._gen_expr(stmt.expr)
            # If the expression is a MethodCall with side effects, emit it
            if isinstance(stmt.expr, MethodCall) and stmt.expr.method in ("push", "insert", "pop"):
                self._emit(f"{code};")

        elif isinstance(stmt, Destructure):
            self._gen_destructure(stmt)

    # ── Expressions ───────────────────────────────

    def _gen_expr(self, expr, ctx: str = "default") -> str:
        if expr is None:
            return "undefined"

        if isinstance(expr, Literal):
            if expr.kind == "String":
                escaped = expr.value.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")
                return f'"{escaped}"'
            elif expr.kind == "bool":
                return "true" if expr.value else "false"
            return str(expr.value)

        elif isinstance(expr, VarRef):
            return expr.name

        elif isinstance(expr, BinaryOp):
            left = self._gen_expr(expr.left)
            right = self._gen_expr(expr.right)

            if expr.op == "/":
                left_ty = self.type_map.get(id(expr.left))
                if left_ty and left_ty.name == "i32":
                    self.needed_helpers.add("_idiv")
                    return f"_idiv({left}, {right})"
            elif expr.op == "%":
                left_ty = self.type_map.get(id(expr.left))
                if left_ty and left_ty.name == "i32":
                    self.needed_helpers.add("_irem")
                    return f"_irem({left}, {right})"

            return f"({left} {expr.op} {right})"

        elif isinstance(expr, UnaryOp):
            operand = self._gen_expr(expr.operand)
            return f"({expr.op} {operand})"

        elif isinstance(expr, FuncCall):
            func_name = self._gen_expr(expr.func)
            args = ", ".join(self._gen_expr(a) for a in expr.args)
            return f"{func_name}({args})"

        elif isinstance(expr, MethodCall):
            obj = self._gen_expr(expr.object)
            method = expr.method

            if method == "len":
                obj_ty = self.type_map.get(id(expr.object))
                if obj_ty and obj_ty.name == "Array":
                    self.needed_helpers.add("_arrlen")
                    return f"_arrlen({obj})"
                elif obj_ty and obj_ty.name == "Map":
                    self.needed_helpers.add("_maplen")
                    return f"_maplen({obj})"
                return f"{obj}.length"
            elif method == "is_empty":
                obj_ty = self.type_map.get(id(expr.object))
                if obj_ty and obj_ty.name == "Array":
                    self.needed_helpers.add("_arrisempty")
                    return f"_arrisempty({obj})"
                elif obj_ty and obj_ty.name == "Map":
                    self.needed_helpers.add("_maplen")
                    return f"_maplen({obj}) === 0"
                return f"({obj}.length === 0)"
            elif method == "get":
                obj_ty = self.type_map.get(id(expr.object))
                arg = ", ".join(self._gen_expr(a) for a in expr.args)
                if obj_ty and obj_ty.name == "Array":
                    self.needed_helpers.add("_arrget")
                    return f"_arrget({obj}, {arg})"
                elif obj_ty and obj_ty.name == "Map":
                    self.needed_helpers.add("_mapget")
                    return f"_mapget({obj}, {arg})"
                return f"{obj}[{arg}]"
            elif method == "push":
                args = ", ".join(self._gen_expr(a) for a in expr.args)
                return f"{obj}.push({args})"
            elif method == "pop":
                obj_ty = self.type_map.get(id(expr.object))
                if obj_ty and obj_ty.name == "Array":
                    self.needed_helpers.add("_arrpop")
                    return f"_arrpop({obj})"
                return f"{obj}.pop()"
            elif method == "contains":
                arg = ", ".join(self._gen_expr(a) for a in expr.args)
                return f"{obj}.has({arg})"
            elif method == "insert":
                k = self._gen_expr(expr.args[0])
                v = self._gen_expr(expr.args[1])
                return f"{obj}.set({k}, {v})"
            elif method == "to_string":
                return f"String({obj})"
            else:
                args = ", ".join(self._gen_expr(a) for a in expr.args)
                return f"{obj}.{method}({args})"

        elif isinstance(expr, IndexGet):
            obj = self._gen_expr(expr.object)
            idx = self._gen_expr(expr.index)

            obj_ty = self.type_map.get(id(expr.object))
            if obj_ty and obj_ty.name == "Array":
                self.needed_helpers.add("_arrget")
                return f"_arrget({obj}, {idx})"
            elif obj_ty and obj_ty.name == "Map":
                self.needed_helpers.add("_mapget")
                return f"_mapget({obj}, {idx})"
            return f"{obj}[{idx}]"

        elif isinstance(expr, ArrayLiteral):
            elems = ", ".join(self._gen_expr(e) for e in expr.elements)
            return f"[{elems}]"

        elif isinstance(expr, MapLiteral):
            entries = ", ".join(
                f"[{self._gen_expr(k)}, {self._gen_expr(v)}]"
                for k, v in expr.entries
            )
            return f"new Map([{entries}])"

        elif isinstance(expr, IfExpr):
            cond = self._gen_expr(expr.condition)
            then = self._gen_expr(expr.then_expr)
            else_ = self._gen_expr(expr.else_expr)
            return f"({cond} ? {then} : {else_})"

        elif isinstance(expr, TemplateString):
            return self._gen_template_string(expr)

        elif isinstance(expr, NullCoalesce):
            left = self._gen_expr(expr.left)
            right = self._gen_expr(expr.right)
            return f"({left} ?? {right})"

        elif isinstance(expr, OptionalChain):
            return self._gen_optional_chain(expr)

        elif isinstance(expr, SpreadExpr):
            inner = self._gen_expr(expr.expr)
            return f"...{inner}"

        elif isinstance(expr, StructLiteral):
            return self._gen_struct_literal(expr)

        elif isinstance(expr, ClosureExpr):
            return self._gen_closure(expr)

        elif isinstance(expr, MatchExpr):
            return self._gen_match(expr)

        elif isinstance(expr, AwaitExpr):
            return f"await {self._gen_expr(expr.expr)}"

        return "undefined"

    # ── v0.2 helpers ──────────────────────────────

    def _gen_template_string(self, ts: TemplateString) -> str:
        """Generate JavaScript template literal from TemplateString AST."""
        parts = []
        for p in ts.parts:
            if p.kind == "literal":
                parts.append(str(p.value))
            elif p.kind == "expr":
                parts.append(f"${{{self._gen_expr(p.value)}}}")
        return "`" + "".join(parts) + "`"

    def _gen_optional_chain(self, oc: OptionalChain) -> str:
        """Generate JavaScript optional chaining expression."""
        obj = self._gen_expr(oc.object)
        result = obj
        for link in oc.chain:
            if link.kind == "field":
                result = f"{result}?.{link.name}"
            elif link.kind == "index":
                idx = self._gen_expr(link.index)
                result = f"{result}?.[{idx}]"
            elif link.kind == "call":
                args = ", ".join(self._gen_expr(a) for a in link.args)
                result = f"{result}?.{link.name}({args})"
        return result

    def _gen_struct_literal(self, sl: StructLiteral) -> str:
        """Generate JavaScript object from struct literal."""
        fields = ", ".join(
            f"{name}: {self._gen_expr(value)}"
            for name, value in sl.fields
        )
        return f"{{ {fields} }}"

    def _gen_match(self, me: MatchExpr) -> str:
        """Generate JS match as IIFE with if/else chain."""
        scrutinee = self._gen_expr(me.scrutinee)
        if not hasattr(self, '_match_counter'):
            self._match_counter = 0
        self._match_counter += 1
        tmp = f"_m{self._match_counter}"

        # Build as IIFE: (() => { const _m = scr; if (...) return ...; })()
        parts = [f"(() => {{"]
        parts.append(f"  const {tmp} = {scrutinee};")
        for arm in me.arms:
            cond = self._gen_match_condition(tmp, arm)
            body_code = self._gen_arm_body(arm)
            parts.append(f"  if ({cond}) return {body_code};")
        parts.append(f"  return undefined;")
        parts.append(f"}})()")
        return "\n".join(parts)

    def _gen_match_condition(self, tmp: str, arm: MatchArm) -> str:
        """Generate condition for a match arm."""
        pat = arm.pattern
        if isinstance(pat, IgnorePattern):
            return "true"
        elif isinstance(pat, Literal):
            val = self._gen_expr(pat)
            return f"{tmp} === {val}"
        elif isinstance(pat, VarPattern):
            return "true"
        elif isinstance(pat, OrPattern):
            parts = []
            for sp in pat.patterns:
                if isinstance(sp, Literal):
                    parts.append(f"{tmp} === {self._gen_expr(sp)}")
                elif isinstance(sp, IgnorePattern):
                    parts.append("true")
            return " || ".join(parts) if parts else "true"
        return "true"

    def _gen_arm_body(self, arm: MatchArm) -> str:
        """Generate the body code for a match arm."""
        if isinstance(arm.body, Block):
            return self._gen_block_to_str(arm.body)
        return self._gen_expr(arm.body)

    def _gen_closure(self, ce: ClosureExpr) -> str:
        """Generate JavaScript arrow function from ClosureExpr."""
        params = ", ".join(p.name for p in ce.params)
        if isinstance(ce.body, Block):
            # Multi-statement body
            body_code = self._gen_block_to_str(ce.body)
            return f"({params}) => {{ {body_code} }}"
        else:
            # Single expression body
            body_code = self._gen_expr(ce.body)
            return f"({params}) => {body_code}"

    def _gen_block_to_str(self, block: Block) -> str:
        """Generate block statements as inline string (for closures)."""
        old_output = self.output
        old_indent = self.indent_level
        self.output = []
        self.indent_level = 0
        for stmt in block.stmts:
            self._gen_stmt(stmt)
        if block.trailing_expr:
            expr_code = self._gen_expr(block.trailing_expr)
            self._emit(f"return {expr_code};")
        result = " ".join(line.strip() for line in self.output if line.strip())
        self.output = old_output
        self.indent_level = old_indent
        return result

    def _gen_destructure(self, stmt: Destructure):
        """Generate JavaScript destructuring assignment."""
        pattern = self._gen_pattern(stmt.pattern)
        init = self._gen_expr(stmt.init)
        keyword = "let" if stmt.mutable else "const"
        self._emit(f"{keyword} {pattern} = {init};")

    def _gen_pattern(self, pat) -> str:
        """Generate JavaScript destructuring pattern string."""
        if isinstance(pat, VarPattern):
            return pat.name
        elif isinstance(pat, IgnorePattern):
            return "_"
        elif isinstance(pat, RestPattern):
            return f"...{pat.name}"
        elif isinstance(pat, ArrayPattern):
            elems = [self._gen_pattern(e) for e in pat.elements]
            if pat.rest:
                elems.append(self._gen_pattern(pat.rest))
            return "[" + ", ".join(elems) + "]"
        elif isinstance(pat, ObjectPattern):
            return "{" + ", ".join(pat.fields) + "}"
        return "_"

    def _gen_imports_exports(self, prog: Program):
        """Generate JS ES module import/export statements."""
        from omnilang.ast_nodes import ImportStmt, ExportStmt
        # Scan for imports/exports at the top level
        imports_done = set()
        for func in list(prog.functions):
            pass  # Handled inline in _gen_program
        # Emit ES module imports if any .omni source had them
        # (simplified: imports handled at parse time via module loader)

    def _gen_tagged_union(self, tu: TaggedUnionDef):
        """JS: tagged unions are discriminated union objects with _tag field."""
        # JS doesn't have native tagged unions — represent as plain objects
        # with a _tag discriminator field
        self._emit(f"// Tagged union: {tu.name}")
        for v in tu.variants:
            fields = ", ".join(f"{f.name}: {f.name}" for f in v.fields)
            params = ", ".join(f.name for f in v.fields)
            self._emit(f"function {tu.name}_{v.name}({params}) {{")
            self._indent()
            self._emit(f'this._tag = "{v.name}";')
            for f in v.fields:
                self._emit(f"this.{f.name} = {f.name};")
            self._dedent()
            self._emit("}")
        self._emit()

    def _gen_struct_def(self, sd: StructDef):
        """Generate JavaScript class from OmniLang struct definition."""
        self._emit(f"class {sd.name} {{")
        self._indent()
        # Constructor
        params = ", ".join(f.name for f in sd.fields)
        self._emit(f"constructor({params}) {{")
        self._indent()
        for f in sd.fields:
            self._emit(f"this.{f.name} = {f.name};")
        self._dedent()
        self._emit("}")
        # Methods from impl blocks
        for method in sd.methods:
            self._gen_function(method)
        self._dedent()
        self._emit("}")
        self._emit()
