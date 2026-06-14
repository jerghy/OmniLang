"""Flutter L1 codegen — Intent IR → Flutter/Dart Widget code.

Generates L1 (readable) Flutter code from OmniView Intent IR.
Target: Flutter 3.x with StatefulWidget, setState, Material widgets.
This is Phase 2a basic validation — validates IR expressiveness for non-web targets.
"""

from omniview.codegen.base import CodegenBase, CodegenResult


# ── Element Mapping ───────────────────────────────────

FLUTTER_ELEMENT_MAP: dict[str, str] = {
    "Container": "Container", "Flex": "Flex", "Grid": "GridView",
    "Stack": "Stack", "Spacer": "Spacer",
    "Text": "Text", "Heading": "Text", "Image": "Image",
    "Link": "GestureDetector", "Divider": "Divider",
    "Button": "ElevatedButton", "Input": "TextField",
    "TextArea": "TextField", "Checkbox": "Checkbox",
    "Select": "DropdownButton",
    "List": "ListView", "ListItem": "ListTile",
    "Spinner": "CircularProgressIndicator",
    "Progress": "LinearProgressIndicator",
    "Alert": "Container", "Card": "Card", "Form": "Form",
}

LAYOUT_PROPS = {"direction", "gap", "align", "justify", "wrap", "flex",
                "padding", "margin", "width", "height"}


class FlutterCodegen(CodegenBase):
    """Generate L1 readable Flutter/Dart from OmniView IR."""

    target = "flutter"
    tier = "L1"
    ELEMENT_MAP = FLUTTER_ELEMENT_MAP

    def __init__(self):
        super().__init__()
        self._in_store_action = False
        self._store_signal_names: set = set()
        self._all_stores: dict = {}
        self._bind_controllers: set = set()  # signal names needing TextEditingController
        self._cross_file_imports: dict[str, str] = {}
        self._store_aliases: set = set()  # store variable names in component state
        self._for_item_names: set = set()  # @for loop item variable names (no _ prefix)

    def _get(self, obj, key, default=None):
        if isinstance(obj, dict):
            return obj.get(key, default)
        return getattr(obj, key, default)

    # ── Public API ─────────────────────────────────────

    def generate_component(self, comp_ir, store_irs: dict | None = None, source_line: int = 1) -> CodegenResult:
        result = CodegenResult()
        try:
            self._all_stores = store_irs or {}
            self._bind_controllers = set()
            dart = self._gen_component(comp_ir, store_irs or {}, source_line)
            name = self._get(comp_ir, "name", "Component")
            result.files[f"{name}.dart"] = dart
        except Exception as e:
            result.errors.append(f"Flutter codegen failed: {e}")
        return result

    def generate_store(self, store_ir, source_line: int = 1) -> CodegenResult:
        result = CodegenResult()
        try:
            name = self._get(store_ir, "name", "Store")
            dart = self._gen_store(store_ir, source_line)
            result.files[f"{name.lower()}_store.dart"] = dart
        except Exception as e:
            result.errors.append(f"Flutter store codegen failed: {e}")
        return result

    # ── Component Generation ───────────────────────────

    def _gen_component(self, comp: dict, stores: dict, source_line: int = 1) -> str:
        name = self._get(comp, "name", "Component")
        states = self._get(comp, "states", [])
        lifecycles = self._get(comp, "lifecycles", [])
        fns = self._get(comp, "fns", [])
        bindings = self._detect_bindings(comp)

        # Build CSS rule lookup: class_name → {prop: value}
        self._css_rules = self._build_css_rules(comp)

        L = []
        L.append("import 'package:flutter/material.dart';")
        L.append("")

        # Store import
        store_refs = self._get(comp, "store_refs", [])
        if store_refs:
            for sname in store_refs:
                L.append(f"import '{sname.lower()}_store.dart';")
            L.append("")

        # StatefulWidget class
        L.append(f"// .omni:{source_line} -- component {name}")
        L.append(f"class {name} extends StatefulWidget {{")
        L.append(f"  const {name}({{super.key}});")
        L.append(f"  @override")
        L.append(f"  State<{name}> createState() => _{name}State();")
        L.append("}")
        L.append("")

        # State class
        L.append(f"class _{name}State extends State<{name}> {{")

        # State fields
        for state in states:
            sig_name = self._get(state, "name", "value")
            init_val = self._expr_to_dart(self._get(state, "initial_value"))
            dart_type = self._map_type_to_dart(self._get(state, "type"))
            L.append(f"  {dart_type} _{sig_name} = {init_val};")

        # TextEditingControllers for bind:value
        for bname, sig_name in bindings:
            if bname in ("value",):
                init = ""
                for s in states:
                    if self._get(s, "name") == sig_name:
                        init = self._expr_to_dart(self._get(s, "initial_value"))
                        break
                L.append(f"  final TextEditingController _{sig_name}Controller = TextEditingController(text: {init});")
                self._bind_controllers.add(sig_name)

        # Store instances
        if store_refs:
            for sname in store_refs:
                store_type = sname  # Store name is already the class name (e.g., "CartStore")
                var_name = sname.lower()
                if var_name.endswith("store"):
                    var_name = var_name[:-5]
                self._store_aliases.add(var_name)
                L.append(f"  final {store_type} {var_name} = {store_type}();")
            L.append("")

        L.append("")

        # initState
        mount_body = ""
        for lc in lifecycles:
            if self._get(lc, "kind") == "mount":
                mount_body = self._gen_block_body_dart(self._get(lc, "body"), {s.get("name", "") for s in states})
        if mount_body.strip():
            L.append("  @override")
            L.append("  void initState() {")
            L.append("    super.initState();")
            L.append(self.indent(f"    // @mount"))
            for line in mount_body.strip().split("\n"):
                if line.strip():
                    L.append(f"    {line}")
            L.append("  }")
            L.append("")

        # dispose (for controllers)
        if self._bind_controllers:
            L.append("  @override")
            L.append("  void dispose() {")
            for sig_name in self._bind_controllers:
                L.append(f"    _{sig_name}Controller.dispose();")
            L.append("    super.dispose();")
            L.append("  }")
            L.append("")

        # Handler methods
        for fn in fns:
            fname = self._get(fn, "name", "handler")
            body = self._gen_block_body_dart(self._get(fn, "body"), {s.get("name", "") for s in states})
            L.append(f"  void _{fname}() {{")
            body_lines = [line for line in body.strip().split("\n") if line.strip()]
            if body_lines:
                # Always wrap in setState — Flutter requires it for rebuilds
                L.append(f"    setState(() {{")
                for line in body_lines:
                    L.append(f"      {line}")
                L.append(f"    }});")
            L.append(f"  }}")
            L.append("")

        # build method
        L.append("  @override")
        L.append("  Widget build(BuildContext context) {")
        view_body = self._gen_view_tree(self._get(comp, "view"))
        if view_body.strip():
            L.append(f"    return {view_body.strip()};")
        else:
            L.append("    return const SizedBox.shrink();")
        L.append("  }")
        L.append("}")
        return "\n".join(L)

    def _detect_bindings(self, comp: dict) -> list[tuple[str, str]]:
        """Find bind:value signals needing TextEditingController.
        Returns list of (prop_name, signal_name) tuples."""
        bindings = []
        seen = set()
        def scan(node):
            if isinstance(node, dict):
                for bname, bref in self._get(node, "bindings", {}).items():
                    sig = self._get(bref, "name", "")
                    if (bname, sig) not in seen:
                        bindings.append((bname, sig))
                        seen.add((bname, sig))
                for val in node.values():
                    scan(val)
            if isinstance(node, list):
                for item in node:
                    scan(item)
        scan(self._get(comp, "view"))
        return bindings

    # ── CSS → Flutter Mapping ──────────────────────────

    def _build_css_rules(self, comp: dict) -> dict[str, dict[str, str]]:
        """Parse style IR into class→props lookup table."""
        rules = {}
        styles = self._get(comp, "styles", [])
        for rule in styles:
            class_name = self._get(rule, "class_name", "")
            props = self._get(rule, "properties", {})
            parsed = {}
            for prop_name, prop_val in props.items():
                raw = self._get(prop_val, "value", str(prop_val))
                # Strip quotes from string values
                if isinstance(raw, str) and raw.startswith('"') and raw.endswith('"'):
                    raw = raw[1:-1]
                # Normalize dimension values: "24 px" → "24px"
                if isinstance(raw, str):
                    raw = raw.replace(" px", "px").replace(" rem", "rem").replace(" em", "em")
                parsed[prop_name] = raw
            rules[class_name] = parsed
        return rules

    def _strip_unit(self, val: str) -> str:
        """Strip CSS units from a value for Flutter: 24px→24, 1.5rem→1.5*14."""
        val = val.strip()
        if val.endswith("px"):
            return val[:-2]
        if val.endswith("rem"):
            return f"({val[:-3]} * 14.0)"
        if val.endswith("em"):
            return f"({val[:-2]} * 14.0)"
        return val

    def _apply_css_to_container(self, css: dict, params: list, children: list) -> tuple[list, list]:
        """Apply CSS rules to a Container widget. Returns (params, children)."""
        # padding → EdgeInsets
        if "padding" in css:
            val = css["padding"]
            if " " in val:
                parts = [self._strip_unit(p) for p in val.split()]
                if len(parts) == 2:
                    params.append(f"padding: EdgeInsets.symmetric(vertical: {parts[0]}, horizontal: {parts[1]})")
                elif len(parts) == 4:
                    params.append(f"padding: EdgeInsets.only(left: {parts[3]}, top: {parts[0]}, right: {parts[1]}, bottom: {parts[2]})")
                else:
                    params.append(f"padding: EdgeInsets.all({parts[0]})")
            else:
                params.append(f"padding: EdgeInsets.all({self._strip_unit(val)})")

        # margin
        if "margin" in css:
            val = css["margin"]
            if " " in val:
                parts = [self._strip_unit(p) for p in val.split()]
                params.append(f"margin: EdgeInsets.symmetric(vertical: {parts[0]}, horizontal: {parts[1]})")
            else:
                params.append(f"margin: EdgeInsets.all({self._strip_unit(val)})")

        # background-color → decoration
        decoration_parts = []
        bg_color = css.get("background-color", "")
        border_radius = css.get("border-radius", "")
        box_shadow = css.get("box-shadow", "")

        if bg_color:
            decoration_parts.append(f"color: Color(0xFF{bg_color.lstrip('#')})")
        if border_radius:
            val = self._strip_unit(border_radius)
            decoration_parts.append(f"borderRadius: BorderRadius.circular({val})")
        if box_shadow:
            # Simple shadow — use a default elevation or parse
            decoration_parts.append("boxShadow: [const BoxShadow(blurRadius: 6, offset: Offset(0, 4), color: Color(0x1A000000))]")

        if decoration_parts:
            params.append(f"decoration: BoxDecoration({', '.join(decoration_parts)})")

        # Layout: display: flex + flex-direction → Column or Row
        if css.get("display") == "flex":
            direction = css.get("flex-direction", "column")
            if direction == "row":
                return params, children  # Flex→Row handled by tag
            # For column, children are already handled
            if "align-items" in css:
                align = css["align-items"]
                align_map = {"center": "CrossAxisAlignment.center", "start": "CrossAxisAlignment.start",
                             "end": "CrossAxisAlignment.end", "stretch": "CrossAxisAlignment.stretch"}
                params.append(f"crossAxisAlignment: {align_map.get(align, 'CrossAxisAlignment.center')}")

        return params, children

    def _apply_css_to_text(self, css: dict, params: list) -> list:
        """Apply CSS text rules to a Text/Heading widget. Returns updated params."""
        css_parts = []
        font_size = css.get("font-size", "")
        font_weight = css.get("font-weight", "")
        color = css.get("color", "")

        if font_size:
            css_parts.append(f"fontSize: {self._strip_unit(font_size)}")
        if font_weight:
            weight_map = {"400": "FontWeight.normal", "500": "FontWeight.w500",
                          "600": "FontWeight.w600", "700": "FontWeight.bold",
                          "bold": "FontWeight.bold", "normal": "FontWeight.normal"}
            css_parts.append(f"fontWeight: {weight_map.get(font_weight, 'FontWeight.bold')}")
        if color:
            css_parts.append(f"color: Color(0xFF{color.lstrip('#')})")

        if css_parts:
            # Merge with existing style param: CSS values override base values
            css_keys = set(p.split(":")[0].strip() for p in css_parts)
            for i, p in enumerate(params):
                if p.startswith("style:"):
                    existing = p
                    remaining = existing[7:].strip()
                    if remaining.startswith("const "):
                        remaining = remaining[6:]
                    if remaining.startswith("TextStyle(") and remaining.endswith(")"):
                        inner = remaining[10:-1]
                        # Filter out base values that CSS overrides
                        base_parts = [bp.strip() for bp in inner.split(",") if bp.strip()]
                        filtered = [bp for bp in base_parts
                                    if bp.split(":")[0].strip() not in css_keys]
                        merged = filtered + css_parts
                        params[i] = f"style: TextStyle({', '.join(merged)})"
                    break
            else:
                params.append(f"style: TextStyle({', '.join(css_parts)})")

        return params

    # ── View Tree → Flutter Widgets ────────────────────

    def _gen_view_tree(self, view) -> str:
        if view is None:
            return "const SizedBox.shrink()"
        if isinstance(view, str):
            return self._escape_dart(view)
        if isinstance(view, list):
            parts = [self._gen_view_tree(c) for c in view if self._gen_view_tree(c).strip()]
            if len(parts) == 1:
                return parts[0]
            return "\n".join(parts)
        if isinstance(view, dict):
            kind = self._get(view, "_kind", "")
            if kind == "Interpolation":
                expr = self._expr_to_dart(self._get(view, "expr"))
                return f'"${{{expr}}}"' if not expr.startswith('"') else expr
            if kind == "ListRenderingIntent" or "iterable" in view:
                return self._gen_list(view)
            if kind == "ConditionalIntent" or "branches" in view:
                return self._gen_conditional(view)
            if kind == "ErrorBoundary":
                return self._gen_error_boundary(view)
            if "tag" in view:
                return self._gen_element(view)
        return "const SizedBox.shrink()"

    def _gen_element(self, elem: dict) -> str:
        tag = self._get(elem, "tag", "Container")
        props = self._get(elem, "props", {})
        bindings = self._get(elem, "bindings", {})
        events = self._get(elem, "events", {})
        children = self._get(elem, "children", [])
        test_id = self._get(elem, "test_id")

        # CSS class lookup
        css_rule = {}
        class_prop = props.pop("class", None)
        if class_prop:
            class_name = self._get(class_prop, "value", str(class_prop))
            css_rule = self._css_rules.get(class_name, {})

        # Build widget params
        params = []

        # Layout → Flex/Row/Column
        if tag == "Flex":
            direction = "row"
            for pname, pval in props.items():
                if pname == "direction":
                    direction = self._extract_style_val(pval)
            widget = "Row" if direction == "row" else "Column"
            layout_props = self._build_layout_params(props, children)
            params.extend(layout_props)
            # Flutter Column defaults to center — override to start for editor alignment
            if widget == "Column" and not any(p.startswith("crossAxisAlignment:") for p in params):
                params.append("crossAxisAlignment: CrossAxisAlignment.start")
            return self._build_widget(widget, params, children, test_id)

        if tag == "Grid":
            params.append("gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(maxCrossAxisExtent: 250)")
            return self._build_widget("GridView", params, children, test_id)

        if tag == "Spacer":
            return "const Spacer(flex: 1)"

        if tag == "Container":
            container_params = []
            # Apply inline props + CSS rules
            for pname, pval in props.items():
                if pname == "padding":
                    val = self._extract_style_val(pval)
                    container_params.append(f"padding: const EdgeInsets.all({val})")
            if css_rule:
                container_params, children = self._apply_css_to_container(css_rule, container_params, children)
            # Wrap multiple children in Column
            if len(children) > 1:
                inner_parts = []
                for child in children:
                    cs = self._gen_view_tree(child)
                    if cs.strip():
                        inner_parts.append(cs)
                inner = "Column(children: [" + ", ".join(inner_parts) + "])"
                container_params.append(f"child: {inner}")
                return self._build_widget("Container", container_params, [], test_id)
            return self._build_widget("Container", container_params, children, test_id)

        if tag == "Heading":
            level_prop = props.get("level", {})
            level = self._get(level_prop, "value", "2")
            sizes = {"1": "28", "2": "20", "3": "16", "4": "15", "5": "14", "6": "13"}
            size = sizes.get(level, "20")
            params.append(f"style: TextStyle(fontSize: {size}, fontWeight: FontWeight.w600)")
            if css_rule:
                params = self._apply_css_to_text(css_rule, params)
            text_content = self._build_text_content(children)
            if text_content:
                params.append(text_content)
            return self._build_widget("Text", params, [], test_id)

        if tag == "Text":
            if css_rule:
                params = self._apply_css_to_text(css_rule, params)
            text_content = self._build_text_content(children)
            if text_content:
                params.append(text_content)
            return self._build_widget("Text", params, [], test_id)

        if tag == "Link":
            # GestureDetector wrapping Text
            child_content = self._build_text_content(children)
            text_params = [child_content] if child_content else []
            inner = f"Text({', '.join(text_params)})" if text_params else "const Text('')"
            for ename, ehandler in events.items():
                handler = self._gen_event_handler(ehandler)
                params.append(f"onTap: {handler}")
            params.append(f"child: {inner}")
            return f"GestureDetector({', '.join(params)})" if params else "const GestureDetector()"

        if tag == "Button":
            # Map variant → Flutter button type
            variant = props.pop("variant", None) if isinstance(props, dict) else None
            variant_val = self._extract_style_val(variant) if variant else "primary"
            variant_map = {"primary": "ElevatedButton", "outline": "OutlinedButton",
                           "ghost": "TextButton", "secondary": "OutlinedButton"}
            flutter_button = variant_map.get(variant_val, "ElevatedButton")
            # Add default styling matching editor design tokens
            has_style = any(p.startswith("style:") for p in params)
            if not has_style:
                if flutter_button == "ElevatedButton":
                    params.append(
                        "style: ElevatedButton.styleFrom("
                        "backgroundColor: const Color(0xFF4338CA), "
                        "foregroundColor: Colors.white, "
                        "shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))"
                        ")"
                    )
                elif flutter_button == "OutlinedButton":
                    params.append(
                        "style: OutlinedButton.styleFrom("
                        "foregroundColor: const Color(0xFF374151), "
                        "side: const BorderSide(color: Color(0xFFE5E7EB)), "
                        "shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))"
                        ")"
                    )
                elif flutter_button == "TextButton":
                    params.append(
                        "style: TextButton.styleFrom("
                        "foregroundColor: const Color(0xFF4338CA)"
                        ")"
                    )
            for ename, ehandler in events.items():
                handler = self._gen_event_handler(ehandler)
                params.append(f"onPressed: {handler}")
            # Ensure onPressed is always provided (Flutter requirement)
            if not any(p.startswith("onPressed:") for p in params):
                params.append("onPressed: null")
            # Wrap button label in Text widget
            label = self._build_text_content(children) if children else '"Button"'
            params.append(f"child: Text({label})")
            return self._build_widget(flutter_button, params, [], test_id)

        if tag == "Input":
            for bname, bref in bindings.items():
                if bname == "value":
                    sig_name = self._get(bref, "name", "")
                    params.append(f"controller: _{sig_name}Controller")
            for pname, pval in props.items():
                if pname in ("placeholder",):
                    val = self._expr_to_dart(pval)
                    params.append(f"decoration: const InputDecoration(hintText: {val})")
            for ename, ehandler in events.items():
                if ename in ("onChange",):
                    handler = self._gen_event_handler(ehandler)
                    params.append(f"onChanged: {handler}")
            return self._build_widget("TextField", params, children, test_id)

        if tag == "Image":
            for pname, pval in props.items():
                if pname == "src":
                    val = self._expr_to_dart(pval)
                    params.append(f"image: NetworkImage({val})")
            return self._build_widget("Image", params, children, test_id)

        if tag == "List":
            return self._build_widget("ListView", params, children, test_id)

        if tag == "Card":
            # Card defaults: white background, 8px radius, elevation shadow
            has_color = any(p.startswith("color:") for p in params)
            has_elevation = any(p.startswith("elevation:") for p in params)
            has_shape = any(p.startswith("shape:") for p in params)
            if not has_color:
                params.append("color: Colors.white")
            if not has_elevation:
                params.append("elevation: 2")
            if not has_shape and not any("borderRadius" in p for p in params):
                params.append("shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8))")
            return self._build_widget("Card", params, children, test_id)

        if tag == "Checkbox":
            for bname, bref in bindings.items():
                if bname == "checked":
                    sig_name = self._get(bref, "name", "")
                    params.append(f"value: _{sig_name}")
                    params.append(f"onChanged: (v) => setState(() => _{sig_name} = v ?? false)")
            return self._build_widget("Checkbox", params, children, test_id)

        # Default: generic widget
        return self._build_widget(self.ELEMENT_MAP.get(tag, "Container"), params, children, test_id)

    # Widgets that accept `children: [...]` (multi-child)
    _CHILDREN_WIDGETS = {"Row", "Column", "Stack", "ListView", "GridView", "Wrap", "Flex"}

    def _build_widget(self, widget: str, params: list, children: list, test_id: str | None = None) -> str:
        """Build a Flutter widget constructor call.

        Handles both single-child widgets (Text, Button, Container)
        and multi-child widgets (Row, Column, Stack, ListView).
        """
        parts = []
        parts.extend(params)
        if test_id:
            parts.append(f'key: const ValueKey("{test_id}")')

        child_strs = []
        for child in children:
            cs = self._gen_view_tree(child)
            if cs.strip():
                child_strs.append(cs)

        # Widget-specific child parameter naming
        _CHILD_PARAM = {"ListTile": "title", "ListItem": "title"}

        if child_strs:
            if widget in self._CHILDREN_WIDGETS:
                children_str = ", ".join(child_strs)
                parts.append(f"children: [{children_str}]")
            elif len(child_strs) > 1:
                # Multiple children for a single-child widget — wrap in Column
                children_str = ", ".join(child_strs)
                parts.append(f"child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [{children_str}])")
            else:
                # Single child — wrap in Text if it's a plain string
                child_content = child_strs[0]
                child_param = _CHILD_PARAM.get(widget, "child")
                if child_param == "child" and widget in ("ElevatedButton", "TextButton", "OutlinedButton",
                              "GestureDetector", "Container", "Card", "Form"):
                    if not child_content.startswith(('Text(', 'Icon(', 'const ', 'SizedBox', 'Column(',
                                                      'Row(', 'ListView(', 'Stack(')) and '"' in child_content:
                        child_content = f"Text({child_content})"
                parts.append(f"{child_param}: {child_content}")

        param_str = ", ".join(parts) if parts else ""
        return f"{widget}({param_str})" if param_str else f"const {widget}()"

    def _build_text_content(self, children: list) -> str:
        """Build a Flutter text string expression from text children.

        Joins plain text and interpolations into a single Dart string expression.
        <Text>Hello {name}!</Text> → 'Hello $name!'
        """
        parts = []
        for child in children:
            cs = self._gen_view_tree(child)
            if not cs.strip():
                continue
            # Interpolation returns "${expr}" — strip outer quotes for Dart
            if cs.startswith('"') and cs.endswith('"'):
                parts.append(cs[1:-1])
            else:
                parts.append(cs)
        if not parts:
            return ""
        content = "".join(parts)
        # If content has variable references, use raw string; otherwise quote it
        if "$" in content:
            return f"'{content}'"
        return f'"{content}"'

    def _build_layout_params(self, props: dict, children: list) -> list:
        """Convert layout props to Flutter Row/Column params."""
        params = []
        for pname, pval in props.items():
            if pname == "gap":
                val = self._extract_style_val(pval)
                params.append(f"spacing: {val}")
            elif pname == "align":
                raw = self._extract_style_val(pval)
                if raw == "center":
                    params.append("mainAxisAlignment: MainAxisAlignment.center")
                    params.append("crossAxisAlignment: CrossAxisAlignment.center")
                elif raw == "start":
                    params.append("crossAxisAlignment: CrossAxisAlignment.start")
            elif pname == "justify":
                raw = self._extract_style_val(pval)
                jmap = {"center": "MainAxisAlignment.center", "start": "MainAxisAlignment.start",
                        "end": "MainAxisAlignment.end", "space_between": "MainAxisAlignment.spaceBetween"}
                if raw in jmap:
                    params.append(f"mainAxisAlignment: {jmap[raw]}")
        return params

    def _gen_conditional(self, cond: dict) -> str:
        branches = self._get(cond, "branches", [])
        fallback = self._get(cond, "fallback")
        # For single if with fallback, use ternary
        if len(branches) == 1 and fallback:
            c = self._expr_to_dart(self._get(branches[0], "condition"))
            then_body = self._gen_view_tree(self._get(branches[0], "body"))
            else_body = self._gen_view_tree(fallback)
            # Flutter: null can't be assigned to Widget — use SizedBox.shrink()
            if else_body.strip() == "null":
                else_body = "const SizedBox.shrink()"
            elif then_body.strip() == "null":
                then_body = "const SizedBox.shrink()"
            return f"({c}) ? {then_body} : {else_body}"
        # Multiple branches: use immediate anonymous function
        L = []
        for branch in branches:
            c = self._expr_to_dart(self._get(branch, "condition"))
            body = self._gen_view_tree(self._get(branch, "body"))
            L.append(f"if ({c}) {{ return {body}; }}")
        if fallback:
            fb = self._gen_view_tree(fallback)
            L.append(f"return {fb};")
        else:
            L.append("return const SizedBox.shrink();")
        inner = " ".join(L)
        return f"(() {{ {inner} }})()"

    def _gen_error_boundary(self, boundary: dict) -> str:
        """Generate Flutter error boundary using ErrorWidget.builder pattern.

        Flutter uses ErrorWidget.builder for global error handling.
        For per-widget boundaries, we emit a comment marker and wrap content.
        """
        name = self._get(boundary, "name", "error")
        children = self._get(boundary, "children", [])
        child_widgets = "\n".join(self._gen_view_tree(c) for c in children)
        # Flutter L1: emit as-is with a comment — full ErrorWidget integration in L2
        return (
            f"// @error({name}) — error boundary\n"
            f"{child_widgets}"
        )

    def _gen_list(self, lst: dict) -> str:
        iterable = self._expr_to_dart(self._get(lst, "iterable"))
        binding = self._get(lst, "item_binding", {})
        item_name = self._get(binding, "name", "item")
        self._for_item_names.add(item_name)
        body = self._gen_view_tree(self._get(lst, "body"))
        self._for_item_names.discard(item_name)
        key_expr = self._expr_to_dart(self._get(lst, "key_expr"))
        # L2 optimization hint
        return (
            f"// L2 optimization: use ListView.builder for virtual scrolling\n"
            f"// Currently using Column + .map() — see docs for L1→L2 upgrade\n"
            f"Column(children: {iterable}.map(({item_name}) => {body}).toList())"
        )

    # ── Event Handlers ─────────────────────────────────

    def _gen_event_handler(self, handler: dict) -> str:
        kind = self._get(handler, "kind", "inline_fn")
        if kind in ("fn_ref", "FN_REF"):
            ref = self._get(handler, "ref", "")
            return f"() => _{ref}()"
        if kind in ("store_action", "STORE_ACTION"):
            ref = self._get(handler, "ref", "")
            parts = ref.split(".")
            if len(parts) == 2:
                store_alias, action = parts
                return f"() => {store_alias}.{action}()"
            return f"() => {{}}"
        # Inline
        body = self._gen_block_body_dart_hand(self._get(handler, "body"))
        return f"() {{ {body} }}"

    # ── Expression → Dart ──────────────────────────────

    def _expr_to_dart(self, expr) -> str:
        if expr is None:
            return "null"
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
                # Resolve "store" → actual store variable name
                if store_alias == "store":
                    store_names = [k for k, v in self._all_stores.items()
                                  if isinstance(v, dict) and "signals" in v and "view" not in v]
                    if store_names:
                        alias = store_names[0].lower()
                        if alias.endswith("store"):
                            alias = alias[:-5]
                        store_alias = alias
                field = self._get(expr, "field_name", "")
                args = self._get(expr, "call_args", [])
                args_str = ", ".join(self._expr_to_dart(a) for a in args)
                return f"{store_alias}.{field}({args_str})"
            if kind == "FieldAccessExpr":
                obj = self._get(expr, "object")
                field = self._get(expr, "field", "")
                obj_str = self._expr_to_dart(obj)
                return f"{obj_str}.{field}"
            if kind == "FormatCall":
                return self._gen_format_call(expr)
            if kind == "TranslateCall":
                key = self._get(expr, "key", "unknown")
                return f"AppLocalizations.of(context)!.{key}"
            if kind == "IdentifierExpr" or ("name" in expr and "op" not in expr and "value" not in expr and len(expr) <= 2):
                name = self._get(expr, "name", "")
                if name == "store":
                    # Resolve to the actual store variable name (derived alias)
                    # Only look at actual stores (have signals, no view — not components)
                    store_names = [k for k, v in self._all_stores.items()
                                  if isinstance(v, dict) and "signals" in v and "view" not in v]
                    if store_names:
                        alias = store_names[0].lower()
                        if alias.endswith("store"):
                            alias = alias[:-5]
                        return alias
                    return "store"
                # Store aliases and @for item names — no underscore prefix
                if name in self._store_aliases:
                    return name
                if name in self._for_item_names:
                    return name  # @for loop item variable
                if self._in_store_action:
                    return name  # Store fields are public, no underscore
                return f"_{name}" if name else "null"
            if kind == "LiteralExpr" or ("value" in expr and "op" not in expr and "name" not in expr):
                val = self._get(expr, "value")
                if isinstance(val, str):
                    if val in ("[]", "{}"):
                        return val
                    if val.startswith("[") or val.startswith("{"):
                        return val
                    escaped = val.replace(r'$', r'\$')
                    return f'"{escaped}"'
                if isinstance(val, bool):
                    return "true" if val else "false"
                return str(val) if val is not None else "null"
            if kind == "ArrayExpr" or ("elements" in expr and "_kind" not in expr):
                parts = []
                for elem in self._get(expr, "elements", []):
                    if isinstance(elem, dict) and elem.get("_kind") == "SpreadExpr":
                        parts.append("..." + self._expr_to_dart(elem.get("expr")))
                    else:
                        parts.append(self._expr_to_dart(elem))
                return "[" + ", ".join(parts) + "]"
            if kind == "BinaryOpExpr" or ("op" in expr and "left" in expr):
                left = self._expr_to_dart(self._get(expr, "left"))
                right = self._expr_to_dart(self._get(expr, "right"))
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
                return f"{op}{self._expr_to_dart(self._get(expr, 'operand'))}"
            if "name" in expr:
                return f"_{self._get(expr, 'name', '')}"
        return "null"

    def _gen_format_call(self, expr: dict) -> str:
        func = self._get(expr, "func", "")
        args = [self._expr_to_dart(a) for a in self._get(expr, "args", [])]
        if func == "format_date":
            val = args[0] if args else "''"
            return f"DateTime.parse({val}).toIso8601String()"
        elif func == "format_number":
            val = args[0] if args else "0"
            decimals = args[1] if len(args) > 1 else "2"
            return f"{val}.toStringAsFixed({decimals})"
        return "null"

    # ── Statement Blocks (Dart) ────────────────────────

    def _gen_block_body_dart(self, body, signal_names: set) -> str:
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
                amount = self._expr_to_dart(self._get(intent, "amount", 1))
                item = self._expr_to_dart(self._get(intent, "item"))

                if target_owner:
                    # Store mutation — target is a public field on the ChangeNotifier
                    if op in ("increment", "INCREMENT"):
                        L.append(f"{target} += {amount};")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"{target}.add({item});")
                    else:
                        val = self._expr_to_dart(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"{target} = {val};")
                else:
                    if op in ("increment", "INCREMENT"):
                        L.append(f"_{target} += {amount};")
                    elif op in ("decrement", "DECREMENT"):
                        L.append(f"_{target} -= {amount};")
                    elif op in ("spread_append", "SPREAD_APPEND"):
                        L.append(f"_{target} = [..._{target}, {item}];")
                    else:
                        val = self._expr_to_dart(self._get(intent, "item", self._get(intent, "amount")))
                        L.append(f"_{target} = {val};")
            elif kind in ("ExprStmt", "ExprStmtIR"):
                L.append(f"{self._expr_to_dart(self._get(stmt, 'expr'))};")
            elif kind in ("IfStmt", "IfStmtIR"):
                cond = self._expr_to_dart(self._get(stmt, "condition"))
                L.append(f"if ({cond}) {{")
                L.append(self.indent(self._gen_block_body_dart(self._get(stmt, "then_body"), signal_names)))
                if self._get(stmt, "else_body"):
                    L.append("} else {")
                    L.append(self.indent(self._gen_block_body_dart(self._get(stmt, "else_body"), signal_names)))
                L.append("}")
        return "\n".join(L)

    def _gen_block_body_dart_hand(self, body) -> str:
        """Generate inline handler body."""
        if body is None:
            return ""
        stmts = self._get(body, "statements", []) if isinstance(body, dict) else []
        parts = []
        for stmt in stmts:
            if not isinstance(stmt, dict):
                continue
            kind = self._get(stmt, "_kind", "")
            if kind == "MutationStmt" or "intent" in stmt:
                intent = self._get(stmt, "intent", {})
                target_ref = self._get(intent, "target", {})
                target = self._get(target_ref, "name", "x")
                op = self._get(intent, "operation", "direct_assign")
                if op in ("increment", "INCREMENT"):
                    parts.append(f"setState(() => _{target}++)")
                elif op in ("spread_append", "SPREAD_APPEND"):
                    item = self._expr_to_dart(self._get(intent, "item"))
                    parts.append(f"setState(() => _{target} = [..._{target}, {item}])")
                else:
                    val = self._expr_to_dart(self._get(intent, "item", self._get(intent, "amount")))
                    parts.append(f"setState(() => _{target} = {val})")
        return "; ".join(parts)

    # ── Store Generation (Simple ChangeNotifier) ───────

    def _gen_store(self, store: dict, source_line: int = 1) -> str:
        name = self._get(store, "name", "Store")
        signals = self._get(store, "signals", {})
        getters = self._get(store, "getters", {})
        actions = self._get(store, "actions", [])
        defaults = self._get(store, "signal_defaults", {})

        L = []
        L.append("import 'package:flutter/material.dart';")
        L.append("")
        L.append(f"// .omni:{source_line} -- store {name}")
        L.append(f"class {name} extends ChangeNotifier {{")

        # Fields — public for direct access (ChangeNotifier pattern)
        for sig_name, sig_type in signals.items():
            dart_type = self._map_type_to_dart(sig_type)
            if sig_name in defaults:
                init_val = self._expr_to_dart(defaults[sig_name])
            else:
                init_val = self._type_default_dart(sig_type)
            L.append(f"  {dart_type} {sig_name} = {init_val};")

        L.append("")

        # Getters
        for getter_name, getter_body in getters.items():
            self._in_store_action = True
            self._store_signal_names = set(signals.keys())
            body_str = self._expr_to_dart(getter_body)
            # Infer return type from body
            ret_type = "dynamic"
            if isinstance(getter_body, dict):
                name = getter_body.get("name", "")
                if name in signals:
                    ret_type = self._map_type_to_dart(signals[name])
            L.append(f"  {ret_type} get {getter_name} => {body_str};")
            self._in_store_action = False

        L.append("")

        # Actions
        for action in actions:
            aname = self._get(action, "name", "action")
            params = self._get(action, "params", [])
            pstr = ", ".join(
                f"{self._map_type_to_dart(self._get(p, 'type'))} {self._get(p, 'name', 'x')}"
                for p in params
            )
            self._in_store_action = True
            body = self._gen_block_body_dart(self._get(action, "body"), set(signals.keys()))
            self._in_store_action = False
            L.append(f"  void {aname}({pstr}) {{")
            for line in body.strip().split("\n"):
                if line.strip():
                    L.append(f"    {line}")
            L.append(f"    notifyListeners();")
            L.append(f"  }}")
            L.append("")

        L.append("}")
        return "\n".join(L)

    # ── Helpers ────────────────────────────────────────

    def _map_type_to_dart(self, type_ir) -> str:
        if type_ir is None:
            return "dynamic"
        if isinstance(type_ir, str):
            M = {"i32": "int", "f64": "double", "bool": "bool",
                 "String": "String", "void": "void", "Array": "List<String>",
                 "Map": "Map<String, String>"}
            return M.get(type_ir, type_ir)
        if isinstance(type_ir, dict):
            k = self._get(type_ir, "kind", self._get(type_ir, "name", "dynamic"))
            M = {"i32": "int", "I32": "int", "f64": "double", "F64": "double",
                 "bool": "bool", "BOOL": "bool", "String": "String", "STRING": "String",
                 "void": "void", "VOID": "void", "Array": "List<String>", "ARRAY": "List<String>",
                 "Map": "Map<String, String>", "MAP": "Map<String, String>"}
            return M.get(k, k)
        return "dynamic"

    def _type_default_dart(self, type_ir) -> str:
        if isinstance(type_ir, str):
            D = {"i32": "0", "f64": "0.0", "bool": "false", "String": "''",
                 "Array": "[]", "Map": "{}", "void": "null"}
            return D.get(type_ir, "null")
        if isinstance(type_ir, dict):
            k = self._get(type_ir, "kind", self._get(type_ir, "name", ""))
            D = {"i32": "0", "I32": "0", "f64": "0.0", "F64": "0.0",
                 "bool": "false", "BOOL": "false", "String": "''", "STRING": "''",
                 "Array": "[]", "ARRAY": "[]", "Map": "{}", "MAP": "{}"}
            return D.get(k, "null")
        return "null"

    @staticmethod
    def _extract_style_val(value) -> str:
        raw = str(value.get("value", value)) if isinstance(value, dict) else str(value)
        for unit in ("px", "rem", "em", "vh", "vw", "%"):
            if raw.endswith(unit):
                return raw[:-len(unit)]
        return raw

    @staticmethod
    def _escape_dart(text: str) -> str:
        if text is None:
            return ""
        return text.replace("$", "\\$")
