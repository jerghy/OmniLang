"""Type checker — validates type consistency and infers types for for-in variables."""

from omnilang.ast_nodes import (
    Program, FunctionDef, Block, VarDecl, Assign, Destructure,
    IfStmt, WhileStmt, ForInStmt, ReturnStmt, PrintStmt, ExprStmt,
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    TypeAnnotation, UnionType, ClosureExpr, MatchExpr, Block as BlockNode,
    Expr,
)
from omnilang.utils import ErrorCollector
from omnilang.semantic.resolver import ScopeInfo, Scope, lookup_var


# ── Builtin method signatures ──────────────────────

# Returns: ([param_types], return_type, is_mut)
BUILTIN_METHODS = {
    "i32": {
        "to_string": (lambda: ([], "String", False)),
    },
    "f64": {
        "to_string": (lambda: ([], "String", False)),
    },
    "bool": {
        "to_string": (lambda: ([], "String", False)),
    },
    "String": {
        "to_string": (lambda: ([], "String", False)),
    },
    "Array": {
        "len":       lambda T: ([], "i32", False),
        "is_empty":  lambda T: ([], "bool", False),
        "get":       lambda T: (["i32"], T, False),
        "push":      lambda T: ([T], "void", True),
        "pop":       lambda T: ([], T, True),
    },
    "Map": {
        "len":       lambda K, V: ([], "i32", False),
        "is_empty":  lambda K, V: ([], "bool", False),
        "get":       lambda K, V: ([K], V, False),
        "contains":  lambda K, V: ([K], "bool", False),
        "insert":    lambda K, V: ([K, V], "void", True),
    },
}

ALLOWED_ELEMENT_TYPES = {"i32", "f64", "bool", "String"}
ALLOWED_KEY_TYPES = {"i32", "bool", "String"}


def _base_type_name(ty) -> str:
    """Get the base type name from a TypeAnnotation or UnionType."""
    if ty is None:
        return "<unknown>"
    if isinstance(ty, UnionType):
        return " | ".join(t.name for t in ty.types)
    return ty.name


def _unwrap_type(ty):
    """Extract a single TypeAnnotation from a potentially union type.

    For union types, returns the first non-null type (or None if all null).
    For simple types, returns the type itself.
    """
    if ty is None:
        return None
    if isinstance(ty, UnionType):
        for t in ty.types:
            if t.name != "null":
                return t
        return ty.types[0] if ty.types else None
    return ty


def check(program: Program, scope_info: ScopeInfo, errors: ErrorCollector) -> dict:
    """Run type checking on the entire program. Returns type_map: {id(expr) → TypeAnnotation}."""
    type_map: dict[int, TypeAnnotation] = {}

    checker = _TypeChecker(program, scope_info, type_map, errors)
    checker.check_program(program)

    return type_map


class _TypeChecker:
    def __init__(self, program: Program, scope_info: ScopeInfo, type_map: dict, errors: ErrorCollector):
        self.program = program
        self.scope_info = scope_info
        self.type_map = type_map
        self.errors = errors
        self.current_scope: Scope = scope_info.global_scope
        self.current_func: FunctionDef | None = None
        # Build function lookup
        self.func_map: dict[str, FunctionDef] = {f.name: f for f in program.functions}

    def _add_type(self, node, ty: TypeAnnotation):
        self.type_map[id(node)] = ty
        return ty

    def _error(self, msg: str, span=None):
        self.errors.add(msg, span)

    # ── Top-level ────────────────────────────────

    def check_program(self, program: Program):
        for func in program.functions:
            self.check_function(func)

    def check_function(self, func: FunctionDef):
        self.current_func = func
        func_scope = self.scope_info.func_scopes.get(func.name)
        if func_scope:
            self.current_scope = func_scope

        if func.body:
            self.check_block(func.body)

        # Check return type consistency
        if func.name == "main":
            return
        ret_ty = _unwrap_type(func.return_type)
        if ret_ty and ret_ty.name != "void":
            # Ensure the function returns a value
            if func.body:
                has_return = any(isinstance(s, ReturnStmt) and s.value is not None
                                for s in func.body.stmts)
                has_trailing = func.body.trailing_expr is not None
                if not has_return and not has_trailing:
                    self._error(f"Function '{func.name}' must return a value of type {func.return_type}",
                               func.span)

        self.current_func = None

    def check_block(self, block: Block):
        old_scope = self.current_scope
        for stmt in block.stmts:
            self.check_stmt(stmt)
        if block.trailing_expr:
            trailing_ty = self.infer_expr(block.trailing_expr)
            self._add_type(block.trailing_expr, trailing_ty)
        self.current_scope = old_scope

    # ── Statements ────────────────────────────────

    def check_stmt(self, stmt):
        if isinstance(stmt, VarDecl):
            self._check_var_decl(stmt)
        elif isinstance(stmt, Destructure):
            self._check_destructure(stmt)
        elif isinstance(stmt, Assign):
            self._check_assign(stmt)
        elif isinstance(stmt, IfStmt):
            self._check_if_stmt(stmt)
        elif isinstance(stmt, WhileStmt):
            self._check_while_stmt(stmt)
        elif isinstance(stmt, ForInStmt):
            self._check_for_stmt(stmt)
        elif isinstance(stmt, ReturnStmt):
            self._check_return_stmt(stmt)
        elif isinstance(stmt, PrintStmt):
            self._check_print_stmt(stmt)
        elif isinstance(stmt, ExprStmt):
            self.infer_expr(stmt.expr)

    def _check_destructure(self, stmt: Destructure):
        """Check destructuring statement. Infer init type and register pattern vars."""
        init_ty = self.infer_expr(stmt.init)
        if init_ty is None:
            return
        from omnilang.ast_nodes import VarPattern
        if isinstance(stmt.pattern, VarPattern):
            self.type_map[id(stmt)] = init_ty
            return

    def _narrow_type(self, expr: Expr, condition: Expr, type_map: dict, negated: bool = False):
        """Basic type narrowing for 'is' checks in if conditions.

        Handles: `if x is Type`, `if x is null` → narrows x's type.
        """
        if isinstance(condition, BinaryOp) and condition.op == "==":
            # Check for `x == null` or `null == x`
            left = condition.left
            right = condition.right
            if isinstance(left, VarRef) and isinstance(right, Literal) and right.kind == "String" and right.value == "null":
                return
        # v0.2 basic: skip full narrowing (comes in Phase 3 full impl)
        pass

    def _check_var_decl(self, stmt: VarDecl):
        init_ty = self.infer_expr(stmt.init)
        declared_ty = stmt.type_ann

        # Type inference: if no type annotation, infer from initializer
        if declared_ty is None:
            self.type_map[id(stmt)] = init_ty
            return

        if not self._types_match(init_ty, declared_ty):
            self._error(f"Type mismatch: cannot assign {init_ty} to variable of type {declared_ty}",
                       stmt.span)

        # Check void not used for variables
        if isinstance(declared_ty, TypeAnnotation) and declared_ty.name == "void":
            self._error("Cannot declare variable of type 'void'", stmt.span)

        # Check container element types
        if declared_ty.name == "Array" and declared_ty.type_args:
            elem_ty = declared_ty.type_args[0]
            if elem_ty.name not in ALLOWED_ELEMENT_TYPES:
                self._error(f"Array element type '{elem_ty}' is not allowed (only i32, f64, bool, String)",
                           stmt.span)
        if declared_ty.name == "Map" and len(declared_ty.type_args) >= 2:
            key_ty, val_ty = declared_ty.type_args[0], declared_ty.type_args[1]
            if key_ty.name not in ALLOWED_KEY_TYPES:
                self._error(f"Map key type '{key_ty}' is not allowed (only i32, bool, String); f64 forbidden",
                           stmt.span)
            if key_ty.name == "f64":
                self._error("f64 is forbidden as Map key type", stmt.span)
            if val_ty.name not in ALLOWED_ELEMENT_TYPES:
                self._error(f"Map value type '{val_ty}' is not allowed (only i32, f64, bool, String)",
                           stmt.span)

    def _check_assign(self, stmt: Assign):
        target_decl = lookup_var(stmt.target, self.current_scope)
        if target_decl is None:
            return  # error already reported by resolver

        if not target_decl.mutable:
            # Non-mut variables can only be assigned via index (container element)
            if stmt.index is None:
                self._error(f"Cannot reassign immutable variable '{stmt.target}'", stmt.span)
                return

        value_ty = self.infer_expr(stmt.value)
        target_ty = target_decl.type_ann

        if stmt.op == "=":
            if stmt.index:
                # Index assignment: target_ty is Array<T> or Map<K,V>, check element type
                if target_ty.name == "Array" and target_ty.type_args:
                    elem_ty = target_ty.type_args[0]
                    if not self._types_match(value_ty, elem_ty):
                        self._error(f"Type mismatch: cannot assign {value_ty} to {elem_ty} array element",
                                   stmt.span)
                elif target_ty.name == "Map" and len(target_ty.type_args) >= 2:
                    val_ty = target_ty.type_args[1]
                    if not self._types_match(value_ty, val_ty):
                        self._error(f"Type mismatch: cannot assign {value_ty} to {val_ty} map value",
                                   stmt.span)
            else:
                if not self._types_match(value_ty, target_ty):
                    self._error(f"Type mismatch: cannot assign {value_ty} to variable of type {target_ty}",
                               stmt.span)
        else:
            # Compound assignment: += -= *= /= %=
            if stmt.op in ("/=", "%="):
                # Check for float division
                if target_ty and target_ty.name == "f64":
                    self._error("Float division/modulo is forbidden", stmt.span)
                    return
            if not self._types_match(value_ty, target_ty):
                self._error(f"Type mismatch in compound assignment: {value_ty} vs {target_ty}",
                           stmt.span)

    def _check_if_stmt(self, stmt: IfStmt):
        cond_ty = self.infer_expr(stmt.condition)
        if cond_ty and cond_ty.name != "bool":
            self._error(f"If condition must be bool, got {cond_ty}", stmt.condition.span)

        self.check_block(stmt.then_body)
        for cond, blk in stmt.else_ifs:
            c_ty = self.infer_expr(cond)
            if c_ty and c_ty.name != "bool":
                self._error(f"If condition must be bool, got {c_ty}", cond.span)
            self.check_block(blk)
        if stmt.else_body:
            self.check_block(stmt.else_body)

    def _check_while_stmt(self, stmt: WhileStmt):
        cond_ty = self.infer_expr(stmt.condition)
        if cond_ty and cond_ty.name != "bool":
            self._error(f"While condition must be bool, got {cond_ty}", stmt.condition.span)
        self.check_block(stmt.body)

    def _check_for_stmt(self, stmt: ForInStmt):
        iter_ty = self.infer_expr(stmt.iterable)
        if iter_ty is None:
            return
        if iter_ty.name != "Array":
            self._error(f"For-in requires Array, got {iter_ty}", stmt.span)
            return
        if not iter_ty.type_args:
            self._error("For-in requires a typed array", stmt.span)
            return

        elem_ty = iter_ty.type_args[0]
        # Infer the loop variable type and register it
        self._add_type(stmt, elem_ty)

        # Update the placeholder declaration with the real type
        for_scope = None
        for child in self.current_scope.children:
            if child.name == "for":
                for_scope = child
                break
        if for_scope and stmt.var_name in for_scope.symbols:
            for_scope.symbols[stmt.var_name] = VarDecl(
                mutable=False, name=stmt.var_name, type_ann=elem_ty,
                init=None, span=stmt.span
            )

        self.check_block(stmt.body)

    def _check_return_stmt(self, stmt: ReturnStmt):
        if self.current_func is None:
            return
        func_return = self.current_func.return_type
        if func_return is None:
            func_return = TypeAnnotation(name="void")

        if stmt.value:
            if func_return.name == "void":
                self._error("Cannot return a value from a void function", stmt.span)
                return
            val_ty = self.infer_expr(stmt.value)
            if not self._types_match(val_ty, func_return):
                self._error(f"Return type mismatch: expected {func_return}, got {val_ty}", stmt.span)
        else:
            if func_return.name != "void":
                self._error(f"Expected return value of type {func_return}", stmt.span)

    def _check_print_stmt(self, stmt: PrintStmt):
        arg_ty = self.infer_expr(stmt.arg)
        if arg_ty and arg_ty.name != "String":
            self._error(f"print/println argument must be String, got {arg_ty}", stmt.arg.span)

    # ── Expression type inference ─────────────────

    def infer_expr(self, expr) -> TypeAnnotation | None:
        """Infer the type of an expression. Returns TypeAnnotation or None on error."""
        if expr is None:
            return None

        if isinstance(expr, Literal):
            return self._infer_literal(expr)
        elif isinstance(expr, VarRef):
            return self._infer_var_ref(expr)
        elif isinstance(expr, BinaryOp):
            return self._infer_binary_op(expr)
        elif isinstance(expr, UnaryOp):
            return self._infer_unary_op(expr)
        elif isinstance(expr, FuncCall):
            return self._infer_func_call(expr)
        elif isinstance(expr, MethodCall):
            return self._infer_method_call(expr)
        elif isinstance(expr, IndexGet):
            return self._infer_index_get(expr)
        elif isinstance(expr, ArrayLiteral):
            return self._infer_array_literal(expr)
        elif isinstance(expr, MapLiteral):
            return self._infer_map_literal(expr)
        elif isinstance(expr, IfExpr):
            return self._infer_if_expr(expr)
        elif isinstance(expr, ClosureExpr):
            return self._infer_closure(expr)
        elif isinstance(expr, MatchExpr):
            return self._infer_match_expr(expr)

        return None

    def _infer_literal(self, expr):
        return self._add_type(expr, TypeAnnotation(name=expr.kind, span=expr.span))

    def _infer_var_ref(self, expr):
        decl = lookup_var(expr.name, self.current_scope)
        if decl:
            ty = decl.type_ann
            self._add_type(expr, ty)
            return ty
        return None

    def _infer_binary_op(self, expr):
        left_ty = self.infer_expr(expr.left)
        right_ty = self.infer_expr(expr.right)
        if left_ty is None or right_ty is None:
            return None

        if expr.op in ("+", "-", "*", "/", "%"):
            # Arithmetic: both operands must be same numeric type
            if left_ty.name != right_ty.name:
                self._error(f"Type mismatch: {left_ty} {expr.op} {right_ty}", expr.span)
                return None
            if expr.op in ("/", "%"):
                if left_ty.name == "f64":
                    self._error("Float division is forbidden", expr.span)
                    return None
                if left_ty.name == "String":
                    self._error("Division not allowed on String", expr.span)
                    return None
                # Integer division/mod returns i32
                return self._add_type(expr, TypeAnnotation(name="i32", span=expr.span))
            if expr.op == "+" and left_ty.name == "String":
                return self._add_type(expr, TypeAnnotation(name="String", span=expr.span))
            # Numeric operations return their operand type
            return self._add_type(expr, left_ty)

        elif expr.op in ("==", "!=", "<", ">", "<=", ">="):
            if left_ty.name != right_ty.name:
                self._error(f"Cannot compare {left_ty} with {right_ty}", expr.span)
                return None
            return self._add_type(expr, TypeAnnotation(name="bool", span=expr.span))

        elif expr.op in ("&&", "||"):
            if left_ty.name != "bool" or right_ty.name != "bool":
                self._error(f"Logical operators require bool operands, got {left_ty} and {right_ty}",
                           expr.span)
                return None
            return self._add_type(expr, TypeAnnotation(name="bool", span=expr.span))

        return None

    def _infer_unary_op(self, expr):
        operand_ty = self.infer_expr(expr.operand)
        if operand_ty is None:
            return None
        if expr.op == "!":
            if operand_ty.name != "bool":
                self._error(f"! requires bool operand, got {operand_ty}", expr.span)
                return None
            return self._add_type(expr, TypeAnnotation(name="bool", span=expr.span))
        elif expr.op == "-":
            if operand_ty.name not in ("i32", "f64"):
                self._error(f"Unary - requires numeric operand, got {operand_ty}", expr.span)
                return None
            return self._add_type(expr, operand_ty)
        return None

    def _infer_closure(self, expr):
        from omnilang.ast_nodes import Block
        if isinstance(expr.body, Block) and expr.body.trailing_expr:
            ret_ty = self.infer_expr(expr.body.trailing_expr)
        elif not isinstance(expr.body, Block):
            ret_ty = self.infer_expr(expr.body)
        else:
            ret_ty = None
        if ret_ty is None:
            ret_ty = TypeAnnotation(name="void", span=expr.span)
        return self._add_type(expr, ret_ty)

    def _infer_match_expr(self, expr):
        if expr.arms:
            first = expr.arms[0]
            if isinstance(first.body, BlockNode) and first.body.trailing_expr:
                ret_ty = self.infer_expr(first.body.trailing_expr)
            elif not isinstance(first.body, BlockNode):
                ret_ty = self.infer_expr(first.body)
            else:
                ret_ty = None
            return self._add_type(expr, ret_ty or TypeAnnotation(name="void", span=expr.span))
        return self._add_type(expr, TypeAnnotation(name="void", span=expr.span))

    def _infer_func_call(self, expr):
        func_name = None
        if isinstance(expr.func, VarRef):
            func_name = expr.func.name
        if func_name is None:
            self._error("Cannot call non-function expression", expr.span)
            return None

        target_func = self.func_map.get(func_name)
        if target_func is None:
            # Check if it's a variable holding a closure/function
            decl = lookup_var(func_name, self.current_scope)
            if decl:
                # Variable exists — skip type checking for now (Phase 3)
                return self._add_type(expr, TypeAnnotation(name="i32", span=expr.span))
            self._error(f"Undefined function '{func_name}'", expr.span)
            return None

        # Check argument count
        if len(expr.args) != len(target_func.params):
            self._error(
                f"Function '{func_name}' expects {len(target_func.params)} args, got {len(expr.args)}",
                expr.span
            )
            return None

        # Check argument types
        for i, (arg, param) in enumerate(zip(expr.args, target_func.params)):
            arg_ty = self.infer_expr(arg)
            if arg_ty and not self._types_match(arg_ty, param.type_ann):
                self._error(
                    f"Argument {i} of '{func_name}': expected {param.type_ann}, got {arg_ty}",
                    expr.span
                )

        ret_ty = target_func.return_type
        if ret_ty is None:
            ret_ty = TypeAnnotation(name="void", span=expr.span)
        return self._add_type(expr, ret_ty)

    def _infer_method_call(self, expr):
        obj_ty = self.infer_expr(expr.object)
        if obj_ty is None:
            return None

        base_name = obj_ty.name
        if base_name not in BUILTIN_METHODS:
            self._error(f"No methods available for type {obj_ty}", expr.span)
            return None

        methods = BUILTIN_METHODS[base_name]
        if expr.method not in methods:
            self._error(f"Type {obj_ty} has no method '{expr.method}'", expr.span)
            return None

        sig_fn = methods[expr.method]
        try:
            if base_name == "Array" and obj_ty.type_args:
                param_types, ret_name, is_mut = sig_fn(obj_ty.type_args[0].name)
            elif base_name == "Map" and len(obj_ty.type_args) >= 2:
                param_types, ret_name, is_mut = sig_fn(
                    obj_ty.type_args[0].name, obj_ty.type_args[1].name
                )
            else:
                param_types, ret_name, is_mut = sig_fn()
        except TypeError:
            param_types, ret_name, is_mut = sig_fn()

        # Check argument count
        if len(expr.args) != len(param_types):
            self._error(f"Method '{expr.method}' expects {len(param_types)} args, got {len(expr.args)}",
                       expr.span)
            return None

        # Check argument types
        for i, (arg, expected) in enumerate(zip(expr.args, param_types)):
            arg_ty = self.infer_expr(arg)
            if arg_ty and arg_ty.name != expected:
                self._error(f"Method '{expr.method}' arg {i}: expected {expected}, got {arg_ty}",
                           expr.span)

        ret_ty = TypeAnnotation(name=ret_name, span=expr.span)
        return self._add_type(expr, ret_ty)

    def _infer_index_get(self, expr):
        obj_ty = self.infer_expr(expr.object)
        index_ty = self.infer_expr(expr.index)
        if obj_ty is None:
            return None

        if obj_ty.name == "Array":
            if index_ty and index_ty.name != "i32":
                self._error(f"Array index must be i32, got {index_ty}", expr.span)
            if obj_ty.type_args:
                return self._add_type(expr, obj_ty.type_args[0])
            return None

        elif obj_ty.name == "Map":
            if obj_ty.type_args and index_ty:
                key_ty = obj_ty.type_args[0]
                if index_ty.name != key_ty.name:
                    self._error(f"Map key type mismatch: expected {key_ty}, got {index_ty}", expr.span)
                if len(obj_ty.type_args) >= 2:
                    return self._add_type(expr, obj_ty.type_args[1])
            return None

        else:
            self._error(f"Cannot index into type {obj_ty}", expr.span)
            return None

    def _infer_array_literal(self, expr):
        # Infer element type from context (caller should pass expected type)
        # For now, infer from the elements themselves
        if expr.elements:
            elem_ty = self.infer_expr(expr.elements[0])
            return self._add_type(expr, TypeAnnotation(
                name="Array", type_args=(elem_ty,) if elem_ty else (),
                span=expr.span
            ))
        return self._add_type(expr, TypeAnnotation(name="Array", type_args=(), span=expr.span))

    def _infer_map_literal(self, expr):
        if expr.entries:
            key_ty = self.infer_expr(expr.entries[0][0])
            val_ty = self.infer_expr(expr.entries[0][1])
            return self._add_type(expr, TypeAnnotation(
                name="Map",
                type_args=(key_ty, val_ty) if key_ty and val_ty else (),
                span=expr.span
            ))
        return self._add_type(expr, TypeAnnotation(name="Map", type_args=(), span=expr.span))

    def _infer_if_expr(self, expr):
        cond_ty = self.infer_expr(expr.condition)
        then_ty = self.infer_expr(expr.then_expr)
        else_ty = self.infer_expr(expr.else_expr)
        if cond_ty and cond_ty.name != "bool":
            self._error("If expression condition must be bool", expr.condition.span)
        if then_ty and else_ty and then_ty.name != else_ty.name:
            self._error(f"If expression branches must have same type: {then_ty} vs {else_ty}", expr.span)
        return self._add_type(expr, then_ty) if then_ty else None

    def _types_match(self, a: TypeAnnotation | None, b: TypeAnnotation | None) -> bool:
        if a is None or b is None:
            return True  # allow unknown types
        if a.name != b.name:
            return False
        # If either side has empty type_args, allow it (empty container literal)
        if not a.type_args or not b.type_args:
            return True
        if len(a.type_args) != len(b.type_args):
            return False
        for ta, tb in zip(a.type_args, b.type_args):
            if not self._types_match(ta, tb):
                return False
        return True
