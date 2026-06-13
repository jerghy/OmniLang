"""Vue 3 L2 codegen — Intent IR → idiomatic Vue 3 SFC + Pinia.

Generates L2 (idiomatic) Vue 3 code from OmniView Intent IR.
Target: Vue 3.4+ with <script setup>, Composition API, Pinia, <style scoped>.
"""

from omniview.codegen.base import CodegenBase, CodegenResult


# ── Element Mapping ───────────────────────────────────

VUE_ELEMENT_MAP: dict[str, str] = {
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

EVENT_MAP = {
    "onPress": "click", "onChange": "change",
    "onSubmit": "submit", "onFocus": "focus", "onBlur": "blur",
    "onHover": "mouseenter", "onLeave": "mouseleave",
    "onKeyDown": "keydown", "onKeyUp": "keyup",
}


class VueCodegen(CodegenBase):
    """Generate L2 idiomatic Vue 3 SFC from OmniView IR."""

    target = "vue"
    tier = "L2"
    ELEMENT_MAP = VUE_ELEMENT_MAP

    def __init__(self):
        super().__init__()
        self._in_store_action = False
        self._store_signal_names: set = set()
        self._store_name = ""
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
            sfc = self._gen_component(comp_ir, store_irs or {}, source_line)
            name = self._get(comp_ir, "name", "Component")
            result.files[f"{name}.vue"] = sfc
        except Exception as e:
            result.errors.append(f"Vue codegen failed: {e}")
        return result

    def generate_store(self, store_ir, source_line: int = 1) -> CodegenResult:
        result = CodegenResult()
        try:
            name = self._get(store_ir, "name", "Store")
            ts = self._gen_store(store_ir, source_line)
            result.files[f"stores/use{name}.ts"] = ts
        except Exception as e:
            result.errors.append(f"Pinia codegen failed: {e}")
        return result

    # ── Component Generation ───────────────────────────

    def _gen_component(self, comp: dict, stores: dict, source_line: int = 1) -> str:
        name = self._get(comp, "name", "Component")
        store_refs = set(self._get(comp, "store_refs", []))
        if not store_refs:
            store_refs = self._detect_store_refs(comp)

        signal_names = {s.get("name", "") for s in self._get(comp, "states", [])}
        imports = self._gen_script_imports(comp, store_refs)
        script = self._gen_script_setup(comp, stores, store_refs, signal_names, source_line)
        template = self._gen_template(comp, source_line)
        style = self._gen_style(comp)

        parts = []
        if imports or script:
            parts.append(f'<script setup lang="ts">\n{imports}\n{script}\n</script>')
        if template:
            parts.append(template)
        if style:
            parts.append(style)
        return "\n\n".join(parts)

    def _gen_script_imports(self, comp: dict, store_refs: set) -> str:
        L = []
        hooks = set()
        if self._get(comp, "states"):
            hooks.add("ref")
        if self._get(comp, "computeds"):
            hooks.add("computed")
        if self._get(comp, "effects"):
            hooks.add("watch")
        if self._get(comp, "lifecycles"):
            for lc in self._get(comp, "lifecycles", []):
                kind = self._get(lc, "kind", "mount")
                hooks.add("onMounted" if kind == "mount" else "onUnmounted")
        if hooks:
            L.append(f'import {{ {", ".join(sorted(hooks))} }} from "vue";')

        if store_refs:
            for sname in store_refs:
                L.append(f'import {{ use{sname} }} from "./stores/use{sname}";')
        return "\n".join(L)

    def _gen_script_setup(self, comp: dict, stores: dict, store_refs: set, signal_names: set, source_line: int) -> str:
        L = []
        name = self._get(comp, "name", "Component")
        L.append(f"// .omni:{source_line} -- component {name}")

        # Props
        prop_defs = self._gen_props(comp)
        if prop_defs:
            L.append(prop_defs)

        # Store instances — idiomatic naming: single store → "store", multi → "storeName"
        self._store_getter_fields = set()
        self._store_var_names: dict[str, str] = {}  # store_name → variable_name
        if store_refs:
            srefs = sorted(store_refs)
            if len(srefs) == 1:
                self._store_var_names[srefs[0]] = "store"
            else:
                for sname in srefs:
                    self._store_var_names[sname] = sname[0].lower() + sname[1:] + "Store"
            for sname in srefs:
                var_name = self._store_var_names[sname]
                L.append(f"const {var_name} = use{sname}();")
                if sname in self._all_stores:
                    for gname in self._get(self._all_stores[sname], "getters", {}):
                        self._store_getter_fields.add(gname)

        L.append("")

        # State (signal → ref)
        for state in self._get(comp, "states", []):
            sig_name = self._get(state, "name", "value")
            init_val = self._expr_to_vue(self._get(state, "initial_value"))
            ts_type = self._map_type_to_ts(self._get(state, "type"))
            L.append(f"const {sig_name} = ref<{ts_type}>({init_val});")

        # Computed
        for computed in self._get(comp, "computeds", []):
            cname = self._get(computed, "name", "value")
            body = self._expr_to_vue(self._get(computed, "body"))
            L.append(f"const {cname} = computed(() => {body});")

        # Lifecycles
        for lc in self._get(comp, "lifecycles", []):
            kind = self._get(lc, "kind", "mount")
            body = self._gen_block_body_vue(self._get(lc, "body"), signal_names)
            if kind == "mount" and body.strip():
                L.append(f"onMounted(() => {{")
                L.append(self.indent(body))
                L.append("});")
            elif kind == "unmount" and body.strip():
                L.append(f"onUnmounted(() => {{")
                L.append(self.indent(body))
                L.append("});")

        # Effects
        for eff in self._get(comp, "effects", []):
            body = self._gen_block_body_vue(self._get(eff, "body"), signal_names)
            deps = self._get(eff, "deps", [])
            deps_str = ", ".join(f"() => {d}.value" for d in deps) if deps else ""
            L.append(f"watch([{deps_str}], () => {{")
            L.append(self.indent(body))
            L.append("});")

        # Functions
        for fn in self._get(comp, "fns", []):
            fname = self._get(fn, "name", "handler")
            params = self._get(fn, "params", [])
            param_str = ", ".join(f"{self._get(p, 'name', 'x')}: {self._map_type_to_ts(self._get(p, 'type'))}" for p in params)
            body = self._gen_block_body_vue(self._get(fn, "body"), signal_names)
            L.append(f"function {fname}({param_str}) {{")
            L.append(self.indent(body))
            L.append("}")

        return "\n".join(L)

    def _gen_props(self, comp: dict) -> str:
        props = self._get(comp, "props", {})
        if not props:
            return ""
        L = ["const props = defineProps<{"]
        for prop_name, prop_type in props.items():
            ts_type = self._map_type_to_ts(prop_type)
            L.append(f"  {prop_name}: {ts_type};")
        L.append("}>();")
        return "\n".join(L)

    # ── Template Generation ────────────────────────────

    def _gen_template(self, comp: dict, source_line: int) -> str:
        view = self._get(comp, "view")
        if not view:
            return ""
        styles = self._get(comp, "styles", [])
        inner = self._gen_view_tree(view, styles)
        return f"<template>\n{inner}\n</template>"

    def _gen_view_tree(self, view, styles: list) -> str:
        if view is None:
            return ""
        if isinstance(view, str):
            return self._escape_template(view)
        if isinstance(view, list):
            return "\n".join(self._gen_view_tree(c, styles) for c in view)
        if isinstance(view, dict):
            kind = self._get(view, "_kind", "")
            if kind == "Interpolation":
                return "{{ " + self._expr_to_vue(self._get(view, "expr")) + " }}"
            if kind == "ListRenderingIntent" or "iterable" in view:
                return self._gen_v_for(view, styles)
            if kind == "ConditionalIntent" or "branches" in view:
                return self._gen_v_if(view, styles)
            if kind == "SlotIntent":
                return "<slot />" if not view.get("name") else f'<slot name="{view.get("name")}" />'
            if kind == "ErrorBoundary":
                return self._gen_error_boundary(view, styles)
            if "tag" in view:
                return self._gen_element(view, styles)
        return ""

    def _gen_error_boundary(self, boundary: dict, styles: list) -> str:
        """Generate Vue error boundary using onErrorCaptured hook."""
        name = self._get(boundary, "name", "error")
        children = self._get(boundary, "children", [])
        child_html = "\n".join(self._gen_view_tree(c, styles) for c in children)
        # Vue 3: use onErrorCaptured + v-if error state
        # For simplicity, emit a container div with the error boundary name as data attr
        return (
            f'<div data-error-boundary="{name}">\n'
            f"{child_html}\n"
            f"</div>"
        )

    def _gen_element(self, elem: dict, styles: list) -> str:
        tag = self._get(elem, "tag", "div")
        if tag == "Heading":
            level_prop = self._get(elem, "props", {}).get("level", {})
            level = self._get(level_prop, "value", "2")
            html_tag = f"h{level}"
        else:
            html_tag = self.ELEMENT_MAP.get(tag, tag)

        if tag == "Spacer":
            return '<div style="flex: 1"></div>'

        props = self._get(elem, "props", {})
        bindings = self._get(elem, "bindings", {})
        events = self._get(elem, "events", {})
        children = self._get(elem, "children", [])
        test_id = self._get(elem, "test_id")

        attrs = []

        # A11y prop mapping: omni → HTML
        A11Y_MAP = {"accessibilityLabel": "aria-label", "accessibilityRole": "role"}

        # Regular props (skip layout props — they go to inline style)
        for pname, pval in props.items():
            if pname in ("class", "key", "level") or pname in LAYOUT_PROPS:
                continue
            html_name = A11Y_MAP.get(pname, pname)
            js_val = self._expr_to_vue(pval)
            if js_val.startswith('"') and js_val.endswith('"'):
                attrs.append(f'{html_name}={js_val}')
            else:
                attrs.append(f':{html_name}="{js_val}"')

        # Layout props → inline style
        layout_style = {}
        for pname, pval in props.items():
            if pname in LAYOUT_PROPS:
                raw = self._extract_style_val(pval)
                css = self._layout_to_css(pname, raw)
                if ":" in css:
                    k, v = css.split(":", 1)
                    layout_style[k.strip()] = v.strip().rstrip(";")

        if layout_style or tag in ("Flex", "Grid"):
            if tag == "Flex" and "display" not in layout_style:
                layout_style["display"] = "flex"
            if tag == "Grid":
                layout_style["display"] = "grid"
                layout_style["grid-template-columns"] = "repeat(auto-fill, minmax(250px, 1fr))"
                if "gap" not in layout_style:
                    layout_style["gap"] = "16px"
            style_str = "; ".join(f"{k}: {v}" for k, v in layout_style.items())
            attrs.append(f'style="{style_str}"')

        # CSS class from style block
        # Priority: 1) test-id exact  2) test-id hyphen→underscore  3) tag (skip "container")
        if styles:
            style_names = [r.get("class_name", "") for r in styles if isinstance(r, dict)]
            tag_lower = tag.lower()
            matched = None
            if test_id and test_id in style_names:
                matched = test_id
            if not matched and test_id:
                test_id_underscored = test_id.replace("-", "_")
                if test_id_underscored in style_names:
                    matched = test_id_underscored
            if not matched and tag_lower in style_names and tag_lower != "container":
                matched = tag_lower
            if matched:
                attrs.append(f'class="{matched}"')

        # v-model for bindings
        for bname, bref in bindings.items():
            sig_name = self._get(bref, "name", "")
            attrs.append(f'v-model="{sig_name}"')
            if bname == "checked" and html_tag == "input":
                attrs.append('type="checkbox"')

        # Events → @event
        for ename, ehandler in events.items():
            vue_event = EVENT_MAP.get(ename, ename.lower())
            handler = self._gen_event_handler(ehandler)
            if "(" in handler or "=>" in handler:
                attrs.append(f'@{vue_event}="{handler}"')
            else:
                attrs.append(f'@{vue_event}="{handler}"')

        if test_id:
            attrs.append(f'data-testid="{test_id}"')

        attr_str = " ".join(attrs)
        tag_open = f"<{html_tag}" + (f" {attr_str}" if attr_str else "") + ">"

        if html_tag in ("img", "hr", "input", "br") and not children:
            return f"<{html_tag} {attr_str} />"

        if not children:
            return f"{tag_open}</{html_tag}>"

        child_strs = [self._gen_view_tree(c, styles) for c in children]
        inner = "\n".join(child_strs)
        if "\n" in inner:
            return f"{tag_open}\n{inner}\n</{html_tag}>"
        return f"{tag_open}{inner}</{html_tag}>"

    def _gen_v_if(self, cond: dict, styles: list) -> str:
        branches = self._get(cond, "branches", [])
        fallback = self._get(cond, "fallback")
        L = []
        for i, branch in enumerate(branches):
            c = self._expr_to_vue(self._get(branch, "condition"))
            body = self._gen_view_tree(self._get(branch, "body"), styles)
            directive = "v-if" if i == 0 else "v-else-if"
            L.append(f'<template {directive}="{c}">')
            L.append(body)
            L.append("</template>")
        if fallback:
            fb = self._gen_view_tree(fallback, styles)
            L.append("<template v-else>")
            L.append(fb)
            L.append("</template>")
        return "\n".join(L)

    def _gen_v_for(self, lst: dict, styles: list) -> str:
        iterable = self._expr_to_vue(self._get(lst, "iterable"))
        binding = self._get(lst, "item_binding", {})
        item_name = self._get(binding, "name", "item")
        body = self._gen_view_tree(self._get(lst, "body"), styles)
        key_expr = self._expr_to_vue(self._get(lst, "key_expr"))
        empty_body = self._get(lst, "empty_body")

        if empty_body:
            empty = self._gen_view_tree(empty_body, styles)
            return (
                f'<template v-if="{iterable}.length === 0">\n{empty}\n</template>\n'
                f'<template v-else>\n'
                f'<template v-for="{item_name} in {iterable}" :key="{key_expr}">\n{body}\n</template>\n'
                f'</template>'
            )
        return f'<template v-for="{item_name} in {iterable}" :key="{key_expr}">\n{body}\n</template>'

    # ── Event Handlers ─────────────────────────────────

    def _gen_event_handler(self, handler: dict) -> str:
        kind = self._get(handler, "kind", "inline_fn")
        if kind in ("fn_ref", "FN_REF"):
            return self._get(handler, "ref", "() => {}")
        if kind in ("store_action", "STORE_ACTION"):
            ref = self._get(handler, "ref", "")
            parts = ref.split(".")
            if len(parts) == 2:
                alias, action = parts
                actual_store = self._resolve_store_name(alias)
                return f"{self._store_var(actual_store)}.{action}"
            return ref
        # Inline function
        params = self._get(handler, "params", [])
        param_str = ", ".join(self._get(p, "name", "e") for p in params)
        body = self._gen_block_body_vue_hand(self._get(handler, "body"))
        if body.strip():
            return f"({param_str}) => {{\n{self.indent(body)}\n}}"
        return f"({param_str}) => {{}}"

    # ── Expression → Vue ───────────────────────────────

    def _expr_to_vue(self, expr) -> str:
        if expr is None:
            return "undefined"
        if isinstance(expr, (int, float)):
            return str(expr)
        if isinstance(expr, bool):
            return "true" if expr else "false"
        if isinstance(expr, str):
            return expr
        if isinstance(expr, dict):
            kind = self._get(expr, "_kind", "")
            if kind == "StoreFieldAccessExpr" or ("store_alias" in expr and "field_name" in expr):
                store_alias = self._get(expr, "store_alias", "store")
                field = self._get(expr, "field_name", "")
                args = self._get(expr, "call_args", [])
                actual_store = self._resolve_store_name(store_alias)
                args_str = ", ".join(self._expr_to_vue(a) for a in args)
                return f"{self._store_var(actual_store)}.{field}({args_str})"
            if kind == "FieldAccessExpr":
                obj = self._get(expr, "object")
                field = self._get(expr, "field", "")
                if isinstance(obj, dict) and self._get(obj, "name") == "store":
                    actual_store = self._resolve_store_name("store")
                    if field in self._store_getter_fields:
                        return f"{self._store_var(actual_store)}.{field}"
                    return f"{self._store_var(actual_store)}.{field}"
                obj_str = self._expr_to_vue(obj)
                return f"{obj_str}.{field}"
            if kind == "ArrayExpr" or ("elements" in expr and "_kind" not in expr):
                parts = []
                for elem in self._get(expr, "elements", []):
                    if isinstance(elem, dict) and elem.get("_kind") == "SpreadExpr":
                        parts.append("..." + self._expr_to_vue(elem.get("expr")))
                    else:
                        parts.append(self._expr_to_vue(elem))
                return "[" + ", ".join(parts) + "]"
            if kind == "SpreadExpr":
                return "..." + self._expr_to_vue(self._get(expr, "expr"))
            if kind == "FormatCall":
                return self._gen_format_call(expr)
            if kind == "TranslateCall":
                key = self._get(expr, "key", "unknown")
                return f"$t('{key}')"
            if kind == "IdentifierExpr" or ("name" in expr and "op" not in expr and "value" not in expr and len(expr) <= 2):
                name = self._get(expr, "name", "undefined")
                # In store action context, ref values need .value suffix
                if self._in_store_action and hasattr(self, '_store_signal_names') and name in self._store_signal_names:
                    return f"{name}.value"
                return name
            if kind == "LiteralExpr" or ("value" in expr and "op" not in expr and "name" not in expr):
                val = self._get(expr, "value")
                if isinstance(val, str):
                    if val in ("[]", "{}") or val.startswith("[") or val.startswith("{"):
                        return val
                    return f'"{val}"'
                if isinstance(val, bool):
                    return "true" if val else "false"
                return str(val) if val is not None else "undefined"
            if kind == "BinaryOpExpr" or ("op" in expr and "left" in expr):
                left = self._expr_to_vue(self._get(expr, "left"))
                right = self._expr_to_vue(self._get(expr, "right"))
                op_map = {"add": "+", "sub": "-", "mul": "*", "div": "/", "mod": "%",
                          "eq": "===", "neq": "!==", "lt": "<", "gt": ">",
                          "lte": "<=", "gte": ">=", "and": "&&", "or": "||",
                          "+": "+", "-": "-", "*": "*", "/": "/", "%": "%",
                          "==": "===", "!=": "!==", "<": "<", ">": ">",
                          "<=": "<=", ">=": ">=", "&&": "&&", "||": "||"}
                op = op_map.get(self._get(expr, "op", "+"), self._get(expr, "op", "+"))
                return f"({left} {op} {right})"
            if kind == "UnaryOpExpr":
                op_raw = self._get(expr, "op", "!")
                op = "!" if op_raw in ("!", "not", "NOT") else "-"
                return f"{op}{self._expr_to_vue(self._get(expr, 'operand'))}"
            if kind == "TernaryIR":
                c = self._expr_to_vue(self._get(expr, "cond"))
                t = self._expr_to_vue(self._get(expr, "then_expr"))
                e = self._expr_to_vue(self._get(expr, "else_expr"))
                return f"({c} ? {t} : {e})"
            if "name" in expr:
                return self._get(expr, "name", "undefined")
        return "undefined"

    def _gen_format_call(self, expr: dict) -> str:
        func = self._get(expr, "func", "")
        args = [self._expr_to_vue(a) for a in self._get(expr, "args", [])]
        if func == "format_date":
            val = args[0] if args else "undefined"
            return f"new Date({val}).toLocaleDateString()"
        elif func == "format_number":
            val = args[0] if args else "0"
            decimals = args[1] if len(args) > 1 else "2"
            return f"({val}).toFixed({decimals})"
        return "undefined"

    # ── Statement Blocks (Vue) ─────────────────────────

    def _gen_block_body_vue(self, body, signal_names: set) -> str:
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
                amount = self._expr_to_vue(self._get(intent, "amount", 1))
                item = self._expr_to_vue(self._get(intent, "item"))

                if target_owner:
                    # Pinia store mutation
                    if op in ("increment", "INCREMENT"):
                        L.append(f"{target}.value += {amount};")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"{target}.value -= {amount};")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"{target}.value = [...{target}.value, {item}];")
                    else:
                        val = self._expr_to_vue(self._get(intent, "item", self._get(intent, "amount")))
                        if val.startswith('"[') and val.endswith(']"'):
                            val = val[1:-1]
                        L.append(f"{target}.value = {val};")
                else:
                    # Component ref mutation
                    if op in ("increment", "INCREMENT"):
                        L.append(f"{target}.value += {amount};")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"{target}.value -= {amount};")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"{target}.value = [...{target}.value, {item}];")
                    else:
                        val = self._expr_to_vue(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"{target}.value = {val};")
            elif kind in ("ExprStmt", "ExprStmtIR"):
                L.append(f"{self._expr_to_vue(self._get(stmt, 'expr'))};")
            elif kind in ("EventEmitStmt", "EventEmitStmtIR"):
                args = ", ".join(self._expr_to_vue(a) for a in self._get(stmt, "args", []))
                L.append(f"emit('{self._get(stmt, 'event_name', '')}', {args});")
            elif kind in ("ReturnStmt", "ReturnStmtIR"):
                L.append(f"return {self._expr_to_vue(self._get(stmt, 'value'))};")
            elif kind in ("IfStmt", "IfStmtIR"):
                cond = self._expr_to_vue(self._get(stmt, "condition"))
                L.append(f"if ({cond}) {{")
                L.append(self.indent(self._gen_block_body_vue(self._get(stmt, "then_body"), signal_names)))
                if self._get(stmt, "else_body"):
                    L.append("} else {")
                    L.append(self.indent(self._gen_block_body_vue(self._get(stmt, "else_body"), signal_names)))
                L.append("}")
            elif kind in ("LetStmt", "LetStmtIR"):
                L.append(f"const {self._get(stmt, 'name', 'x')} = {self._expr_to_vue(self._get(stmt, 'value'))};")
        return "\n".join(L)

    def _gen_block_body_vue_hand(self, body) -> str:
        """Generate block body for inline event handlers (no .value needed)."""
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
                op = self._get(intent, "operation", "direct_assign")
                amount = self._expr_to_vue(self._get(intent, "amount", 1))
                item = self._expr_to_vue(self._get(intent, "item"))
                if op in ("increment", "INCREMENT"):
                    L.append(f"{target}.value += {amount};")
                elif op in ("decrement", "DECREMENT"):
                    L.append(f"{target}.value -= {amount};")
                elif op in ("spread_append", "SPREAD_APPEND"):
                    L.append(f"{target}.value = [...{target}.value, {item}];")
                else:
                    val = self._expr_to_vue(self._get(intent, "item", self._get(intent, "amount")))
                    L.append(f"{target}.value = {val};")
            elif kind in ("ExprStmt", "ExprStmtIR"):
                L.append(f"{self._expr_to_vue(self._get(stmt, 'expr'))};")
        return "\n".join(L)

    # ── Style Generation ───────────────────────────────

    def _gen_style(self, comp: dict) -> str:
        styles = self._get(comp, "styles", [])
        if not styles:
            return ""
        L = ['<style scoped>']
        for rule in styles:
            cname = self._get(rule, "class_name", "")
            props = self._get(rule, "properties", {})
            L.append(f".{cname} {{")
            for prop, value in props.items():
                css_line = self._style_to_css(prop, value)
                if css_line:
                    for part in css_line.rstrip(";").split(";"):
                        part = part.strip()
                        if part:
                            L.append(f"  {part};")
            L.append("}")
            L.append("")
        L.append("</style>")
        return "\n".join(L)

    def _style_to_css(self, prop: str, value) -> str:
        val_str = self._extract_style_val(value)
        if val_str.startswith('"') and val_str.endswith('"'):
            val_str = val_str[1:-1]
        css_prop = prop.replace("_", "-")
        from omniview.codegen.css_mapping import lookup_css
        result = lookup_css(prop, val_str)
        if result is not None:
            return result.rstrip(";") + ";" if result and not result.endswith(";") else result
        return f"{css_prop}: {val_str};"

    @staticmethod
    def _extract_style_val(value) -> str:
        raw = str(value.get("value", value)) if isinstance(value, dict) else str(value)
        for unit in ("px", "rem", "em", "vh", "vw", "%"):
            if raw.endswith(unit):
                return raw[:-len(unit)]
        return raw

    # ── Pinia Store Generation ─────────────────────────

    def _gen_store(self, store: dict, source_line: int = 1) -> str:
        name = self._get(store, "name", "Store")
        signals = self._get(store, "signals", {})
        getters = self._get(store, "getters", {})
        actions = self._get(store, "actions", [])
        defaults = self._get(store, "signal_defaults", {})

        L = []
        L.append('import { ref, computed } from "vue";')
        L.append('import { defineStore } from "pinia";')
        L.append("")
        L.append(f"// .omni:{source_line} -- store {name}")

        # State interface
        L.append(f"interface {name}State {{")
        for sig_name, sig_type in signals.items():
            L.append(f"  {sig_name}: {self._map_type_to_ts(sig_type)};")
        L.append("}")
        L.append("")

        L.append(f"export const use{name} = defineStore('{name}', () => {{")

        # Signals → ref()
        for sig_name, sig_type in signals.items():
            if sig_name in defaults:
                init_val = self._expr_to_vue(defaults[sig_name])
                L.append(f"  const {sig_name} = ref<{self._map_type_to_ts(sig_type)}>({init_val});")
            else:
                L.append(f"  const {sig_name} = ref<{self._map_type_to_ts(sig_type)}>({self._type_default(sig_type)});")

        # Getters → computed
        for getter_name, getter_body in getters.items():
            body_str = self._expr_to_vue(getter_body)
            if isinstance(getter_body, dict) and getter_body.get("_kind") == "IdentifierExpr":
                L.append(f"  const {getter_name} = computed(() => {body_str}.value);")
            else:
                L.append(f"  const {getter_name} = computed(() => {body_str});")

        # Actions → functions
        self._in_store_action = True
        self._store_signal_names = set(signals.keys())
        for action in actions:
            aname = self._get(action, "name", "action")
            params = self._get(action, "params", [])
            pstr = ", ".join(f"{self._get(p, 'name', 'x')}: {self._map_type_to_ts(self._get(p, 'type'))}" for p in params)
            body = self._gen_block_body_vue(self._get(action, "body"), set(signals.keys()))
            L.append(f"  function {aname}({pstr}) {{")
            L.append(self.indent(body))
            L.append("  }")
        self._in_store_action = False

        # Return all public members
        all_names = list(signals.keys()) + list(getters.keys()) + [self._get(a, "name", "") for a in actions]
        L.append(f"  return {{ {', '.join(all_names)} }};")
        L.append("});")
        return "\n".join(L)

    # ── Helpers ────────────────────────────────────────

    def _resolve_store_name(self, alias: str) -> str:
        """Get the actual store name from alias."""
        for sname, sir in self._all_stores.items():
            if "signals" in sir and "view" not in sir:
                return sname
        return alias.capitalize()

    def _store_var(self, store_name: str) -> str:
        """Get the Vue variable name for a store instance."""
        if hasattr(self, '_store_var_names') and store_name in self._store_var_names:
            return self._store_var_names[store_name]
        # Fallback: use "store" for first store
        return "store"

    def _detect_store_refs(self, comp: dict) -> set:
        refs = set()
        view = self._get(comp, "view")
        if view:
            self._scan_store_refs(view, refs)
        return refs

    def _scan_store_refs(self, node, refs: set):
        if isinstance(node, str) or node is None:
            return
        if isinstance(node, list):
            for child in node:
                self._scan_store_refs(child, refs)
        if isinstance(node, dict):
            for key, val in node.items():
                if key in ("object",) and isinstance(val, dict):
                    if self._get(val, "name") == "store":
                        refs.add(self._resolve_store_name("store"))
                self._scan_store_refs(val, refs)
            if "children" in node:
                self._scan_store_refs(node["children"], refs)
            if "body" in node:
                self._scan_store_refs(node["body"], refs)
            if "branches" in node:
                self._scan_store_refs(node["branches"], refs)

    def _map_type_to_ts(self, type_ir) -> str:
        if type_ir is None:
            return "any"
        if isinstance(type_ir, str):
            M = {"i32": "number", "f64": "number", "bool": "boolean",
                 "String": "string", "void": "void", "null": "null",
                 "Array": "any[]", "Map": "Record<string, any>"}
            return M.get(type_ir, type_ir)
        if isinstance(type_ir, dict):
            k = self._get(type_ir, "kind", self._get(type_ir, "name", "any"))
            M = {"i32": "number", "I32": "number", "f64": "number", "F64": "number",
                 "bool": "boolean", "BOOL": "boolean", "String": "string", "STRING": "string",
                 "void": "void", "VOID": "void", "null": "null", "NULL": "null",
                 "Array": "any[]", "ARRAY": "any[]", "Map": "Record<string, any>", "MAP": "Record<string, any>"}
            return M.get(k, k)
        return "any"

    def _type_default(self, type_ir) -> str:
        if isinstance(type_ir, str):
            D = {"i32": "0", "f64": "0", "bool": "false", "String": '""', "Array": "[]", "Map": "{}"}
            return D.get(type_ir, "undefined")
        if isinstance(type_ir, dict):
            k = self._get(type_ir, "kind", self._get(type_ir, "name", ""))
            D = {"i32": "0", "I32": "0", "f64": "0", "F64": "0", "bool": "false", "BOOL": "false",
                 "String": '""', "STRING": '""', "Array": "[]", "ARRAY": "[]", "Map": "{}", "MAP": "{}"}
            return D.get(k, "undefined")
        return "undefined"

    def _layout_to_css(self, prop: str, val: str) -> str:
        M = {
            "direction": {"row": "flex-direction: row", "column": "flex-direction: column"},
            "gap": lambda v: f"gap: {v}px",
            "align": {"center": "align-items: center", "start": "align-items: flex-start",
                      "end": "align-items: flex-end"},
            "justify": {"center": "justify-content: center",
                        "space-between": "justify-content: space-between"},
            "padding": lambda v: f"padding: {v}px",
            "margin": lambda v: f"margin: {v}px",
        }
        m = M.get(prop)
        if m is None:
            return f"{prop}: {val}"
        if callable(m):
            return m(val)
        if isinstance(m, dict):
            return m.get(val, f"{prop}: {val}")
        return f"{prop}: {val}"

    @staticmethod
    def _escape_template(text: str) -> str:
        if text is None:
            return ""
        return text.replace("<", "&lt;").replace(">", "&gt;").replace("{", "&#123;").replace("}", "&#125;")