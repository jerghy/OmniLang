"""OmniLang editor — .omni.json importer (`.omni.json` snapshot → `.omni` source text).

The importer reconstructs human-readable .omni source from a structured .omni.json
snapshot. It generates structurally valid .omni text, then optionally passes it
through OmniViewFormatter for canonical formatting and diff stability.
"""

from __future__ import annotations

from typing import Any


class OmniJsonImporter:
    """Reconstruct .omni source text from an OmniJsonDocument snapshot.

    Usage:
        importer = OmniJsonImporter()
        source = importer.import_document(doc)          # Basic reconstruction
        source = importer.import_document(doc, format=True)  # Canonical formatting
    """

    # ── Public API ──────────────────────────────────────────

    def import_document(self, doc, format: bool = True) -> str:
        """Reconstruct .omni source from an OmniJsonDocument.

        Args:
            doc: OmniJsonDocument instance
            format: If True (default), pass through OmniViewFormatter for
                    canonical output and diff stability.

        Returns:
            Reconstructed .omni source text.
        """
        source = self._reconstruct(doc)

        if format:
            try:
                source = self._format_source(source)
            except Exception:
                # If formatting fails, return raw reconstruction
                pass

        # Re-inject @editor:managed marker after formatting (formatter may strip comments).
        # The marker is the contract for hand-written detection on re-import.
        if not getattr(getattr(doc, 'meta', None), 'is_hand_written', True):
            marker = "// @editor:managed\n"
            if not source.startswith(marker):
                source = marker + source

        return source

    def import_from_dict(self, data: dict[str, Any], format: bool = True) -> str:
        """Reconstruct from a plain dict (deserialized JSON).

        Args:
            data: Dict from OmniJsonSerializer.to_dict()
            format: Whether to apply canonical formatting

        Returns:
            Reconstructed .omni source text.
        """
        from editor.server.omni_json.serializer import OmniJsonSerializer
        serializer = OmniJsonSerializer()
        doc = serializer.from_dict(data)
        return self.import_document(doc, format=format)

    # ── Reconstruction ──────────────────────────────────────

    def _reconstruct(self, doc) -> str:
        """Build .omni source text from the document snapshot."""
        parts: list[str] = []

        # Inject @editor:managed marker so re-import preserves managed status.
        # Without this, the hand-written detection heuristic (no markers + has comments)
        # would incorrectly classify the reconstructed file as hand-written.
        if not getattr(getattr(doc, 'meta', None), 'is_hand_written', True):
            parts.append("// @editor:managed")

        # Imports
        for imp in getattr(doc, 'imports', []) or []:
            parts.append(imp)
        if parts:
            parts.append("")

        # Components
        for comp in getattr(doc, 'components', []) or []:
            parts.append(self._reconstruct_component(comp))
            parts.append("")

        # Stores
        for store in getattr(doc, 'stores', []) or []:
            parts.append(self._reconstruct_store(store))
            parts.append("")

        # Themes
        for theme in getattr(doc, 'themes', []) or []:
            parts.append(self._reconstruct_theme(theme))
            parts.append("")

        return "\n".join(parts).rstrip("\n") + "\n"

    def _reconstruct_component(self, comp) -> str:
        lines: list[str] = []
        comp_name = getattr(comp, 'name', 'Unnamed')
        lines.append(f"component {comp_name} {{")

        # Props
        for prop_name, prop_type in (getattr(comp, 'props', {}) or {}).items():
            lines.append(f"  prop {prop_name}: {prop_type};")

        # Signals
        for sig_name, sig_info in (getattr(comp, 'signals', {}) or {}).items():
            sig_type = "i32"
            sig_init = "0"
            if isinstance(sig_info, dict):
                sig_type = sig_info.get("type", "i32")
                sig_init = str(sig_info.get("init", "0"))
            lines.append(f"  signal {sig_name}: {sig_type} = {sig_init};")

        # Computeds
        for comp_name_inner, comp_info in (getattr(comp, 'computeds', {}) or {}).items():
            if isinstance(comp_info, dict):
                comp_type = comp_info.get("type", "i32")
                lines.append(f"  computed {comp_name_inner}: {comp_type};")

        # Events
        for evt in (getattr(comp, 'events', []) or []):
            lines.append(f"  event {evt};")

        # Refs
        for ref in (getattr(comp, 'refs', []) or []):
            lines.append(f"  ref {ref};")

        # Store refs
        for store_ref in (getattr(comp, 'store_refs', []) or []):
            lines.append(f"  @store {store_ref};")

        # Functions
        for fn in (getattr(comp, 'fns', []) or []):
            if isinstance(fn, dict):
                fn_name = fn.get("name", "fn")
                params = fn.get("params", [])
                ret_type = fn.get("returnType", "void")
                body = fn.get("body", "")
                param_str = ", ".join(
                    f"{p.get('name', 'x')}: {p.get('type', 'i32')}"
                    for p in params
                ) if params else ""
                lines.append(f"  fn {fn_name}({param_str}) -> {ret_type} {{")
                if body:
                    for body_line in body.split("\n"):
                        lines.append(f"    {body_line}")
                lines.append("  }")

        # Lifecycle hooks
        for hook_name, hook_bodies in (getattr(comp, 'lifecycles', {}) or {}).items():
            for hook_body in (hook_bodies or []):
                lines.append(f"  @{hook_name} {{")
                if hook_body:
                    for bl in hook_body.split("\n"):
                        lines.append(f"    {bl}")
                lines.append("  }}")

        # View
        view = getattr(comp, 'view', None)
        if view is not None:
            view_text = self._reconstruct_view_node(view, indent=2)
            lines.append(f"  view {{\n{view_text}\n  }}")

        # Style
        style = getattr(comp, 'style', {}) or {}
        for class_name, props in style.items():
            lines.append(f"  style {class_name} {{")
            for prop_name, prop_val in (props or {}).items():
                lines.append(f"    {prop_name}: {prop_val};")
            lines.append("  }")

        lines.append("}")
        return "\n".join(lines)

    def _reconstruct_store(self, store) -> str:
        lines: list[str] = []
        store_name = getattr(store, 'name', 'Unnamed')
        lines.append(f"store {store_name} {{")

        # Signals
        for sig_name, sig_type in (getattr(store, 'signals', {}) or {}).items():
            # sig_type may be a string directly, a dict with "type" key, or empty
            if isinstance(sig_type, dict):
                sig_type = sig_type.get("type", "")
            elif not isinstance(sig_type, str):
                sig_type = str(sig_type) if sig_type is not None else ""
            # Default: guess type from signal name or default value
            if not sig_type or not sig_type.strip():
                sig_type = self._guess_signal_type(sig_name, store)
            default = (getattr(store, 'signal_defaults', {}) or {}).get(sig_name, "0")
            # Quote string defaults
            if sig_type == "String":
                if isinstance(default, str) and default and not default.startswith('"'):
                    default = f'"{default}"'
                elif not default or default == "0":
                    default = '""'
            lines.append(f"  signal {sig_name}: {sig_type} = {default};")

        # Getters
        for getter_name, getter_info in (getattr(store, 'getters', {}) or {}).items():
            ret = "i32"
            body_expr = "true"
            if isinstance(getter_info, dict):
                ret = getter_info.get("returnType", "")
                body_expr = getter_info.get("body", "") or "true"
                if not ret or not ret.strip():
                    ret = self._guess_signal_type(getter_name, store)
            lines.append(f"  getter {getter_name}: {ret} = {body_expr};")

        # Actions
        for action in (getattr(store, 'actions', []) or []):
            if isinstance(action, dict):
                action_name = action.get("name", "action")
                params = action.get("params", [])
                param_str = ", ".join(
                    f"{p.get('name', 'x')}: {p.get('type', 'i32')}"
                    for p in params
                ) if params else ""
                body = action.get("body", "")
                lines.append(f"  action {action_name}({param_str}) {{")
                if body:
                    for body_line in body.split("\n"):
                        lines.append(f"    {body_line}")
                lines.append("  }")

        lines.append("}")
        return "\n".join(lines)

    def _reconstruct_theme(self, theme) -> str:
        lines: list[str] = []
        theme_name = getattr(theme, 'name', 'theme')
        lines.append(f"theme {theme_name} {{")
        for cat_name, entries in (getattr(theme, 'categories', {}) or {}).items():
            lines.append(f"  {cat_name} {{")
            for key, val in (entries or {}).items():
                lines.append(f"    {key}: {val};")
            lines.append("  }")
        lines.append("}")
        return "\n".join(lines)

    def _reconstruct_view_node(self, node, indent: int = 0) -> str:
        """Recursively reconstruct view tree nodes."""
        pad = "  " * indent
        kind = getattr(node, '_kind', 'text')

        if kind == "text":
            text = getattr(node, 'text', '') or ""
            # Text nodes with {} content are actually interpolations in the view tree
            if text.startswith("{") and text.endswith("}"):
                return f"{pad}<Text>{text}</Text>"
            return f"{pad}<Text>{text}</Text>"

        if kind == "interpolation":
            expr = getattr(node, 'expression', '') or ""
            # Strip outer braces if already wrapped (from view tree converter)
            if expr.startswith("{") and expr.endswith("}"):
                expr = expr[1:-1]
            return f"{pad}<Text>{{{expr}}}</Text>"

        if kind == "element":
            return self._reconstruct_element(node, indent)

        if kind == "if":
            return self._reconstruct_if_directive(node, indent)

        if kind == "for":
            return self._reconstruct_for_directive(node, indent)

        if kind == "slot":
            slot_name = getattr(node, 'slot_name', '') or "default"
            children = getattr(node, 'children', []) or []
            if not children:
                return f"{pad}@slot {slot_name};"
            children_src = "\n".join(
                self._reconstruct_view_node(c, indent + 1) for c in children
            )
            return f"{pad}@slot {slot_name} {{\n{children_src}\n{pad}}}"

        if kind == "native":
            target = getattr(node, 'target', '') or "react"
            code = getattr(node, 'native_code', '') or ""
            return f"{pad}@native({target}) {{\n{pad}  {code}\n{pad}}}"

        if kind == "fragment":
            children = getattr(node, 'children', []) or []
            if not children:
                return ""
            return "\n".join(
                self._reconstruct_view_node(c, indent) for c in children
            )

        # Unknown kind — treat as pass-through fragment
        children = getattr(node, 'children', []) or []
        if children:
            return "\n".join(
                self._reconstruct_view_node(c, indent) for c in children
            )
        return f"{pad}<!-- unknown node kind: {kind} -->"

    def _reconstruct_element(self, node, indent: int = 0) -> str:
        pad = "  " * indent
        tag = getattr(node, 'tag', '') or "View"
        test_id = getattr(node, 'test_id', '') or ""

        props_parts: list[str] = []
        if test_id:
            props_parts.append(f'test-id="{test_id}"')

        for key, val in (getattr(node, 'props', {}) or {}).items():
            if key == "data-component" or key == "data-directive":
                continue  # Internal metadata, not source props
            # Convert snake_case prop names back to camelCase for .omni syntax
            prop_display = self._snake_to_camel(key)
            if isinstance(val, str):
                props_parts.append(f'{prop_display}="{val}"')
            elif isinstance(val, bool):
                props_parts.append(f'{prop_display}={str(val).lower()}')
            else:
                props_parts.append(f'{prop_display}={{{val}}}')

        # Bindings — OmniView syntax: bind:propName={signalName}
        for prop_name, sig_name in (getattr(node, 'bindings', {}) or {}).items():
            bind_display = self._snake_to_camel(prop_name)
            props_parts.append(f"bind:{bind_display}={{{sig_name}}}")

        # Events
        for evt_name, evt_info in (getattr(node, 'events', {}) or {}).items():
            ref = ""
            if isinstance(evt_info, dict):
                ref = evt_info.get("ref", "")
            else:
                ref = str(evt_info)
            # Strip trailing () from fn refs (display artifact from view tree converter)
            if ref.endswith("()"):
                ref = ref[:-2]
            # Convert snake_case event name back to camelCase (e.g. on_press → onPress)
            evt_display = self._snake_to_camel(evt_name)
            if ref:
                props_parts.append(f"{evt_display}={{{ref}}}")

        # Style classes
        for cls in (getattr(node, 'style_classes', []) or []):
            props_parts.append(f'className="{{{cls}}}"')

        props_str = " ".join(props_parts)
        if props_str:
            props_str = " " + props_str

        children = getattr(node, 'children', []) or []
        if not children:
            return f"{pad}<{tag}{props_str} />"

        children_src = "\n".join(
            self._reconstruct_view_node(c, indent + 1) for c in children
        )
        return f"{pad}<{tag}{props_str}>\n{children_src}\n{pad}</{tag}>"

    def _reconstruct_if_directive(self, node, indent: int = 0) -> str:
        pad = "  " * indent
        condition = getattr(node, 'condition', '') or "true"
        children = getattr(node, 'children', []) or []
        children_src = "\n".join(
            self._reconstruct_view_node(c, indent + 1) for c in children
        )

        lines = [f"{pad}@if {condition} {{\n{children_src}\n{pad}}}"]

        # Branches (else-if)
        for branch in (getattr(node, 'branches', []) or []):
            branch_cond = branch.get("condition", "true") if isinstance(branch, dict) else "true"
            branch_body = branch.get("body", []) if isinstance(branch, dict) else []
            branch_src = "\n".join(
                self._reconstruct_view_node(c, indent + 1) for c in branch_body
            )
            lines.append(f"{pad}@else if {branch_cond} {{\n{branch_src}\n{pad}}}")

        # Fallback (else)
        fallback = getattr(node, 'fallback', None)
        if fallback:
            fallback_src = "\n".join(
                self._reconstruct_view_node(c, indent + 1) for c in fallback
            )
            lines.append(f"{pad}@else {{\n{fallback_src}\n{pad}}}")

        return "\n".join(lines)

    def _reconstruct_for_directive(self, node, indent: int = 0) -> str:
        pad = "  " * indent
        iterable = getattr(node, 'iterable', '') or "items"
        item_binding = getattr(node, 'item_binding', '') or "item"
        key_expr = getattr(node, 'key_expr', '') or ""
        children = getattr(node, 'children', []) or []
        children_src = "\n".join(
            self._reconstruct_view_node(c, indent + 1) for c in children
        )

        key_part = f":{key_expr}" if key_expr else ""
        lines = [f"{pad}@for {item_binding} in {iterable}{key_part} {{\n{children_src}\n{pad}}}"]

        # Empty body
        empty_body = getattr(node, 'empty_body', None)
        if empty_body:
            empty_src = "\n".join(
                self._reconstruct_view_node(c, indent + 1) for c in empty_body
            )
            lines.append(f"{pad}@empty {{\n{empty_src}\n{pad}}}")

        return "\n".join(lines)

    # ── Name conversion ────────────────────────────────────

    @staticmethod
    def _guess_signal_type(sig_name: str, store=None) -> str:
        """Guess a signal type from naming conventions and defaults."""
        # Type hints from common naming patterns
        lower = sig_name.lower()
        if any(kw in lower for kw in ("loading", "is_", "has_", "should_", "can_")):
            return "bool"
        if any(kw in lower for kw in ("count", "index", "age", "total")):
            return "i32"
        if any(kw in lower for kw in ("price", "rate", "ratio", "amount")):
            return "f64"
        if any(kw in lower for kw in ("name", "title", "label", "text", "username", "email", "url")):
            return "String"
        # Check default value if store provides it
        if store:
            defaults = getattr(store, 'signal_defaults', {}) or {}
            default = defaults.get(sig_name)
            if isinstance(default, bool):
                return "bool"
            if isinstance(default, float):
                return "f64"
            if isinstance(default, int):
                return "i32"
            if isinstance(default, str) and default:
                return "String"
        return "i32"

    @staticmethod
    def _snake_to_camel(name: str) -> str:
        """Convert snake_case to camelCase for .omni syntax output."""
        if "_" not in name:
            return name
        parts = name.split("_")
        return parts[0] + "".join(p.capitalize() for p in parts[1:])

    # ── Formatting ──────────────────────────────────────────

    @staticmethod
    def _format_source(source: str) -> str:
        """Pass reconstructed source through OmniViewFormatter for canonical output."""
        from omniview.formatter import OmniViewFormatter
        formatter = OmniViewFormatter()
        return formatter.format(source)


# ── Module-level convenience ──────────────────────────────────


def import_omni_json(data: dict[str, Any], format: bool = True) -> str:
    """Convenience function: import a .omni.json dict and return .omni source."""
    importer = OmniJsonImporter()
    return importer.import_from_dict(data, format=format)
