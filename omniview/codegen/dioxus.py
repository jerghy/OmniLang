"""Dioxus L2 codegen — Intent IR → idiomatic Dioxus 0.5+ RSX + GlobalSignal.

Generates L2 (idiomatic) Dioxus Rust code from OmniView Intent IR.
Target: Dioxus 0.5+ with rsx! macro, use_signal, use_memo, GlobalSignal.
"""

from omniview.codegen.base import CodegenBase, CodegenResult


# ── Element Mapping ───────────────────────────────────

DIOXUS_ELEMENT_MAP: dict[str, str] = {
    "Container": "div", "Flex": "div", "Grid": "div",
    "Stack": "div", "Spacer": "div",
    "Text": "span", "Heading": "h2", "Image": "img",
    "Link": "a", "Divider": "hr",
    "Button": "button", "Input": "input", "TextArea": "textarea",
    "Checkbox": "input", "Select": "select",
    "List": "ul", "ListItem": "li",
    "Spinner": "div", "Progress": "progress",
    "Alert": "div", "Card": "div", "Form": "form",
}

LAYOUT_PROPS = {"direction", "gap", "align", "justify", "wrap", "flex",
                "padding", "margin", "width", "height"}

# Dioxus event names are all lowercase
EVENT_MAP = {
    "onPress": "onclick", "onChange": "oninput",
    "onSubmit": "onsubmit", "onFocus": "onfocus", "onBlur": "onblur",
    "onHover": "onmouseenter", "onLeave": "onmouseleave",
}


class DioxusCodegen(CodegenBase):
    """Generate L2 idiomatic Dioxus RSX from OmniView IR."""

    target = "dioxus"
    tier = "L2"
    ELEMENT_MAP = DIOXUS_ELEMENT_MAP

    def __init__(self):
        super().__init__()
        self._in_store_action = False
        self._store_signal_names: set = set()
        self._all_stores: dict = {}
        self._cross_file_imports: dict[str, str] = {}

    def _get(self, obj, key, default=None):
        if isinstance(obj, dict):
            return obj.get(key, default)
        return getattr(obj, key, default)

    # ── Public API ─────────────────────────────────────

    def generate_component(self, comp_ir, store_irs: dict | None = None, source_line: int = 1) -> CodegenResult:
        result = CodegenResult()
        try:
            self._all_stores = store_irs or {}
            rs = self._gen_component(comp_ir, store_irs or {}, source_line)
            name = self._get(comp_ir, "name", "Component")
            result.files[f"{name}.rs"] = rs
        except Exception as e:
            result.errors.append(f"Dioxus codegen failed: {e}")
        return result

    def generate_store(self, store_ir, source_line: int = 1) -> CodegenResult:
        result = CodegenResult()
        try:
            name = self._get(store_ir, "name", "Store")
            rs = self._gen_store(store_ir, source_line)
            result.files[f"{name.lower()}_store.rs"] = rs
        except Exception as e:
            result.errors.append(f"Dioxus store codegen failed: {e}")
        return result

    # ── Component Generation ───────────────────────────

    def _gen_component(self, comp: dict, stores: dict, source_line: int = 1) -> str:
        name = self._get(comp, "name", "Component")
        store_refs = set(self._get(comp, "store_refs", []))
        signal_names = {s.get("name", "") for s in self._get(comp, "states", [])}

        L = []
        # Imports
        hooks = set()
        if self._get(comp, "states"): hooks.add("use_signal")
        if self._get(comp, "computeds"): hooks.add("use_memo")
        if self._get(comp, "effects") or self._get(comp, "lifecycles"): hooks.add("use_effect")
        L.append("use dioxus::prelude::*;")
        L.append("")

        # Store imports
        if store_refs:
            for sname in sorted(store_refs):
                L.append(f"mod {sname.lower()}_store;")
                L.append(f"use {sname.lower()}_store::use_{sname.lower()};")
            L.append("")

        # Props struct
        props = self._get(comp, "props", {})
        if props:
            L.append("#[derive(Clone, PartialEq, Props)]")
            L.append(f"pub struct {name}Props {{")
            for pname, ptype in props.items():
                rust_type = self._map_type_to_rust(ptype)
                L.append(f"    pub {pname}: {rust_type},")
            L.append("}")
            L.append("")

        # Component function
        L.append(f"// .omni:{source_line} -- component {name}")
        L.append("#[component]")
        if props:
            L.append(f"pub fn {name}(props: {name}Props) -> Element {{")
        else:
            L.append(f"pub fn {name}() -> Element {{")

        # Store instances
        if store_refs:
            for sname in sorted(store_refs):
                L.append(f"    let {sname.lower()} = use_{sname.lower()}();")
            L.append("")

        # State signals
        for state in self._get(comp, "states", []):
            sig_name = self._get(state, "name", "value")
            init_val = self._expr_to_rust(self._get(state, "initial_value"))
            ts_type = self._map_type_to_rust(self._get(state, "type"))
            L.append(f"    let mut {sig_name} = use_signal(|| {init_val});")

        # Computed
        for computed in self._get(comp, "computeds", []):
            cname = self._get(computed, "name", "value")
            body = self._expr_to_rust(self._get(computed, "body"))
            L.append(f"    let {cname} = use_memo(|| {body});")

        # Lifecycles
        for lc in self._get(comp, "lifecycles", []):
            kind = self._get(lc, "kind", "mount")
            body = self._gen_block_body_rs(self._get(lc, "body"), signal_names)
            if kind == "mount" and body.strip():
                L.append(f"    use_effect(|| {{")
                L.append(self.indent(self.indent(body)))
                L.append(self.indent("    // @mount"))
                L.append(f"    }});")

        # Effects
        for eff in self._get(comp, "effects", []):
            body = self._gen_block_body_rs(self._get(eff, "body"), signal_names)
            deps = self._get(eff, "deps", [])
            # Clone deps before move
            for dep in deps:
                L.append(f"    let {dep}_cloned = {dep}.clone();")
            L.append(f"    use_effect(move || {{")
            L.append(self.indent(self.indent(body)))
            L.append(f"    }});")

        # Functions
        for fn in self._get(comp, "fns", []):
            fname = self._get(fn, "name", "handler")
            params = self._get(fn, "params", [])
            param_str = ", ".join(
                f"{self._get(p, 'name', 'x')}: {self._map_type_to_rust(self._get(p, 'type'))}"
                for p in params
            )
            body = self._gen_block_body_rs(self._get(fn, "body"), signal_names)
            L.append(f"    let {fname} = move |{param_str}| {{")
            L.append(self.indent(self.indent(body)))
            L.append("    };")

        # View
        L.append("")
        L.append("    rsx! {")
        view_rsx = self._gen_view_tree(self._get(comp, "view"))
        L.append(self.indent(self.indent(view_rsx)))
        L.append("    }")
        L.append("}")

        return "\n".join(L)

    # ── View Tree → RSX ────────────────────────────────

    def _gen_view_tree(self, view) -> str:
        if view is None:
            return ""
        if isinstance(view, str):
            return self._escape_rsx(view)
        if isinstance(view, list):
            return "\n".join(self._gen_view_tree(c) for c in view)
        if isinstance(view, dict):
            kind = self._get(view, "_kind", "")
            if kind == "Interpolation":
                return "{" + self._expr_to_rust(self._get(view, "expr")) + "}"
            if kind == "ListRenderingIntent" or "iterable" in view:
                return self._gen_list(view)
            if kind == "ConditionalIntent" or "branches" in view:
                return self._gen_conditional(view)
            if kind == "ErrorBoundary":
                return self._gen_error_boundary(view)
            if "tag" in view:
                return self._gen_element(view)
        return ""

    def _gen_element(self, elem: dict) -> str:
        tag = self._get(elem, "tag", "div")
        if tag == "Heading":
            level_prop = self._get(elem, "props", {}).get("level", {})
            level = self._get(level_prop, "value", "2")
            html_tag = f"h{level}"
        else:
            html_tag = self.ELEMENT_MAP.get(tag, tag)

        if tag == "Spacer":
            return "div { flex: \"1\" }"

        props = self._get(elem, "props", {})
        bindings = self._get(elem, "bindings", {})
        events = self._get(elem, "events", {})
        children = self._get(elem, "children", [])
        test_id = self._get(elem, "test_id")

        attrs = []

        # Layout props → Dioxus style attributes
        style_parts = []
        A11Y_MAP_DX = {"accessibilityLabel": "aria-label", "accessibilityRole": "role"}
        for pname, pval in props.items():
            if pname in LAYOUT_PROPS or pname in ("level",):
                continue
            html_name = A11Y_MAP_DX.get(pname, pname)
            val = self._expr_to_rust(pval)
            # Strip outer quotes (LiteralExpr adds them)
            if val.startswith('"') and val.endswith('"'):
                val = val[1:-1]
            attrs.append(f'{html_name}: "{val}"')

        # Layout → CSS-like attributes in rsx
        for pname, pval in props.items():
            if pname in LAYOUT_PROPS:
                raw = self._extract_style_val(pval)
                css = self._layout_to_rsx(pname, raw)
                if css:
                    style_parts.append(css)

        if style_parts or tag in ("Flex", "Grid"):
            if tag == "Flex":
                style_parts.insert(0, "display: flex")
            if tag == "Grid":
                style_parts.insert(0, "display: grid")
                if not any("grid-template-columns" in s for s in style_parts):
                    style_parts.append("grid-template-columns: repeat(auto-fill, minmax(250px, 1fr))")
            style_str = "; ".join(style_parts)
            attrs.append(f'style: "{style_str}"')

        # Bindings → manual two-way
        for bname, bref in bindings.items():
            sig_name = self._get(bref, "name", "")
            if bname == "value":
                attrs.append(f'value: "{{{sig_name}}}"')
                attrs.append(f'oninput: move |e| {sig_name}.set(e.value())')
            elif bname == "checked":
                attrs.append(f'checked: "{{{sig_name}}}"')
                attrs.append(f'onchange: move |e| {sig_name}.set(e.checked())')

        # Events
        for ename, ehandler in events.items():
            dx_event = EVENT_MAP.get(ename, ename.lower())
            handler = self._gen_event_handler(ehandler)
            attrs.append(f'{dx_event}: {handler}')

        if test_id:
            attrs.append(f'data-testid: "{test_id}"')

        attr_str = ", ".join(attrs)
        tag_open = f"{html_tag} {{" + (f" {attr_str}" if attr_str else "")

        if not children:
            return f"{tag_open} }}"

        child_strs = []
        for child in children:
            child_strs.append(self._gen_view_tree(child))

        inner = "\n".join(child_strs)
        return f"{tag_open}\n{inner}\n}}"

    def _gen_conditional(self, cond: dict) -> str:
        branches = self._get(cond, "branches", [])
        fallback = self._get(cond, "fallback")
        L = []
        for branch in branches:
            c = self._expr_to_rust(self._get(branch, "condition"))
            body = self._gen_view_tree(self._get(branch, "body"))
            L.append(f"if {c} {{")
            L.append(self.indent(f"rsx! {{ {body} }}"))
            L.append("}")
        if fallback:
            fb = self._gen_view_tree(fallback)
            L.append(f"else {{")
            L.append(self.indent(f"rsx! {{ {fb} }}"))
            L.append("}")
        return "\n".join(L)

    def _gen_error_boundary(self, boundary: dict) -> str:
        """Generate Dioxus error boundary using match on Result.

        Dioxus doesn't have native error boundaries like React.
        Emit a wrapped div with data-error-boundary attribute.
        P-level warning is documented in the execution plan.
        """
        name = self._get(boundary, "name", "error")
        children = self._get(boundary, "children", [])
        child_rsx = "\n".join(self._gen_view_tree(c) for c in children)
        # Dioxus L2 limitation: error boundaries are documented only.
        # Emit as regular children with a comment marker.
        return (
            f"// @error({name}) — error boundary (Dioxus: manual catch_unwind needed)\n"
            f"{{ {child_rsx} }}"
        )

    def _gen_list(self, lst: dict) -> str:
        iterable = self._expr_to_rust(self._get(lst, "iterable"))
        binding = self._get(lst, "item_binding", {})
        item_name = self._get(binding, "name", "item")
        body = self._gen_view_tree(self._get(lst, "body"))
        key_expr = self._expr_to_rust(self._get(lst, "key_expr"))
        empty_body = self._get(lst, "empty_body")

        map_body = f"{{{iterable}.iter().map(|{item_name}| rsx! {{ {body} }})}}"
        if empty_body:
            empty = self._gen_view_tree(empty_body)
            return f"if {iterable}.is_empty() {{\n    rsx! {{ {empty} }}\n}} else {{\n    {map_body}\n}}"
        return map_body

    # ── Event Handlers ─────────────────────────────────

    def _gen_event_handler(self, handler: dict) -> str:
        kind = self._get(handler, "kind", "inline_fn")
        if kind in ("fn_ref", "FN_REF"):
            ref = self._get(handler, "ref", "")
            return f"move |_| {ref}()"
        if kind in ("store_action", "STORE_ACTION"):
            ref = self._get(handler, "ref", "")
            parts = ref.split(".")
            # In Dioxus, store actions are accessed via GlobalSignal or module functions
            if len(parts) == 2:
                _, action = parts
                return f"move |_| {action}()"
            return f"move |_| {ref}()"
        # Inline function
        body = self._gen_block_body_rs_hand(self._get(handler, "body"))
        params = self._get(handler, "params", [])
        param_str = ", ".join(self._get(p, "name", "e") for p in params)
        if body.strip():
            return f"move |{param_str}| {{\n{self.indent(self.indent(body))}\n    }}"
        return f"move |{param_str}| {{}}"

    # ── Expression → Rust ──────────────────────────────

    def _expr_to_rust(self, expr) -> str:
        if expr is None:
            return "None"
        if isinstance(expr, (int, float)):
            return str(expr)
        if isinstance(expr, bool):
            return "true" if expr else "false"
        if isinstance(expr, str):
            return expr
        if isinstance(expr, dict):
            kind = self._get(expr, "_kind", "")
            if kind == "StoreFieldAccessExpr" or ("store_alias" in expr and "field_name" in expr):
                field = self._get(expr, "field_name", "")
                args = self._get(expr, "call_args", [])
                args_str = ", ".join(self._expr_to_rust(a) for a in args)
                return f"{field}({args_str})"
            if kind == "FieldAccessExpr":
                obj = self._get(expr, "object")
                field = self._get(expr, "field", "")
                if isinstance(obj, dict) and self._get(obj, "name") == "store":
                    # store.field in template → call the getter/action function directly
                    return f"{field}()"
                obj_str = self._expr_to_rust(obj)
                return f"{obj_str}.{field}"
            if kind == "FormatCall":
                return self._gen_format_call(expr)
            if kind == "TranslateCall":
                key = self._get(expr, "key", "unknown")
                return f'i18n.t("{key}")'
            if kind == "IdentifierExpr" or ("name" in expr and "op" not in expr and "value" not in expr and len(expr) <= 2):
                name = self._get(expr, "name", "undefined")
                # In store context (getter/action), prefix with state. for Dioxus struct fields
                if self._in_store_action and name in self._store_signal_names:
                    return f"state.{name}"
                return name
            if kind == "LiteralExpr" or ("value" in expr and "op" not in expr and "name" not in expr):
                val = self._get(expr, "value")
                if isinstance(val, str):
                    if val in ("[]", "{}"):
                        return val
                    if val.startswith("[") or val.startswith("{"):
                        return val
                    return f'"{val}"'
                if isinstance(val, bool):
                    return "true" if val else "false"
                return str(val) if val is not None else "None"
            if kind == "BinaryOpExpr" or ("op" in expr and "left" in expr):
                left = self._expr_to_rust(self._get(expr, "left"))
                right = self._expr_to_rust(self._get(expr, "right"))
                op_map = {"add": "+", "sub": "-", "mul": "*", "div": "/", "mod": "%",
                          "eq": "==", "neq": "!=", "lt": "<", "gt": ">",
                          "lte": "<=", "gte": ">=", "and": "&&", "or": "||",
                          "+": "+", "-": "-", "*": "*", "/": "/", "%": "%",
                          "==": "==", "!=": "!=", "<": "<", ">": ">",
                          "<=": "<=", ">=": ">=", "&&": "&&", "||": "||"}
                op = op_map.get(self._get(expr, "op", "+"), "+")
                return f"({left} {op} {right})"
            if kind == "UnaryOpExpr":
                op = "!" if self._get(expr, "op") in ("!", "not") else "-"
                return f"{op}{self._expr_to_rust(self._get(expr, 'operand'))}"
            if "name" in expr:
                return self._get(expr, "name", "undefined")
        return "()"

    def _gen_format_call(self, expr: dict) -> str:
        func = self._get(expr, "func", "")
        args = [self._expr_to_rust(a) for a in self._get(expr, "args", [])]
        if func == "format_date":
            val = args[0] if args else '""'
            return f'{val}.to_string()'
        elif func == "format_number":
            val = args[0] if args else "0"
            decimals = args[1] if len(args) > 1 else "2"
            return f'format!("{{:.{decimals}}}", {val})'
        return "()"

    # ── Statement Blocks (Dioxus/Rust) ─────────────────

    def _gen_block_body_rs(self, body, signal_names: set) -> str:
        if body is None:
            return ""
        stmts = self._get(body, "statements", []) if isinstance(body, dict) else []
        L = []
        for stmt in stmts:
            if not isinstance(stmt, dict):
                continue
            kind = self._get(stmt, "_kind", "")
            if kind == "MutationStmt" or "intent" in stmt:
                intent = self._get(stmt, "intent", {})
                target_ref = self._get(intent, "target", {})
                target = self._get(target_ref, "name", "x")
                target_owner = self._get(target_ref, "owner")
                op = self._get(intent, "operation", "direct_assign")
                amount = self._expr_to_rust(self._get(intent, "amount", 1))
                item = self._expr_to_rust(self._get(intent, "item"))

                if target_owner:
                    # Store action: state struct has plain fields (mutable via &mut self)
                    if op in ("increment", "INCREMENT"):
                        L.append(f"state.{target} += {amount};")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"state.{target} -= {amount};")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"state.{target}.push({item});")
                    else:
                        val = self._expr_to_rust(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"state.{target} = {val};")
                else:
                    # Component signal — use Dioxus Signal API
                    if op in ("increment", "INCREMENT"):
                        L.append(f"{target}.set({target}() + {amount});")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"{target}.set({target}() - {amount});")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"{target}.with_mut(|v| v.push({item}));")
                    else:
                        val = self._expr_to_rust(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"{target}.set({val});")
            elif kind in ("ExprStmt", "ExprStmtIR"):
                L.append(f"{self._expr_to_rust(self._get(stmt, 'expr'))};")
            elif kind in ("ReturnStmt", "ReturnStmtIR"):
                L.append(f"return {self._expr_to_rust(self._get(stmt, 'value'))};")
            elif kind in ("IfStmt", "IfStmtIR"):
                cond = self._expr_to_rust(self._get(stmt, "condition"))
                L.append(f"if {cond} {{")
                L.append(self.indent(self._gen_block_body_rs(self._get(stmt, "then_body"), signal_names)))
                if self._get(stmt, "else_body"):
                    L.append("} else {")
                    L.append(self.indent(self._gen_block_body_rs(self._get(stmt, "else_body"), signal_names)))
                L.append("}")
            elif kind in ("LetStmt", "LetStmtIR"):
                L.append(f"let {self._get(stmt, 'name', 'x')} = {self._expr_to_rust(self._get(stmt, 'value'))};")
        return "\n".join(L)

    def _gen_block_body_rs_hand(self, body) -> str:
        """Generate block body for inline event handlers."""
        if body is None:
            return ""
        stmts = self._get(body, "statements", []) if isinstance(body, dict) else []
        L = []
        for stmt in stmts:
            if not isinstance(stmt, dict):
                continue
            kind = self._get(stmt, "_kind", "")
            if kind == "MutationStmt" or "intent" in stmt:
                intent = self._get(stmt, "intent", {})
                target_ref = self._get(intent, "target", {})
                target = self._get(target_ref, "name", "x")
                target_owner = self._get(target_ref, "owner")
                op = self._get(intent, "operation", "direct_assign")
                item = self._expr_to_rust(self._get(intent, "item"))
                if target_owner:
                    # Store mutation in action — direct field access
                    if op in ("increment", "INCREMENT"):
                        L.append(f"state.{target} += 1;")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"state.{target} -= 1;")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"state.{target}.push({item});")
                    else:
                        val = self._expr_to_rust(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"state.{target} = {val};")
                else:
                    # Component signal — Dioxus Signal API
                    if op in ("increment", "INCREMENT"):
                        L.append(f"{target}.set({target}() + 1);")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"{target}.set({target}() - 1);")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"{target}.with_mut(|v| v.push({item}));")
                    else:
                        val = self._expr_to_rust(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"{target}.set({val});")
            elif kind in ("ExprStmt", "ExprStmtIR"):
                L.append(f"{self._expr_to_rust(self._get(stmt, 'expr'))};")
        return "\n".join(L)

    # ── Store Generation (GlobalSignal) ────────────────

    def _gen_store(self, store: dict, source_line: int = 1) -> str:
        name = self._get(store, "name", "Store")
        signals = self._get(store, "signals", {})
        getters = self._get(store, "getters", {})
        actions = self._get(store, "actions", [])
        defaults = self._get(store, "signal_defaults", {})

        L = []
        L.append("use dioxus::prelude::*;")
        L.append("")
        L.append(f"// .omni:{source_line} -- store {name}")

        # State struct
        L.append("#[derive(Clone, PartialEq)]")
        L.append(f"pub struct {name}State {{")
        for sig_name, sig_type in signals.items():
            rust_type = self._map_type_to_rust(sig_type)
            L.append(f"    pub {sig_name}: {rust_type},")
        L.append("}")
        L.append("")

        # Default impl
        L.append(f"impl Default for {name}State {{")
        L.append("    fn default() -> Self {")
        L.append("        Self {")
        for sig_name, sig_type in signals.items():
            if sig_name in defaults:
                init_val = self._expr_to_rust(defaults[sig_name])
            else:
                init_val = self._type_default_rust(sig_type)
            L.append(f"            {sig_name}: {init_val},")
        L.append("        }")
        L.append("    }")
        L.append("}")
        L.append("")

        # GlobalSignal
        L.append(f"pub static {name.upper()}: GlobalSignal<{name}State> = "
                  f"GlobalSignal::new(|| {name}State::default());")
        L.append("")

        # Getter functions
        self._in_store_action = True
        self._store_signal_names = set(signals.keys())
        for getter_name, getter_body in getters.items():
            body_str = self._expr_to_rust(getter_body)
            L.append(f"pub fn {getter_name}() -> {self._infer_return_type(getter_body, signals)} {{")
            L.append(f"    let state = {name.upper()}.read();")
            L.append(f"    {body_str}")
            L.append("}")
            L.append("")
        self._in_store_action = False

        # Action functions
        self._in_store_action = True
        for action in actions:
            aname = self._get(action, "name", "action")
            params = self._get(action, "params", [])
            pstr = ", ".join(
                f"{self._get(p, 'name', 'x')}: {self._map_type_to_rust(self._get(p, 'type'))}"
                for p in params
            )
            body = self._gen_block_body_rs(self._get(action, "body"), set(signals.keys()))
            L.append(f"pub fn {aname}({pstr}) {{")
            L.append(f"    let mut state = {name.upper()}.write();")
            L.append(self.indent(body))
            L.append("}")
            L.append("")
        self._in_store_action = False

        # Hook to use in components
        L.append(f"pub fn use_{name.lower()}() -> Signal<{name}State> {{")
        L.append(f"    {name.upper()}.into()")
        L.append("}")

        return "\n".join(L)

    # ── Helpers ────────────────────────────────────────

    def _map_type_to_rust(self, type_ir) -> str:
        if type_ir is None:
            return "()"
        if isinstance(type_ir, str):
            M = {"i32": "i32", "f64": "f64", "bool": "bool",
                 "String": "String", "void": "()", "Array": "Vec<String>", "Map": "std::collections::HashMap<String, String>"}
            return M.get(type_ir, type_ir)
        if isinstance(type_ir, dict):
            k = self._get(type_ir, "kind", self._get(type_ir, "name", ""))
            M = {"i32": "i32", "I32": "i32", "f64": "f64", "F64": "f64",
                 "bool": "bool", "BOOL": "bool", "String": "String", "STRING": "String",
                 "void": "()", "VOID": "()", "null": "()", "NULL": "()",
                 "Array": "Vec<String>", "ARRAY": "Vec<String>",
                 "Map": "std::collections::HashMap<String, String>", "MAP": "std::collections::HashMap<String, String>"}
            return M.get(k, k)
        return "()"

    def _type_default_rust(self, type_ir) -> str:
        if isinstance(type_ir, str):
            D = {"i32": "0", "f64": "0.0", "bool": "false", "String": 'String::new()',
                 "Array": "Vec::new()", "Map": "HashMap::new()", "void": "()"}
            return D.get(type_ir, "()")
        if isinstance(type_ir, dict):
            k = self._get(type_ir, "kind", self._get(type_ir, "name", ""))
            D = {"i32": "0", "I32": "0", "f64": "0.0", "F64": "0.0",
                 "bool": "false", "BOOL": "false", "String": 'String::new()', "STRING": 'String::new()',
                 "Array": "Vec::new()", "ARRAY": "Vec::new()",
                 "Map": "HashMap::new()", "MAP": "HashMap::new()"}
            return D.get(k, "()")
        return "()"

    def _infer_return_type(self, body, signals: dict) -> str:
        if isinstance(body, dict) and body.get("name"):
            sig_name = body.get("name")
            if sig_name in signals:
                return self._map_type_to_rust(signals[sig_name])
        return "()"

    def _layout_to_rsx(self, prop: str, val: str) -> str:
        M = {
            "direction": {"row": "flex-direction: row", "column": "flex-direction: column"},
            "gap": lambda v: f"gap: {v}px",
            "align": {"center": "align-items: center", "start": "align-items: flex-start"},
            "justify": {"center": "justify-content: center", "start": "justify-content: flex-start"},
            "padding": lambda v: f"padding: {v}px",
            "margin": lambda v: f"margin: {v}px",
            "flex": lambda v: f"flex: {v}",
        }
        m = M.get(prop)
        if m is None:
            return ""
        if callable(m):
            return m(val)
        if isinstance(m, dict):
            return m.get(val, "")
        return ""

    @staticmethod
    def _extract_style_val(value) -> str:
        raw = str(value.get("value", value)) if isinstance(value, dict) else str(value)
        for unit in ("px", "rem", "em", "vh", "vw", "%"):
            if raw.endswith(unit):
                return raw[:-len(unit)]
        return raw

    @staticmethod
    def _escape_rsx(text: str) -> str:
        """Escape text for rsx! content. Text nodes need quoting."""
        if text is None:
            return ""
        # Strip and check if it's pure whitespace
        stripped = text.strip()
        if not stripped:
            return ""
        # Quote text content — rsx! needs string literals for text
        escaped = stripped.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'