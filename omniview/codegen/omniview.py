"""
OmniView Codegen — generates pure JavaScript using the OmniView signal-based runtime.

Maps Intent IR to OmniView runtime API:
  signal(init, meta)    ← StateIntent
  computed(fn, meta)    ← ComputedIntent
  effect(fn)            ← EffectIntent / LifecycleIntent
  store(name, fn)       ← StoreIR
  h(tag, attrs, kids)   ← ElementIR / ViewTree
  render(vnode, el)     ← root mount
  bindTextContent       ← reactive text binding
  renderConditional     ← ConditionalIntent
  renderList            ← ListRenderingIntent

No React, no JSX, no VDOM. Direct signal→DOM binding via subscribe().
"""

from __future__ import annotations

from omniview.codegen.base import CodegenBase, CodegenResult


class OmniViewCodegen(CodegenBase):
    """Generate pure-JS OmniView runtime code from Intent IR dicts."""

    target = "omniview"
    tier = "L2"

    # Tag mapping: OmniView element → HTML tag
    ELEMENT_MAP = {
        "Container": "div",
        "Flex": "div",
        "Grid": "div",
        "Text": "span",
        "Heading": None,  # resolved by level prop → h1-h6
        "Button": "button",
        "Input": "input",
        "Image": "img",
        "Card": "div",
        "Spacer": "div",
        "List": "ul",
        "ListItem": "li",
        "Link": "a",
        "Checkbox": "input",
        "TextArea": "textarea",
        "Select": "select",
        "Option": "option",
    }

    def __init__(self, debug_mode: bool = True):
        super().__init__()
        self.debug_mode = debug_mode
        self._import_set: set[str] = set()
        self._in_store_action = False
        self._store_name = ""
        self._fn_trace_id = ""  # current function's trace ID for per-statement tracing

    # ── Shared helpers ─────────────────────────────────

    @staticmethod
    def _get(obj, key, default=None):
        """Safe access for both dict and object attributes."""
        if isinstance(obj, dict):
            return obj.get(key, default)
        return getattr(obj, key, default)

    @staticmethod
    def _pascal(s: str) -> str:
        """Convert snake_case to PascalCase."""
        return "".join(part.capitalize() for part in s.split("_"))

    @staticmethod
    def _camel(s: str) -> str:
        """Convert snake_case or kebab-case to camelCase."""
        if "_" in s:
            parts = s.split("_")
            return parts[0] + "".join(p.capitalize() for p in parts[1:])
        if "-" in s:
            parts = s.split("-")
            return parts[0] + "".join(p.capitalize() for p in parts[1:])
        return s

    # ── Public API ─────────────────────────────────────────

    def generate_component(
        self,
        comp_ir: dict,
        store_irs: dict | None = None,
        source_line: int = 1,
    ) -> CodegenResult:
        """Generate .js file for a component."""
        result = CodegenResult()
        try:
            store_irs = store_irs or {}
            self._import_set = set()
            js = self._gen_component(comp_ir, store_irs, source_line)
            name = self._get(comp_ir, "name", "Component")
            result.files[f"{name}.js"] = js
        except Exception as exc:
            result.errors.append(f"Codegen error: {exc}")
        return result

    def generate_store(
        self,
        store_ir: dict,
        source_line: int = 1,
    ) -> CodegenResult:
        """Generate .js file for a store."""
        result = CodegenResult()
        try:
            self._import_set = set()
            js = self._gen_store(store_ir, source_line)
            name = self._get(store_ir, "name", "Store")
            result.files[f"{name}.js"] = js
        except Exception as exc:
            result.errors.append(f"Store codegen error: {exc}")
        return result

    # ── Component Generation ───────────────────────────────

    def _gen_component(self, comp: dict, store_irs: dict, source_line: int) -> str:
        name = self._get(comp, "name", "Component")
        L: list[str] = []

        # ---- imports ----
        states = self._get(comp, "states", [])
        computeds = self._get(comp, "computeds", [])
        effects = self._get(comp, "effects", [])
        lifecycles = self._get(comp, "lifecycles", [])
        fns = self._get(comp, "fns", [])
        store_refs = self._get(comp, "store_refs", [])
        view = self._get(comp, "view")
        styles = self._get(comp, "styles", [])

        needs = set()
        if states:
            needs.add("signal")
        if computeds:
            needs.add("computed")
        if effects or lifecycles:
            needs.add("effect")
            needs.add("onCleanup")
        if store_refs:
            needs.add("signal")  # store fields are signals
        if view is not None:
            needs.add("h")
            needs.add("render")
            needs.add("bindTextContent")
            needs.add("dispose")
            needs.add("onCleanup")
            # Check for conditionals and lists in view tree
            if self._has_conditionals(view):
                needs.add("renderConditional")
            if self._has_lists(view):
                needs.add("renderList")
        if fns:
            needs.add("traceExecution")

        # Emit imports
        IMPORT_MAP = {
            "signal": 'import { signal } from "./omniview-runtime";',
            "computed": 'import { computed } from "./omniview-runtime";',
            "effect": 'import { effect } from "./omniview-runtime";',
            "onCleanup": 'import { onCleanup } from "./omniview-runtime";',
            "dispose": 'import { dispose } from "./omniview-runtime";',
            "h": 'import { h } from "./omniview-runtime";',
            "render": 'import { render } from "./omniview-runtime";',
            "bindTextContent": 'import { bindTextContent, bindProperty, bindAttribute } from "./omniview-runtime";',
            "renderConditional": 'import { renderConditional } from "./omniview-runtime";',
            "renderList": 'import { renderList } from "./omniview-runtime";',
            "traceExecution": 'import { traceExecution, withTrace } from "./omniview-runtime";',
        }
        for key in sorted(needs):
            imp = IMPORT_MAP.get(key)
            if imp:
                L.append(imp)

        # ---- store imports ----
        for sref in store_refs:
            L.append(f'import {{ {sref} }} from "./{sref}.js";')

        L.append("")
        L.append(f"// Component: {name}")
        L.append("")

        # ---- exported mount function ----
        props_list = self._get_prop_names(comp)
        if props_list:
            props_param = "{ " + ", ".join(props_list) + " }"
        else:
            props_param = ""

        L.append(f"/** Mount {name} into a container element. */")
        if props_param:
            L.append(f"export function {name}(mountEl, {props_param}) {{")
        else:
            L.append(f"export function {name}(mountEl) {{")

        # ---- stores: import store references ----
        for sref in store_refs:
            L.append(self.indent(f"// Store: {sref} (auto-unwrapped via Proxy)"))

        # ---- signals ----
        for state in states:
            sig_name = self._get(state, "name", "x")
            sig_type = self._format_type(self._get(state, "type"))
            init_val = self._expr_to_js(self._get(state, "initial_value"))
            if not init_val:
                init_val = self._default_for_type(sig_type)
            sig_id = f"{name}.{sig_name}"
            L.append(self.indent(
                f'const {sig_name} = signal({init_val}, {{ id: "{sig_id}", name: "{sig_name}", type: "{sig_type}" }});'
            ))

        # ---- refs ----
        refs = self._get(comp, "refs", [])
        for ref in refs:
            ref_name = self._get(ref, "name", "ref")
            L.append(self.indent(f"let {ref_name} = null;"))

        # ---- computeds ----
        for compd in computeds:
            c_name = self._get(compd, "name", "c")
            c_body = self._expr_to_js(self._get(compd, "body"))
            deps_raw = self._get(compd, "deps", [])
            # Wrap expression in arrow function with auto-tracked signal reads
            c_id = f"{name}.{c_name}"
            L.append(self.indent(
                f'const {c_name} = computed(() => {c_body}, {{ id: "{c_id}", name: "{c_name}" }});'
            ))

        L.append("")

        # ---- lifecycle ----
        for lc in lifecycles:
            lc_kind = self._get(lc, "kind", "mount")
            lc_body = self._gen_block_body(self._get(lc, "body"))
            if lc_kind == "mount":
                L.append(self.indent("effect(() => {"))
                L.append(self.indent(lc_body, 2))
                L.append(self.indent("});"))
            elif lc_kind == "unmount":
                L.append(self.indent("onCleanup(() => {"))
                L.append(self.indent(lc_body, 2))
                L.append(self.indent("});"))
            L.append("")

        # ---- effects ----
        for eff in effects:
            eff_body = self._gen_block_body(self._get(eff, "body"))
            L.append(self.indent("effect(() => {"))
            L.append(self.indent(eff_body, 2))
            L.append(self.indent("});"))
            L.append("")

        # ---- functions ----
        for fn in fns:
            fname = self._get(fn, "name", "handler")
            params = self._get(fn, "params", [])
            param_str = ", ".join(self._get(p, "name", "x") for p in params)
            body_ir = self._get(fn, "body")

            # Per-statement tracing: set trace ID for block body generation
            self._fn_trace_id = f"{name}.{fname}"

            body = self._gen_block_body(body_ir)

            # Wrap function with execution trace
            trace_id = f"{name}.{fname}"
            L.append(self.indent(f"const {fname} = ({param_str}) => {{"))
            L.append(self.indent(f'traceExecution("{trace_id}");'))
            L.append(self.indent(body, 2) if body.strip() else "")
            L.append(self.indent("};"))

        L.append("")

        # ---- view rendering ----
        if view is not None:
            view_js = self._gen_view_tree(view, styles, name)
            L.append(self.indent("// Render view tree"))
            L.append(self.indent(f"const __root = {view_js};"))
            L.append(self.indent("render(__root, mountEl);"))
            L.append("")

        # ---- styles ----
        if styles:
            style_css = self._gen_styles(styles, name)
            if style_css:
                L.append(self.indent("// Inject styles"))
                L.append(self.indent(
                    f'const __style = document.createElement("style");'
                ))
                L.append(self.indent(
                    f'__style.textContent = {self._escape_js_string(style_css)};'
                ))
                L.append(self.indent("document.head.appendChild(__style);"))
                L.append(self.indent("onCleanup(() => __style.remove());"))
                L.append("")

        # ---- cleanup registration ----
        L.append(self.indent("onCleanup(() => dispose());"))

        # ---- return public API ----
        public_exports = []
        for state in states:
            public_exports.append(self._get(state, "name", "x"))
        for compd in computeds:
            public_exports.append(self._get(compd, "name", "c"))
        for fn in fns:
            public_exports.append(self._get(fn, "name", "handler"))
        if public_exports:
            L.append(self.indent(
                f"return {{ {', '.join(public_exports)} }};"
            ))

        L.append("}")
        return "\n".join(L)

    # ── View Tree → h() calls ──────────────────────────────

    def _gen_view_tree(self, view, styles: list, comp_name: str = "Component") -> str:
        """Convert ViewTree IR to h() call string."""
        if view is None:
            return "null"
        if isinstance(view, list):
            parts = [self._gen_view_node(v, styles, comp_name) for v in view]
            return f"[{', '.join(p for p in parts if p)}]"
        return self._gen_view_node(view, styles, comp_name) or "null"

    def _gen_view_node(self, node, styles: list, comp_name: str) -> str:
        """Convert a single ViewTree node to h() call or text."""
        if isinstance(node, str):
            # Plain text
            return self._escape_js_string(node)

        if not isinstance(node, dict):
            return str(node)

        kind = self._get(node, "_kind", "")

        # Interpolation: {expr} → signal binding through h()
        if kind == "Interpolation" or "expr" in node:
            expr = self._get(node, "expr")
            # For bare signal/computed references, pass the signal object directly
            # so h() can subscribe for reactive updates
            if isinstance(expr, dict):
                ek = self._get(expr, "_kind", "")
                if ek in ("IdentifierExpr", "SignalRefIR"):
                    name = self._get(expr, "name", "x")
                    owner = self._get(expr, "owner", "")
                    if owner:
                        actual = self._resolve_store_alias(owner)
                        return f"{actual}.{name}"
                    return name
                # Compound expression (e.g. count + 1): wrap in anonymous computed
                # For now, evaluate once at render time
            expr_js = self._expr_to_js(expr)
            return expr_js

        # Conditional
        if kind == "ConditionalIntent" or "branches" in node:
            return self._gen_conditional(node, styles, comp_name)

        # List rendering
        if kind == "ListRenderingIntent" or "iterable" in node:
            return self._gen_list(node, styles, comp_name)

        # Slot
        if kind == "SlotIntent":
            slot_name = self._get(node, "name", "children")
            return f"props.{slot_name}"

        # Element
        if "tag" in node:
            return self._gen_element(node, styles, comp_name)

        # Fallback: treat as text
        return str(self._get(node, "content", node))

    def _gen_element(self, elem: dict, styles: list, comp_name: str) -> str:
        """Generate h() call for an element."""
        tag = self._get(elem, "tag", "div")
        test_id = self._get(elem, "test_id", "")

        # Map OmniView tag to HTML tag
        html_tag = self.ELEMENT_MAP.get(tag, tag.lower())

        # Heading level
        if tag == "Heading":
            level = self._get(elem, "props", {}).get("level", 1) if isinstance(self._get(elem, "props", {}), dict) else 1
            try:
                level = int(level)
            except (ValueError, TypeError):
                level = 1
            html_tag = f"h{max(1, min(6, level))}"

        # Build attrs object
        attrs_parts: list[str] = []

        # CSS class from styles
        class_names = []
        if test_id:
            # Match test_id to style rules
            css_class = self._find_style_class(test_id, styles)
            if css_class:
                class_names.append(css_class)
        if class_names:
            attrs_parts.append(f'class: "{" ".join(class_names)}"')

        # Element props
        props = self._get(elem, "props", {})
        if isinstance(props, dict):
            for prop_name, prop_val in props.items():
                if prop_name in ("level",):
                    continue  # handled above
                if isinstance(prop_val, str):
                    attrs_parts.append(f'{self._camel(prop_name)}: {self._escape_js_string(prop_val)}')
                elif isinstance(prop_val, dict) and "_kind" in prop_val:
                    attrs_parts.append(f'{self._camel(prop_name)}: {self._expr_to_js(prop_val)}')
                else:
                    attrs_parts.append(f'{self._camel(prop_name)}: {self._expr_to_js(prop_val)}')

        # Events
        events = self._get(elem, "events", {})
        if isinstance(events, dict):
            for event_name, handler in events.items():
                handler_name = self._get(handler, "handler", self._get(handler, "name", "handler"))
                handler_js = self._expr_to_js(handler)
                # Map OmniView event names to DOM events
                dom_event = self._map_event(event_name)
                attrs_parts.append(f'{dom_event}: {handler_js}')

        # Inline styles
        inline_style = self._get(elem, "inline_style")
        if inline_style:
            style_props = self._get(inline_style, "properties", {})
            if isinstance(style_props, dict) and style_props:
                style_entries = []
                for sp, sv in style_props.items():
                    css_prop = self._to_css_prop(sp)
                    css_val = self._expr_to_js(sv)
                    style_entries.append(f'{css_prop}: {css_val}')
                if style_entries:
                    attrs_parts.append(f'style: {{ {", ".join(style_entries)} }}')

        # Layout properties from props
        layout_map = {
            "gap": "gap",
            "direction": "flexDirection",
            "align": "alignItems",
            "justify": "justifyContent",
            "wrap": "flexWrap",
            "padding": "padding",
            "margin": "margin",
            "width": "width",
            "height": "height",
        }
        layout_styles = []
        if isinstance(props, dict):
            for omni_prop, css_prop in layout_map.items():
                if omni_prop in props:
                    val = props[omni_prop]
                    if isinstance(val, (int, float)):
                        layout_styles.append(f'{css_prop}: "{val}px"')
                    elif isinstance(val, str):
                        layout_styles.append(f'{css_prop}: {self._escape_js_string(val)}')
        if layout_styles:
            # Merge with existing style
            existing_style = next((a for a in attrs_parts if a.startswith("style:")), None)
            if existing_style:
                idx = attrs_parts.index(existing_style)
                attrs_parts[idx] = existing_style.rstrip(" }") + ", " + ", ".join(layout_styles) + " }"
            else:
                attrs_parts.append(f'style: {{ {", ".join(layout_styles)} }}')

        # data-testid
        if test_id:
            attrs_parts.append(f'"data-testid": "{test_id}"')

        # Ref
        ref_name = self._get(elem, "ref_name", "")
        if ref_name:
            attrs_parts.append(f'ref: (el) => {{ {ref_name} = el; }}')

        # Children
        children = self._get(elem, "children", [])
        child_parts = []
        if isinstance(children, list):
            for child in children:
                child_js = self._gen_view_node(child, styles, comp_name)
                if child_js:
                    child_parts.append(child_js)

        attr_str = "{ " + ", ".join(attrs_parts) + " }" if attrs_parts else "{}"
        child_str = ", ".join(child_parts) if child_parts else ""

        if child_str:
            return f'h("{html_tag}", {attr_str}, {child_str})'
        else:
            return f'h("{html_tag}", {attr_str})'

    def _gen_conditional(self, node: dict, styles: list, comp_name: str) -> str:
        """Generate renderConditional() call."""
        branches = self._get(node, "branches", [])
        if not branches:
            return "null"

        first_branch = branches[0]
        condition = self._expr_to_js(self._get(first_branch, "condition", "true"))
        then_body = self._gen_view_tree(self._get(first_branch, "body"), styles, comp_name)

        fallback = self._get(node, "fallback")
        if fallback:
            else_body = self._gen_view_tree(fallback, styles, comp_name)
            return f'renderConditional(container, {condition}, () => {then_body}, () => {else_body})'
        return f'renderConditional(container, {condition}, () => {then_body})'

    def _gen_list(self, node: dict, styles: list, comp_name: str) -> str:
        """Generate renderList() call."""
        iterable = self._expr_to_js(self._get(node, "iterable", "[]"))
        item_name = self._get(node, "item_binding", {}).get("name", "item") if isinstance(self._get(node, "item_binding"), dict) else "item"
        body = self._gen_view_tree(self._get(node, "body"), styles, comp_name)
        key_expr = self._get(node, "key_expr")

        if key_expr:
            key_fn = f'(item) => {self._expr_to_js(key_expr)}'
        else:
            key_fn = "(item, i) => String(i)"

        return f'renderList(container, {iterable}, ({item_name}, __i) => {body}, {key_fn})'

    # ── Statements ─────────────────────────────────────────

    def _gen_block_body(self, body) -> str:
        """Convert statement block IR to JS code."""
        if body is None:
            return ""
        stmts = self._get(body, "statements", []) if isinstance(body, dict) else []
        L = []
        for i, stmt in enumerate(stmts):
            if not isinstance(stmt, dict):
                continue

            # Per-statement trace injection
            if self._fn_trace_id:
                L.append(f'traceExecution("{self._fn_trace_id}.stmt{i}");')

            kind = self._get(stmt, "_kind", "")
            if kind == "MutationStmt" or "intent" in stmt:
                intent = self._get(stmt, "intent", {})
                target_ref = self._get(intent, "target", {})
                target = self._get(target_ref, "name", "x")
                target_owner = self._get(target_ref, "owner")
                op = self._get(intent, "operation", "direct_assign")
                amount = self._expr_to_js(self._get(intent, "amount", 1))
                item = self._expr_to_js(self._get(intent, "item"))

                if target_owner:
                    # Store field mutation: StoreName.field = value (via Proxy)
                    actual_store = self._resolve_store_alias(target_owner)
                    if op in ("increment", "INCREMENT"):
                        L.append(f"{actual_store}.{target} += {amount};")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"{actual_store}.{target} -= {amount};")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"{actual_store}.{target} = [...{actual_store}.{target}, {item}];")
                    else:
                        val = item or amount
                        L.append(f"{actual_store}.{target} = {val};")
                else:
                    # Component signal mutation: sig.set(newValue)
                    if op in ("increment", "INCREMENT"):
                        L.append(f"{target}.set({target}.get() + {amount});")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"{target}.set({target}.get() - {amount});")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"{target}.set([...{target}.get(), {item}]);")
                    else:
                        val = self._expr_to_js(self._get(intent, "item", self._get(intent, "amount")))
                        # Check if RHS references the same signal — convert to functional
                        if target in val and ".get()" not in val:
                            # Simple assignment: target = value → target.set(value)
                            L.append(f"{target}.set({val});")
                        else:
                            L.append(f"{target}.set({val});")

            elif kind in ("EventEmitStmt", "EventEmitStmtIR"):
                args = ", ".join(self._expr_to_js(a) for a in self._get(stmt, "args", []))
                L.append(f"props.{self._get(stmt, 'event_name', '')}({args});")

            elif kind in ("ReturnStmt", "ReturnStmtIR"):
                L.append(f"return {self._expr_to_js(self._get(stmt, 'value'))};")

            elif kind in ("ExprStmt", "ExprStmtIR"):
                expr_js = self._expr_to_js(self._get(stmt, "expr"))
                L.append(f"{expr_js};")

            elif kind in ("IfStmt", "IfStmtIR"):
                cond = self._expr_to_js(self._get(stmt, "condition"))
                L.append(f"if ({cond}) {{")
                L.append(self._gen_block_body(self._get(stmt, "then_body")))
                if self._get(stmt, "else_body"):
                    L.append("} else {")
                    L.append(self._gen_block_body(self._get(stmt, "else_body")))
                L.append("}")

            elif kind in ("LetStmt", "LetStmtIR"):
                L.append(f"const {self._get(stmt, 'name', 'x')} = {self._expr_to_js(self._get(stmt, 'value'))};")

        return "\n".join(L)

    # ── Expression → JS ────────────────────────────────────

    def _expr_to_js(self, expr) -> str:
        """Convert an expression IR node to JavaScript code.

        Signal-aware: identifier references are emitted as-is (the variable IS the signal).
        In expression position, .get() is added unless we're in a write context.
        """
        if expr is None:
            return "undefined"
        if isinstance(expr, (bool, int, float)):
            return "true" if expr is True else ("false" if expr is False else str(expr))
        if isinstance(expr, str):
            return f'"{expr}"'

        if not isinstance(expr, dict):
            return str(expr)

        kind = self._get(expr, "_kind", "")

        # Literal
        if kind == "LiteralExpr" or "value" in expr:
            val = self._get(expr, "value")
            if val is None:
                return "null"
            if isinstance(val, bool):
                return "true" if val else "false"
            if isinstance(val, (int, float)):
                return str(val)
            if isinstance(val, str):
                # Handle JSON-like values (arrays, objects)
                if val.startswith("[") or val.startswith("{"):
                    return val
                return f'"{val}"'
            return str(val)

        # Identifier / signal reference
        if kind == "IdentifierExpr" or (kind == "" and "name" in expr):
            name = self._get(expr, "name", "x")
            owner = self._get(expr, "owner", "")
            if owner:
                # store.field → StoreName.field (auto-unwrapped via Proxy)
                actual_store = self._resolve_store_alias(owner)
                return f"{actual_store}.{name}"
            # Component signal → signal.get() in read context
            return f"{name}.get()"

        # SignalRefIR
        if kind == "SignalRefIR":
            name = self._get(expr, "name", "x")
            owner = self._get(expr, "owner", "")
            if owner:
                actual_store = self._resolve_store_alias(owner)
                return f"{actual_store}.{name}"
            return f"{name}.get()"

        # Field access
        if kind == "FieldAccessExpr" or "object" in expr:
            obj = self._expr_to_js(self._get(expr, "object"))
            field = self._get(expr, "field", "")
            return f"{obj}.{field}"

        # Binary operation
        if kind == "BinaryOpExpr" or "op" in expr:
            op = self._get(expr, "op", "+")
            left = self._expr_to_js(self._get(expr, "left"))
            right = self._expr_to_js(self._get(expr, "right"))
            op_map = {
                "ADD": "+", "SUB": "-", "MUL": "*", "DIV": "/", "MOD": "%",
                "EQ": "===", "NEQ": "!==", "LT": "<", "GT": ">",
                "LTE": "<=", "GTE": ">=", "AND": "&&", "OR": "||",
            }
            js_op = op_map.get(op, op)
            return f"({left} {js_op} {right})"

        # Unary operation
        if kind == "UnaryOpExpr":
            op = self._get(expr, "op", "NOT")
            operand = self._expr_to_js(self._get(expr, "operand"))
            js_op = "!" if op in ("NOT", "!") else "-"
            return f"{js_op}{operand}"

        # Ternary
        if kind == "TernaryIR":
            cond = self._expr_to_js(self._get(expr, "cond"))
            then_expr = self._expr_to_js(self._get(expr, "then_expr"))
            else_expr = self._expr_to_js(self._get(expr, "else_expr"))
            return f"({cond} ? {then_expr} : {else_expr})"

        # Function call
        if kind == "FuncCallIR" or "func" in expr:
            func_name = self._get(expr, "func", "")
            args = self._get(expr, "args", [])
            args_js = ", ".join(self._expr_to_js(a) for a in args)
            return f"{func_name}({args_js})"

        # Format call
        if kind == "FormatCall":
            func = self._get(expr, "func", "format")
            args = self._get(expr, "args", [])
            args_js = ", ".join(self._expr_to_js(a) for a in args)
            return f"{func}({args_js})"

        # Array
        if kind == "ArrayExpr" or "elements" in expr:
            elements = self._get(expr, "elements", [])
            parts = [self._expr_to_js(e) for e in elements]
            return f"[{', '.join(parts)}]"

        # Spread
        if kind == "SpreadExpr":
            inner = self._expr_to_js(self._get(expr, "expr"))
            return f"...{inner}"

        # StoreFieldAccessExpr: store.action(args)
        if kind == "StoreFieldAccessExpr":
            store_alias = self._get(expr, "store_alias", "")
            field_name = self._get(expr, "field_name", "")
            call_args = self._get(expr, "call_args", [])
            actual_store = self._resolve_store_alias(store_alias)
            if call_args:
                args_js = ", ".join(self._expr_to_js(a) for a in call_args)
                return f"{actual_store}.{field_name}({args_js})"
            return f"{actual_store}.{field_name}"

        # StoreFieldAccessExpr without _kind
        if "store_alias" in expr:
            store_alias = self._get(expr, "store_alias", "")
            field_name = self._get(expr, "field_name", "")
            call_args = self._get(expr, "call_args", [])
            actual_store = self._resolve_store_alias(store_alias)
            if call_args:
                args_js = ", ".join(self._expr_to_js(a) for a in call_args)
                return f"{actual_store}.{field_name}({args_js})"
            return f"{actual_store}.{field_name}"

        # Intent with target
        if "intent" in expr:
            return self._gen_block_body({"statements": [expr]})

        # Event handler reference
        if kind == "EventHandlerIR" or "ref" in expr:
            ref = self._get(expr, "ref", "")
            if ref:
                return ref
            # Inline handler body
            body = self._get(expr, "body")
            if body:
                return self._expr_to_js(body)
            return "(() => {})"

        # Fallback for unknown
        return "undefined"

    # ── Store Generation ───────────────────────────────────

    def _gen_store(self, store_ir: dict, source_line: int) -> str:
        """Generate store("Name", () => {...}) code."""
        name = self._get(store_ir, "name", "Store")
        signals = self._get(store_ir, "signals", {})
        signal_defaults = self._get(store_ir, "signal_defaults", {})
        getters = self._get(store_ir, "getters", {})
        actions = self._get(store_ir, "actions", [])

        L: list[str] = []
        L.append('import { signal, computed, store } from "./omniview-runtime";')
        L.append("")

        L.append(f"export const {name} = store(\"{name}\", () => {{")

        # Signals
        for sig_name, sig_type in (signals if isinstance(signals, dict) else {}).items():
            default_val = signal_defaults.get(sig_name, None) if isinstance(signal_defaults, dict) else None
            default_js = self._expr_to_js(default_val) if default_val else self._default_for_type(self._format_type(sig_type))
            type_str = self._format_type(sig_type)
            L.append(self.indent(
                f'const {sig_name} = signal({default_js}, {{ id: "{name}.{sig_name}", name: "{sig_name}", type: "{type_str}" }});'
            ))

        L.append("")

        # Getters → computed (or regular getters in return object)
        if getters and isinstance(getters, dict):
            for g_name, g_body in getters.items():
                g_js = self._expr_to_js(g_body)
                L.append(self.indent(
                    f'const _{g_name} = computed(() => {g_js}, {{ id: "{name}.{g_name}", name: "{g_name}" }});'
                ))

        L.append("")

        # Actions
        if actions:
            for action in actions:
                a_name = self._get(action, "name", "action")
                a_params = self._get(action, "params", [])
                a_params_js = ", ".join(self._get(p, "name", "x") for p in a_params)
                a_async = "async " if self._get(action, "is_async", False) else ""
                a_body = self._get(action, "body")

                self._in_store_action = True
                self._store_name = name
                body_js = self._gen_block_body(a_body)
                self._in_store_action = False

                L.append(self.indent(f"{a_async}function {a_name}({a_params_js}) {{"))
                L.append(self.indent(body_js, 2) if body_js.strip() else "")
                L.append(self.indent("}"))
                L.append("")

        # Return object with signals, getters, and actions
        return_items = []

        # Signals: return as-is (store() Proxy auto-unwraps)
        for sig_name in (signals if isinstance(signals, dict) else {}).keys():
            return_items.append(sig_name)

        # Getters: return the computed signal (Proxy auto-unwraps .get())
        if getters and isinstance(getters, dict):
            for g_name in getters.keys():
                return_items.append(g_name)

        # Actions: return the function
        if actions:
            for action in actions:
                return_items.append(self._get(action, "name", "action"))

        L.append(self.indent(f"return {{ {', '.join(return_items)} }};"))
        L.append("});")

        return "\n".join(L)

    # ── Styles ──────────────────────────────────────────────

    def _gen_styles(self, styles: list, comp_name: str) -> str:
        """Generate CSS string from style rules."""
        if not styles:
            return ""
        L = []
        for rule in styles:
            cname = self._get(rule, "class_name", "")
            props = self._get(rule, "properties", {})
            if not cname or not props:
                continue
            L.append(f".{cname}_{comp_name} {{")
            for prop, value in props.items():
                css_line = self._style_to_css(prop, value)
                if css_line:
                    for part in css_line.rstrip(";").split(";"):
                        if part.strip():
                            L.append(f"  {part.strip()};")
            L.append("}")
        return "\n".join(L)

    def _find_style_class(self, test_id: str, styles: list) -> str:
        """Find matching CSS class name from style rules."""
        for rule in styles:
            cname = self._get(rule, "class_name", "")
            if cname == test_id or cname.replace("-", "_") == test_id:
                return f"{cname}_Component"
        return ""

    # ── Helpers ─────────────────────────────────────────────

    def _get_prop_names(self, comp: dict) -> list[str]:
        props = self._get(comp, "props", {})
        if isinstance(props, dict):
            return list(props.keys())
        return []

    def _format_type(self, type_ir) -> str:
        """Convert TypeIR to a string type name."""
        if type_ir is None:
            return "any"
        if isinstance(type_ir, str):
            return type_ir
        if isinstance(type_ir, dict):
            kind = self._get(type_ir, "kind", "any")
            type_map = {
                "void": "void", "i32": "number", "f64": "number",
                "bool": "boolean", "String": "string",
                "Array": "array", "Map": "object", "null": "null",
            }
            return type_map.get(kind, "any")
        return "any"

    def _default_for_type(self, type_str: str) -> str:
        """Get JS default value for a type string."""
        defaults = {
            "number": "0", "boolean": "false", "string": '""',
            "array": "[]", "object": "{}",
        }
        return defaults.get(type_str, "null")

    def _resolve_store_alias(self, alias: str) -> str:
        """Resolve a store alias to its actual store name."""
        # If alias is already a store name (PascalCase), return as-is
        # If alias is "store" or a generic name, try to resolve
        alias_map = {"store": self._store_name if self._store_name else "Store"}
        return alias_map.get(alias, alias)

    @staticmethod
    def _map_event(event_name: str) -> str:
        """Map OmniView event name to DOM event attribute."""
        event_map = {
            "onPress": "onClick",
            "onChange": "onChange",
            "onInput": "onInput",
            "onSubmit": "onSubmit",
            "onFocus": "onFocus",
            "onBlur": "onBlur",
            "onHover": "onMouseEnter",
            "onLeave": "onMouseLeave",
            "onKeyDown": "onKeyDown",
            "onKeyUp": "onKeyUp",
            "onScroll": "onScroll",
        }
        return event_map.get(event_name, event_name)

    @staticmethod
    def _extract_style_val(value) -> str:
        raw = str(value.get("value", value)) if isinstance(value, dict) else str(value)
        for unit in ("px", "rem", "em", "vh", "vw"):
            if raw.endswith(unit):
                return raw[:-len(unit)]
        return raw

    @staticmethod
    def _snake_to_kebab(name: str) -> str:
        return name.replace("_", "-")

    def _style_to_css(self, prop: str, value) -> str:
        val_str = self._extract_style_val(value)
        if val_str.startswith('"') and val_str.endswith('"'):
            val_str = val_str[1:-1]
        elif val_str.startswith("'") and val_str.endswith("'"):
            val_str = val_str[1:-1]
        css_prop = self._snake_to_kebab(prop)
        from omniview.codegen.css_mapping import lookup_css
        result = lookup_css(prop, val_str)
        if result is not None:
            return result.rstrip(";") + ";" if result and not result.endswith(";") else result
        return f"{css_prop}: {val_str};"

    @staticmethod
    def _escape_js_string(s: str) -> str:
        """Escape a string for embedding in JS code."""
        escaped = s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\r", "\\r")
        return f'"{escaped}"'

    def _has_conditionals(self, view) -> bool:
        """Check if view tree contains any ConditionalIntent nodes."""
        if isinstance(view, dict):
            if self._get(view, "_kind", "") == "ConditionalIntent" or "branches" in view:
                return True
            for key in ("children", "body", "fallback"):
                child = self._get(view, key)
                if self._has_conditionals(child):
                    return True
        elif isinstance(view, list):
            for v in view:
                if self._has_conditionals(v):
                    return True
        return False

    def _has_lists(self, view) -> bool:
        """Check if view tree contains any ListRenderingIntent nodes."""
        if isinstance(view, dict):
            if self._get(view, "_kind", "") == "ListRenderingIntent" or "iterable" in view:
                return True
            for key in ("children", "body", "empty_body"):
                child = self._get(view, key)
                if self._has_lists(child):
                    return True
        elif isinstance(view, list):
            for v in view:
                if self._has_lists(v):
                    return True
        return False
