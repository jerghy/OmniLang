"""Closure capture analysis for OmniLang.

Identifies free variables in closures, determines capture kind
(reference vs mutable vs move), and performs escape analysis.

This information is used by:
- Rust codegen: Cell/RefCell wrapping for mutable captures
- Python codegen: nonlocal declarations
- Analyze rules: P0106 (capture semantics), E0306 (escape warnings)
"""

from dataclasses import dataclass
from omnilang.ast_nodes import (
    Program, FunctionDef, Block, VarDecl, Assign,
    ClosureExpr, VarRef, Expr,
)


class CaptureKind:
    """Capture kind classification."""
    REFERENCE = "reference"   # Immutable borrow (read-only)
    MUTABLE = "mutable"       # Mutable borrow (read-write)
    MOVE = "move"             # Ownership transfer (for escaping closures)


@dataclass
class CaptureInfo:
    """Capture information for one variable."""
    name: str
    kind: str  # CaptureKind value
    var_type: object | None = None  # TypeAnnotation or None
    escaped: bool = False  # True if closure escapes its defining scope


def analyze_captures(program: Program) -> dict[int, list[CaptureInfo]]:
    """Analyze closure captures throughout the program.

    Returns: {id(ClosureExpr) → [CaptureInfo, ...]}
    """
    analyzer = CaptureAnalyzer()
    analyzer.analyze_program(program)
    return analyzer.capture_map


class CaptureAnalyzer:
    """Walks AST to find closures and identify their captured variables."""

    def __init__(self):
        self.capture_map: dict[int, list[CaptureInfo]] = {}
        self._scope_stack: list[set[str]] = []  # Variables in each scope
        self._assignments: set[str] = set()  # Variables assigned in current scope
        self._current_closure: ClosureExpr | None = None
        self._closure_free_vars: dict[str, CaptureInfo] = {}
        self._in_closure: int = 0  # Nesting depth

    def analyze_program(self, program: Program):
        """Analyze all closures in a program."""
        for func in program.functions:
            self._analyze_function(func)

    def _analyze_function(self, func: FunctionDef):
        """Analyze closures within a function."""
        # Setup scope
        scope = set()
        for p in func.params:
            scope.add(p.name)
        self._scope_stack.append(scope)
        self._assignments = set()

        if func.body:
            self._analyze_block(func.body)

        self._scope_stack.pop()

    def _analyze_block(self, block: Block):
        """Walk block statements to find closures and track variables."""
        for stmt in block.stmts:
            self._analyze_stmt(stmt)
        if block.trailing_expr:
            self._analyze_expr(block.trailing_expr)

    def _analyze_stmt(self, stmt):
        """Track variable declarations and find closures in statements."""
        if isinstance(stmt, VarDecl):
            if self._scope_stack:
                self._scope_stack[-1].add(stmt.name)
            if stmt.init:
                self._analyze_expr(stmt.init)

        elif isinstance(stmt, Assign):
            if self._in_closure > 0 and self._current_closure:
                # Check if target is captured from enclosing scope
                for i in range(len(self._scope_stack) - 1, -1, -1):
                    if stmt.target in self._scope_stack[i]:
                        if i < len(self._scope_stack) - 1:
                            self._closure_free_vars[stmt.target] = CaptureInfo(
                                name=stmt.target, kind=CaptureKind.MUTABLE
                            )
                        break
            self._assignments.add(stmt.target)
            if stmt.value:
                self._analyze_expr(stmt.value)

        # Walk all expression-valued children generically
        for attr in ['value', 'condition', 'iterable', 'arg', 'expr', 'init']:
            if hasattr(stmt, attr):
                val = getattr(stmt, attr)
                if isinstance(val, Expr):
                    self._analyze_expr(val)

        # Recurse into block-like statements
        for child_attr in ['body', 'then_body', 'else_body']:
            if hasattr(stmt, child_attr):
                child = getattr(stmt, child_attr)
                if isinstance(child, Block):
                    self._analyze_block(child)

        # Recurse into else_ifs
        if hasattr(stmt, 'else_ifs') and stmt.else_ifs:
            for _, elif_body in stmt.else_ifs:
                if isinstance(elif_body, Block):
                    self._analyze_block(elif_body)

    def _analyze_expr(self, expr):
        """Walk expression to find closures and free variable references."""
        if isinstance(expr, ClosureExpr):
            self._analyze_closure(expr)

        elif isinstance(expr, VarRef):
            if self._in_closure > 0 and self._current_closure:
                # Check if this references a variable from an outer scope
                name = expr.name
                for i in range(len(self._scope_stack) - 1, -1, -1):
                    if name in self._scope_stack[i]:
                        if i < len(self._scope_stack) - 1:
                            # Found a free variable!
                            if name not in self._closure_free_vars:
                                self._closure_free_vars[name] = CaptureInfo(
                                    name=name, kind=CaptureKind.REFERENCE
                                )
                            if name in self._assignments:
                                self._closure_free_vars[name].kind = CaptureKind.MUTABLE
                        break

        # Recurse into children
        if isinstance(expr, ClosureExpr):
            return  # Already handled in _analyze_closure
        for slot in getattr(expr, '__dataclass_fields__', {}):
            val = getattr(expr, slot)
            if isinstance(val, Expr):
                self._analyze_expr(val)
            elif isinstance(val, list):
                for item in val:
                    if isinstance(item, Expr):
                        self._analyze_expr(item)
                    elif isinstance(item, tuple):
                        for sub in item:
                            if isinstance(sub, Expr):
                                self._analyze_expr(sub)

    def _analyze_closure(self, ce: ClosureExpr):
        """Analyze a closure: identify free variables and determine capture kinds."""
        prev_closure = self._current_closure
        prev_free_vars = self._closure_free_vars
        self._current_closure = ce
        self._closure_free_vars = {}
        self._in_closure += 1

        # Create scope for closure parameters
        scope = set(p.name for p in ce.params)
        self._scope_stack.append(scope)

        # Analyze body
        if isinstance(ce.body, Block):
            self._analyze_block(ce.body)
        else:
            self._analyze_expr(ce.body)

        self._scope_stack.pop()

        # Detect escape: if closure is returned or passed to another function
        # Simple heuristic: closures in return statements escape
        escaped = self._is_escaping_context(ce)

        # Mark captures with escape info
        captures = []
        for info in self._closure_free_vars.values():
            info.escaped = escaped
            if escaped and info.kind == CaptureKind.REFERENCE:
                # For Rust: non-Copy types in escaping closures need move
                info.kind = CaptureKind.MOVE
            captures.append(info)

        self.capture_map[id(ce)] = captures
        self._current_closure = prev_closure
        self._closure_free_vars = prev_free_vars
        self._in_closure -= 1

    def _is_escaping_context(self, ce: ClosureExpr) -> bool:
        """Simple heuristic: is this closure in a context where it escapes?"""
        # For now, assume closures are potentially escaping.
        # Phase 2.5 will add proper escape analysis via data flow.
        return True
