"""Rust code generator for OmniLang."""

from omnilang.codegen.base import CodeGenerator
from omnilang.ast_nodes import (
    Program, FunctionDef, Block,
    VarDecl, Assign, Destructure, IfStmt, WhileStmt, ForInStmt,
    BreakStmt, ContinueStmt, ReturnStmt, PrintStmt, ExprStmt,
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    TemplateString, NullCoalesce, ClosureExpr, AwaitExpr, SpreadExpr, StructLiteral, StructDef,
    MatchExpr, MatchArm, OrPattern, StructPattern,
    ArrayPattern, VarPattern, IgnorePattern, RestPattern,
    TaggedUnionDef,
    TypeAnnotation,
)

RUST_TYPE_MAP = {
    "void": "()",
    "i32": "i32",
    "f64": "f64",
    "bool": "bool",
    "String": "String",
}


class RustGenerator(CodeGenerator):

    def __init__(self):
        super().__init__()
        self.temp_counter = 0
        self.has_map = False
        self.current_scope_stack: list[dict] = []  # for variable type tracking
        self.needs_clone: set[str] = set()  # container vars that are let, not let mut
        self.for_vars: set[str] = set()  # for-in loop variables (need deref for Copy types)
        self.cell_vars: set[str] = set()  # vars wrapped in Cell
        self.refcell_vars: set[str] = set()  # vars wrapped in RefCell
        self.capture_info: dict = {}  # from capture analyzer

    def _rust_type(self, ty: TypeAnnotation | None, param: bool = False) -> str:
        if ty is None:
            return "()"
        if ty.name in RUST_TYPE_MAP:
            return RUST_TYPE_MAP[ty.name]
        if ty.name == "Array":
            inner = self._rust_type(ty.type_args[0]) if ty.type_args else "i32"
            return f"Vec<{inner}>"
        if ty.name == "Map":
            k = self._rust_type(ty.type_args[0]) if len(ty.type_args) > 0 else "String"
            v = self._rust_type(ty.type_args[1]) if len(ty.type_args) > 1 else "i32"
            self.has_map = True
            if param:
                return f"&HashMap<{k}, {v}>"
            return f"HashMap<{k}, {v}>"
        return ty.name

    def _new_temp(self) -> str:
        self.temp_counter += 1
        return f"_tmp_{self.temp_counter}"

    def _lookup_var_type(self, name: str) -> TypeAnnotation | None:
        for scope in reversed(self.current_scope_stack):
            if name in scope:
                return scope[name]
        return None

    def _register_var(self, name: str, ty: TypeAnnotation):
        if self.current_scope_stack:
            self.current_scope_stack[-1][name] = ty

    def _is_copy(self, ty: TypeAnnotation | None) -> bool:
        if ty is None:
            return True
        return ty.name in ("i32", "f64", "bool")

    def _is_non_copy(self, ty: TypeAnnotation | None) -> bool:
        return not self._is_copy(ty)

    # ── Import/Export generation ──────────────────

    def _gen_imports_exports(self, prog):
        def _rust_path(path):
            """Convert OmniLang module path to Rust module path."""
            p = path.replace('.omni', '')
            if p.startswith('./'):
                p = p[2:]
            return p.replace('/', '::')

        # Named imports: use crate::module::{Name1, Name2};
        for imp in getattr(prog, 'imports', []):
            module_path = _rust_path(imp.path)
            names = ", ".join(imp.names)
            self._emit(f'use crate::{module_path}::{{{names}}};')

        # Namespace imports: use crate::module as alias;
        for imp in getattr(prog, 'namespace_imports', []):
            module_path = _rust_path(imp.path)
            self._emit(f'use crate::{module_path} as {imp.alias};')

        # Type-only imports: same as named in Rust
        for imp in getattr(prog, 'type_imports', []):
            module_path = _rust_path(imp.path)
            names = ", ".join(imp.names)
            self._emit(f'use crate::{module_path}::{{{names}}};')

        # Re-exports: pub use crate::module::{Names};
        for exp in getattr(prog, 'export_froms', []):
            module_path = _rust_path(exp.path)
            names = ", ".join(exp.names)
            self._emit(f'pub use crate::{module_path}::{{{names}}};')

        if getattr(prog, 'imports', []) or getattr(prog, 'namespace_imports', []):
            self._emit()

    # ── Top level ─────────────────────────────────

    def _gen_program(self, prog: Program):
        self._gen_imports_exports(prog)
        self.program = prog  # store for FuncCall lookup
        self.has_map = False
        if any(f for f in prog.functions if self._func_uses_map(f)):
            self.has_map = True

        # Run capture analysis for Cell/RefCell wrapping
        from omnilang.semantic.capture_analyzer import analyze_captures, CaptureKind
        self.capture_info = analyze_captures(prog)
        self.cell_vars.clear()
        self.refcell_vars.clear()
        for ce_id, caps in self.capture_info.items():
            for c in caps:
                if c.kind == CaptureKind.MUTABLE:
                    if c.var_type and self._is_copy(c.var_type):
                        self.cell_vars.add(c.name)
                    else:
                        self.refcell_vars.add(c.name)

        needs_cell = bool(self.cell_vars)
        needs_refcell = bool(self.refcell_vars)
        if needs_cell and needs_refcell:
            self._emit("use std::cell::{Cell, RefCell};")
        elif needs_cell:
            self._emit("use std::cell::Cell;")
        elif needs_refcell:
            self._emit("use std::cell::RefCell;")

        if self.has_map:
            self._emit("use std::collections::HashMap;")
            self._emit()

        # Detect async main for tokio::main attribute
        main_func = next((f for f in prog.functions if f.name == "main"), None)
        if main_func and main_func.is_async:
            self._emit("#[tokio::main]")

        # Emit struct definitions first
        for struct in getattr(prog, 'structs', []):
            self._gen_struct_def(struct)

        for tu in getattr(prog, 'tagged_unions', []):
            self._gen_tagged_union(tu)
        for func in prog.functions:
            self._gen_function(func)

    def _func_uses_map(self, func: FunctionDef) -> bool:
        """Quick scan for Map type usage."""
        def scan_block(block):
            for stmt in block.stmts:
                if isinstance(stmt, VarDecl) and stmt.type_ann and stmt.type_ann.name == "Map":
                    return True
            return False
        return func.body and scan_block(func.body)

    def _gen_function(self, func: FunctionDef):
        self.current_scope_stack.append({})
        # Register parameters
        for p in func.params:
            self._register_var(p.name, p.type_ann)

        ret = ""
        if func.return_type and func.return_type.name != "void":
            ret = f" -> {self._rust_type(func.return_type)}"

        # Generic parameters
        generic_str = ""
        if func.generic_params:
            generics = []
            for gp in func.generic_params:
                bounds = " + ".join(gp.bounds) if gp.bounds else ""
                if bounds:
                    generics.append(f"{gp.name}: {bounds}")
                else:
                    generics.append(gp.name)
            generic_str = f"<{', '.join(generics)}>"

        params_parts = []
        for p in func.params:
            if p.type_ann and p.type_ann.name == "String":
                param_ty = "&str"
            else:
                param_ty = self._rust_type(p.type_ann, param=True)
                if self._is_non_copy(p.type_ann) and not param_ty.startswith("&"):
                    param_ty = f"&{param_ty}"
            params_parts.append(f"{p.name}: {param_ty}")
        params_str = ", ".join(params_parts)

        vis = "pub " if getattr(func, 'is_pub', False) else ""
        prefix = "async " if func.is_async else ""
        self._emit(f"{vis}{prefix}fn {func.name}{generic_str}({params_str}){ret} {{")
        self._indent()
        if func.body:
            self._gen_block(func.body)
        self._dedent()
        self._emit("}")
        self._emit()

        self.current_scope_stack.pop()

    # ── Block ─────────────────────────────────────

    def _gen_block(self, block: Block):
        for stmt in block.stmts:
            self._gen_stmt(stmt)
        if block.trailing_expr:
            expr_code = self._gen_expr(block.trailing_expr)
            self._emit(expr_code)

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
        is_container = stmt.type_ann and stmt.type_ann.name in ("Array", "Map", "String")
        needs_mut = stmt.mutable or (is_container and not stmt.mutable)
        keyword = "let mut" if needs_mut else "let"
        ty_str = self._rust_type(stmt.type_ann)

        # Cell/RefCell wrapping for captured mutable variables
        if stmt.name in self.cell_vars:
            ty_str = f"Cell<{ty_str}>"
            keyword = "let"
        elif stmt.name in self.refcell_vars:
            ty_str = f"RefCell<{ty_str}>"
            keyword = "let"

        init = self._gen_expr(stmt.init, need_ownership=True)

        # Wrap initializer for Cell/RefCell
        if stmt.name in self.cell_vars:
            init = f"Cell::new({init})"
        elif stmt.name in self.refcell_vars:
            init = f"RefCell::new({init})"

        # Handle cloning from variable
        if isinstance(stmt.init, VarRef) and is_container:
            var_ty = self._lookup_var_type(stmt.init.name)
            if var_ty:
                if var_ty.name == "String":
                    init = self._gen_expr(stmt.init) + ".to_string()"
                elif var_ty.name in ("Array", "Map"):
                    init = self._gen_expr(stmt.init) + ".clone()"

        self._emit(f"{keyword} {stmt.name}: {ty_str} = {init};")
        self._register_var(stmt.name, stmt.type_ann)

    def _gen_assign_stmt(self, stmt):
        if stmt.op == "=":
            # Cell/RefCell: use .set() or .borrow_mut() assignment
            if stmt.target in self.cell_vars:
                val = self._emit_call_arg_expr(stmt.value)
                self._emit(f"{stmt.target}.set({val});")
                return
            elif stmt.target in self.refcell_vars:
                val = self._emit_call_arg_expr(stmt.value)
                self._emit(f"*{stmt.target}.borrow_mut() = {val};")
                return
            if stmt.index:
                idx = self._gen_expr(stmt.index)
                val = self._emit_call_arg_expr(stmt.value)
                target_ty = self._lookup_var_type(stmt.target)
                if target_ty and target_ty.name == "Map":
                    # For insert, key is taken by value, not reference
                    k = self._gen_expr(stmt.index)
                    if isinstance(stmt.index, VarRef):
                        var_ty = self._lookup_var_type(stmt.index.name)
                        if var_ty and var_ty.name == "String":
                            k = f"{k}.clone()"
                    self._emit(f"{stmt.target}.insert({k}, {val});")
                else:
                    self._emit(f"{stmt.target}[{idx} as usize] = {val};")
            else:
                val = self._gen_expr(stmt.value)
                self._emit(f"{stmt.target} = {val};")
        elif stmt.op in ("/=", "%="):
            val = self._gen_expr(stmt.value)
            if stmt.index:
                idx = self._gen_expr(stmt.index)
                self._emit(f"{stmt.target}[{idx} as usize] {stmt.op} {val};")
            else:
                # Cell/RefCell: use .set() or .borrow_mut()
                if stmt.target in self.cell_vars:
                    self._emit(f"{stmt.target}.set({stmt.target}.get() {stmt.op} {val});")
                elif stmt.target in self.refcell_vars:
                    self._emit(f"{stmt.target}.borrow_mut().{stmt.op.lstrip('=' )} {val};")
                else:
                    self._emit(f"{stmt.target} {stmt.op} {val};")
        else:
            val = self._gen_expr(stmt.value)
            if stmt.index:
                idx = self._gen_expr(stmt.index)
                self._emit(f"{stmt.target}[{idx} as usize] {stmt.op} {val};")
            else:
                op_map = {"+=": "+=", "-=": "-=", "*=": "*="}
                self._emit(f"{stmt.target} {op_map.get(stmt.op, stmt.op)} {val};")

    def _gen_if_stmt(self, stmt):
        cond = self._gen_expr(stmt.condition)
        self._emit(f"if {cond} {{")
        self._indent()
        self._gen_block(stmt.then_body)
        self._dedent()
        for elif_cond, elif_body in stmt.else_ifs:
            c = self._gen_expr(elif_cond)
            self._emit(f"}} else if {c} {{")
            self._indent()
            self._gen_block(elif_body)
            self._dedent()
        if stmt.else_body:
            self._emit("} else {")
            self._indent()
            self._gen_block(stmt.else_body)
            self._dedent()
        self._emit("}")

    def _gen_while_stmt(self, stmt):
        cond = self._gen_expr(stmt.condition)
        self._emit(f"while {cond} {{")
        self._indent()
        self._gen_block(stmt.body)
        self._dedent()
        self._emit("}")

    def _gen_for_in_stmt(self, stmt):
        iterable_expr = self._gen_expr(stmt.iterable)
        # Register loop variable and track for deref
        iter_ty = self.type_map.get(id(stmt))
        self._register_var(stmt.var_name, iter_ty)
        self.for_vars.add(stmt.var_name)

        # Reference the array
        iter_ref = f"&{iterable_expr}"
        self._emit(f"for {stmt.var_name} in {iter_ref} {{")
        self._indent()
        self._gen_block(stmt.body)
        self._dedent()
        self._emit("}")

        self.for_vars.discard(stmt.var_name)

    def _gen_return_stmt(self, stmt):
        if stmt.value:
            val = self._gen_expr(stmt.value, need_ownership=True)
            self._emit(f"return {val};")
        else:
            self._emit("return;")

    def _gen_print_stmt(self, stmt):
        arg = self._gen_expr(stmt.arg)
        if stmt.newline:
            self._emit(f'println!("{{}}", {arg});')
        else:
            self._emit(f'print!("{{}}", {arg});')

    def _gen_expr_stmt(self, stmt):
        # Generate expression statement (with semicolon)
        expr_code = self._gen_expr(stmt.expr)
        self._emit(f"{expr_code};")

    def _gen_destructure_stmt(self, stmt):
        # Lower destructuring to manual index access
        init = self._gen_expr(stmt.init)
        tmp = self._new_temp()
        self._emit(f"let {tmp} = {init};")
        self._gen_destructure_lowered(stmt, tmp)

    # ── Helper: argument generation for calls ─────

    def _emit_call_arg(self, expr, param_type: TypeAnnotation | None) -> str:
        """Generate a function call argument, adding & for non-Copy types."""
        if param_type is None:
            return self._gen_expr(expr)

        if self._is_copy(param_type):
            return self._gen_expr(expr)

        # Non-copy: need reference
        if isinstance(expr, Literal) and expr.kind == "String":
            # Pass string literal as &str directly
            escaped = expr.value.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")
            return f'"{escaped}"'
        elif isinstance(expr, VarRef):
            return f"&{self._gen_expr(expr)}"
        else:
            # Expression — need temporary variable to hold the value, then reference it
            val = self._gen_expr(expr)
            tmp = self._new_temp()
            rust_ty = self._rust_type(param_type)
            self._emit(f"let {tmp}: {rust_ty} = {val};")
            return f"&{tmp}"

    def _emit_call_arg_expr(self, expr) -> str:
        """Generate a value for passing to a method (push, insert, etc.),
        cloning variables but moving temporaries."""
        if isinstance(expr, VarRef):
            var_ty = self._lookup_var_type(expr.name)
            if var_ty and self._is_non_copy(var_ty):
                if var_ty.name == "String":
                    return f"{expr.name}.to_string()"
                else:
                    return f"{expr.name}.clone()"
        return self._gen_expr(expr)

    # ── Expressions ───────────────────────────────

    def _gen_expr(self, expr, need_ownership: bool = False, ctx: str = "default") -> str:
        if expr is None:
            return "()"

        if isinstance(expr, Literal):
            return self._gen_literal_expr(expr)
        elif isinstance(expr, VarRef):
            return self._gen_var_ref_expr(expr, need_ownership=need_ownership, ctx=ctx)
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

        return "()"

    def _gen_literal_expr(self, expr):
        if expr.kind == "String":
            escaped = expr.value.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")
            return f'String::from("{escaped}")'
        elif expr.kind == "bool":
            return "true" if expr.value else "false"
        return str(expr.value)

    def _gen_var_ref_expr(self, expr, need_ownership: bool = False, ctx: str = "default"):
        name = expr.name
        var_ty = self._lookup_var_type(name)
        # Cell/RefCell: use .get() or .borrow() for read access
        if name in self.cell_vars and ctx != "method_receiver":
            return f"{name}.get()"
        elif name in self.refcell_vars and ctx != "method_receiver":
            return f"{name}.borrow()"
        # Auto-deref for-in loop variables (Copy types only, for non-method-receiver use)
        if name in self.for_vars and var_ty and self._is_copy(var_ty) and ctx != "method_receiver":
            return f"(*{name})"
        if need_ownership and var_ty and self._is_non_copy(var_ty):
            if var_ty.name == "String":
                return f"{name}.to_string()"
            elif var_ty.name in ("Array", "Map"):
                return f"{name}.clone()"
        # Auto-deref for-in loop variables (Copy types only, for non-method-receiver use)
        if name in self.for_vars and var_ty and self._is_copy(var_ty) and ctx != "method_receiver":
            return f"(*{name})"
        return name

    def _gen_binary_op_expr(self, expr):
        left = self._gen_expr(expr.left)
        right = self._gen_expr(expr.right)

        if expr.op == "+":
            left_ty = self.type_map.get(id(expr.left))
            if left_ty and left_ty.name == "String":
                return f'format!("{{}}{{}}", {left}, {right})'
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
                items = ", ".join(
                    self._gen_expr(e) for e in expr.args[0].elements
                )
                return f"futures::future::join_all(vec![{items}]).await"
            inner = self._gen_expr(expr.args[0])
            return f"futures::future::join_all({inner}).await"
        if raw_name == "race":
            if expr.args and isinstance(expr.args[0], ArrayLiteral):
                branches = ",\n".join(
                    f"        result = {self._gen_expr(e)} => result"
                    for e in expr.args[0].elements
                )
                return f"tokio::select! {{\n{branches},\n    }}"
            inner = self._gen_expr(expr.args[0])
            return f"tokio::select! {{\n        result = {inner} => result,\n    }}"
        if raw_name == "tryCatch":
            future = self._gen_expr(expr.args[0])
            return f"async {{ Ok({future}.await) }}"

        # Use _emit_call_arg for proper reference handling
        # Find the function definition in the program
        func_def = None
        if isinstance(expr.func, VarRef):
            for f in self.program.functions if hasattr(self, 'program') else []:
                if f.name == expr.func.name:
                    func_def = f
                    break

        args = []
        for i, arg in enumerate(expr.args):
            param_type = None
            if func_def and i < len(func_def.params):
                param_type = func_def.params[i].type_ann
            args.append(self._emit_call_arg(arg, param_type))

        args_str = ", ".join(args)
        return f"{func_name}({args_str})"

    def _gen_method_call_expr(self, expr):
        obj = self._gen_expr(expr.object, ctx="method_receiver")
        method = expr.method
        obj_ty = self.type_map.get(id(expr.object))

        if method == "len":
            return f"({obj}.len() as i32)"
        elif method == "is_empty":
            return f"{obj}.is_empty()"
        elif method == "get":
            if obj_ty and obj_ty.name == "Array":
                arg = ", ".join(self._gen_expr(a) for a in expr.args)
                if obj_ty.type_args:
                    elem_ty = obj_ty.type_args[0]
                    if elem_ty.name == "String":
                        return f"{obj}[{arg} as usize].to_string()"
                    return f"{obj}[{arg} as usize]"
                return f"{obj}[{arg} as usize]"
            elif obj_ty and obj_ty.name == "Map":
                key = self._emit_map_key(expr.args[0])
                if obj_ty.type_args:
                    elem_ty = obj_ty.type_args[1] if len(obj_ty.type_args) > 1 else None
                    if elem_ty and elem_ty.name == "String":
                        return f"{obj}[{key}].to_string()"
                    return f"{obj}[{key}]"
                return f"{obj}[{key}]"
            return f"{obj}[...]"
        elif method == "push":
            arg = self._emit_call_arg_expr(expr.args[0])
            return f"{obj}.push({arg})"
        elif method == "pop":
            return f'{obj}.pop().expect("empty")'
        elif method == "contains":
            arg = self._emit_map_key(expr.args[0])
            return f"{obj}.contains_key({arg})"
        elif method == "insert":
            # insert takes ownership of key and value
            k = self._gen_expr(expr.args[0])
            if isinstance(expr.args[0], VarRef):
                var_ty = self._lookup_var_type(expr.args[0].name)
                if var_ty and var_ty.name == "String":
                    k = f"{k}.clone()"
            v = self._emit_call_arg_expr(expr.args[1])
            return f"{obj}.insert({k}, {v})"
        elif method == "to_string":
            return f"{obj}.to_string()"
        else:
            args = ", ".join(self._gen_expr(a) for a in expr.args)
            return f"{obj}.{method}({args})"

    def _gen_index_get_expr(self, expr):
        obj = self._gen_expr(expr.object)
        idx = self._gen_expr(expr.index)
        obj_ty = self.type_map.get(id(expr.object))

        if obj_ty and obj_ty.type_args:
            elem_ty = obj_ty.type_args[0]
            if obj_ty.name == "Array":
                if elem_ty.name == "String":
                    return f"{obj}[{idx} as usize].to_string()"
                return f"{obj}[{idx} as usize]"
            elif obj_ty.name == "Map":
                key = self._emit_map_key(expr.index)
                if elem_ty.name == "String":
                    return f"{obj}[{key}].to_string()"
                return f"{obj}[{key}]"
        return f"{obj}[{idx} as usize]"

    def _gen_array_literal_expr(self, expr):
        elems = ", ".join(
            self._emit_call_arg_expr(e) if isinstance(e, VarRef)
            else self._gen_expr(e)
            for e in expr.elements
        )
        return f"vec![{elems}]"

    def _gen_map_literal_expr(self, expr):
        entries = ", ".join(
            f"({self._gen_expr(k)}, {self._gen_expr(v)})"
            for k, v in expr.entries
        )
        self.has_map = True
        return f"HashMap::from([{entries}])"

    def _gen_if_expr(self, expr):
        cond = self._gen_expr(expr.condition)
        then = self._gen_expr(expr.then_expr)
        else_ = self._gen_expr(expr.else_expr)
        return f"if {cond} {{ {then} }} else {{ {else_} }}"

    def _gen_null_coalesce_expr(self, expr):
        left = self._gen_expr(expr.left)
        right = self._gen_expr(expr.right)
        return f"{left}.unwrap_or({right})"

    def _gen_spread_expr(self, expr):
        inner = self._gen_expr(expr.expr)
        return inner  # In array context: vec![...inner]

    def _gen_await_expr(self, expr):
        return f"{self._gen_expr(expr.expr)}.await"

    # ── v0.2 helpers ──────────────────────────────

    def _gen_match(self, me: MatchExpr) -> str:
        """Generate Rust match expression."""
        scrutinee = self._gen_expr(me.scrutinee)
        arms = []
        for arm in me.arms:
            pat = self._gen_rust_match_pattern(arm.pattern)
            body = self._gen_rust_arm_body(arm)
            arms.append(f"{pat} => {body}")
        arms_str = ",\n".join(arms)
        return f"match {scrutinee} {{\n{arms_str}\n}}"

    def _gen_rust_match_pattern(self, pat) -> str:
        if isinstance(pat, IgnorePattern):
            return "_"
        elif isinstance(pat, Literal):
            return self._gen_expr(pat)
        elif isinstance(pat, VarPattern):
            return pat.name
        elif isinstance(pat, OrPattern):
            return " | ".join(self._gen_rust_match_pattern(p) for p in pat.patterns)
        elif isinstance(pat, StructPattern):
            if pat.fields:
                fields = ", ".join(
                    f"{name}: {self._gen_rust_match_pattern(p)}"
                    for name, p in pat.fields
                )
                return f"{pat.name} {{ {fields} }}"
            return pat.name
        return "_"

    def _gen_rust_arm_body(self, arm: MatchArm) -> str:
        if isinstance(arm.body, Block):
            old = self.output
            self.output = []
            self._gen_block(arm.body)
            result = "".join(self.output)
            self.output = old
            return f"{{ {result} }}"
        return f"{{ {self._gen_expr(arm.body)} }}"

    def _gen_closure(self, ce: ClosureExpr) -> str:
        """Generate Rust closure with Cell/RefCell auto-wrapping for mutable captures.

        Strategy:
        - Copy type + mut capture → Cell<T> (e.g., Cell<i32>)
        - Non-Copy type + mut capture → RefCell<T> (e.g., RefCell<String>)
        - Immutable capture → direct (Copy) or reference (non-Copy)
        - Escaping closure → `move` keyword
        """
        params_parts = []
        for p in ce.params:
            rust_ty = self._rust_type(p.type_ann) if p.type_ann else "i32"
            if self._is_non_copy(p.type_ann) and p.type_ann and p.type_ann.name == "String":
                rust_ty = "&str"
            params_parts.append(f"{p.name}: {rust_ty}")
        params_str = ", ".join(params_parts)

        # Check capture_info from semantic analysis
        from omnilang.semantic.capture_analyzer import analyze_captures, CaptureKind
        capture_map = analyze_captures(self.program) if hasattr(self, 'program') else {}
        captures = capture_map.get(id(ce), [])

        # Emit Cell/RefCell preamble if needed
        needs_cell = any(c.kind == CaptureKind.MUTABLE and c.var_type and self._is_copy(c.var_type) for c in captures)
        needs_refcell = any(c.kind == CaptureKind.MUTABLE and c.var_type and self._is_non_copy(c.var_type) for c in captures)
        has_escaped = any(c.escaped for c in captures)

        preamble = ""
        if needs_cell:
            preamble += "use std::cell::Cell; "
        if needs_refcell:
            preamble += "use std::cell::RefCell; "

        move_kw = "move " if has_escaped else ""

        if isinstance(ce.body, Block):
            old = self.output
            block_output = []
            self.output = block_output
            self._gen_block(ce.body)
            self.output = old
            body_str = "\n".join(block_output)
            return f"{move_kw}|{params_str}| {{\n{body_str}\n}}"
        else:
            body_code = self._gen_expr(ce.body)
            return f"{move_kw}|{params_str}| {{ {body_code} }}" if params_str else f"{move_kw}|| {{ {body_code} }}"

    def _gen_template_string(self, ts: TemplateString) -> str:
        parts = []
        format_args = []
        for p in ts.parts:
            if p.kind == "literal":
                parts.append(str(p.value).replace("{", "{{").replace("}", "}}"))
            elif p.kind == "expr":
                parts.append("{}")
                format_args.append(self._gen_expr(p.value))
        fmt_str = "".join(parts)
        if format_args:
            args = ", ".join(format_args)
            return f'format!("{fmt_str}", {args})'
        return f'"{fmt_str}".to_string()'

    def _gen_struct_literal(self, sl: StructLiteral) -> str:
        fields = ", ".join(
            f"{name}: {self._gen_expr(value)}"
            for name, value in sl.fields
        )
        return f"{sl.type_name} {{ {fields} }}"

    def _gen_destructure_lowered(self, stmt: Destructure, tmp_var: str):
        """Lower array destructuring to index access + let bindings."""
        pat = stmt.pattern
        if isinstance(pat, ArrayPattern):
            for i, elem in enumerate(pat.elements):
                if isinstance(elem, VarPattern):
                    self._emit(f"let {elem.name} = {tmp_var}[{i} as usize];")
                elif isinstance(elem, IgnorePattern):
                    pass  # _ is ignored
            if pat.rest and isinstance(pat.rest, RestPattern):
                self._emit(
                    f"let {pat.rest.name}: Vec<_> = "
                    f"{tmp_var}[{len(pat.elements)}..].to_vec();"
                )

    # Note: _gen_program defined above with struct support

    def _gen_tagged_union(self, tu: TaggedUnionDef):
        """Rust: tagged unions become native Rust enums."""
        generic_str = ""
        if tu.generic_params:
            gens = [g.name for g in tu.generic_params]
            generic_str = f"<{', '.join(gens)}>"
        self._emit("#[derive(Debug, Clone)]")
        self._emit(f"enum {tu.name}{generic_str} {{")
        self._indent()
        for v in tu.variants:
            if v.fields:
                fields = ", ".join(
                    f"{f.name}: {self._rust_type(f.type_ann)}" for f in v.fields
                )
                self._emit(f"{v.name} {{ {fields} }},")
            else:
                self._emit(f"{v.name},")
        self._dedent()
        self._emit("}")
        self._emit()

    def _gen_struct_def(self, sd: StructDef):
        """Generate Rust struct from OmniLang struct definition."""
        # Derive attributes
        derives = ["Debug", "Clone"]
        if not any(f.type_ann and f.type_ann.name == "String" and not self._is_copy(f.type_ann)
                   for f in sd.fields):
            derives.append("Copy")
        self._emit("#[derive({})]".format(", ".join(derives)))
        vis = "pub " if getattr(sd, 'is_pub', False) else ""
        self._emit(f"{vis}struct {sd.name} {{")
        self._indent()
        for f in sd.fields:
            rust_ty = self._rust_type(f.type_ann)
            self._emit(f"pub {f.name}: {rust_ty},")
        self._dedent()
        self._emit("}")
        self._emit()

    def _emit_map_key(self, expr) -> str:
        """Generate a Map key reference with proper & handling."""
        if isinstance(expr, Literal):
            if expr.kind == "String":
                return f"&{self._gen_expr(expr)}"
            return f"&{self._gen_expr(expr)}"
        elif isinstance(expr, VarRef):
            return f"&{expr.name}"
        return f"&({self._gen_expr(expr)})"
