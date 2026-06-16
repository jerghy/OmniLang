"""IR Adapter: convert Rust IR JSON dict to Python IR dataclass instances.

The Rust compiler outputs IR as JSON with _kind tags and serde field names.
This adapter converts that JSON to the Python dataclass types defined in ir_types.py.

Usage:
    from omniview.codegen.ir_adapter import IRAdapter

    ir_json = RustBridge.compile_to_ir(source)       # dict from Rust DLL
    ir_program = IRAdapter.adapt_program(ir_json)     # {name: ComponentIR}
    for name, comp in ir_program.items():
        result = ReactCodegen().generate_component(comp)
"""

from __future__ import annotations

from typing import Any

from omniview import ir_types as ir


class IRAdapter:
    """Convert Rust IR JSON dict → Python ir_types dataclass instances."""

    # ═══════════════════════════════════════════════════════════
    # Top-level
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def adapt_program(ir_dict: dict) -> dict[str, ir.ComponentIR]:
        """Convert a Rust IRProgram dict to {component_name: ComponentIR}."""
        components: dict[str, ir.ComponentIR] = {}
        for name, comp_dict in ir_dict.get("components", {}).items():
            components[name] = IRAdapter._adapt_component(comp_dict)
        return components

    # ═══════════════════════════════════════════════════════════
    # Component
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def _adapt_component(d: dict) -> ir.ComponentIR:
        return ir.ComponentIR(
            name=d.get("name", ""),
            generic_params=d.get("generic_params", []),
            props={k: IRAdapter._adapt_type(v) for k, v in d.get("props", {}).items()},
            states=[IRAdapter._adapt_state(s) for s in d.get("states", [])],
            computeds=[IRAdapter._adapt_computed(c) for c in d.get("computeds", [])],
            effects=[IRAdapter._adapt_effect(e) for e in d.get("effects", [])],
            lifecycles=[IRAdapter._adapt_lifecycle(lc) for lc in d.get("lifecycles", [])],
            fns=d.get("fns", []),  # ComponentIR.fns is list[dict] — pass through
            store_refs=d.get("store_refs", []),
            refs=d.get("refs", []),  # refs is list[dict] — pass through
            view=IRAdapter._adapt_view_tree(d.get("view")),
            styles=[IRAdapter._adapt_style_rule(s) for s in d.get("styles", [])],
        )

    # ═══════════════════════════════════════════════════════════
    # State / Computed / Effect / Lifecycle
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def _adapt_state(d: dict) -> ir.StateIntent:
        return ir.StateIntent(
            name=d.get("name", ""),
            type=IRAdapter._adapt_type(d.get("type", {})),
            initial_value=IRAdapter._adapt_expr(d.get("initial_value")),
            mutability=ir.Mutability(d.get("mutability", "mutable")),
            update_semantics=d.get("update_semantics", "setter-call"),
            accessor_style=d.get("accessor_style", "direct"),
            is_copy=d.get("is_copy", False),
        )

    @staticmethod
    def _adapt_computed(d: dict) -> ir.ComputedIntent:
        return ir.ComputedIntent(
            name=d.get("name", ""),
            type=IRAdapter._adapt_type(d.get("type", {})),
            body=IRAdapter._adapt_expr(d.get("body")),
            deps=d.get("deps", []),
        )

    @staticmethod
    def _adapt_effect(d: dict) -> ir.EffectIntent:
        return ir.EffectIntent(
            deps=d.get("deps", []),
            body=IRAdapter._adapt_block(d.get("body")),
        )

    @staticmethod
    def _adapt_lifecycle(d: dict) -> ir.LifecycleIntent:
        kind_str = d.get("kind", "mount")
        try:
            kind = ir.LifecycleKind(kind_str)
        except ValueError:
            kind = ir.LifecycleKind.MOUNT
        return ir.LifecycleIntent(
            kind=kind,
            body=IRAdapter._adapt_block(d.get("body")),
        )

    # ═══════════════════════════════════════════════════════════
    # Block / Statement
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def _adapt_block(d: dict | None) -> ir.BlockIR | None:
        if not d:
            return None
        return ir.BlockIR(
            statements=[IRAdapter._adapt_statement(s) for s in d.get("statements", [])],
            execution_mode=d.get("execution_mode", "client-only"),
        )

    @staticmethod
    def _adapt_statement(d: dict) -> ir.StatementIR:
        kind = d.get("_kind", "")
        if kind == "LetStmtIR":
            return ir.LetStmtIR(
                name=d.get("name", ""),
                value=IRAdapter._adapt_expr(d.get("value")),
                type=IRAdapter._adapt_type(d["type"]) if d.get("type") else None,
            )
        elif kind == "ReturnStmtIR":
            return ir.ReturnStmtIR(
                value=IRAdapter._adapt_expr(d.get("value")),
            )
        elif kind == "IfStmtIR":
            return ir.IfStmtIR(
                condition=IRAdapter._adapt_expr(d.get("condition")),
                then_body=IRAdapter._adapt_block(d.get("then_body")),
                else_body=IRAdapter._adapt_block(d.get("else_body")),
            )
        elif kind == "ExprStmtIR":
            return ir.ExprStmtIR(
                expr=IRAdapter._adapt_expr(d.get("expr")),
            )
        elif kind == "MutationStmt":
            return ir.MutationStmt(
                intent=IRAdapter._adapt_mutation(d["intent"]) if d.get("intent") else None,
            )
        elif kind == "EventEmitStmtIR":
            return ir.EventEmitStmtIR(
                event_name=d.get("event_name", ""),
                args=[IRAdapter._adapt_expr(a) for a in d.get("args", [])],
            )
        else:
            # Fallback: expression statement
            return ir.ExprStmtIR(expr=None)

    @staticmethod
    def _adapt_mutation(d: dict | None) -> ir.MutationIntent | None:
        if not d:
            return None
        target_dict = d.get("target", {})
        return ir.MutationIntent(
            target=ir.SignalRefIR(
                owner=target_dict.get("owner"),
                name=target_dict.get("name", ""),
            ),
            operation=IRAdapter._adapt_mutation_op(d.get("operation", "direct_assign")),
            amount=IRAdapter._adapt_expr(d.get("amount")),
        )

    @staticmethod
    def _adapt_mutation_op(op_str: str) -> ir.MutationOp:
        mapping = {
            "direct_assign": ir.MutationOp.DIRECT_ASSIGN,
            "spread_append": ir.MutationOp.SPREAD_APPEND,
            "spread_prepend": ir.MutationOp.SPREAD_PREPEND,
            "increment": ir.MutationOp.INCREMENT,
            "decrement": ir.MutationOp.DECREMENT,
            "map_update": ir.MutationOp.MAP_UPDATE,
            "filter_update": ir.MutationOp.FILTER_UPDATE,
            "field_update": ir.MutationOp.FIELD_UPDATE,
        }
        return mapping.get(op_str, ir.MutationOp.DIRECT_ASSIGN)

    # ═══════════════════════════════════════════════════════════
    # Expression
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def _adapt_expr(d: dict | None) -> ir.ExprIR | None:
        if not d or not isinstance(d, dict):
            return None
        kind = d.get("_kind", "")
        if kind == "LiteralExpr":
            return ir.LiteralExpr(value=d.get("value"))
        elif kind == "IdentifierExpr":
            return ir.IdentifierExpr(name=d.get("name", ""))
        elif kind == "BinaryOpExpr":
            return ir.BinaryOpExpr(
                op=IRAdapter._adapt_binary_op(d.get("op", "+")),
                left=IRAdapter._adapt_expr(d.get("left")),
                right=IRAdapter._adapt_expr(d.get("right")),
            )
        elif kind == "UnaryOpExpr":
            return ir.UnaryOpExpr(
                op=IRAdapter._adapt_unary_op(d.get("op", "!")),
                operand=IRAdapter._adapt_expr(d.get("operand")),
            )
        elif kind == "FieldAccessExpr":
            return ir.FieldAccessExpr(
                object=IRAdapter._adapt_expr(d.get("object")),
                field=d.get("field", ""),
            )
        elif kind == "StoreFieldAccessExpr":
            return ir.StoreFieldAccessExpr(
                store_alias=d.get("store_alias", ""),
                field_name=d.get("field_name", ""),
                call_args=[IRAdapter._adapt_expr(a) for a in d.get("call_args", [])],
            )
        elif kind == "TernaryIR":
            return ir.TernaryIR(
                cond=IRAdapter._adapt_expr(d.get("cond")),
                then_expr=IRAdapter._adapt_expr(d.get("then_expr")),
                else_expr=IRAdapter._adapt_expr(d.get("else_expr")),
            )
        elif kind == "FuncCallExpr":
            return ir.FuncCallExpr(
                func=IRAdapter._adapt_expr(d.get("func")),
                args=[IRAdapter._adapt_expr(a) for a in d.get("args", [])],
            )
        elif kind == "MethodCallExpr":
            return ir.MethodCallExpr(
                object=IRAdapter._adapt_expr(d.get("object")),
                method=d.get("method", ""),
                args=[IRAdapter._adapt_expr(a) for a in d.get("args", [])],
            )
        else:
            # Fallback: generic ExprIR
            return ir.ExprIR()

    @staticmethod
    def _adapt_binary_op(op_str: str) -> ir.BinaryOp:
        mapping = {
            "+": ir.BinaryOp.ADD, "-": ir.BinaryOp.SUB,
            "*": ir.BinaryOp.MUL, "/": ir.BinaryOp.DIV, "%": ir.BinaryOp.MOD,
            "==": ir.BinaryOp.EQ, "!=": ir.BinaryOp.NEQ,
            "<": ir.BinaryOp.LT, ">": ir.BinaryOp.GT,
            "<=": ir.BinaryOp.LTE, ">=": ir.BinaryOp.GTE,
            "&&": ir.BinaryOp.AND, "||": ir.BinaryOp.OR,
            "add": ir.BinaryOp.ADD, "sub": ir.BinaryOp.SUB,
            "mul": ir.BinaryOp.MUL, "div": ir.BinaryOp.DIV, "mod": ir.BinaryOp.MOD,
            "eq": ir.BinaryOp.EQ, "neq": ir.BinaryOp.NEQ,
            "lt": ir.BinaryOp.LT, "gt": ir.BinaryOp.GT,
            "lteq": ir.BinaryOp.LTE, "gteq": ir.BinaryOp.GTE,
            "and": ir.BinaryOp.AND, "or": ir.BinaryOp.OR,
        }
        return mapping.get(op_str, ir.BinaryOp.ADD)

    @staticmethod
    def _adapt_unary_op(op_str: str) -> ir.UnaryOp:
        mapping = {"!": ir.UnaryOp.NOT, "-": ir.UnaryOp.NEG}
        return mapping.get(op_str, ir.UnaryOp.NOT)

    # ═══════════════════════════════════════════════════════════
    # View Tree
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def _adapt_view_tree(d: dict | None) -> ir.ViewTree:
        """Convert a dict to a ViewTree type (ElementIR | ConditionalIntent | ... | None)."""
        if not d:
            return None
        if not isinstance(d, dict):
            return None

        kind = d.get("_kind", "")
        if not kind:
            # Untagged: detect from structure
            if "tag" in d:
                return IRAdapter._adapt_element(d)
            elif "branches" in d:
                return IRAdapter._adapt_conditional(d)
            elif "iterable" in d:
                return IRAdapter._adapt_list_rendering(d)
            return None

        if kind == "ElementIR":
            return IRAdapter._adapt_element(d)
        elif kind == "ConditionalIntent":
            return IRAdapter._adapt_conditional(d)
        elif kind == "ListRenderingIntent":
            return IRAdapter._adapt_list_rendering(d)
        elif kind == "SlotIntent":
            return ir.SlotIntent(
                name=d.get("name"),
                body=IRAdapter._adapt_view_tree(d.get("body")),
            )
        return None

    @staticmethod
    def _adapt_element(d: dict) -> ir.ElementIR:
        props: dict[str, ir.ExprIR] = {}
        for k, v in d.get("props", {}).items():
            if isinstance(v, dict):
                props[k] = IRAdapter._adapt_expr(v) or ir.LiteralExpr(value=v)
            else:
                props[k] = ir.LiteralExpr(value=v)

        bindings: dict[str, ir.SignalRefIR] = {}
        for k, v in d.get("bindings", {}).items():
            bindings[k] = ir.SignalRefIR(
                owner=v.get("owner") if isinstance(v, dict) else None,
                name=v.get("name", "") if isinstance(v, dict) else str(v),
            )

        events: dict[str, ir.EventHandlerIR] = {}
        for k, v in d.get("events", {}).items():
            events[k] = IRAdapter._adapt_event_handler(v)

        children = [IRAdapter._adapt_view_tree(c) for c in d.get("children", [])]
        # Filter out None children
        children = [c for c in children if c is not None]

        return ir.ElementIR(
            tag=d.get("tag", "div"),
            props=props,
            bindings=bindings,
            events=events,
            style_classes=None,  # Rust doesn't generate StyleIntent yet
            inline_style=None,   # Rust doesn't generate LayoutIntent yet
            children=children,
            test_id=d.get("test_id"),
            ref_name=d.get("ref_name"),
            construction_kind=d.get("construction_kind", "tag-mapping"),
        )

    @staticmethod
    def _adapt_conditional(d: dict) -> ir.ConditionalIntent:
        branches = []
        for b in d.get("branches", []):
            branches.append(ir.ConditionalBranch(
                condition=IRAdapter._adapt_expr(b.get("condition")),
                body=IRAdapter._adapt_view_tree(b.get("body")),
            ))
        return ir.ConditionalIntent(
            branches=branches,
            fallback=IRAdapter._adapt_view_tree(d.get("fallback")),
        )

    @staticmethod
    def _adapt_list_rendering(d: dict) -> ir.ListRenderingIntent:
        item_binding = d.get("item_binding", {})
        return ir.ListRenderingIntent(
            iterable=IRAdapter._adapt_expr(d.get("iterable")),
            item_binding=ir.BindingIR(
                name=item_binding.get("name", "item"),
                type=IRAdapter._adapt_type(item_binding.get("type")) if isinstance(item_binding, dict) and item_binding.get("type") else None,
            ),
            body=IRAdapter._adapt_view_tree(d.get("body")),
            key_expr=IRAdapter._adapt_expr(d.get("key_expr")),
            empty_body=IRAdapter._adapt_view_tree(d.get("empty_body")),
        )

    @staticmethod
    def _adapt_event_handler(d: dict) -> ir.EventHandlerIR:
        if not isinstance(d, dict):
            return ir.EventHandlerIR()
        ek = d.get("kind", "fn_ref")
        try:
            kind = ir.EventHandlerKind(ek)
        except ValueError:
            kind = ir.EventHandlerKind.FN_REF
        return ir.EventHandlerIR(
            kind=kind,
            ref=d.get("ref"),
            body=IRAdapter._adapt_block(d.get("body")),
        )

    # ═══════════════════════════════════════════════════════════
    # Style
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def _adapt_style_rule(d: dict) -> ir.StyleRuleIR:
        return ir.StyleRuleIR(
            class_name=d.get("class_name", ""),
            properties={k: IRAdapter._adapt_expr(v) or ir.ExprIR() for k, v in d.get("properties", {}).items()},
        )

    # ═══════════════════════════════════════════════════════════
    # Type
    # ═══════════════════════════════════════════════════════════

    @staticmethod
    def _adapt_type(d: dict | None) -> ir.TypeIR | None:
        if not d:
            return None
        kind_str = d.get("kind", "void")
        try:
            kind = ir.TypeKind(kind_str)
        except ValueError:
            kind = ir.TypeKind.VOID
        return ir.TypeIR(
            kind=kind,
            type_args=[IRAdapter._adapt_type(a) for a in d.get("type_args", []) if IRAdapter._adapt_type(a)],
            is_nullable=d.get("is_nullable", False),
        )
