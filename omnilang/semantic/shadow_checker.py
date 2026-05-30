"""Variable shadowing detection and rename planning.

Detects nested-scope variable shadowing and generates rename plans for
Dart and Python backends (which lack block-level scoping).

Rename plan format:
    list of dicts, where list[0] = global scope, list[1] = first nested scope, etc.
    Each dict maps {original_name → new_name} for that scope level.
    The list is consumed by code generators in order as they enter scopes.
"""

from omnilang.ast_nodes import (
    Program, FunctionDef, Block, VarDecl, ForInStmt, Stmt,
    IfStmt, WhileStmt, VarRef, MethodCall, FuncCall, IndexGet,
    BinaryOp, UnaryOp, ArrayLiteral, MapLiteral, IfExpr,
)
from omnilang.utils import ErrorCollector
from omnilang.semantic.resolver import ScopeInfo, Scope


def check(program: Program, scope_info: ScopeInfo, errors: ErrorCollector) -> list:
    """Detect shadowing and generate rename plans for scope levels.

    Returns: list of dicts, consumed in order by code generators.
    """
    scope_renames: list[dict] = [{}]  # level 0 = function scope, no renames
    _counter = [0]

    for func in program.functions:
        func_scope = scope_info.func_scopes.get(func.name)
        if func_scope and func.body:
            _check_block(func.body, func_scope, None, scope_renames, _counter, errors)

    return scope_renames


def _find_child_scope(parent: Scope, name_hint: str) -> Scope | None:
    """Find a child scope by name hint (e.g. 'if', 'while', 'for', 'else', 'else_if')."""
    for child in parent.children:
        if child.name == name_hint:
            return child
    # Fallback: return first child
    if parent.children:
        return parent.children[0]
    return None


def _check_block(block: Block, scope: Scope, parent_scope_idx: int | None,
                 scope_renames: list, counter: list, errors: ErrorCollector):
    for stmt in block.stmts:
        if isinstance(stmt, VarDecl):
            _register_shadow(stmt, scope, parent_scope_idx, scope_renames, counter, errors)

        if isinstance(stmt, IfStmt):
            child_scope = _find_child_scope(scope, "if")
            if child_scope:
                _new_scope_level(scope_renames, scope)
                _check_block(stmt.then_body, child_scope, len(scope_renames) - 1,
                            scope_renames, counter, errors)
            for _, blk in stmt.else_ifs:
                child_scope2 = _find_child_scope(scope, "else_if")
                if child_scope2:
                    _new_scope_level(scope_renames, scope)
                    _check_block(blk, child_scope2, len(scope_renames) - 1,
                                scope_renames, counter, errors)
            if stmt.else_body:
                child_scope3 = _find_child_scope(scope, "else")
                if child_scope3:
                    _new_scope_level(scope_renames, scope)
                    _check_block(stmt.else_body, child_scope3, len(scope_renames) - 1,
                                scope_renames, counter, errors)

        elif isinstance(stmt, WhileStmt):
            child_scope = _find_child_scope(scope, "while")
            if child_scope:
                _new_scope_level(scope_renames, scope)
                _check_block(stmt.body, child_scope, len(scope_renames) - 1,
                            scope_renames, counter, errors)

        elif isinstance(stmt, ForInStmt):
            child_scope = _find_child_scope(scope, "for")
            if child_scope:
                _new_scope_level(scope_renames, scope)
                # Register loop variable shadowing (check against child_scope's parent)
                _register_shadow_for_var(stmt.var_name, stmt.span, child_scope,
                                         scope_renames, counter, errors)
                _check_block(stmt.body, child_scope, len(scope_renames) - 1,
                            scope_renames, counter, errors)


def _new_scope_level(scope_renames: list, scope: Scope) -> dict:
    """Create a new scope level and return it."""
    new_level = {}
    scope_renames.append(new_level)
    return new_level


def _register_shadow(var_decl: VarDecl, scope: Scope, parent_idx: int | None,
                     scope_renames: list, counter: list, errors: ErrorCollector):
    _register_shadow_for_var(var_decl.name, var_decl.span, scope,
                             scope_renames, counter, errors)


def _register_shadow_for_var(name: str, span, scope: Scope,
                             scope_renames: list, counter: list,
                             errors: ErrorCollector):
    """Check if name shadows a variable in ancestor scope."""
    ancestor = scope.parent
    while ancestor:
        if name in ancestor.symbols:
            counter[0] += 1
            new_name = f"{name}_shadow_{counter[0]}"
            # Register in the CURRENT scope level (the one just pushed)
            if len(scope_renames) > 1:
                scope_renames[-1][name] = new_name
            return
        ancestor = ancestor.parent


def lookup_rename(scope_renames: list, scope_level: int, name: str) -> str:
    """Look up a renamed name in the given scope level and all ancestor levels."""
    for level in range(scope_level, -1, -1):
        if level < len(scope_renames):
            if name in scope_renames[level]:
                return scope_renames[level][name]
    return name
