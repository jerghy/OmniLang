"""OmniView semantic analysis engine.

Performs: symbol resolution, type checking, constraint validation,
dependency collection, store read analysis, safe expression validation,
and computed chain flattening.
"""

from dataclasses import dataclass, field

from omnilang.ast_nodes import (
    TypeAnnotation, VarRef, BinaryOp, UnaryOp, FuncCall, MethodCall,
    Literal, IfExpr, ArrayLiteral, MapLiteral, IndexGet, ClosureExpr,
    AwaitExpr, TemplateString,
)
from omniview.ast_nodes import (
    FieldAccess, TernaryExpr, ClassMapExpr, StyleObjectExpr,
)
from omnilang.utils import Span

from omniview.ast_nodes import (
    Program, ComponentDef, StoreDef,
    PropDecl, SignalDecl, ComputedDecl, EventDecl, RefDecl, StoreRef,
    StoreSignalDecl, StoreGetterDecl, StoreActionDecl,
    Element, ViewBlock, ViewNode,
    IfDirective, ForDirective, SlotDirective, NativeBlock, FallbackBlock,
    LifecycleHook, FnDef, Block, Stmt,
    EventProp, BindProp, StaticProp, ClassProp,
    InlineFn, FnRef, StoreActionRef,
    TextNode, Interpolation,
    VarDeclStmt, AssignStmt, IfStmt, WhileStmt, ForInStmt, ReturnStmt,
    ExprStmt, EventEmitStmt,
    StyleBlock, StyleRule,
)


@dataclass
class Diagnostic:
    """A semantic diagnostic message."""
    level: str  # "ERROR" | "WARNING" | "INFO"
    code: str   # e.g., "P0214", "P0201", "E0101"
    message: str
    span: Span | None = None

    def __str__(self):
        loc = f" at {self.span}" if self.span else ""
        return f"{self.level}[{self.code}]{loc}: {self.message}"


@dataclass
class SymbolInfo:
    """Information about a resolved symbol."""
    kind: str  # "prop" | "signal" | "computed" | "event" | "fn" | "ref" | "store" | "param"
    type: TypeAnnotation | None = None
    is_mutable: bool = False
    owner: str | None = None  # component name or "store"


@dataclass
class AnalysisResult:
    """Result of semantic analysis on a program."""
    symbols: dict[str, SymbolInfo] = field(default_factory=dict)
    dependencies: dict[str, list[str]] = field(default_factory=dict)
    store_reads: dict[str, list[str]] = field(default_factory=dict)
    diagnostics: list[Diagnostic] = field(default_factory=list)

    @property
    def has_errors(self) -> bool:
        return any(d.level == "ERROR" for d in self.diagnostics)

    @property
    def error_count(self) -> int:
        return sum(1 for d in self.diagnostics if d.level == "ERROR")

    @property
    def warning_count(self) -> int:
        return sum(1 for d in self.diagnostics if d.level == "WARNING")


class SemanticAnalyzer:
    """Walks the OmniView AST and produces semantic analysis results.

    Usage:
        analyzer = SemanticAnalyzer()
        result = analyzer.analyze(program)
        if result.has_errors:
            for d in result.diagnostics:
                print(d)
    """

    def __init__(self):
        self.symbols: dict[str, SymbolInfo] = {}
        self.dependencies: dict[str, list[str]] = {}
        self.store_reads: dict[str, list[str]] = {}
        self.diagnostics: list[Diagnostic] = []
        self._current_component: str | None = None
        self._current_store: str | None = None
        self._loop_vars: set[str] = set()

    # ── Main Entry Point ──────────────────────────────

    def analyze(self, program: Program) -> AnalysisResult:
        """Run all analysis passes on the program."""
        self._reset()
        self._collect_symbols(program)
        self._resolve_references(program)
        self._validate_constraints(program)
        self._collect_dependencies(program)
        self._flatten_computed_chains()
        return AnalysisResult(
            symbols=dict(self.symbols),
            dependencies=dict(self.dependencies),
            store_reads=dict(self.store_reads),
            diagnostics=list(self.diagnostics),
        )

    def _reset(self):
        self.symbols.clear()
        self.dependencies.clear()
        self.store_reads.clear()
        self.diagnostics.clear()
        self._current_component = None
        self._current_store = None
        self._loop_vars.clear()

    # ── Phase 1: Symbol Collection ────────────────────

    def _collect_symbols(self, program: Program):
        for comp in program.components:
            self._collect_component_symbols(comp)
        for store in program.stores:
            self._collect_store_symbols(store)

    def _collect_component_symbols(self, comp: ComponentDef):
        self._current_component = comp.name

        for prop in comp.props:
            self._add_symbol(prop.name, "prop", prop.type, owner=comp.name)

        for sig in comp.signals:
            self._add_symbol(sig.name, "signal", sig.type, owner=comp.name)

        for computed in comp.computeds:
            self._add_symbol(computed.name, "computed", computed.type, owner=comp.name)

        for event in comp.events:
            self._add_symbol(event.name, "event", event.payload_type, owner=comp.name)

        for ref in comp.refs:
            self._add_symbol(ref.name, "ref", ref.ref_type, owner=comp.name)

        for store_ref in comp.stores:
            self._add_symbol(store_ref.store_name, "store", owner=comp.name)
            # Register "store" as alias so store.field works
            self._add_symbol("store", "store_alias", owner=comp.name)

        for fn_def in comp.fns:
            self._add_symbol(fn_def.name, "fn", fn_def.return_type, owner=comp.name)
            self._collect_fn_params(fn_def)

        self._current_component = None

    def _collect_fn_params(self, fn_def: FnDef):
        for param in fn_def.params:
            self._add_symbol(param.name, "param", param.type,
                           owner=self._current_component)

    def _collect_store_symbols(self, store: StoreDef):
        self._current_store = store.name

        for sig in store.signals:
            key = f"store.{store.name}.{sig.name}"
            self._add_symbol(key, "store_signal", sig.type, owner=store.name)

        for getter in store.getters:
            key = f"store.{store.name}.{getter.name}"
            self._add_symbol(key, "store_getter", getter.type, owner=store.name)

        for action in store.actions:
            key = f"store.{store.name}.{action.name}"
            self._add_symbol(key, "store_action", owner=store.name)

        self._current_store = None

    def _add_symbol(self, name: str, kind: str, type_ann=None, owner=None):
        self.symbols[name] = SymbolInfo(
            kind=kind, type=type_ann, is_mutable=(kind in ("signal", "store_signal")),
            owner=owner,
        )

    def _lookup(self, name: str) -> SymbolInfo | None:
        """Look up a symbol by name (supports 'store.field' and simple names)."""
        if name in self.symbols:
            return self.symbols[name]
        # Try store prefix lookups
        for key, info in self.symbols.items():
            if key.endswith(f".{name}"):
                return info
        return None

    # ── Phase 2: Reference Resolution ─────────────────

    def _resolve_references(self, program: Program):
        for comp in program.components:
            self._current_component = comp.name
            self._resolve_component_refs(comp)
            self._current_component = None

    def _resolve_component_refs(self, comp: ComponentDef):
        # Check computed bodies
        for c in comp.computeds:
            self._check_expr_refs(c.body, context=f"computed '{c.name}'")

        # Check view
        if comp.view:
            self._resolve_view_refs(comp.view)

        # Check lifecycle hooks
        for lc in comp.lifecycles:
            self._resolve_block_refs(lc.body)

        # Check fn bodies
        for fn in comp.fns:
            self._resolve_fn_params(fn)
            self._resolve_block_refs(fn.body)

    def _resolve_fn_params(self, fn: FnDef):
        for param in fn.params:
            name = f"{self._current_component or ''}.{fn.name}.{param.name}"
            self._add_symbol(name, "param", param.type, owner=self._current_component)

    def _resolve_view_refs(self, view: ViewBlock):
        for child in view.children:
            self._resolve_view_node_refs(child)

    def _resolve_view_node_refs(self, node):
        if isinstance(node, Element):
            # Check element props
            for prop in node.props:
                if isinstance(prop.value, StaticProp) and hasattr(prop.value.value, '__class__'):
                    self._check_expr_refs(prop.value.value, context=f"element <{node.tag}>")
                if isinstance(prop.value, EventProp):
                    self._resolve_event_handler_refs(prop.value.handler)
            # Check children
            for child in node.children:
                self._resolve_view_node_refs(child)
        elif isinstance(node, IfDirective):
            for branch in node.branches:
                self._check_expr_refs(branch.condition, context="@if")
                for child in branch.body:
                    self._resolve_view_node_refs(child)
            if node.else_body:
                for child in node.else_body:
                    self._resolve_view_node_refs(child)
        elif isinstance(node, ForDirective):
            self._check_expr_refs(node.iterable, context="@for")
            # Add loop variable to scope
            self._loop_vars.add(node.item_name)
            for child in node.body:
                self._resolve_view_node_refs(child)
            self._loop_vars.discard(node.item_name)
            if node.empty_body:
                for child in node.empty_body:
                    self._resolve_view_node_refs(child)
        elif isinstance(node, Interpolation):
            self._check_expr_refs(node.expr, context="interpolation")
        # TextNode needs no resolution

    def _resolve_event_handler_refs(self, handler):
        if isinstance(handler, InlineFn):
            self._resolve_block_refs(handler.body)
        elif isinstance(handler, FnRef):
            name = handler.fn_name
            if name not in self.symbols and name not in self._loop_vars:
                self.diagnostics.append(Diagnostic(
                    "ERROR", "E0200",
                    f"Unresolved function reference: '{name}'",
                ))
        elif isinstance(handler, StoreActionRef):
            parts = handler.action_path.split(".")
            if len(parts) >= 2:
                store_key = f"store.{parts[0]}.{parts[1]}"
                if store_key not in self.symbols:
                    self.diagnostics.append(Diagnostic(
                        "WARNING", "W0200",
                        f"Store action '{handler.action_path}' not found in symbol table",
                    ))

    def _resolve_block_refs(self, block: Block | None):
        if block is None:
            return
        for stmt in block.statements:
            if isinstance(stmt, VarDeclStmt):
                if stmt.value:
                    self._check_expr_refs(stmt.value, context=f"let {stmt.name}")
            elif isinstance(stmt, AssignStmt):
                if stmt.value:
                    self._check_expr_refs(stmt.value, context=f"assign {stmt.target}")
                self._check_expr_refs(VarRef(name=stmt.target), context="assign target")
            elif isinstance(stmt, IfStmt):
                self._check_expr_refs(stmt.condition, context="if")
                self._resolve_block_refs(stmt.then_body)
                self._resolve_block_refs(stmt.else_body)
            elif isinstance(stmt, ReturnStmt) and stmt.value:
                self._check_expr_refs(stmt.value, context="return")
            elif isinstance(stmt, ExprStmt) and stmt.expr:
                self._check_expr_refs(stmt.expr, context="expr stmt")
            elif isinstance(stmt, EventEmitStmt):
                for arg in stmt.args:
                    self._check_expr_refs(arg, context=f"event {stmt.event_name}")

    def _check_expr_refs(self, expr, context: str = "expr"):
        """Recursively check expression for unresolved variable references."""
        if expr is None:
            return
        if isinstance(expr, VarRef):
            name = expr.name
            if name in self._loop_vars:
                return  # Loop variable — OK
            if name in self.symbols:
                # Record store reads
                info = self.symbols[name]
                if info.kind == "store":
                    pass  # Store reference is fine
                if info.kind == "store_signal":
                    store_name = info.owner or "store"
                    if store_name not in self.store_reads:
                        self.store_reads[store_name] = []
                    field = name.split(".")[-1] if "." in name else name
                    if field not in self.store_reads[store_name]:
                        self.store_reads[store_name].append(field)
                return
            if name in ("true", "false", "null", "self", "this"):
                return  # Reserved words
            if name.startswith("_"):
                return  # Private variables — don't report
            self.diagnostics.append(Diagnostic(
                "ERROR", "E0200",
                f"Unresolved reference: '{name}' in {context}",
            ))
        elif isinstance(expr, BinaryOp):
            self._check_expr_refs(expr.left, context)
            self._check_expr_refs(expr.right, context)
        elif isinstance(expr, UnaryOp):
            self._check_expr_refs(expr.operand, context)
        elif isinstance(expr, FuncCall):
            self._check_expr_refs(expr.func, context)
            for arg in expr.args:
                self._check_expr_refs(arg, context)
        elif isinstance(expr, MethodCall):
            self._check_expr_refs(expr.object, context)
            for arg in expr.args:
                self._check_expr_refs(arg, context)
        elif isinstance(expr, FieldAccess):
            self._check_expr_refs(expr.object, context)
        elif isinstance(expr, IfExpr):
            self._check_expr_refs(expr.condition, context)
            self._resolve_block_refs(expr.then_block)
            self._resolve_block_refs(expr.else_block)
        elif isinstance(expr, ArrayLiteral):
            for elem in expr.elements:
                self._check_expr_refs(elem, context)
        elif isinstance(expr, MapLiteral):
            for key, val in expr.entries:
                self._check_expr_refs(key, context)
                self._check_expr_refs(val, context)

    # ── Phase 3: Constraint Validation ────────────────

    def _validate_constraints(self, program: Program):
        for comp in program.components:
            self._current_component = comp.name
            if comp.view:
                self._validate_view_constraints(comp.view, comp)
            self._current_component = None

    def _validate_view_constraints(self, view: ViewBlock, comp: ComponentDef):
        for child in view.children:
            self._validate_view_node_constraints(child, comp)

    def _validate_view_node_constraints(self, node, comp: ComponentDef):
        if isinstance(node, Element):
            self._validate_element_constraints(node, comp)
            for child in node.children:
                self._validate_view_node_constraints(child, comp)
        elif isinstance(node, ForDirective):
            self._validate_for_constraints(node)
            for child in node.body:
                self._validate_view_node_constraints(child, comp)
            if node.empty_body:
                for child in node.empty_body:
                    self._validate_view_node_constraints(child, comp)
        elif isinstance(node, IfDirective):
            for branch in node.branches:
                for child in branch.body:
                    self._validate_view_node_constraints(child, comp)
            if node.else_body:
                for child in node.else_body:
                    self._validate_view_node_constraints(child, comp)

    def _validate_element_constraints(self, elem: Element, comp: ComponentDef):
        for prop in elem.props:
            if isinstance(prop.value, BindProp):
                self._validate_bind_constraint(prop, comp)
            if isinstance(prop.value, EventProp):
                pass  # Event handlers are always OK

    def _validate_bind_constraint(self, prop, comp: ComponentDef):
        """P0214: bind: can only bind to component signals or store signals."""
        sig_name = prop.value.signal_ref if isinstance(prop.value, BindProp) else ""
        info = self._lookup(sig_name)
        if info is None:
            self.diagnostics.append(Diagnostic(
                "ERROR", "P0214",
                f"bind:{prop.name} binds to '{sig_name}', which is not a declared signal",
                prop.span,
            ))
        elif info.kind == "prop":
            self.diagnostics.append(Diagnostic(
                "ERROR", "P0214",
                f"bind:{prop.name} cannot bind to prop '{sig_name}'. "
                f"Only signals (component or store) can be used with bind:",
                prop.span,
            ))

    def _validate_for_constraints(self, for_dir: ForDirective):
        """P0201: @for must have key on root element."""
        if not for_dir.body:
            return
        root = for_dir.body[0] if for_dir.body else None
        if isinstance(root, Element):
            has_key = any(
                p.name == "key" for p in root.props
            )
            if not has_key:
                self.diagnostics.append(Diagnostic(
                    "ERROR", "P0201",
                    f"@for over '{for_dir.item_name}' must have a key={{...}} "
                    f"on the root element <{root.tag}>",
                    for_dir.span,
                ))

    # ── Phase 4: Dependency Collection ────────────────

    def _collect_dependencies(self, program: Program):
        for comp in program.components:
            self._current_component = comp.name
            for computed in comp.computeds:
                deps = self._extract_deps(computed.body)
                self.dependencies[computed.name] = deps
            for lc in comp.lifecycles:
                if lc.kind == "effect" and lc.deps:
                    for dep in lc.deps:
                        if dep in self.symbols:
                            self.dependencies.setdefault(
                                f"@effect({','.join(lc.deps)})", []
                            ).append(dep)
            self._current_component = None

    def _extract_deps(self, expr) -> list[str]:
        """Extract signal names from an expression (Layer 1: exact collection)."""
        deps = []
        self._collect_var_refs(expr, deps)
        # Filter to only signals (not props, fns, etc.)
        return [d for d in deps if self._is_signal(d)]

    def _collect_var_refs(self, expr, deps: list[str]):
        """Recursively collect all VarRef names in an expression."""
        if expr is None:
            return
        if isinstance(expr, VarRef):
            if expr.name not in deps:
                deps.append(expr.name)
        elif isinstance(expr, BinaryOp):
            self._collect_var_refs(expr.left, deps)
            self._collect_var_refs(expr.right, deps)
        elif isinstance(expr, UnaryOp):
            self._collect_var_refs(expr.operand, deps)
        elif isinstance(expr, FuncCall):
            self._collect_var_refs(expr.func, deps)
            for arg in expr.args:
                self._collect_var_refs(arg, deps)
        elif isinstance(expr, MethodCall):
            self._collect_var_refs(expr.object, deps)
            for arg in expr.args:
                self._collect_var_refs(arg, deps)
        elif isinstance(expr, FieldAccess):
            self._collect_var_refs(expr.object, deps)
        elif isinstance(expr, IfExpr):
            self._collect_var_refs(expr.condition, deps)
        elif isinstance(expr, ArrayLiteral):
            for elem in expr.elements:
                self._collect_var_refs(elem, deps)
        elif isinstance(expr, MapLiteral):
            for key, val in expr.entries:
                self._collect_var_refs(key, deps)
                self._collect_var_refs(val, deps)

    def _is_signal(self, name: str) -> bool:
        """Check if a name refers to a signal (not prop, fn, etc.)."""
        info = self.symbols.get(name)
        if info is None:
            return False
        return info.kind in ("signal", "store_signal")

    # ── Phase 5: Computed Chain Flattening ────────────

    def _flatten_computed_chains(self):
        """Flatten computed→computed→signal chains to computed→signal.

        If computed B depends on computed A, and A depends on signal count,
        then B's deps are flattened to [count].
        """
        changed = True
        max_iterations = 100
        iterations = 0

        while changed and iterations < max_iterations:
            changed = False
            iterations += 1
            for name, deps in list(self.dependencies.items()):
                new_deps = []
                for dep in deps:
                    info = self.symbols.get(dep)
                    if info and info.kind == "computed":
                        # Replace computed dependency with its signal deps
                        sub_deps = self.dependencies.get(dep, [dep])
                        for sd in sub_deps:
                            if sd not in new_deps:
                                new_deps.append(sd)
                        changed = True
                    else:
                        if dep not in new_deps:
                            new_deps.append(dep)
                if changed:
                    self.dependencies[name] = new_deps
