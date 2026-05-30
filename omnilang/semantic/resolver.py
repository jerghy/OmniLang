"""Symbol resolution pass — builds scope tree and links variable references to declarations."""

from dataclasses import dataclass, field
from omnilang.ast_nodes import (
    Program, FunctionDef, Block, VarDecl, VarRef, ForInStmt,
    Assign, IfStmt, WhileStmt, Stmt
)
from omnilang.utils import ErrorCollector


@dataclass
class Scope:
    name: str
    parent: 'Scope | None' = None
    symbols: dict[str, VarDecl] = field(default_factory=dict)
    children: list['Scope'] = field(default_factory=list)
    is_loop: bool = False
    is_function: bool = False


@dataclass
class ScopeInfo:
    """Result of symbol resolution — passed to subsequent passes."""
    global_scope: Scope
    func_scopes: dict[str, Scope]  # function name → scope
    var_decl_to_scope: dict[int, Scope]  # id(VarDecl) → declaring scope


def _new_child(parent: Scope, name: str, **kwargs) -> Scope:
    child = Scope(name=name, parent=parent, **kwargs)
    parent.children.append(child)
    return child


def resolve(program: Program, errors: ErrorCollector) -> ScopeInfo:
    global_scope = Scope(name="global")
    func_scopes: dict[str, Scope] = {}
    var_decl_to_scope: dict[int, Scope] = {}
    function_names: set[str] = {f.name for f in program.functions}

    for func in program.functions:
        func_scope = _new_child(global_scope, func.name, is_function=True)
        func_scopes[func.name] = func_scope

        # Register parameters
        for param in func.params:
            if param.name.startswith("_"):
                errors.add(f"Identifier '{param.name}' starts with '_' (reserved)", param.span)
            if param.name in func_scope.symbols:
                errors.add(f"Duplicate parameter '{param.name}'", param.span)
            func_scope.symbols[param.name] = VarDecl(
                mutable=False, name=param.name, type_ann=param.type_ann,
                init=None, span=param.span
            )
            var_decl_to_scope[id(func_scope.symbols[param.name])] = func_scope

        # Process function body
        _resolve_block(func.body, func_scope, var_decl_to_scope, function_names, errors)

    return ScopeInfo(
        global_scope=global_scope,
        func_scopes=func_scopes,
        var_decl_to_scope=var_decl_to_scope,
    )


def _resolve_block(block: Block, scope: Scope, decl_map: dict, func_names: set, errors: ErrorCollector):
    """Walk statements in a block, registering declarations in the given scope."""
    if not block:
        return

    for stmt in block.stmts:
        _resolve_stmt(stmt, scope, decl_map, func_names, errors)

    if block.trailing_expr:
        _resolve_expr(block.trailing_expr, scope, func_names, errors)


def _resolve_stmt(stmt: Stmt, scope: Scope, decl_map: dict, func_names: set, errors: ErrorCollector):
    if isinstance(stmt, VarDecl):
        if stmt.name.startswith("_"):
            errors.add(f"Identifier '{stmt.name}' starts with '_' (reserved)", stmt.span)
        if stmt.name in scope.symbols:
            errors.add(f"Variable '{stmt.name}' already declared in this scope", stmt.span)
        scope.symbols[stmt.name] = stmt
        decl_map[id(stmt)] = scope
        _resolve_expr(stmt.init, scope, func_names, errors)

    elif isinstance(stmt, Assign):
        if stmt.index:
            _resolve_expr(stmt.index, scope, func_names, errors)
        _resolve_expr(stmt.value, scope, func_names, errors)

    elif isinstance(stmt, IfStmt):
        _resolve_expr(stmt.condition, scope, func_names, errors)
        child_scope = _new_child(scope, "if")
        _resolve_block(stmt.then_body, child_scope, decl_map, func_names, errors)
        for cond, blk in stmt.else_ifs:
            _resolve_expr(cond, scope, func_names, errors)
            child_scope2 = _new_child(scope, "else_if")
            _resolve_block(blk, child_scope2, decl_map, func_names, errors)
        if stmt.else_body:
            child_scope3 = _new_child(scope, "else")
            _resolve_block(stmt.else_body, child_scope3, decl_map, func_names, errors)

    elif isinstance(stmt, WhileStmt):
        _resolve_expr(stmt.condition, scope, func_names, errors)
        child_scope = _new_child(scope, "while", is_loop=True)
        _resolve_block(stmt.body, child_scope, decl_map, func_names, errors)

    elif isinstance(stmt, ForInStmt):
        _resolve_expr(stmt.iterable, scope, func_names, errors)
        child_scope = _new_child(scope, "for", is_loop=True)
        if stmt.var_name in child_scope.symbols:
            errors.add(f"Loop variable '{stmt.var_name}' shadows outer variable", stmt.span)
        else:
            from omnilang.ast_nodes import TypeAnnotation
            placeholder_ty = TypeAnnotation(name="<inferred>", span=stmt.span)
            placeholder_decl = VarDecl(mutable=False, name=stmt.var_name,
                                       type_ann=placeholder_ty, init=None, span=stmt.span)
            child_scope.symbols[stmt.var_name] = placeholder_decl
            decl_map[id(placeholder_decl)] = child_scope
        _resolve_block(stmt.body, child_scope, decl_map, func_names, errors)

    elif hasattr(stmt, 'expr'):
        if hasattr(stmt, 'expr') and stmt.expr:
            _resolve_expr(stmt.expr, scope, func_names, errors)
        if hasattr(stmt, 'arg') and stmt.arg:
            _resolve_expr(stmt.arg, scope, func_names, errors)
        if hasattr(stmt, 'value') and stmt.value:
            _resolve_expr(stmt.value, scope, func_names, errors)


def _resolve_expr(expr, scope: Scope, func_names: set, errors: ErrorCollector):
    """Walk an expression tree and validate variable references."""
    if expr is None:
        return

    from omnilang.ast_nodes import (
        VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
        ArrayLiteral, MapLiteral, IfExpr,
    )

    if isinstance(expr, VarRef):
        # Skip name resolution for function names in func_call context
        if expr.name in func_names:
            return
        s = scope
        found = False
        while s:
            if expr.name in s.symbols:
                found = True
                break
            s = s.parent
        if not found:
            errors.add(f"Undefined variable '{expr.name}'", expr.span)

    elif isinstance(expr, BinaryOp):
        _resolve_expr(expr.left, scope, func_names, errors)
        _resolve_expr(expr.right, scope, func_names, errors)

    elif isinstance(expr, UnaryOp):
        _resolve_expr(expr.operand, scope, func_names, errors)

    elif isinstance(expr, MethodCall):
        _resolve_expr(expr.object, scope, func_names, errors)
        for arg in expr.args:
            _resolve_expr(arg, scope, func_names, errors)

    elif isinstance(expr, FuncCall):
        if not isinstance(expr.func, VarRef) or expr.func.name not in func_names:
            _resolve_expr(expr.func, scope, func_names, errors)
        for arg in expr.args:
            _resolve_expr(arg, scope, func_names, errors)

    elif isinstance(expr, IndexGet):
        _resolve_expr(expr.object, scope, func_names, errors)
        _resolve_expr(expr.index, scope, func_names, errors)

    elif isinstance(expr, ArrayLiteral):
        for elem in expr.elements:
            _resolve_expr(elem, scope, func_names, errors)

    elif isinstance(expr, MapLiteral):
        for key, val in expr.entries:
            _resolve_expr(key, scope, func_names, errors)
            _resolve_expr(val, scope, func_names, errors)

    elif isinstance(expr, IfExpr):
        _resolve_expr(expr.condition, scope, func_names, errors)
        _resolve_expr(expr.then_expr, scope, func_names, errors)
        _resolve_expr(expr.else_expr, scope, func_names, errors)


def lookup_var(name: str, scope: Scope) -> VarDecl | None:
    """Look up a variable declaration by name, walking up scope chain."""
    s = scope
    while s:
        if name in s.symbols:
            return s.symbols[name]
        s = s.parent
    return None
