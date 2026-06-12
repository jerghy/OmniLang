"""React L2 codegen — Intent IR → idiomatic React TypeScript + CSS Modules.

Generates L2 (idiomatic) React code from OmniView Intent IR.
Target: React 18+ with Function Components, Hooks, Zustand, CSS Modules.
"""

from omniview.codegen.base import CodegenBase, CodegenResult


# ── Element Mapping ───────────────────────────────────

REACT_ELEMENT_MAP: dict[str, str] = {
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

# Semantic layout props that become inline styles (not HTML attrs)
LAYOUT_PROPS = {"direction", "gap", "align", "justify", "wrap", "flex",
                "padding", "margin", "width", "height"}

# Event name mapping: OmniView → React
EVENT_MAP = {
    "onPress": "onClick", "onChange": "onChange",
    "onSubmit": "onSubmit", "onFocus": "onFocus", "onBlur": "onBlur",
    "onHover": "onMouseEnter", "onLeave": "onMouseLeave",
    "onKeyDown": "onKeyDown", "onKeyUp": "onKeyUp",
}


class ReactCodegen(CodegenBase):
    """Generate L2 idiomatic React TypeScript from OmniView IR."""

    target = "react"
    tier = "L2"
    ELEMENT_MAP = REACT_ELEMENT_MAP

    def __init__(self):
        super().__init__()
        self._in_store_action = False  # context flag for store vs component mutations
        self._store_name = ""
        self._error_boundaries: set = set()  # track ErrorBoundary class names
        self._cross_file_imports: dict[str, str] = {}  # component_name → relative_import_path
        self._for_depth: int = 0  # @for nesting depth for scoped keys (G3 fix)
        self._for_item_name: str = ""  # current @for item name for key scoping

    def _get(self, obj, key, default=None):
        if isinstance(obj, dict):
            return obj.get(key, default)
        return getattr(obj, key, default)

    # ── Public API ─────────────────────────────────────

    def generate_component(self, comp_ir, store_irs: dict | None = None, source_line: int = 1) -> CodegenResult:
        result = CodegenResult()
        try:
            self._all_stores = store_irs or {}
            tsx = self._gen_component(comp_ir, store_irs or {}, source_line)
            css = self._gen_css_module(comp_ir)
            name = self._get(comp_ir, "name", "Component")
            result.files[f"{name}.tsx"] = tsx
            if css:
                result.files[f"{name}.module.css"] = css
        except Exception as e:
            result.errors.append(f"React codegen failed: {e}")
        return result

    def generate_store(self, store_ir, source_line: int = 1) -> CodegenResult:
        result = CodegenResult()
        try:
            name = self._get(store_ir, "name", "Store")
            ts = self._gen_store(store_ir, source_line)
            result.files[f"use{name}.ts"] = ts
        except Exception as e:
            result.errors.append(f"Store codegen failed: {e}")
        return result

    # ── Component Generation ───────────────────────────

    def _gen_component(self, comp: dict, stores: dict, source_line: int = 1) -> str:
        name = self._get(comp, "name", "Component")
        L = []

        # Imports — only include hooks that are used
        hooks = set()
        if self._get(comp, "states"):
            hooks.add("useState")
        if self._get(comp, "computeds"):
            hooks.add("useMemo")
        if self._get(comp, "effects") or self._get(comp, "lifecycles"):
            hooks.add("useEffect")
        if self._get(comp, "fns"):
            hooks.add("useCallback")
        refs = self._get(comp, "refs", [])
        if refs:
            hooks.add("useRef")
        hooks_str = ", ".join(sorted(hooks))
        L.append(f'import React, {{ {hooks_str} }} from "react";')
        has_styles = bool(self._get(comp, "styles", []))
        if has_styles:
            L.append(f'import styles from "./{name}.module.css";')

        # Store imports — from IR store_refs or detect from view
        store_refs = set(self._get(comp, "store_refs", []))
        if not store_refs:
            store_refs = self._detect_store_refs(comp)
        if store_refs:
            for sname in store_refs:
                L.append(f'import {{ use{sname} }} from "./use{sname}";')

        # Cross-file component imports (from other .omni files)
        if self._cross_file_imports:
            for comp_name, import_path in self._cross_file_imports.items():
                if comp_name != name:  # Don't self-import
                    L.append(f'import {{ {comp_name} }} from "{import_path}";')

        L.append("")

        # Props interface
        props_iface = self._gen_props_interface(comp)
        if props_iface:
            L.append(props_iface)
            L.append("")

        # Component function (plain function, no React.FC per project conventions)
        generic_params = self._get(comp, "generic_params", [])
        generic_str = f"<{', '.join(generic_params)}>" if generic_params else ""
        has_props = bool(self._get_prop_names(comp))
        if has_props:
            L.append(f"export function {name}{generic_str}(props: {name}Props{generic_str}) {{")
        else:
            L.append(f"export function {name}{generic_str}() {{")
        L.append(self.indent(f"// .omni:{source_line} -- component {name}"))

        # Destructure props
        prop_names = self._get_prop_names(comp)
        if prop_names:
            L.append(self.indent(f"const {{ {', '.join(prop_names)} }} = props;"))

        L.append("")

        # Store hooks — separate data fields from actions
        self._store_getter_fields = set()  # fields that are getters (need () call)
        if store_refs:
            for sname in store_refs:
                fields = self._detect_store_fields(comp, sname)
                store_actions = set()
                store_getters = set()
                if sname in self._all_stores:
                    for action in self._get(self._all_stores[sname], "actions", []):
                        store_actions.add(self._get(action, "name", ""))
                    for gname in self._get(self._all_stores[sname], "getters", {}):
                        store_getters.add(gname)
                self._store_getter_fields = store_getters
                data_fields = [f for f in fields if f not in store_actions]
                for f in data_fields:
                    if f in store_getters:
                        # For getters, directly select the underlying signal if possible
                        # Fallback: select the getter function and call it
                        L.append(self.indent(f"const {f} = use{sname}(s => s.{f}());"))
                    else:
                        L.append(self.indent(f"const {f} = use{sname}(s => s.{f});"))

        # State hooks
        for state in self._get(comp, "states", []):
            sig_name = self._get(state, "name", "value")
            init_val = self._expr_to_js(self._get(state, "initial_value"))
            ts_type = self._map_type_to_ts(self._get(state, "type"))
            L.append(self.indent(f"const [{sig_name}, set{self._pascal(sig_name)}] = useState<{ts_type}>({init_val});"))

        # Ref hooks
        for ref in refs:
            ref_name = self._get(ref, "name", "ref")
            ref_type = self._map_type_to_ts(self._get(ref, "type"))
            L.append(self.indent(f"const {ref_name}Ref = useRef<{ref_type}>(null);"))

        # Computed hooks
        for computed in self._get(comp, "computeds", []):
            cname = self._get(computed, "name", "value")
            body = self._expr_to_js(self._get(computed, "body"))
            deps = self._get(computed, "deps", [])
            L.append(self.indent(f"const {cname} = useMemo(() => {body}, [{', '.join(deps)}]);"))

        # Effects & lifecycles
        for lc in self._get(comp, "lifecycles", []):
            kind = self._get(lc, "kind", "mount")
            body = self._gen_block_body(self._get(lc, "body"))
            if kind == "mount":
                # Skip if body is just setting state to its initial value (redundant)
                if body.strip():
                    L.append(self.indent("useEffect(() => {"))
                    L.append(self.indent(body, 2))
                    L.append(self.indent("}, []); // @mount"))
            elif kind == "unmount":
                L.append(self.indent("useEffect(() => {"))
                L.append(self.indent("return () => {", 2))
                L.append(self.indent(body, 3))
                L.append(self.indent("};", 2))
                L.append(self.indent("}, []); // @unmount"))
        for eff in self._get(comp, "effects", []):
            body = self._gen_block_body(self._get(eff, "body"))
            deps = self._get(eff, "deps", [])
            L.append(self.indent("useEffect(() => {"))
            L.append(self.indent(body, 2))
            L.append(self.indent(f"}}, [{', '.join(deps)}]); // @effect"))

        # Function definitions
        for fn in self._get(comp, "fns", []):
            fname = self._get(fn, "name", "handler")
            params = self._get(fn, "params", [])
            param_str = ", ".join(self._get(p, "name", "x") for p in params)
            body_ir = self._get(fn, "body")
            body = self._gen_block_body(body_ir)
            # Derive dependencies from signal references in fn body
            signal_names = {s.get("name", "") for s in self._get(comp, "states", [])}
            deps = sorted(self._detect_signal_refs(body_ir, signal_names))
            # Exclude signals that are only written (via setter), not read
            body_code = self._gen_block_body(body_ir)
            deps = [d for d in deps if d in body_code and f"set{self._pascal(d)}" not in body_code
                    or body_code.count(d) > body_code.count(f"set{self._pascal(d)}")]
            deps_str = ", ".join(deps)
            L.append(self.indent(f"const {fname} = useCallback(({param_str}) => {{"))
            L.append(self.indent(body, 2))
            L.append(self.indent(f"}}, [{deps_str}]);"))

        # View → JSX
        L.append("")
        L.append(self.indent("return ("))
        view_jsx = self._gen_view_tree(self._get(comp, "view"), self._get(comp, "styles", []))
        L.append(self.indent(view_jsx, 2))
        L.append(self.indent(");"))
        L.append("}")
        return "\n".join(L)

    # ── Props ──────────────────────────────────────────

    def _get_prop_names(self, comp: dict) -> list[str]:
        props = self._get(comp, "props", {})
        if isinstance(props, dict):
            return list(props.keys())
        return []

    def _gen_format_call(self, expr: dict) -> str:
        """Generate framework-specific formatting code."""
        func = self._get(expr, "func", "")
        args = [self._expr_to_js(a) for a in self._get(expr, "args", [])]
        if func == "format_date":
            val = args[0] if args else "undefined"
            pattern = args[1] if len(args) > 1 else '"yyyy-MM-dd"'
            return f'new Date({val}).toLocaleDateString(undefined, {{}})'
        elif func == "format_number":
            val = args[0] if args else "0"
            decimals = args[1] if len(args) > 1 else "2"
            return f'({val}).toFixed({decimals})'
        return "undefined"

    def _gen_props_interface(self, comp: dict) -> str:
        props = self._get(comp, "props", {})
        if not props:
            return ""
        name = self._get(comp, "name", "Component")
        generic_params = self._get(comp, "generic_params", [])
        generic_str = f"<{', '.join(generic_params)}>" if generic_params else ""
        L = [f"interface {name}Props{generic_str} {{"]
        for prop_name, prop_type in props.items():
            ts_type = self._map_type_to_ts(prop_type)
            L.append(f"  {prop_name}: {ts_type};")
        L.append("}")
        return "\n".join(L)

    # ── Store Detection ────────────────────────────────

    def _resolve_store_name(self, alias: str, field_or_action: str = "") -> str:
        """Resolve 'store' alias to the actual store name.

        When field_or_action is provided, finds the specific store that owns
        that field/action. Otherwise returns the first store (backward compat).
        """
        stores = {sname: sir for sname, sir in self._all_stores.items()
                  if "signals" in sir and "view" not in sir}
        if not stores:
            return alias.capitalize()
        # If a specific field/action name is given, find the owning store
        if field_or_action:
            for sname, sir in stores.items():
                signals = self._get(sir, "signals", {})
                getters = self._get(sir, "getters", {})
                actions = self._get(sir, "actions", [])
                action_names = {self._get(a, "name", "") for a in actions}
                if (field_or_action in signals or field_or_action in getters
                        or field_or_action in action_names):
                    return sname
        # Fallback: return the first store
        return next(iter(stores))

    def _detect_store_refs(self, comp: dict) -> set[str]:
        """Detect which stores are referenced in the component."""
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
            # Check for store.field in expressions
            for key, val in node.items():
                if key in ("object",) and isinstance(val, dict):
                    if self._get(val, "name") == "store":
                        refs.add(self._resolve_store_name("store"))
                self._scan_store_refs(val, refs)
            # Recurse into children
            if "children" in node:
                self._scan_store_refs(node["children"], refs)
            if "body" in node:
                self._scan_store_refs(node["body"], refs)
            if "branches" in node:
                self._scan_store_refs(node["branches"], refs)
            if "view" in node:
                self._scan_store_refs(node["view"], refs)

    def _detect_store_fields(self, comp: dict, store_name: str) -> list[str]:
        """Detect which store fields are accessed from a specific store.

        Only returns fields that actually belong to store_name (exist in its
        signals or getters), avoiding cross-store field pollution.
        """
        all_fields = set()
        view = self._get(comp, "view")
        if view:
            self._scan_store_fields(view, all_fields)
        # Filter: only return fields that belong to THIS store
        store_ir = self._all_stores.get(store_name, {})
        store_signals = set(self._get(store_ir, "signals", {}).keys())
        store_getters = set(self._get(store_ir, "getters", {}).keys())
        valid_fields = store_signals | store_getters
        return sorted(f for f in all_fields if f in valid_fields)

    def _scan_store_fields(self, node, fields: set):
        if isinstance(node, list):
            for child in node:
                self._scan_store_fields(child, fields)
        if isinstance(node, dict):
            if self._get(node, "_kind") == "FieldAccessExpr":
                obj = self._get(node, "object")
                if isinstance(obj, dict) and self._get(obj, "name") == "store":
                    fields.add(self._get(node, "field", "unknown"))
            # Also check StoreFieldAccessExpr (used for store action calls)
            if "store_alias" in node and "field_name" in node:
                fields.add(self._get(node, "field_name", "unknown"))
            for val in node.values():
                self._scan_store_fields(val, fields)

    # ── View Tree → JSX ────────────────────────────────

    def _gen_view_tree(self, view, styles: list, indent_level: int = 0) -> str:
        if view is None:
            return "null"
        if isinstance(view, str):
            return self._escape_jsx_text(view)
        if isinstance(view, list):
            return "\n".join(self._gen_view_tree(c, styles, indent_level) for c in view)
        if isinstance(view, dict):
            kind = self._get(view, "_kind", "")
            if kind == "Interpolation":
                return "{" + self._expr_to_js(self._get(view, "expr")) + "}"
            if kind == "ListRenderingIntent" or "iterable" in view:
                return self._gen_list(view, styles, indent_level)
            if kind == "ConditionalIntent" or "branches" in view:
                return self._gen_conditional(view, styles, indent_level)
            if kind == "SlotIntent":
                return "{children}" if not view.get("name") else f"{{props.{view.get('name')}}}"
            if kind == "ErrorBoundary":
                return self._gen_error_boundary(view, styles, indent_level)
            if "tag" in view:
                return self._gen_element(view, styles, indent_level)
        return "null"

    def _gen_element(self, elem: dict, styles: list, indent_level: int) -> str:
        tag = self._get(elem, "tag", "div")
        # Heading with level prop: <Heading level="1"> → <h1>
        if tag == "Heading":
            level_prop = self._get(elem, "props", {}).get("level", {})
            level = self._get(level_prop, "value", "2")
            html_tag = f"h{level}"
        else:
            # Cross-file imported components keep PascalCase, built-ins map to HTML
            if tag in self._cross_file_imports:
                html_tag = tag  # imported component, keep PascalCase
            else:
                html_tag = self.ELEMENT_MAP.get(tag, tag)

        # Spacer: flex grow
        if tag == "Spacer":
            return "<div style={{flex: 1}}></div>"
        props = self._get(elem, "props", {})
        bindings = self._get(elem, "bindings", {})
        events = self._get(elem, "events", {})
        children = self._get(elem, "children", [])
        test_id = self._get(elem, "test_id")

        # Separate layout props from regular props
        layout_props = {}
        regular_props = {}
        for pname, pval in props.items():
            if pname in LAYOUT_PROPS:
                layout_props[pname] = pval
            else:
                regular_props[pname] = pval

        attrs = []

        # A11y prop mapping: omni → HTML
        A11Y_MAP = {"accessibilityLabel": "aria-label", "accessibilityRole": "role"}

        # Regular props
        for pname, pval in regular_props.items():
            # Skip semantic-only props (not valid HTML attributes)
            if pname in ("class", "key", "level"):
                continue
            # Map A11y props to HTML attributes
            html_name = A11Y_MAP.get(pname, pname)
            js_val = self._expr_to_js(pval)
            if js_val.startswith('"') and js_val.endswith('"'):
                # String literal
                if pname == "style":
                    # Convert CSS string to React style object
                    style_obj = self._parse_css_string_to_react(js_val[1:-1])
                    if style_obj:
                        style_parts = ", ".join(
                            f"{k}: {v if v.isdigit() or v.replace('.','').replace('-','').isdigit() else repr(v)}"
                            for k, v in style_obj.items()
                        )
                        attrs.append(f"style={{{{{style_parts}}}}}")
                    else:
                        attrs.append(f'{html_name}={js_val}')
                else:
                    attrs.append(f'{html_name}={js_val}')
            else:
                attrs.append(f'{html_name}={{{js_val}}}')

        # Layout props → inline style
        style_obj = {}
        for pname, pval in layout_props.items():
            raw = self._extract_style_val(pval)
            css_prop = self._layout_to_css(pname, raw)
            if ":" in css_prop:
                key, val = css_prop.split(":", 1)
                style_obj[key.strip()] = val.strip().rstrip(";")

        if style_obj or tag in ("Flex", "Grid"):
            if not style_obj:
                style_obj = {}
            # Flex element needs display:flex base
            if tag == "Flex" and "display" not in style_obj:
                style_obj["display"] = "flex"
            # Grid element needs display:grid base
            if tag == "Grid":
                style_obj["display"] = "grid"
                style_obj["gridTemplateColumns"] = "repeat(auto-fill, minmax(250px, 1fr))"
                if "gap" not in style_obj:
                    style_obj["gap"] = "16px"
            style_parts = ", ".join(
                f"{k}: {v if v.isdigit() or v.replace('.','').isdigit() else repr(v)}"
                for k, v in style_obj.items()
            )
            attrs.append(f"style={{{{{style_parts}}}}}")

        # Auto-apply CSS module class from style block
        # Priority: 1) test-id exact match  2) test-id hyphen→underscore
        #           3) element tag match (only if NOT "container" — too generic)
        if styles:
            style_names = [r.get("class_name", "") for r in styles if isinstance(r, dict)]
            tag_lower = tag.lower()
            matched = None
            # 1) test-id exact match (e.g. test-id="sidebar" → .sidebar)
            if test_id and test_id in style_names:
                matched = test_id
            # 2) test-id with hyphens→underscores (e.g. test-id="card-typo" → .card_typo)
            if not matched and test_id:
                test_id_underscored = test_id.replace("-", "_")
                if test_id_underscored in style_names:
                    matched = test_id_underscored
            # 3) Tag match — only for specific tags, skip "container" (too generic)
            if not matched and tag_lower in style_names and tag_lower != "container":
                matched = tag_lower
            if matched:
                attrs.append(f"className={{styles.{matched}}}")

        # Bindings → value + onChange
        for bname, bref in bindings.items():
            sig_name = self._get(bref, "name", "")
            # Detect store.field bindings (e.g., bind:value={store.email})
            is_store_binding = "." in sig_name
            if is_store_binding:
                parts = sig_name.split(".", 1)
                store_alias, field = parts[0], parts[1]
                store_name = self._resolve_store_name(store_alias, field)
                store_hook = f"use{store_name}"
                if bname == "value":
                    attrs.append(f"value={{{field}}}")
                    attrs.append(f"onChange={{e => {store_hook}.setState({{ {field}: e.target.value }})}}")
                elif bname == "checked":
                    attrs.append(f"checked={{{field}}}")
                    attrs.append(f"onChange={{e => {store_hook}.setState({{ {field}: e.target.checked }})}}")
            else:
                if bname == "value":
                    attrs.append(f"value={{{sig_name}}}")
                    attrs.append(f"onChange={{e => set{self._pascal(sig_name)}(e.target.value)}}")
                elif bname == "checked":
                    attrs.append(f"checked={{{sig_name}}}")
                    attrs.append(f"onChange={{e => set{self._pascal(sig_name)}(e.target.checked)}}")

        # Events
        for ename, ehandler in events.items():
            react_event = EVENT_MAP.get(ename, ename)
            attrs.append(f"{react_event}={{{self._gen_event_handler(ehandler)}}}")

        # Style classes
        style_classes = self._get(elem, "style_classes")
        if style_classes:
            class_str = self._gen_class_string(style_classes)
            if class_str:
                attrs.append(f"className={{{class_str}}}")

        if test_id:
            attrs.append(f'data-testid="{test_id}"')

        # Ref — DOM node reference (useRef)
        ref_name = self._get(elem, "ref_name")
        if ref_name:
            attrs.append(f"ref={{{ref_name}Ref}}")

        # Key for list items
        key_expr = self._get(elem, "key_expr")
        if key_expr:
            attrs.append(f"key={{{self._expr_to_js(key_expr)}}}")

        # Build tag
        attr_str = " ".join(attrs)
        tag_open = f"<{html_tag}" + (f" {attr_str}" if attr_str else "") + ">"

        if html_tag in ("img", "hr", "input", "br") and not children:
            return f"<{html_tag} {attr_str} />"

        if not children:
            return f"{tag_open}</{html_tag}>"

        child_lines = []
        for child in children:
            child_lines.append(self._gen_view_tree(child, styles, indent_level + 1))

        inner = "\n".join(child_lines)
        if "\n" in inner:
            return f"{tag_open}\n{inner}\n</{html_tag}>"
        return f"{tag_open}{inner}</{html_tag}>"

    def _layout_to_css(self, prop: str, val: str) -> str:
        """Convert semantic layout prop → CSS style string."""
        M = {
            "direction": {"row": "flexDirection: row", "column": "flexDirection: column"},
            "gap": lambda v: f"gap: {v}px",
            "align": {"center": "alignItems: center", "start": "alignItems: flex-start",
                      "end": "alignItems: flex-end"},
            "justify": {"center": "justifyContent: center",
                        "space-between": "justifyContent: space-between"},
            "padding": lambda v: f"padding: {v}px",
            "margin": lambda v: f"margin: {v}px",
            "flex": lambda v: f"flex: {v}",
        }
        m = M.get(prop)
        if m is None:
            return f"{prop}: {val}"
        if callable(m):
            return m(val)
        if isinstance(m, dict):
            return m.get(val, f"{prop}: {val}")
        return f"{prop}: {val}"

    def _gen_conditional(self, cond: dict, styles: list, indent_level: int) -> str:
        branches = self._get(cond, "branches", [])
        fallback = self._get(cond, "fallback")
        result = self._gen_view_tree(fallback, styles) if fallback else "null"
        for branch in reversed(branches):
            c = self._expr_to_js(self._get(branch, "condition"))
            body = self._gen_view_tree(self._get(branch, "body"), styles)
            # Strip outer braces from result (fallback) too
            result_stripped = result.strip()
            if result_stripped.startswith("{") and result_stripped.endswith("}"):
                result = result_stripped[1:-1].strip()
            # Wrap multi-child body — if it starts with { (JSX expr), keep it inside <>
            # otherwise strip {} first since <> will provide JSX context
            has_multiple = "\n" in body
            if has_multiple:
                body_stripped = body.strip()
                # If body is wrapped in {expr}, keep the braces inside <>
                if body_stripped.startswith("{") and body_stripped.endswith("}"):
                    body = f"<>\n{body_stripped}\n</>"
                else:
                    # Strip outer braces from single-expression wrappers
                    if body_stripped.startswith("{") and body_stripped.endswith("}"):
                        body = body_stripped[1:-1].strip()
                    body = f"<>\n{body}\n</>"
            result = f"{c} ? {body} : ({result})"
        return f"{{{result}}}"

    def _gen_error_boundary(self, boundary: dict, styles: list, indent_level: int) -> str:
        """Generate React ErrorBoundary wrapper.

        Uses a class-based ErrorBoundary that catches render errors
        and displays fallback UI.
        """
        name = self._get(boundary, "name", "error")
        children = self._get(boundary, "children", [])
        child_jsx = "\n".join(
            self._gen_view_tree(c, styles, indent_level + 1) for c in children
        )

        # For React L2: emit a self-contained ErrorBoundary class
        # and wrap children with it. The class is emitted once per component.
        boundary_class = f"{name}ErrorBoundary"
        self._error_boundaries.add(boundary_class)

        prefix = " " * (indent_level * 4)
        child_prefix = " " * ((indent_level + 1) * 4)
        return (
            f"{prefix}<{boundary_class}>\n"
            f"{child_prefix}{child_jsx}\n"
            f"{prefix}</{boundary_class}>"
        )

    def _gen_list(self, lst: dict, styles: list, indent_level: int) -> str:
        iterable = self._expr_to_js(self._get(lst, "iterable"))
        binding = self._get(lst, "item_binding", {})
        item_name = self._get(binding, "name", "item")
        key_expr = self._expr_to_js(self._get(lst, "key_expr"))
        empty_body = self._get(lst, "empty_body")

        # G3 fix: track @for depth for scoped keys
        self._for_depth += 1
        prev_item = self._for_item_name
        self._for_item_name = item_name

        body = self._gen_view_tree(self._get(lst, "body"), styles)

        self._for_depth -= 1
        self._for_item_name = prev_item

        # G3: generate scoped key when nested to prevent collisions
        if key_expr:
            # Use composite key: `${depth}-${item_key}` for nested loops
            if self._for_depth > 0:
                key_expr = f"`${{{self._for_item_name}}}-${{{item_name}}}`"
            else:
                key_expr = f"{{{item_name}}}"

        map_body = f"{iterable}.map({item_name} => (\n{body}\n))"
        if empty_body:
            empty_jsx = self._gen_view_tree(empty_body, styles)
            return f"{{{iterable}.length === 0 ? {empty_jsx} : {map_body}}}"
        return f"{{{map_body}}}"

    # ── Event Handlers ─────────────────────────────────

    def _gen_event_handler(self, handler: dict) -> str:
        kind = self._get(handler, "kind", "inline_fn")
        if kind in ("fn_ref", "FN_REF"):
            return self._get(handler, "ref", "() => {}")
        if kind in ("store_action", "STORE_ACTION"):
            ref = self._get(handler, "ref", "() => {}")
            parts = ref.split(".")
            if len(parts) == 2:
                alias, action = parts
                # Find which store owns this action
                actual_store = self._resolve_store_name(alias, field_or_action=action)
                return f"use{actual_store}.getState().{action}"
            return ref
        # Inline function
        params = self._get(handler, "params", [])
        param_str = ", ".join(self._get(p, "name", "e") for p in params)
        body = self._gen_block_body(self._get(handler, "body"))
        return f"({param_str}) => {{\n{self.indent(body)}\n}}"

    # ── Dependency Detection ──────────────────────────

    def _detect_signal_refs(self, node, signal_names: set) -> set:
        """Scan IR tree for references to component signals (for useCallback deps)."""
        refs = set()
        if node is None:
            return refs
        if isinstance(node, dict):
            kind = self._get(node, "_kind", "")
            # IdentifierExpr referencing a component signal
            if kind == "IdentifierExpr":
                name = self._get(node, "name", "")
                if name in signal_names:
                    refs.add(name)
            # SignalRefIR with no owner = component signal
            if "owner" in node and "name" in node and self._get(node, "owner") is None:
                name = self._get(node, "name", "")
                if name in signal_names:
                    refs.add(name)
            # BinaryOpExpr → check left and right
            if kind == "BinaryOpExpr":
                refs.update(self._detect_signal_refs(node.get("left"), signal_names))
                refs.update(self._detect_signal_refs(node.get("right"), signal_names))
            # UnaryOpExpr → check operand
            if kind == "UnaryOpExpr":
                refs.update(self._detect_signal_refs(node.get("operand"), signal_names))
            # StoreFieldAccessExpr → check call_args for signal refs
            if kind == "StoreFieldAccessExpr" or "store_alias" in node:
                for arg in self._get(node, "call_args", []):
                    refs.update(self._detect_signal_refs(arg, signal_names))
            # MutationStmt → check intent target
            if "intent" in node:
                refs.update(self._detect_signal_refs(node.get("intent"), signal_names))
            # MutationIntent → check target and item
            if "target" in node and "operation" in node:
                refs.update(self._detect_signal_refs(node.get("target"), signal_names))
                refs.update(self._detect_signal_refs(node.get("item"), signal_names))
            # Recurse into all values
            for val in node.values():
                refs.update(self._detect_signal_refs(val, signal_names))
        if isinstance(node, list):
            for item in node:
                refs.update(self._detect_signal_refs(item, signal_names))
        return refs

    # ── Class String ───────────────────────────────────

    def _gen_class_string(self, style_classes: dict) -> str:
        classes = self._get(style_classes, "classes", [])
        parts = []
        for cls in classes:
            kind = self._get(cls, "kind", "style_ref")
            if kind == "style_ref":
                parts.append(f"styles.{self._get(cls, 'value', '')}")
            elif kind == "string_literal":
                parts.append(f'"{self._get(cls, "value", "")}"')
            elif kind == "class_map":
                for cname, cond in self._get(cls, "value", {}).items():
                    parts.append(f"({self._expr_to_js(cond)}) ? styles.{cname} : ''")
        return " + ' ' + ".join(parts) if parts else "''"

    # ── Expression → JavaScript ────────────────────────

    def _expr_to_js(self, expr) -> str:
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
            # FieldAccessExpr — handle store.field specially
            if kind == "FieldAccessExpr":
                obj = self._get(expr, "object")
                field = self._get(expr, "field", "")
                if isinstance(obj, dict) and self._get(obj, "name") == "store":
                    return field  # Already extracted as hook, value is direct
                obj_str = self._expr_to_js(obj)
                return f"{obj_str}.{field}"
            if kind == "StoreFieldAccessExpr" or ("store_alias" in expr and "field_name" in expr):
                store_alias = self._get(expr, "store_alias", "store")
                field = self._get(expr, "field_name", "")
                args = self._get(expr, "call_args", [])
                actual_store = self._resolve_store_name(store_alias, field_or_action=field)
                args_str = ", ".join(self._expr_to_js(a) for a in args)
                return f"use{actual_store}.getState().{field}({args_str})"
            if kind == "ArrayExpr" or ("elements" in expr and "_kind" not in expr):
                parts = []
                for elem in self._get(expr, "elements", []):
                    if isinstance(elem, dict) and elem.get("_kind") == "SpreadExpr":
                        parts.append("..." + self._expr_to_js(elem.get("expr")))
                    else:
                        parts.append(self._expr_to_js(elem))
                return "[" + ", ".join(parts) + "]"
            if kind == "SpreadExpr":
                return "..." + self._expr_to_js(self._get(expr, "expr"))
            if kind == "FormatCall":
                return self._gen_format_call(expr)
            if kind == "FuncCallIR":
                func_name = self._get(expr, "func", "")
                args = self._get(expr, "args", [])
                args_str = ", ".join(self._expr_to_js(a) for a in args)
                return f"{func_name}({args_str})"
            if kind == "TranslateCall":
                key = self._get(expr, "key", "unknown")
                return f"t('{key}')"
            if kind == "IdentifierExpr" or ("name" in expr and "op" not in expr and "value" not in expr and len(expr) <= 2):
                name = self._get(expr, "name", "undefined")
                # In store action context, prefix signal names with state.
                if self._in_store_action and hasattr(self, '_store_signal_names') and name in self._store_signal_names:
                    return f"state.{name}"
                return name
            if kind == "LiteralExpr" or ("value" in expr and "op" not in expr and "name" not in expr):
                val = self._get(expr, "value")
                if isinstance(val, str):
                    # JS literal values that should NOT be quoted
                    if val in ("[]", "{}") or val.startswith("[") or val.startswith("{"):
                        return val
                    return f'"{val}"'
                if isinstance(val, bool):
                    return "true" if val else "false"
                return str(val) if val is not None else "undefined"
            if kind == "BinaryOpExpr" or ("op" in expr and "left" in expr):
                left = self._expr_to_js(self._get(expr, "left"))
                right = self._expr_to_js(self._get(expr, "right"))
                op_map = {"add": "+", "sub": "-", "mul": "*", "div": "/", "mod": "%",
                          "eq": "===", "neq": "!==", "lt": "<", "gt": ">",
                          "lte": "<=", "gte": ">=", "and": "&&", "or": "||",
                          # Also handle raw enum values
                          "+": "+", "-": "-", "*": "*", "/": "/", "%": "%",
                          "==": "===", "!=": "!==", "<": "<", ">": ">",
                          "<=": "<=", ">=": ">=", "&&": "&&", "||": "||"}
                op = op_map.get(self._get(expr, "op", "+"), self._get(expr, "op", "+"))
                return f"({left} {op} {right})"
            if kind == "UnaryOpExpr":
                op_raw = self._get(expr, "op", "!")
                op = "!" if op_raw in ("!", "not", "NOT") else "-"
                return f"{op}{self._expr_to_js(self._get(expr, 'operand'))}"
            if kind == "TernaryIR":
                c = self._expr_to_js(self._get(expr, "cond"))
                t = self._expr_to_js(self._get(expr, "then_expr"))
                e = self._expr_to_js(self._get(expr, "else_expr"))
                return f"({c} ? {t} : {e})"
            if "name" in expr:
                return self._get(expr, "name", "undefined")
        return "undefined"

    def _gen_block_body(self, body) -> str:
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
                amount = self._expr_to_js(self._get(intent, "amount", 1))
                item = self._expr_to_js(self._get(intent, "item"))

                if target_owner:
                    if self._in_store_action:
                        # Inside store action: set(state => ({ field: ... }))
                        if op in ("increment", "INCREMENT"):
                            L.append(f"set(state => ({{ {target}: state.{target} + {amount} }}));")
                        elif op in ("decrement", "DECREMENT"):
                            L.append(f"set(state => ({{ {target}: state.{target} - {amount} }}));")
                        elif op in ("spread_append", "SPREAD_APPEND"):
                            L.append(f"set(state => ({{ {target}: [...state.{target}, {item}] }}));")
                        else:
                            val = self._expr_to_js(self._get(intent, "item", self._get(intent, "amount")))
                            if val.startswith('"[') and val.endswith(']"'):
                                val = val[1:-1]
                            L.append(f"set(state => ({{ {target}: {val} }}));")
                    else:
                        # Component-level store.field = value → use{Store}.setState()
                        actual_store = self._resolve_store_name(target_owner, target)
                        store_hook = f"use{actual_store}"
                        if op in ("increment", "INCREMENT"):
                            L.append(f"{store_hook}.setState(s => ({{ {target}: s.{target} + {amount} }}));")
                        elif op in ("decrement", "DECREMENT"):
                            L.append(f"{store_hook}.setState(s => ({{ {target}: s.{target} - {amount} }}));")
                        elif op in ("spread_append", "SPREAD_APPEND"):
                            L.append(f"{store_hook}.setState(s => ({{ {target}: [...s.{target}, {item}] }}));")
                        else:
                            val = self._expr_to_js(self._get(intent, "item", self._get(intent, "amount")))
                            if val.startswith('"[') and val.endswith(']"'):
                                val = val[1:-1]
                            L.append(f"{store_hook}.setState({{ {target}: {val} }});")
                else:
                    # Component mutation: setX(newValue)
                    if op in ("increment", "INCREMENT"):
                        L.append(f"set{self._pascal(target)}({target} + {amount});")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"set{self._pascal(target)}({target} - {amount});")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"set{self._pascal(target)}([...{target}, {item}]);")
                    else:
                        val = self._expr_to_js(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"set{self._pascal(target)}({val});")
            elif kind in ("EventEmitStmt", "EventEmitStmtIR"):
                args = ", ".join(self._expr_to_js(a) for a in self._get(stmt, "args", []))
                L.append(f"props.{self._get(stmt, 'event_name', '')}({args});")
            elif kind in ("ReturnStmt", "ReturnStmtIR"):
                L.append(f"return {self._expr_to_js(self._get(stmt, 'value'))};")
            elif kind in ("ExprStmt", "ExprStmtIR"):
                L.append(f"{self._expr_to_js(self._get(stmt, 'expr'))};")
            elif kind in ("IfStmt", "IfStmtIR"):
                cond = self._expr_to_js(self._get(stmt, "condition"))
                L.append(f"if ({cond}) {{")
                L.append(self.indent(self._gen_block_body(self._get(stmt, "then_body"))))
                if self._get(stmt, "else_body"):
                    L.append("} else {")
                    L.append(self.indent(self._gen_block_body(self._get(stmt, "else_body"))))
                L.append("}")
            elif kind in ("LetStmt", "LetStmtIR"):
                L.append(f"const {self._get(stmt, 'name', 'x')} = {self._expr_to_js(self._get(stmt, 'value'))};")
        return "\n".join(L)

    # ── CSS Module Generation ──────────────────────────

    def _gen_css_module(self, comp: dict) -> str:
        styles = self._get(comp, "styles", [])
        if not styles:
            return ""
        L = []
        for rule in styles:
            cname = self._get(rule, "class_name", "")
            props = self._get(rule, "properties", {})
            L.append(f".{cname} {{")
            for prop, value in props.items():
                css_line = self._style_to_css(prop, value)
                if css_line:
                    # Split compound declarations (e.g. "display: flex; flex-direction: column;")
                    for part in css_line.rstrip(";").split(";"):
                        part = part.strip()
                        if part:
                            L.append(f"  {part};")
            L.append("}")
            L.append("")
        return "\n".join(L)

    @staticmethod
    def _extract_style_val(value) -> str:
        raw = str(value.get("value", value)) if isinstance(value, dict) else str(value)
        for unit in ("px", "rem", "em", "vh", "vw", "%"):
            if raw.endswith(unit):
                return raw[:-len(unit)]
        return raw

    @staticmethod
    def _snake_to_kebab(name: str) -> str:
        """Convert snake_case CSS property name to kebab-case."""
        return name.replace("_", "-")

    def _style_to_css(self, prop: str, value) -> str:
        val_str = self._extract_style_val(value)
        # Strip surrounding quotes from string values
        if val_str.startswith('"') and val_str.endswith('"'):
            val_str = val_str[1:-1]
        elif val_str.startswith("'") and val_str.endswith("'"):
            val_str = val_str[1:-1]
        # Convert property name to kebab-case
        css_prop = self._snake_to_kebab(prop)
        # First, try the full lookup table from css_mapping.py
        from omniview.codegen.css_mapping import lookup_css
        result = lookup_css(prop, val_str)
        if result is not None:
            return result.rstrip(";") + ";" if result and not result.endswith(";") else result
        # Fallback: translate property name (kebab-case) and value directly
        return f"{css_prop}: {val_str};"

    # ── Store Generation (Zustand L2) ──────────────────

    def _gen_store(self, store: dict, source_line: int = 1) -> str:
        name = self._get(store, "name", "Store")
        signals = self._get(store, "signals", {})
        getters = self._get(store, "getters", {})
        actions = self._get(store, "actions", [])
        self._store_name = name

        L = []
        L.append('import { create } from "zustand";')
        L.append("")
        L.append(f"// .omni:{source_line} -- store {name}")
        L.append("")

        # State interface
        L.append(f"interface {name}State {{")
        for sig_name, sig_type in signals.items():
            L.append(f"  {sig_name}: {self._map_type_to_ts(sig_type)};")
        for getter_name, getter_body in getters.items():
            ret_type = self._infer_getter_return_type(getter_body, signals)
            L.append(f"  {getter_name}: () => {ret_type};")
        for action in actions:
            aname = self._get(action, "name", "action")
            params = self._get(action, "params", [])
            pstr = ", ".join(f"{self._get(p, 'name', 'x')}: {self._map_type_to_ts(self._get(p, 'type'))}"
                           for p in params)
            L.append(f"  {aname}: ({pstr}) => void;")
        L.append("}")
        L.append("")

        # Create store
        L.append(f"export const use{name} = create<{name}State>()((set, get) => ({{")

        # Signals → initial state
        defaults = self._get(store, "signal_defaults", {})
        for sig_name, sig_type in signals.items():
            if sig_name in defaults:
                init_val = self._expr_to_js(defaults[sig_name])
                L.append(f"  {sig_name}: {init_val},")
            else:
                default_val = self._type_default(sig_type)
                if default_val == "[]":
                    L.append(f"  {sig_name}: [],")
                elif default_val == "{}":
                    L.append(f"  {sig_name}: {{}},")
                else:
                    L.append(f"  {sig_name}: {default_val},")

        # Getters → functions that access state
        # Build return type map for f64 rounding
        getter_types = {}
        for getter_name, getter_body in getters.items():
            getter_types[getter_name] = self._infer_getter_return_type(getter_body, signals)
        for getter_name, getter_body in getters.items():
            body_str = self._expr_to_js(getter_body)
            # Prefix signal references with state. to access Zustand state
            getter_expr = body_str
            for sig_name in sorted(signals.keys(), key=len, reverse=True):
                getter_expr = self._replace_identifier(getter_expr, sig_name, f"state.{sig_name}")
            # Round f64 (number) getters to 2 decimal places
            if getter_types.get(getter_name) == "number" and "/" in getter_expr:
                getter_expr = f"Math.round(({getter_expr}) * 100) / 100"
            L.append(f"  {getter_name}: () => {{")
            L.append(f"    const state = get();")
            L.append(f"    return {getter_expr};")
            L.append(f"  }},")

        # Actions → set(state => ...)
        self._in_store_action = True
        self._store_signal_names = set(signals.keys())
        for action in actions:
            aname = self._get(action, "name", "action")
            params = self._get(action, "params", [])
            pstr = ", ".join(self._get(p, "name", "x") for p in params)
            body = self._gen_block_body(self._get(action, "body"))
            if not body.strip():
                body = f"console.warn('Action \"{aname}\" is not implemented');"
            L.append(f"  {aname}: ({pstr}) => {{")
            L.append(self.indent(body))
            L.append(f"  }},")
        self._in_store_action = False

        L.append("}));")
        return "\n".join(L)

    # ── Type Mapping ───────────────────────────────────

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
            type_args = self._get(type_ir, "type_args", [])
            # Handle generic types: Array<String> → string[], Map<String, i32> → Record<string, number>
            if k in ("Array", "ARRAY") and type_args:
                inner = self._map_type_to_ts(type_args[0]) if type_args else "any"
                return f"{inner}[]"
            if k in ("Map", "MAP") and type_args and len(type_args) >= 2:
                kt = self._map_type_to_ts(type_args[0])
                vt = self._map_type_to_ts(type_args[1])
                return f"Record<{kt}, {vt}>"
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

    def _infer_getter_return_type(self, body, signals: dict) -> str:
        """Infer the return type of a getter from its body expression."""
        # Simple case: getter body is just a signal reference
        if isinstance(body, dict) and body.get("name"):
            sig_name = body.get("name")
            if sig_name in signals:
                return self._map_type_to_ts(signals[sig_name])
        return "any"

    @staticmethod
    def _parse_css_string_to_react(css_str: str) -> dict:
        """Parse a CSS string like 'text_decoration: line-through; color: red'
        into a React camelCase style dict like {'textDecoration': 'line-through', 'color': 'red'}.
        """
        result = {}
        for part in css_str.split(";"):
            part = part.strip()
            if not part or ":" not in part:
                continue
            key, val = part.split(":", 1)
            key = key.strip()
            val = val.strip()
            if not key or not val:
                continue
            # Convert snake_case/kebab-case CSS property to camelCase
            camel_key = key.replace("_", " ").replace("-", " ")
            camel_key = "".join(
                word.capitalize() if i > 0 else word.lower()
                for i, word in enumerate(camel_key.split())
            )
            result[camel_key] = val
        return result

    @staticmethod
    def _replace_identifier(expr: str, name: str, replacement: str) -> str:
        """Replace identifier `name` with `replacement` at word boundaries.

        Avoids partial matches (e.g., 'count' should NOT match 'count_active').
        """
        import re
        pattern = r'(?<![a-zA-Z0-9_])' + re.escape(name) + r'(?![a-zA-Z0-9_])'
        return re.sub(pattern, replacement, expr)

    @staticmethod
    def _pascal(s: str) -> str:
        if not s:
            return s
        return s[0].upper() + s[1:]

    @staticmethod
    def _escape_jsx_text(text: str) -> str:
        if text is None:
            return ""
        return text.replace("{", "&#123;").replace("}", "&#125;")
