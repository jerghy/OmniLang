"""AST → Intent IR lowering pass.

Converts OmniView AST nodes (from parser) into Intent IR types (for codegen).
This is the bridge between parsing and code generation.

Pipeline: .omni → Parser → AST → Lowering → Intent IR → Codegen → Target Code
"""

from dataclasses import asdict

from omnilang.ast_nodes import (
    VarRef, BinaryOp, UnaryOp, Literal, FuncCall, MethodCall,
    ArrayLiteral, MapLiteral, IfExpr, TypeAnnotation, SpreadExpr,
)

from omniview.ast_nodes import (
    Program, ComponentDef, StoreDef,
    PropDecl, SignalDecl, ComputedDecl, EventDecl, RefDecl, StoreRef,
    StoreSignalDecl, StoreGetterDecl, StoreActionDecl,
    Element, ViewBlock,
    IfDirective, ForDirective, SlotDirective, NativeBlock, FallbackBlock,
    ErrorBoundary, RefProp,
    LifecycleHook, FnDef, Block,
    InlineFn, FnRef, StoreActionRef,
    BindProp, EventProp, StaticProp, ClassProp,
    TextNode, Interpolation,
    FieldAccess, TernaryExpr,
    VarDeclStmt, AssignStmt, IfStmt, ReturnStmt, ExprStmt, EventEmitStmt,
    StyleBlock, StyleRule,
)

from omniview.ir_types import (
    # Enums
    Mutability, MutationOp, LifecycleKind, BindableElement,
    EventHandlerKind, TypeKind,
    # Basic IR
    ExprIR, LiteralExpr, IdentifierExpr, FieldAccessExpr,
    BinaryOpExpr, UnaryOpExpr, TernaryIR,
    SignalRefIR, BindingIR, TypeIR,
    EventHandlerIR, BlockIR,
    # Statements
    MutationStmt, EventEmitStmtIR, ReturnStmtIR, ExprStmtIR, IfStmtIR, LetStmtIR,
    # Core Intents
    StateIntent, MutationIntent, ComputedIntent,
    EffectIntent, LifecycleIntent,
    ConditionalBranch, ConditionalIntent,
    ListRenderingIntent, BindIntent, EventIntent, SlotIntent,
    # Style
    StyleRuleIR, StyleIntent, LayoutIntent, ClassNameExpr, ClassNameKind,
    # Element & Top
    ElementIR, ComponentIR, StoreIR,
    # Store field access
    StoreFieldAccessExpr,
    # Enums needed
    BinaryOp as IRBinaryOp, UnaryOp as IRUnaryOp,
)


def ir_to_dict(ir_obj) -> dict:
    """Convert an IR dataclass object to a plain dict for codegen."""
    import enum as _enum
    if ir_obj is None:
        return None
    if isinstance(ir_obj, (str, int, float, bool)):
        return ir_obj
    if isinstance(ir_obj, _enum.Enum):
        return ir_obj.value
    if isinstance(ir_obj, list):
        return [ir_to_dict(item) for item in ir_obj]
    if isinstance(ir_obj, tuple):
        return [ir_to_dict(item) for item in ir_obj]
    if isinstance(ir_obj, dict):
        return {k: ir_to_dict(v) for k, v in ir_obj.items()}
    if hasattr(ir_obj, '__dataclass_fields__'):
        result = {}
        cls_name = type(ir_obj).__name__
        # Add _kind marker for expression and IR types
        if cls_name.endswith('Expr') or cls_name.endswith('IR') or cls_name.endswith('Intent') or cls_name == 'ConditionalBranch':
            result['_kind'] = cls_name
        for field_name in ir_obj.__dataclass_fields__:
            value = getattr(ir_obj, field_name)
            result[field_name] = ir_to_dict(value)
        return result
    return str(ir_obj)


class IRFixture:
    """Lower AST nodes to Intent IR."""

    def __init__(self):
        self._current_store_name: str | None = None
        self._comp_store_aliases: set[str] = set()

    # ── Top Level ──────────────────────────────────────

    def lower_program(self, program: Program) -> dict:
        """Lower entire program to IR dicts for codegen."""
        result = {}
        for comp in program.components:
            ir = self.lower_component(comp)
            result[comp.name] = ir_to_dict(ir)
        for store in program.stores:
            ir = self.lower_store(store)
            result[store.name] = ir_to_dict(ir)
        return result

    def lower_component(self, comp: ComponentDef) -> ComponentIR:
        """Lower a ComponentDef AST node to ComponentIR."""
        # Resolve store aliases: UIStore→ui, CartStore→cart
        self._comp_store_aliases = set()
        for sr in comp.stores:
            sname = sr.store_name
            alias = sname.lower()
            if alias.endswith("store"):
                alias = alias[:-5]
            self._comp_store_aliases.add(alias)
        # Always include "store" for backward compatibility
        self._comp_store_aliases.add("store")

        props = {}
        for p in comp.props:
            props[p.name] = self._lower_type(p.type)

        states = []
        for s in comp.signals:
            states.append(StateIntent(
                name=s.name,
                type=self._lower_type(s.type),
                initial_value=self._lower_expr(s.initial_value),
                mutability=Mutability.MUTABLE,
            ))

        computeds = []
        for c in comp.computeds:
            computeds.append(ComputedIntent(
                name=c.name,
                type=self._lower_type(c.type),
                body=self._lower_expr(c.body),
                deps=[],  # Will be filled by semantic analysis
            ))

        effects = []
        lifecycles = []
        for lc in comp.lifecycles:
            if lc.kind == "effect":
                effects.append(EffectIntent(
                    deps=list(lc.deps) if lc.deps else [],
                    body=self._lower_block(lc.body),
                ))
            else:
                kind = LifecycleKind.MOUNT if lc.kind == "mount" else LifecycleKind.UNMOUNT
                lifecycles.append(LifecycleIntent(
                    kind=kind,
                    body=self._lower_block(lc.body),
                ))

        view = None
        if comp.view:
            view = self._lower_view(comp.view)

        styles = []
        if comp.style:
            styles = self._lower_styles(comp.style)

        fns = []
        for fn in comp.fns:
            fns.append({
                "name": fn.name,
                "params": [{"name": p.name, "type": str(p.type) if p.type else None}
                          for p in fn.params],
                "body": self._lower_block(fn.body),
            })

        store_refs = [sr.store_name for sr in comp.stores]

        refs = []
        for ref in comp.refs:
            refs.append({
                "name": ref.name,
                "type": str(ref.ref_type) if ref.ref_type else "HTMLElement",
            })

        return ComponentIR(
            name=comp.name,
            generic_params=list(comp.generic_params),
            props=props,
            states=states,
            computeds=computeds,
            effects=effects,
            lifecycles=lifecycles,
            fns=fns,
            store_refs=store_refs,
            refs=refs,
            view=view,
            styles=styles,
        )

    def lower_store(self, store: StoreDef) -> StoreIR:
        """Lower a StoreDef AST node to StoreIR."""
        self._current_store_name = store.name
        signals = {}
        signal_defaults = {}
        for s in store.signals:
            signals[s.name] = self._lower_type(s.type)
            if hasattr(s, 'initial_value') and s.initial_value is not None:
                signal_defaults[s.name] = self._lower_expr(s.initial_value)

        getters = {}
        for g in store.getters:
            getters[g.name] = self._lower_expr(g.body)

        actions = []
        for a in store.actions:
            actions.append({
                "name": a.name,
                "params": [{"name": p.name, "type": str(p.type) if p.type else None}
                          for p in a.params],
                "body": self._lower_block(a.body),
                "is_async": a.is_async,
            })

        self._current_store_name = None
        return StoreIR(
            name=store.name,
            signals=signals,
            signal_defaults=signal_defaults,
            getters=getters,
            actions=actions,
        )

    # ── View Tree ──────────────────────────────────────

    def _lower_view(self, view: ViewBlock):
        """Lower ViewBlock children to ViewTree."""
        if not view.children:
            return None
        if len(view.children) == 1:
            return self._lower_view_node(view.children[0])
        # Multiple children → return as list
        return [self._lower_view_node(c) for c in view.children]

    def _lower_view_node(self, node):
        """Lower a single ViewNode to IR."""
        if node is None:
            return None
        if isinstance(node, TextNode):
            return node.content
        if isinstance(node, Interpolation):
            # Return as expression dict so codegen knows to wrap in { }
            expr = self._lower_expr(node.expr) if node.expr else LiteralExpr(value="")
            return {"_kind": "Interpolation", "expr": ir_to_dict(expr)}
        if isinstance(node, Element):
            return self._lower_element(node)
        if isinstance(node, IfDirective):
            return self._lower_if(node)
        if isinstance(node, ForDirective):
            return self._lower_for(node)
        if isinstance(node, SlotDirective):
            return SlotIntent(name=node.name)
        if isinstance(node, NativeBlock):
            return f"@native({node.target}) {{ {node.raw_content} }}"
        if isinstance(node, FallbackBlock):
            return [self._lower_view_node(c) for c in node.children]
        if isinstance(node, ErrorBoundary):
            children = [self._lower_view_node(c) for c in node.children if c is not None]
            return {"_kind": "ErrorBoundary", "name": node.name,
                    "children": children}
        return str(node)

    def _lower_element(self, el: Element) -> ElementIR:
        """Lower an Element to ElementIR."""
        props = {}
        bindings = {}
        events = {}

        for prop in el.props:
            pv = prop.value
            if isinstance(pv, StaticProp):
                raw_value = pv.value
                if isinstance(raw_value, str) and not raw_value.startswith('"'):
                    props[prop.name] = LiteralExpr(value=raw_value)
                elif isinstance(raw_value, (int, float, bool)):
                    props[prop.name] = LiteralExpr(value=raw_value)
                else:
                    props[prop.name] = self._lower_expr(raw_value)
            elif isinstance(pv, BindProp):
                bindings[prop.name] = SignalRefIR(name=pv.signal_ref)
            elif isinstance(pv, EventProp):
                events[prop.name] = self._lower_event_handler(pv.handler)
            elif isinstance(pv, RefProp):
                pass  # ref_name will be set below

        # Handle ref on element
        ref_name = None
        for prop in el.props:
            if isinstance(prop.value, RefProp):
                ref_name = prop.value.ref_name
                break

        children = []
        for child in el.children:
            lowered = self._lower_view_node(child)
            if lowered is not None:
                children.append(lowered)

        return ElementIR(
            tag=el.tag,
            props=props,
            bindings=bindings,
            events=events,
            children=children,
            test_id=el.test_id,
            ref_name=ref_name,
        )

    # ── Directives ─────────────────────────────────────

    def _lower_if(self, node: IfDirective) -> ConditionalIntent:
        branches = []
        for b in node.branches:
            body = [self._lower_view_node(c) for c in b.body if c is not None]
            branches.append(ConditionalBranch(
                condition=self._lower_expr(b.condition),
                body=body[0] if len(body) == 1 else body,
            ))

        fallback = None
        if node.else_body:
            fb = [self._lower_view_node(c) for c in node.else_body if c is not None]
            fallback = fb[0] if len(fb) == 1 else fb

        return ConditionalIntent(branches=branches, fallback=fallback)

    def _lower_for(self, node: ForDirective) -> ListRenderingIntent:
        body = [self._lower_view_node(c) for c in node.body if c is not None]
        empty = None
        if node.empty_body:
            eb = [self._lower_view_node(c) for c in node.empty_body if c is not None]
            empty = eb[0] if len(eb) == 1 else eb

        return ListRenderingIntent(
            iterable=self._lower_expr(node.iterable),
            item_binding=BindingIR(name=node.item_name),
            body=body[0] if len(body) == 1 else body,
            key_expr=None,  # Will be filled from element props
            empty_body=empty,
        )

    # ── Expressions ────────────────────────────────────

    def _lower_expr(self, expr) -> ExprIR | None:
        """Lower an OmniLang expression to ExprIR."""
        if expr is None:
            return None
        if isinstance(expr, Literal):
            return LiteralExpr(value=expr.value)
        if isinstance(expr, VarRef):
            return IdentifierExpr(name=expr.name)
        if isinstance(expr, BinaryOp):
            op_map = {
                "+": IRBinaryOp.ADD, "-": IRBinaryOp.SUB, "*": IRBinaryOp.MUL,
                "/": IRBinaryOp.DIV, "%": IRBinaryOp.MOD,
                "==": IRBinaryOp.EQ, "!=": IRBinaryOp.NEQ,
                "<": IRBinaryOp.LT, ">": IRBinaryOp.GT,
                "<=": IRBinaryOp.LTE, ">=": IRBinaryOp.GTE,
                "&&": IRBinaryOp.AND, "||": IRBinaryOp.OR,
            }
            return BinaryOpExpr(
                op=op_map.get(expr.op, IRBinaryOp.ADD),
                left=self._lower_expr(expr.left),
                right=self._lower_expr(expr.right),
            )
        if isinstance(expr, UnaryOp):
            op_map = {"!": IRUnaryOp.NOT, "-": IRUnaryOp.NEG}
            return UnaryOpExpr(
                op=op_map.get(expr.op, IRUnaryOp.NOT),
                operand=self._lower_expr(expr.operand),
            )
        if isinstance(expr, TernaryExpr):
            return TernaryIR(
                cond=self._lower_expr(expr.condition),
                then_expr=self._lower_expr(expr.then_expr),
                else_expr=self._lower_expr(expr.else_expr),
            )
        if isinstance(expr, FieldAccess):
            return FieldAccessExpr(
                object=self._lower_expr(expr.object),
                field=expr.field,
            )
        if isinstance(expr, FuncCall):
            func_name = expr.func.name if hasattr(expr.func, 'name') else str(expr.func)
            # Built-in formatting functions — preserve args
            if func_name in ("format_date", "format_number"):
                return {
                    "_kind": "FormatCall",
                    "func": func_name,
                    "args": [self._lower_expr(a) for a in (expr.args or [])],
                }
            if func_name == "t":
                # Translation function: t("key") → TranslateCall
                key = expr.args[0].value if expr.args and hasattr(expr.args[0], 'value') else "unknown"
                return {"_kind": "TranslateCall", "key": key}
            # Generic function call — preserve name and args for codegen
            return {
                "_kind": "FuncCallIR",
                "func": func_name,
                "args": [self._lower_expr(a) for a in (expr.args or [])],
            }
        if isinstance(expr, MethodCall):
            # MethodCall serves dual purpose: field access (no args) and method call (with args)
            obj = self._lower_expr(expr.object)
            # Check if this is a store action call
            if isinstance(expr.object, VarRef) and expr.object.name == "store":
                return StoreFieldAccessExpr(
                    store_alias="store",
                    field_name=expr.method,
                    call_args=[self._lower_expr(a) for a in (expr.args or [])] if expr.args else [],
                )
            # Regular field access or method call
            if expr.args:
                # Method call with args — preserve as FieldAccessExpr with known-call semantics
                # The codegen uses StoreFieldAccessExpr for store; for other objects,
                # fall back to FieldAccessExpr (the field name is the method)
                return FieldAccessExpr(
                    object=obj,
                    field=expr.method,
                )
            else:
                # Simple field access (no args)
                return FieldAccessExpr(
                    object=obj,
                    field=expr.method,
                )
        if isinstance(expr, SpreadExpr):
            # Spread in array literal: ...expr
            lowered = self._lower_expr(expr.expr)
            return {"_kind": "SpreadExpr", "expr": ir_to_dict(lowered)}

        if isinstance(expr, ArrayLiteral):
            if not expr.elements:
                return LiteralExpr(value="[]")
            # Lower elements, keeping spread markers structured
            elements = []
            for e in expr.elements:
                elements.append(ir_to_dict(self._lower_expr(e)))
            return {"_kind": "ArrayExpr", "elements": elements}
        if isinstance(expr, MapLiteral):
            return LiteralExpr(value="{}")
        if isinstance(expr, IfExpr):
            return TernaryIR(
                cond=self._lower_expr(expr.condition),
                then_expr=self._lower_expr(expr.then_expr),
                else_expr=self._lower_expr(expr.else_expr),
            )
        return IdentifierExpr(name="unknown")

    # ── Event Handlers ─────────────────────────────────

    def _lower_event_handler(self, handler) -> EventHandlerIR:
        if isinstance(handler, InlineFn):
            return EventHandlerIR(
                kind=EventHandlerKind.INLINE_FN,
                body=self._lower_block(handler.body),
            )
        if isinstance(handler, FnRef):
            return EventHandlerIR(
                kind=EventHandlerKind.FN_REF,
                ref=handler.fn_name,
            )
        if isinstance(handler, StoreActionRef):
            return EventHandlerIR(
                kind=EventHandlerKind.STORE_ACTION,
                ref=handler.action_path,
            )
        return EventHandlerIR(kind=EventHandlerKind.INLINE_FN)

    # ── Block/Statements ───────────────────────────────

    def _lower_block(self, block: Block | None) -> BlockIR | None:
        if block is None:
            return None
        stmts = []
        for s in block.statements:
            lowered = self._lower_stmt(s)
            if lowered is not None:
                stmts.append(lowered)
        return BlockIR(statements=stmts)

    def _lower_stmt(self, stmt):
        if isinstance(stmt, VarDeclStmt):
            return LetStmtIR(
                name=stmt.name,
                value=self._lower_expr(stmt.value),
            )
        if isinstance(stmt, AssignStmt):
            # Detect store.field = value (dotted target)
            target_name = stmt.target
            owner = self._current_store_name
            if "." in target_name:
                parts = target_name.split(".", 1)
                owner = parts[0]  # store alias → store name
                target_name = parts[1]  # field name
            target_ref = SignalRefIR(
                name=target_name,
                owner=owner,  # store alias or None for component signals
            )
            op = MutationOp.DIRECT_ASSIGN
            if stmt.op == "+=":
                op = MutationOp.INCREMENT
            elif stmt.op == "-=":
                op = MutationOp.DECREMENT
            return MutationStmt(intent=MutationIntent(
                target=target_ref,
                operation=op,
                amount=LiteralExpr(value=1),
                item=self._lower_expr(stmt.value),
            ))
        if isinstance(stmt, IfStmt):
            return IfStmtIR(
                condition=self._lower_expr(stmt.condition),
                then_body=self._lower_block(stmt.then_body),
                else_body=self._lower_block(stmt.else_body),
            )
        if isinstance(stmt, ReturnStmt):
            return ReturnStmtIR(value=self._lower_expr(stmt.value))
        if isinstance(stmt, ExprStmt):
            # Detect store.action() / store.action(args) — dispatch to StoreFieldAccessExpr
            expr = stmt.expr
            # Case 1: FuncCall(MethodCall(VarRef(alias), "action"), args)
            if isinstance(expr, FuncCall):
                inner = expr.func
                if isinstance(inner, MethodCall) and isinstance(inner.object, VarRef):
                    alias = inner.object.name
                    if alias in self._comp_store_aliases:
                        return ExprStmtIR(expr=StoreFieldAccessExpr(
                            store_alias=alias,
                            field_name=inner.method,
                            call_args=[self._lower_expr(a) for a in (expr.args or [])],
                        ))
                # FuncCall(FieldAccess(VarRef(alias), "action"), args) — from OmniView parser
                if isinstance(inner, FieldAccess) and isinstance(inner.object, VarRef):
                    alias = inner.object.name
                    if alias in self._comp_store_aliases:
                        return ExprStmtIR(expr=StoreFieldAccessExpr(
                            store_alias=alias,
                            field_name=inner.field,
                            call_args=[self._lower_expr(a) for a in (expr.args or [])],
                        ))
            # Case 2: MethodCall(VarRef(alias), "action") — may or may not have args
            if isinstance(expr, MethodCall) and isinstance(expr.object, VarRef):
                alias = expr.object.name
                if alias in self._comp_store_aliases:
                    return ExprStmtIR(expr=StoreFieldAccessExpr(
                        store_alias=alias,
                        field_name=expr.method,
                        call_args=[self._lower_expr(a) for a in (expr.args or [])],
                    ))
            return ExprStmtIR(expr=self._lower_expr(stmt.expr))
        if isinstance(stmt, EventEmitStmt):
            return EventEmitStmtIR(
                event_name=stmt.event_name,
                args=[self._lower_expr(a) for a in stmt.args],
            )
        return None

    # ── Styles ─────────────────────────────────────────

    def _lower_styles(self, style: StyleBlock) -> list[StyleRuleIR]:
        rules = []
        for rule in style.rules:
            props = {}
            for key, val in rule.properties:
                if hasattr(val, 'value'):
                    props[key] = LiteralExpr(value=str(val.value))
                else:
                    props[key] = LiteralExpr(value=str(val))
            rules.append(StyleRuleIR(
                class_name=rule.class_name,
                properties=props,
            ))
        return rules

    # ── Types ──────────────────────────────────────────

    def _lower_type(self, type_ann: TypeAnnotation | None) -> TypeIR | None:
        if type_ann is None:
            return None
        kind_map = {
            "void": TypeKind.VOID, "i32": TypeKind.I32, "f64": TypeKind.F64,
            "bool": TypeKind.BOOL, "String": TypeKind.STRING,
            "Array": TypeKind.ARRAY, "Map": TypeKind.MAP,
            "null": TypeKind.NULL,
        }
        kind = kind_map.get(type_ann.name, TypeKind.STRUCT)
        type_args = [self._lower_type(a) for a in type_ann.type_args if a is not None]
        return TypeIR(
            kind=kind,
            type_args=[t for t in type_args if t is not None],
        )
