"""Dart code generator for OmniLang."""

from omnilang.codegen.base import CodeGenerator
from omnilang.ast_nodes import (
    Program, FunctionDef, Block,
    VarDecl, Assign, Destructure, IfStmt, WhileStmt, ForInStmt,
    BreakStmt, ContinueStmt, ReturnStmt, PrintStmt, ExprStmt,
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    TemplateString, NullCoalesce, OptionalChain, ClosureExpr, SpreadExpr, StructLiteral, StructDef,
    MatchExpr, MatchArm, OrPattern, AwaitExpr,
    ArrayPattern, VarPattern, IgnorePattern, RestPattern,
    TypeAnnotation, UnionType,
)

DART_TYPE_MAP = {
    "void": "void",
    "i32": "int",
    "f64": "double",
    "bool": "bool",
    "String": "String",
}


class DartGenerator(CodeGenerator):

    def __init__(self):
        super().__init__()
        self.has_output = False

    def _dart_type(self, ty: TypeAnnotation | None) -> str:
        if ty is None:
            return "void"
        if ty.name in DART_TYPE_MAP:
            return DART_TYPE_MAP[ty.name]
        if ty.name == "Array":
            inner = self._dart_type(ty.type_args[0]) if ty.type_args else "dynamic"
            return f"List<{inner}>"
        if ty.name == "Map":
            k = self._dart_type(ty.type_args[0]) if len(ty.type_args) > 0 else "dynamic"
            v = self._dart_type(ty.type_args[1]) if len(ty.type_args) > 1 else "dynamic"
            return f"Map<{k}, {v}>"
        return ty.name

    # ── Top level ─────────────────────────────────

    def _gen_program(self, prog: Program):
        self.has_output = False

        # Generate imports
        for imp in getattr(prog, 'imports', []):
            dart_path = imp.path.replace('.omni', '.dart')
            names = ", ".join(imp.names)
            self._emit(f"import '{dart_path}' show {names};")

        for imp in getattr(prog, 'namespace_imports', []):
            dart_path = imp.path.replace('.omni', '.dart')
            self._emit(f"import '{dart_path}' as {imp.alias};")

        # Type-only imports: omitted in Dart
        # Re-exports:
        for exp in getattr(prog, 'export_froms', []):
            dart_path = exp.path.replace('.omni', '.dart')
            names = ", ".join(exp.names)
            self._emit(f"export '{dart_path}' show {names};")

        if getattr(prog, 'imports', []) or getattr(prog, 'namespace_imports', []):
            self._emit()

        for struct in getattr(prog, 'structs', []):
            self._gen_struct_def(struct)
        for tu in getattr(prog, 'tagged_unions', []):
            self._gen_tagged_union(tu)
        for cls in getattr(prog, 'classes', []):
            self._gen_class_def(cls)
        for func in prog.functions:
            self._gen_function(func)

        if self.has_output:
            self.output.insert(0, "import 'dart:io';")
            self.output.insert(0, "")

    def _gen_function(self, func: FunctionDef):
        ret = "void"
        if func.return_type:
            ret = self._dart_type(func.return_type)

        # Generic parameters
        generic_str = ""
        if func.generic_params:
            gens = [gp.name for gp in func.generic_params]
            generic_str = f"<{', '.join(gens)}>"

        params_parts = []
        for p in func.params:
            params_parts.append(f"final {self._dart_type(p.type_ann)} {p.name}")
        params_str = ", ".join(params_parts)

        ret_prefix = ""
        if func.is_async:
            ret = f"Future<{ret}>" if ret != "void" else "Future<void>"
            ret_prefix = "async "

        if func.body and func.body.stmts == [] and func.body.trailing_expr:
            expr = self._gen_expr(func.body.trailing_expr)
            self._emit(f"{ret_prefix}{ret} {func.name}{generic_str}({params_str}) => {expr};")
        else:
            self._emit(f"{ret_prefix}{ret} {func.name}{generic_str}({params_str}) {{")
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
            self._gen_var_decl_stmt(stmt)
        elif isinstance(stmt, Assign):
            self._gen_assign_stmt(stmt)
        elif isinstance(stmt, IfStmt):
            self._gen_if_stmt(stmt)
        elif isinstance(stmt, WhileStmt):
            self._gen_while_stmt(stmt)
        elif isinstance(stmt, ForInStmt):
            self._gen_for_in_stmt(stmt)
        elif isinstance(stmt, BreakStmt):
            self._emit("break;")
        elif isinstance(stmt, ContinueStmt):
            self._emit("continue;")
        elif isinstance(stmt, ReturnStmt):
            self._gen_return_stmt(stmt)
        elif isinstance(stmt, PrintStmt):
            self._gen_print_stmt(stmt)
        elif isinstance(stmt, ExprStmt):
            self._gen_expr_stmt(stmt)
        elif isinstance(stmt, Destructure):
            self._gen_destructure_stmt(stmt)

    def _gen_var_decl_stmt(self, stmt):
        ty_name = self._dart_type(stmt.type_ann)
        init = self._gen_expr(stmt.init)

        if isinstance(stmt.init, VarRef):
            container_ty = self.type_map.get(id(stmt.init))
            if container_ty and container_ty.name == "Array":
                init = f"List.of({init})"
            elif container_ty and container_ty.name == "Map":
                init = f"Map.of({init})"
        elif isinstance(stmt.init, ArrayLiteral):
            inner = ""
            if stmt.type_ann.type_args:
                inner = f"<{self._dart_type(stmt.type_ann.type_args[0])}>"
            init = f"{inner}{init}"
        elif isinstance(stmt.init, MapLiteral):
            inner = ""
            if len(stmt.type_ann.type_args) >= 2:
                k = self._dart_type(stmt.type_ann.type_args[0])
                v = self._dart_type(stmt.type_ann.type_args[1])
                inner = f"<{k}, {v}>"
            init = f"{inner}{init}"

        name = self._rename(stmt.name)
        if stmt.mutable:
            self._emit(f"{ty_name} {name} = {init};")
        else:
            self._emit(f"final {ty_name} {name} = {init};")

    def _gen_assign_stmt(self, stmt):
        value = self._gen_expr(stmt.value)
        target = self._rename(stmt.target)

        if stmt.op == "=":
            if stmt.index:
                idx = self._gen_expr(stmt.index)
                self._emit(f"{target}[{idx}] = {value};")
            else:
                self._emit(f"{target} = {value};")
        elif stmt.op in ("/=", "%="):
            if stmt.op == "/=":
                if stmt.index:
                    idx = self._gen_expr(stmt.index)
                    self._emit(f"{target}[{idx}] ~/= {value};")
                else:
                    self._emit(f"{target} ~/= {value};")
            else:
                if stmt.index:
                    idx = self._gen_expr(stmt.index)
                    self._emit(f"{target}[{idx}] %= {value};")
                else:
                    self._emit(f"{target} %= {value};")
        else:
            if stmt.index:
                idx = self._gen_expr(stmt.index)
                self._emit(f"{target}[{idx}] {stmt.op} {value};")
            else:
                self._emit(f"{target} {stmt.op} {value};")

    def _gen_if_stmt(self, stmt):
        cond = self._gen_expr(stmt.condition)
        self._emit(f"if ({cond}) {{")
        self._indent()
        self._enter_scope()
        self._gen_block(stmt.then_body)
        self._leave_scope()
        self._dedent()
        for elif_cond, elif_body in stmt.else_ifs:
            c = self._gen_expr(elif_cond)
            self._emit(f"}} else if ({c}) {{")
            self._indent()
            self._enter_scope()
            self._gen_block(elif_body)
            self._leave_scope()
            self._dedent()
        if stmt.else_body:
            self._emit("} else {")
            self._indent()
            self._enter_scope()
            self._gen_block(stmt.else_body)
            self._leave_scope()
            self._dedent()
        self._emit("}")

    def _gen_while_stmt(self, stmt):
        cond = self._gen_expr(stmt.condition)
        self._emit(f"while ({cond}) {{")
        self._indent()
        self._enter_scope()
        self._gen_block(stmt.body)
        self._leave_scope()
        self._dedent()
        self._emit("}")

    def _gen_for_in_stmt(self, stmt):
        iterable = self._gen_expr(stmt.iterable)
        var_name = self._rename(stmt.var_name)
        self._emit(f"for (final {var_name} in {iterable}) {{")
        self._indent()
        self._enter_scope()
        self._gen_block(stmt.body)
        self._leave_scope()
        self._dedent()
        self._emit("}")

    def _gen_return_stmt(self, stmt):
        if stmt.value:
            val = self._gen_expr(stmt.value)
            self._emit(f"return {val};")
        else:
            self._emit("return;")

    def _gen_print_stmt(self, stmt):
        self.has_output = True
        arg = self._gen_expr(stmt.arg)
        if stmt.newline:
            self._emit(f"stdout.writeln({arg});")
        else:
            self._emit(f"stdout.write({arg});")

    def _gen_expr_stmt(self, stmt):
        code = self._gen_expr(stmt.expr)
        if isinstance(stmt.expr, MethodCall) and stmt.expr.method in ("push", "insert", "pop"):
            self._emit(f"{code};")

    def _gen_destructure_stmt(self, stmt):
        init = self._gen_expr(stmt.init)
        tmp = f"_tmp{self._cnt()}"
        self._emit(f"final {tmp} = {init};")
        self._gen_destructure_lowered(stmt, tmp)

    # ── Expressions ───────────────────────────────

    def _rename(self, name: str) -> str:
        """Look up rename for the current scope level and ancestors."""
        level = len(self.scope_stack) - 1 if hasattr(self, 'scope_stack') else 0
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

    def _gen_expr(self, expr, ctx: str = "default") -> str:
        if expr is None:
            return "null"

        if isinstance(expr, Literal):
            return self._gen_literal_expr(expr)
        elif isinstance(expr, VarRef):
            return self._gen_var_ref_expr(expr)
        elif isinstance(expr, BinaryOp):
            return self._gen_binary_op_expr(expr)
        elif isinstance(expr, UnaryOp):
            return self._gen_unary_op_expr(expr)
        elif isinstance(expr, FuncCall):
            return self._gen_func_call_expr(expr)
        elif isinstance(expr, MethodCall):
            return self._gen_method_call_expr(expr)
        elif isinstance(expr, IndexGet):
            return self._gen_index_get_expr(expr)
        elif isinstance(expr, ArrayLiteral):
            return self._gen_array_literal_expr(expr)
        elif isinstance(expr, MapLiteral):
            return self._gen_map_literal_expr(expr)
        elif isinstance(expr, IfExpr):
            return self._gen_if_expr(expr)
        elif isinstance(expr, TemplateString):
            return self._gen_template_string(expr)
        elif isinstance(expr, NullCoalesce):
            return self._gen_null_coalesce_expr(expr)
        elif isinstance(expr, OptionalChain):
            return self._gen_optional_chain(expr)
        elif isinstance(expr, SpreadExpr):
            return self._gen_spread_expr(expr)
        elif isinstance(expr, StructLiteral):
            return self._gen_struct_literal(expr)
        elif isinstance(expr, ClosureExpr):
            return self._gen_closure(expr)
        elif isinstance(expr, MatchExpr):
            return self._gen_match(expr)
        elif isinstance(expr, AwaitExpr):
            return self._gen_await_expr(expr)

        return "null"

    def _gen_literal_expr(self, expr):
        if expr.kind == "String":
            escaped = expr.value.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")
            return f"'{escaped}'"
        elif expr.kind == "bool":
            return "true" if expr.value else "false"
        return str(expr.value)

    def _gen_var_ref_expr(self, expr):
        return self._rename(expr.name)

    def _gen_binary_op_expr(self, expr):
        left = self._gen_expr(expr.left)
        right = self._gen_expr(expr.right)
        if expr.op == "/":
            return f"({left} ~/ {right})"
        return f"({left} {expr.op} {right})"

    def _gen_unary_op_expr(self, expr):
        operand = self._gen_expr(expr.operand)
        return f"({expr.op} {operand})"

    def _gen_func_call_expr(self, expr):
        func_name = self._gen_expr(expr.func)
        raw_name = expr.func.name if isinstance(expr.func, VarRef) else None

        # ── Async built-in functions ─────────────
        if raw_name == "all":
            if expr.args and isinstance(expr.args[0], ArrayLiteral):
                items = ", ".join(self._gen_expr(e) for e in expr.args[0].elements)
                return f"Future.wait([{items}])"
            inner = self._gen_expr(expr.args[0])
            return f"Future.wait({inner})"
        if raw_name == "race":
            if expr.args and isinstance(expr.args[0], ArrayLiteral):
                items = ", ".join(self._gen_expr(e) for e in expr.args[0].elements)
                return f"Future.any([{items}])"
            inner = self._gen_expr(expr.args[0])
            return f"Future.any({inner})"
        if raw_name == "tryCatch":
            future = self._gen_expr(expr.args[0])
            return f"{future}.then((v) => Ok(v)).catchError((e) => Err(e))"

        args = ", ".join(self._gen_expr(a) for a in expr.args)
        return f"{func_name}({args})"

    def _gen_method_call_expr(self, expr):
        obj = self._gen_expr(expr.object)
        method = expr.method

        if method == "len":
            return f"{obj}.length"
        elif method == "is_empty":
            return f"{obj}.isEmpty"
        elif method == "get":
            arg = ", ".join(self._gen_expr(a) for a in expr.args)
            obj_ty = self.type_map.get(id(expr.object))
            if obj_ty and obj_ty.name == "Map":
                return f"({obj}[{arg}] ?? (throw Exception('Key not found')))"
            return f"{obj}[{arg}]"
        elif method == "push":
            args = ", ".join(self._gen_expr(a) for a in expr.args)
            return f"{obj}.add({args})"
        elif method == "pop":
            return f"{obj}.removeLast()"
        elif method == "contains":
            arg = ", ".join(self._gen_expr(a) for a in expr.args)
            return f"{obj}.containsKey({arg})"
        elif method == "insert":
            k = self._gen_expr(expr.args[0])
            v = self._gen_expr(expr.args[1])
            return f"{obj}[{k}] = {v}"
        elif method == "to_string":
            return f"{obj}.toString()"
        else:
            args = ", ".join(self._gen_expr(a) for a in expr.args)
            return f"{obj}.{method}({args})"

    def _gen_index_get_expr(self, expr):
        obj = self._gen_expr(expr.object)
        idx = self._gen_expr(expr.index)
        obj_ty = self.type_map.get(id(expr.object))
        if obj_ty and obj_ty.name == "Map":
            return f"({obj}[{idx}] ?? (throw Exception('Key not found')))"
        return f"{obj}[{idx}]"

    def _gen_array_literal_expr(self, expr):
        elems = ", ".join(self._gen_expr(e) for e in expr.elements)
        return f"[{elems}]"

    def _gen_map_literal_expr(self, expr):
        entries = ", ".join(
            f"{self._gen_expr(k)}: {self._gen_expr(v)}"
            for k, v in expr.entries
        )
        return f"{{{entries}}}"

    def _gen_if_expr(self, expr):
        cond = self._gen_expr(expr.condition)
        then = self._gen_expr(expr.then_expr)
        else_ = self._gen_expr(expr.else_expr)
        return f"({cond} ? {then} : {else_})"

    def _gen_null_coalesce_expr(self, expr):
        left = self._gen_expr(expr.left)
        right = self._gen_expr(expr.right)
        return f"({left} ?? {right})"

    def _gen_spread_expr(self, expr):
        inner = self._gen_expr(expr.expr)
        return f"...{inner}"

    def _gen_await_expr(self, expr):
        return f"await {self._gen_expr(expr.expr)}"

    # ── v0.2 helpers ──────────────────────────────

    def _cnt(self):
        if not hasattr(self, '_counter'):
            self._counter = 0
        self._counter += 1
        return self._counter

    def _gen_match(self, me: MatchExpr) -> str:
        """Generate Dart if/else chain for match expression."""
        scrutinee = self._gen_expr(me.scrutinee)
        tmp = f"_m{self._cnt()}"
        self._emit(f"final {tmp} = {scrutinee};")
        self._emit("var _result;")
        for i, arm in enumerate(me.arms):
            cond = self._gen_dart_match_cond(tmp, arm)
            body_code = self._gen_dart_arm_body(arm)
            kw = "if" if i == 0 else "else if"
            self._emit(f"{kw} ({cond}) {{")
            self._indent()
            self._emit(f"_result = {body_code};")
            self._dedent()
            self._emit("}")
        self._emit("return _result;")
        return "_result"

    def _gen_dart_match_cond(self, tmp: str, arm: MatchArm) -> str:
        pat = arm.pattern
        if isinstance(pat, IgnorePattern):
            return "true"
        elif isinstance(pat, Literal):
            return f"{tmp} == {self._gen_expr(pat)}"
        elif isinstance(pat, VarPattern):
            return "true"
        elif isinstance(pat, OrPattern):
            return " || ".join(
                f"{tmp} == {self._gen_expr(p)}" if isinstance(p, Literal) else "true"
                for p in pat.patterns
            )
        return "true"

    def _gen_dart_arm_body(self, arm: MatchArm) -> str:
        if isinstance(arm.body, Block):
            return self._gen_block_inline(arm.body)
        return self._gen_expr(arm.body)

    def _gen_closure(self, ce: ClosureExpr) -> str:
        """Generate Dart closure (arrow or block) from ClosureExpr."""
        params_parts = []
        for p in ce.params:
            dart_ty = self._dart_type(p.type_ann) if p.type_ann else "dynamic"
            params_parts.append(f"{dart_ty} {p.name}")
        params_str = ", ".join(params_parts)
        if isinstance(ce.body, Block):
            body_str = self._gen_block_inline(ce.body)
            return f"({params_str}) {{ {body_str} }}"
        else:
            body_code = self._gen_expr(ce.body)
            return f"({params_str}) => {body_code}"

    def _gen_block_inline(self, block: Block) -> str:
        """Generate block statements as inline string (for closures)."""
        old_output = self.output
        self.output = []
        for stmt in block.stmts:
            self._gen_stmt(stmt)
        if block.trailing_expr:
            expr_code = self._gen_expr(block.trailing_expr)
            self._emit(f"return {expr_code};")
        result = " ".join(line.strip() for line in self.output if line.strip())
        self.output = old_output
        return result

    def _gen_template_string(self, ts: TemplateString) -> str:
        parts = []
        for p in ts.parts:
            if p.kind == "literal":
                parts.append(str(p.value).replace(r'$', r'\$'))
            elif p.kind == "expr":
                parts.append(f"${{{self._gen_expr(p.value)}}}")
        return "'" + "".join(parts) + "'"

    def _gen_optional_chain(self, oc: OptionalChain) -> str:
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
        fields = ", ".join(
            f"{name}: {self._gen_expr(value)}"
            for name, value in sl.fields
        )
        return f"{sl.type_name}({fields})"

    def _gen_destructure_lowered(self, stmt: Destructure, tmp_var: str):
        pat = stmt.pattern
        if isinstance(pat, ArrayPattern):
            for i, elem in enumerate(pat.elements):
                if isinstance(elem, VarPattern):
                    self._emit(f"final {elem.name} = {tmp_var}[{i}];")
            if pat.rest and isinstance(pat.rest, RestPattern):
                self._emit(
                    f"final {pat.rest.name} = {tmp_var}"
                    f".sublist({len(pat.elements)});"
                )

    # Note: _gen_program defined above with struct support

    def _gen_tagged_union(self, tu):
        """Dart: tagged union → sealed class hierarchy."""
        self._emit(f"sealed class {tu.name} {{")
        self._indent()
        for v in tu.variants:
            fields = ", ".join(
                f"final {self._dart_type(f.type_ann)} {f.name}" for f in v.fields
            )
            self._emit(f"const {v.name}({{{fields}}});" if fields else f"const {v.name}();")
        self._dedent()
        self._emit("}")
        self._emit()

    def _gen_class_def(self, cls):
        """Dart: class → standard Dart class."""
        base = f" extends {cls.extends}" if cls.extends else ""
        impl = f" implements {', '.join(cls.implements)}" if cls.implements else ""
        self._emit(f"class {cls.name}{base}{impl} {{")
        self._indent()
        for m in cls.members:
            if m.func_def:
                self._gen_function(m.func_def)
            elif m.name:
                priv = "_" if m.visibility == "private" else ""
                self._emit(f"final {self._dart_type(m.type_ann)} {priv}{m.name};")
        if cls.members:
            params = ", ".join(
                f"required this.{'_' if m.visibility == 'private' else ''}{m.name}"
                for m in cls.members if not m.func_def
            )
            self._emit(f"const {cls.name}({{{params}}});" if params else f"const {cls.name}();")
        self._dedent()
        self._emit("}")
        self._emit()

    def _gen_struct_def(self, sd: StructDef):
        self._emit(f"class {sd.name} {{")
        self._indent()
        for f in sd.fields:
            dart_ty = self._dart_type(f.type_ann)
            self._emit(f"final {dart_ty} {f.name};")
        self._emit()
        # Constructor
        params = ", ".join(f"required this.{f.name}" for f in sd.fields)
        self._emit(f"const {sd.name}({{{params}}});")
        self._dedent()
        self._emit("}")
        self._emit()

    def _dart_type(self, ty) -> str:
        if ty is None:
            return "void"
        # Handle UnionType (T | null → T?)
        if isinstance(ty, UnionType):
            non_null = [t for t in ty.types if t.name != "null"]
            if non_null:
                base = self._dart_type(non_null[0])
                return f"{base}?"
            return "void"
        mapping = {
            "i32": "int", "f64": "double", "bool": "bool",
            "String": "String", "void": "void",
        }
        if ty.name in mapping:
            return mapping[ty.name]
        if ty.name == "Array" and ty.type_args:
            return f"List<{self._dart_type(ty.type_args[0])}>"
        if ty.name == "Map" and len(ty.type_args) >= 2:
            return f"Map<{self._dart_type(ty.type_args[0])}, {self._dart_type(ty.type_args[1])}>"
        return ty.name
