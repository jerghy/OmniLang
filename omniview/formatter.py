"""AST-based formatter for OmniView .omni source files.

Pretty-prints .omni source with consistent formatting:
  - 4-space indentation
  - Opening braces on the same line
  - One property per line in style blocks
  - Consistent spacing around operators
  - Sorted imports
"""

from omniview.ast_nodes import (
    Program, ComponentDef, StoreDef, ThemeDef, ImportStmt,
    PropDecl, SignalDecl, ComputedDecl, EventDecl, RefDecl, StoreRef,
    StoreSignalDecl, StoreGetterDecl, StoreActionDecl,
    Element, ViewBlock, ElementProp, StaticProp, BindProp, EventProp,
    ClassProp, StyleProp, RefProp,
    IfDirective, ForDirective, SlotDirective, NativeBlock, FallbackBlock,
    ErrorBoundary, IfBranch,
    StyleBlock, StyleRule,
    FnDef, LifecycleHook, Block,
    TextNode, Interpolation, InlineFn, FnRef, StoreActionRef,
    StyleRefClass, StringClass, ClassMapClass,
    Stmt, VarDeclStmt, AssignStmt, IfStmt, WhileStmt, ForInStmt,
    ReturnStmt, ExprStmt, EventEmitStmt,
    TernaryExpr, FieldAccess,
)
from omnilang.ast_nodes import (
    Expr, Literal, VarRef, BinaryOp, UnaryOp, FuncCall, MethodCall,
    ArrayLiteral, MapLiteral, IfExpr, TemplateString, SpreadExpr,
    TypeAnnotation, UnionType,
)


class OmniViewFormatter:
    """AST-based formatter for OmniView .omni files.

    Usage:
        formatter = OmniViewFormatter()
        formatted = formatter.format(source)
    """

    def __init__(self, indent_size: int = 4):
        self.indent_size = indent_size
        self._indent_level = 0

    def format(self, source: str) -> str:
        """Format OmniView source code from the full pipeline."""
        from omniview.preprocessor import preprocess
        from omnilang.lexer import create_omniview_parser
        from omniview.parser import OmniViewTransformer

        pp = preprocess(source)
        parser = create_omniview_parser()
        tree = parser.parse(pp.processed_source)
        transformer = OmniViewTransformer(block_map=pp.blocks)
        program = transformer.transform(tree)

        lines = []
        self._indent_level = 0

        # Lib imports
        for lib in program.lib_imports:
            lines.append(f"import {lib};")
        if program.lib_imports:
            lines.append("")

        # Cross-file imports (sorted)
        sorted_imports = sorted(program.imports, key=lambda i: i.path)
        for imp in sorted_imports:
            names_str = ", ".join(imp.names)
            lines.append(f'import {{ {names_str} }} from "{imp.path}";')
        if sorted_imports:
            lines.append("")

        # Stores first, then components, then themes
        for store in program.stores:
            lines.extend(self._format_store(store))
            lines.append("")

        for comp in program.components:
            lines.extend(self._format_component(comp))
            lines.append("")

        for theme in program.themes:
            lines.extend(self._format_theme(theme))
            lines.append("")

        return "\n".join(lines).rstrip("\n") + "\n"

    def _i(self) -> str:
        """Current indentation string."""
        return " " * (self._indent_level * self.indent_size)

    # ── Component ────────────────────────────────────────

    def _format_component(self, comp: ComponentDef) -> list[str]:
        L = []
        generic = f"<{', '.join(comp.generic_params)}>" if comp.generic_params else ""
        L.append(f"component {comp.name}{generic} {{")
        self._indent_level += 1
        i = self._i()

        # Props
        for p in comp.props:
            L.append(f"{i}prop {p.name}: {self._format_type(p.type)};")
        if comp.props:
            L.append("")

        # Signals
        for s in comp.signals:
            L.append(f"{i}signal {s.name}: {self._format_type(s.type)} = {self._format_expr(s.initial_value)};")
        if comp.signals:
            L.append("")

        # Computeds
        for c in comp.computeds:
            L.append(f"{i}computed {c.name}: {self._format_type(c.type)} = {self._format_expr(c.body)};")
        if comp.computeds:
            L.append("")

        # Events
        for e in comp.events:
            opt = "?" if e.is_optional else ""
            L.append(f"{i}event {e.name}: {self._format_type(e.payload_type)}{opt};")
        if comp.events:
            L.append("")

        # Refs
        for r in comp.refs:
            L.append(f"{i}ref {r.name}: {self._format_type(r.ref_type)};")
        if comp.refs:
            L.append("")

        # Store refs
        for s in comp.stores:
            L.append(f"{i}@store {s.store_name};")
        if comp.stores:
            L.append("")

        # Lifecycle hooks
        for lc in comp.lifecycles:
            L.extend(self._format_lifecycle(lc))
            L.append("")

        # Fns
        for fn in comp.fns:
            L.extend(self._format_fn(fn))
            L.append("")

        # View
        if comp.view:
            L.extend(self._format_view(comp.view))

        # Style
        if comp.style:
            L.extend(self._format_style(comp.style))

        self._indent_level -= 1
        L.append("}")
        return L

    # ── Store ────────────────────────────────────────────

    def _format_store(self, store: StoreDef) -> list[str]:
        L = []
        L.append(f"store {store.name} {{")
        self._indent_level += 1
        i = self._i()

        for s in store.signals:
            L.append(f"{i}signal {s.name}: {self._format_type(s.type)} = {self._format_expr(s.initial_value)};")

        if store.getters:
            L.append("")
            for g in store.getters:
                L.append(f"{i}getter {g.name}: {self._format_type(g.type)} = {self._format_expr(g.body)};")

        if store.actions:
            L.append("")
            for a in store.actions:
                L.extend(self._format_action(a, i))

        self._indent_level -= 1
        L.append("}")
        return L

    def _format_action(self, action: StoreActionDecl, i: str) -> list[str]:
        L = []
        async_kw = "async " if action.is_async else ""
        params = ", ".join(f"{p.name}: {self._format_type(p.type)}" for p in action.params)
        ret = f" -> {self._format_type(action.return_type)}" if action.return_type else ""
        L.append(f"{i}{async_kw}action {action.name}({params}){ret} {{")
        if action.body:
            self._indent_level += 1
            for stmt in action.body.statements:
                L.append(self._format_stmt(stmt))
            self._indent_level -= 1
        L.append(f"{i}}}")
        return L

    # ── Functions ─────────────────────────────────────────

    def _format_fn(self, fn: FnDef) -> list[str]:
        L = []
        i = self._i()
        async_kw = "async " if fn.is_async else ""
        params = ", ".join(f"{p.name}: {self._format_type(p.type)}" for p in fn.params)
        ret = f" -> {self._format_type(fn.return_type)}" if fn.return_type else ""
        L.append(f"{i}{async_kw}fn {fn.name}({params}){ret} {{")
        if fn.body:
            self._indent_level += 1
            for stmt in fn.body.statements:
                L.append(self._format_stmt(stmt))
            self._indent_level -= 1
        L.append(f"{i}}}")
        return L

    def _format_lifecycle(self, lc: LifecycleHook) -> list[str]:
        L = []
        i = self._i()
        if lc.kind == "mount":
            L.append(f"{i}@mount {{")
        elif lc.kind == "unmount":
            L.append(f"{i}@unmount {{")
        elif lc.kind == "effect":
            deps = ", ".join(lc.deps) if lc.deps else ""
            L.append(f"{i}@effect({deps}) {{")
        if lc.body:
            self._indent_level += 1
            for stmt in lc.body.statements:
                L.append(self._format_stmt(stmt))
            self._indent_level -= 1
        L.append(f"{i}}}")
        return L

    # ── View ──────────────────────────────────────────────

    def _format_view(self, view: ViewBlock) -> list[str]:
        L = []
        L.append(f"{self._i()}view {{")
        self._indent_level += 1
        for child in view.children:
            L.extend(self._format_view_node(child))
        self._indent_level -= 1
        L.append(f"{self._i()}}}")
        return L

    def _format_view_node(self, node) -> list[str]:
        i = self._i()
        if isinstance(node, Element):
            return self._format_element(node)
        elif isinstance(node, IfDirective):
            return self._format_if_directive(node)
        elif isinstance(node, ForDirective):
            return self._format_for_directive(node)
        elif isinstance(node, TextNode):
            return [f"{i}{node.content}"]
        elif isinstance(node, Interpolation):
            return [f"{i}{{{self._format_expr(node.expr)}}}"]
        elif isinstance(node, SlotDirective):
            name_attr = f' name="{node.name}"' if node.name else ""
            return [f"{i}@slot{name_attr}"]
        return [f"{i}<!-- unknown node -->"]

    def _format_element(self, el: Element) -> list[str]:
        L = []
        i = self._i()
        props_str = ""
        for p in el.props:
            props_str += " " + self._format_prop(p)
        if el.children:
            L.append(f"{i}<{el.tag}{props_str}>")
            self._indent_level += 1
            for child in el.children:
                L.extend(self._format_view_node(child))
            self._indent_level -= 1
            L.append(f"{self._i()}</{el.tag}>")
        else:
            L.append(f"{i}<{el.tag}{props_str} />")
        return L

    def _format_prop(self, prop: ElementProp) -> str:
        name = prop.name
        value = prop.value
        if isinstance(value, StaticProp):
            v = value.value
            if isinstance(v, str) and not name.startswith("on"):
                return f'{name}="{v}"'
            return f"{name}={{{self._format_expr(v)}}}"
        elif isinstance(value, BindProp):
            return f"{name}={{{value.signal_ref}}}"
        elif isinstance(value, EventProp):
            return f"{name}={{{self._format_event_handler(value.handler)}}}"
        elif isinstance(value, ClassProp):
            cv = value.class_value
            if isinstance(cv, StyleRefClass):
                return f'class={{style.{cv.class_name}}}'
            elif isinstance(cv, StringClass):
                return f'class="{cv.value}"'
            return f"class={{...}}"
        elif isinstance(value, StyleProp):
            entries = ", ".join(f'{k}: {self._format_expr(v)}' for k, v in value.properties)
            return f"style={{{{ {entries} }}}}"
        elif isinstance(value, RefProp):
            return f"ref={{{{{value.ref_name}}}}}"
        return f'{name}="..."'

    def _format_event_handler(self, handler) -> str:
        if isinstance(handler, FnRef):
            return handler.fn_name
        elif isinstance(handler, StoreActionRef):
            return handler.action_path
        elif isinstance(handler, InlineFn):
            params = ", ".join(p.name for p in handler.params)
            return f"fn({params}) {{ ... }}"
        return ""

    def _format_if_directive(self, d: IfDirective) -> list[str]:
        L = []
        i = self._i()
        for idx, branch in enumerate(d.branches):
            if idx == 0:
                L.append(f"{i}@if {self._format_expr(branch.condition)} {{")
            else:
                L.append(f"{i}@else if {self._format_expr(branch.condition)} {{")
            self._indent_level += 1
            for child in branch.body:
                L.extend(self._format_view_node(child))
            self._indent_level -= 1
            L.append(f"{self._i()}}}")
        if d.else_body is not None:
            L.append(f"{i}@else {{")
            self._indent_level += 1
            for child in d.else_body:
                L.extend(self._format_view_node(child))
            self._indent_level -= 1
            L.append(f"{self._i()}}}")
        return L

    def _format_for_directive(self, d: ForDirective) -> list[str]:
        L = []
        i = self._i()
        L.append(f"{i}@for {d.item_name} in {self._format_expr(d.iterable)} {{")
        self._indent_level += 1
        for child in d.body:
            L.extend(self._format_view_node(child))
        self._indent_level -= 1
        L.append(f"{self._i()}}}")
        if d.empty_body:
            L.append(f"{i}@empty {{")
            self._indent_level += 1
            for child in d.empty_body:
                L.extend(self._format_view_node(child))
            self._indent_level -= 1
            L.append(f"{self._i()}}}")
        return L

    # ── Style ─────────────────────────────────────────────

    def _format_style(self, style: StyleBlock) -> list[str]:
        L = []
        L.append(f"{self._i()}style {{")
        self._indent_level += 1
        i = self._i()
        for rule in style.rules:
            L.append(f"{i}.{rule.class_name} {{")
            for k, v in rule.properties:
                L.append(f"{i}    {k}: {self._format_expr(v)};")
            L.append(f"{i}}}")
        self._indent_level -= 1
        L.append(f"{self._i()}}}")
        return L

    # ── Theme ─────────────────────────────────────────────

    def _format_theme(self, theme: ThemeDef) -> list[str]:
        L = []
        L.append("theme {")
        self._indent_level += 1
        i = self._i()
        for cat in theme.categories:
            L.append(f"{i}{cat.name} {{")
            for k, v in cat.values:
                L.append(f"{i}    {k}: \"{v}\";")
            L.append(f"{i}}}")
        self._indent_level -= 1
        L.append("}")
        return L

    # ── Statements ────────────────────────────────────────

    def _format_stmt(self, stmt) -> str:
        """Format a single statement."""
        i = self._i()
        if isinstance(stmt, VarDeclStmt):
            mut = "mut " if stmt.is_mutable else ""
            type_str = f": {self._format_type(stmt.type)}" if stmt.type else ""
            val = self._format_expr(stmt.value) if stmt.value else ""
            return f"{i}let {mut}{stmt.name}{type_str} = {val};"
        elif isinstance(stmt, AssignStmt):
            val = self._format_expr(stmt.value) if stmt.value else ""
            return f"{i}{stmt.target} {stmt.op} {val};"
        elif isinstance(stmt, IfStmt):
            return self._format_stmt_if(stmt)
        elif isinstance(stmt, ReturnStmt):
            val = self._format_expr(stmt.value) if stmt.value else ""
            return f"{i}return{(' ' + val) if val else ''};"
        elif isinstance(stmt, ExprStmt):
            return f"{i}{self._format_expr(stmt.expr)};" if stmt.expr else ""
        return f"{i}// unknown stmt"

    def _format_stmt_if(self, stmt: IfStmt) -> str:
        i = self._i()
        return f"{i}if {self._format_expr(stmt.condition)} {{ ... }}"

    # ── Type formatting ───────────────────────────────────

    def _format_type(self, t) -> str:
        if t is None:
            return "void"
        if isinstance(t, TypeAnnotation):
            if t.type_args:
                args = ", ".join(self._format_type(a) for a in t.type_args)
                return f"{t.name}<{args}>"
            return t.name
        if isinstance(t, UnionType):
            return " | ".join(self._format_type(u) for u in t.types)
        if isinstance(t, str):
            return t
        return str(t)

    # ── Expression formatting ─────────────────────────────

    def _format_expr(self, e) -> str:
        if e is None:
            return ""
        if isinstance(e, Literal):
            if e.kind == "String":
                return f'"{e.value}"'
            elif e.kind == "bool":
                return "true" if e.value else "false"
            return str(e.value)
        if isinstance(e, VarRef):
            return e.name
        if isinstance(e, BinaryOp):
            return f"{self._format_expr(e.left)} {e.op} {self._format_expr(e.right)}"
        if isinstance(e, UnaryOp):
            return f"{e.op}{self._format_expr(e.operand)}"
        if isinstance(e, FuncCall):
            args = ", ".join(self._format_expr(a) for a in e.args)
            return f"{self._format_expr(e.func)}({args})"
        if isinstance(e, MethodCall):
            args = ", ".join(self._format_expr(a) for a in e.args)
            return f"{self._format_expr(e.object)}.{e.method}({args})"
        if isinstance(e, ArrayLiteral):
            return f"[{', '.join(self._format_expr(x) for x in e.elements)}]"
        if isinstance(e, MapLiteral):
            entries = ", ".join(f"{self._format_expr(k)}: {self._format_expr(v)}" for k, v in e.entries)
            return f"{{{{{entries}}}}}"
        if isinstance(e, SpreadExpr):
            return f"...{self._format_expr(e.expr)}"
        if isinstance(e, IfExpr):
            return f"if {self._format_expr(e.condition)} {{ ... }} else {{ ... }}"
        if isinstance(e, TernaryExpr):
            return f"{self._format_expr(e.condition)} ? {self._format_expr(e.then_expr)} : {self._format_expr(e.else_expr)}"
        if isinstance(e, FieldAccess):
            return f"{self._format_expr(e.object)}.{e.field}"
        return str(e)
