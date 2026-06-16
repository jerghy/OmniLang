"""Reverse codegen: editor canvas state → .omni source.

Converts the editor's .omni.json document (elements + wires + code tabs)
back into valid .omni source code. This completes the round-trip:
  .omni source → parse → visual editor → generate → .omni source

Usage:
    from omniview.codegen.source import OmniViewSourceCodegen
    codegen = OmniViewSourceCodegen()
    source = codegen.generate(document, component_name="App")
"""

from __future__ import annotations

from typing import Any


# ── Types ──────────────────────────────────────────────────

# Expected .omni.json document shape:
# {
#   "design": { "elements": [...], "wires": [...] },
#   "codeTabs": { "tabs": [...], "openTabs": [...], "activeTab": "main" },
#   "config": { "target": "react" },
#   "meta": { "projectName": "..." },
# }


class OmniViewSourceCodegen:
    """Reverse codegen: editor canvas state → .omni source."""

    def __init__(self, indent_size: int = 4):
        self.indent_size = indent_size
        self._indent_level = 0

    # ── Public API ─────────────────────────────────────────

    def generate(
        self, document: dict[str, Any], component_name: str | None = None
    ) -> str:
        """Generate complete .omni source from a .omni.json document.

        Args:
            document: The full .omni.json document dict (matching the
                      structure constructed by editor save flow).
            component_name: Name of the top-level component.
                            Auto-detected from meta.projectName if not provided.

        Returns:
            Valid .omni source code string.
        """
        if component_name is None:
            meta = document.get("meta", {}) or {}
            component_name = meta.get("projectName", "") or "App"

        design = document.get("design", {}) or {}
        design_elements: list[dict] = design.get("elements", [])
        design_wires: list[dict] = design.get("wires", [])

        code_tabs_snapshot = document.get("codeTabs", {}) or {}
        tabs: list[dict] = code_tabs_snapshot.get("tabs", [])

        config = document.get("config", {}) or {}

        lines: list[str] = []
        self._indent_level = 0
        _i = self._i

        # ── 1. Top-level declarations: signals, computeds ──
        signals = self._collect_signals(tabs)
        computeds = self._collect_computeds(tabs)

        # ── 2. Function definitions (from code tabs) ──
        fn_defs: list[list[str]] = []
        for tab in tabs:
            fn_lines = self._emit_tab_functions(tab)
            if fn_lines:
                fn_defs.append(fn_lines)

        # ── 3. View block ──
        view_lines = self._emit_view(design_elements, design_wires)

        # ── 4. Assemble component ──
        lines.append(f"component {component_name} {{")

        self._indent_level = 1

        # Signals
        for sig in signals:
            lines.append(f"{_i()}{sig}")
        if signals:
            lines.append("")

        # Computeds
        for comp in computeds:
            lines.append(f"{_i()}{comp}")
        if computeds:
            lines.append("")

        # Functions
        for fn_group in fn_defs:
            for fl in fn_group:
                lines.append(f"{_i()}{fl}")
            lines.append("")

        # View
        if view_lines:
            for vl in view_lines:
                lines.append(vl)
        else:
            lines.append(f"{_i()}view {{")
            lines.append(f"{_i()}}}")

        self._indent_level = 0
        lines.append("}")

        return "\n".join(lines) + "\n"

    # ── Indentation helper ─────────────────────────────────

    def _i(self) -> str:
        return " " * (self._indent_level * self.indent_size)

    # ── Signal / Computed collection ───────────────────────

    def _collect_signals(self, tabs: list[dict]) -> list[str]:
        """Collect signal declarations from all code tabs.

        Sources:
        - Tab-level `variables` array (SignalDefinition[])
        - Graph elements with tag="signal"
        Deduplicated by name.
        """
        seen: set[str] = set()
        result: list[str] = []

        for tab in tabs:
            graph = (tab.get("graph", {}) or {})
            elements: list[dict] = graph.get("elements", [])

            # Collect from graph elements (signal nodes)
            for el in elements:
                if el.get("tag") != "signal":
                    continue
                props = el.get("props", {}) or {}
                name = props.get("name", "")
                if not name or name in seen:
                    continue
                seen.add(name)
                type_ann = props.get("typeAnnotation", "") or "i32"
                source = (el.get("source") or "").strip()
                # If source field is present, use it verbatim
                if source:
                    result.append(source)
                else:
                    result.append(f"signal {name}: {type_ann} = 0;")

            # Collect from tab variables
            variables: list[dict] = tab.get("variables", []) or []
            for var in variables:
                name = var.get("name", "")
                if not name or name in seen:
                    continue
                seen.add(name)
                vtype = var.get("type", "i32")
                default = var.get("defaultValue")
                if default is None:
                    default_str = "0" if vtype in ("i32", "f64") else '""'
                elif isinstance(default, str):
                    default_str = f'"{default}"'
                else:
                    default_str = str(default)
                result.append(f"signal {name}: {vtype} = {default_str};")

        return result

    def _collect_computeds(self, tabs: list[dict]) -> list[str]:
        """Collect computed declarations from all code tabs."""
        seen: set[str] = set()
        result: list[str] = []

        for tab in tabs:
            graph = (tab.get("graph", {}) or {})
            elements: list[dict] = graph.get("elements", [])
            for el in elements:
                if el.get("tag") != "computed":
                    continue
                props = el.get("props", {}) or {}
                name = props.get("name", "")
                if not name or name in seen:
                    continue
                seen.add(name)
                type_ann = props.get("typeAnnotation", "") or "i32"
                source = (el.get("source") or "").strip()
                if source:
                    result.append(source)
                else:
                    result.append(
                        f"computed {name}() -> {type_ann} {{ /* TODO */ }}"
                    )

        return result

    # ── Function emission from code tabs ───────────────────

    def _emit_tab_functions(self, tab: dict) -> list[str] | None:
        """Generate function definitions from a code tab.

        For each customFn node in the tab's graph, emit an action/fn block.
        """
        graph = (tab.get("graph", {}) or {})
        elements: list[dict] = graph.get("elements", [])
        wires: list[dict] = graph.get("wires", [])

        tab_type = tab.get("type", "logic")  # "event" | "logic"

        custom_fns = [el for el in elements if el.get("tag") == "customFn"]
        if not custom_fns:
            return None

        result_lines: list[str] = []
        for fn_el in custom_fns:
            fn_lines = self._emit_single_function(
                fn_el, elements, wires, tab_type
            )
            result_lines.extend(fn_lines)
            result_lines.append("")

        return result_lines

    def _emit_single_function(
        self,
        fn_el: dict,
        elements: list[dict],
        wires: list[dict],
        tab_type: str,
    ) -> list[str]:
        """Generate a single function definition from a customFn node + its wire graph.

        Priority: fn_el.source (hand-written) > graph reconstruction.
        """
        config = fn_el.get("config", {}) or {}
        fn_name = config.get("fnName", "") or fn_el.get("props", {}).get("name", "")
        fn_params: list[dict] = config.get("fnParams", []) or []
        fn_return_type = config.get("fnReturnType", "") or "void"
        description = config.get("description", "") or ""

        # Use hand-written source if present
        source = (fn_el.get("source") or "").strip()
        if source:
            lines: list[str] = [source]
            return lines

        # Build function signature
        if tab_type == "event":
            keyword = "action"
        else:
            keyword = "fn"

        params_str = ", ".join(
            f"{p.get('name', '')}: {p.get('type', 'any')}"
            for p in fn_params
        )

        if keyword == "fn" and fn_return_type:
            sig = f"{keyword} {fn_name}({params_str}) -> {fn_return_type} {{"
        else:
            sig = f"{keyword} {fn_name}({params_str}) {{"

        lines = []
        if description:
            lines.append(f"// {description}")
        lines.append(sig)

        # Reconstruct body from wire graph
        body_lines = self._reconstruct_function_body(
            fn_el, elements, wires
        )
        for bl in body_lines:
            lines.append(f"    {bl}")
        lines.append("}")

        return lines

    # ── Function body reconstruction from wire graph ───────

    def _reconstruct_function_body(
        self,
        fn_el: dict,
        elements: list[dict],
        wires: list[dict],
    ) -> list[str]:
        """Reconstruct function body statements from the wire graph.

        Algorithm:
        1. Build forward wire map: fromId → [(toId, toPort, wire)]
        2. Find terminal operations: wires entering signal.set or return.value
        3. Trace each terminal backward through the expression graph
        4. Emit statements
        """
        # Build element index
        el_map: dict[str, dict] = {el["id"]: el for el in elements}

        # Build forward wire map: fromId → list of wires
        fwd_map: dict[str, list[dict]] = {}
        for w in wires:
            fwd_map.setdefault(w.get("fromId", ""), []).append(w)

        # Also build backward map: toId → list of wires (for upward tracing)
        bwd_map: dict[str, list[dict]] = {}
        for w in wires:
            bwd_map.setdefault(w.get("toId", ""), []).append(w)

        statements: list[str] = []

        # Collect customFn's own parameter names for scope
        config = fn_el.get("config", {}) or {}
        fn_params: list[dict] = config.get("fnParams", []) or []
        param_names = {p.get("name", "") for p in fn_params}

        # Find all "set" operations (wires into signal.set ports)
        for w in wires:
            to_id = w.get("toId", "")
            to_port = w.get("toPort", "")
            to_el = el_map.get(to_id)

            if to_el and to_el.get("tag") == "signal" and to_port == "set":
                signal_name = to_el.get("props", {}).get("name", "")
                from_id = w.get("fromId", "")
                from_port = w.get("fromPort", "")
                expr = self._trace_expression(
                    from_id, from_port, el_map, fwd_map
                )
                if expr:
                    statements.append(f"{signal_name}.set({expr});")

        # Find return operations (wires into return.value)
        return_exprs: list[str] = []
        for w in wires:
            to_id = w.get("toId", "")
            to_port = w.get("toPort", "")
            to_el = el_map.get(to_id)

            if to_el and to_el.get("tag") == "return" and to_port == "value":
                from_id = w.get("fromId", "")
                from_port = w.get("fromPort", "")
                expr = self._trace_expression(
                    from_id, from_port, el_map, fwd_map
                )
                if expr:
                    return_exprs.append(expr)

        # Statements from set operations first, then return
        for s in statements:
            pass  # already collected above

        if return_exprs:
            for rexpr in return_exprs:
                statements.append(f"return {rexpr};")

        return statements

    def _trace_expression(
        self,
        from_id: str,
        from_port: str,
        el_map: dict[str, dict],
        fwd_map: dict[str, list[dict]],
        visited: set[str] | None = None,
    ) -> str:
        """Trace backward from an output port to reconstruct an expression string.

        Args:
            from_id: The source element ID (whose output port we're tracing from).
            from_port: The source port name.
            el_map: ID → element dict.
            fwd_map: fromId → [wire dicts] forward wire map.
            visited: Set of visited element IDs (+ port) to prevent infinite loops.

        Returns:
            Expression string (e.g., "count.value + 1", "42", "cartTotal.value + 999.99").
        """
        if visited is None:
            visited = set()

        visit_key = f"{from_id}:{from_port}"
        if visit_key in visited:
            return "..."  # cycle detected
        visited.add(visit_key)

        el = el_map.get(from_id)
        if not el:
            return str(from_id)  # fallback

        tag = el.get("tag", "")
        props = el.get("props", {}) or {}
        config = el.get("config", {}) or {}

        # ── Literal node ──
        if tag == "literal":
            cfg_value = config.get("value")
            if cfg_value is not None:
                if isinstance(cfg_value, bool):
                    return "true" if cfg_value else "false"
                if isinstance(cfg_value, str):
                    # Check if it's a numeric string
                    return cfg_value
                return str(cfg_value)
            # Fallback: check props
            val = props.get("value")
            if val is not None:
                return str(val)
            return "0"

        # ── Signal read (.value) ──
        if tag == "signal":
            name = props.get("name", "")
            return f"{name}.value" if name else "?.value"

        # ── Computed read (.value) ──
        if tag == "computed":
            name = props.get("name", "")
            return f"{name}.value" if name else "?.value"

        # ── getVariable ──
        if tag == "getVariable":
            var_name = config.get("variableName", "")
            return var_name if var_name else "?"

        # ── Arithmetic nodes (add/sub/mul/div) ──
        ops = {"add": "+", "sub": "-", "mul": "*", "div": "/"}
        if tag in ops:
            a_expr = self._find_and_trace_input(
                el["id"], "a", el_map, fwd_map, visited
            )
            b_expr = self._find_and_trace_input(
                el["id"], "b", el_map, fwd_map, visited
            )
            return f"({a_expr} {ops[tag]} {b_expr})"

        # ── Compare ──
        if tag == "compare":
            op = config.get("op", "==")
            a_expr = self._find_and_trace_input(
                el["id"], "a", el_map, fwd_map, visited
            )
            b_expr = self._find_and_trace_input(
                el["id"], "b", el_map, fwd_map, visited
            )
            return f"({a_expr} {op} {b_expr})"

        # ── Logic nodes ──
        if tag == "and":
            a_expr = self._find_and_trace_input(
                el["id"], "a", el_map, fwd_map, visited
            )
            b_expr = self._find_and_trace_input(
                el["id"], "b", el_map, fwd_map, visited
            )
            return f"({a_expr} && {b_expr})"

        if tag == "or":
            a_expr = self._find_and_trace_input(
                el["id"], "a", el_map, fwd_map, visited
            )
            b_expr = self._find_and_trace_input(
                el["id"], "b", el_map, fwd_map, visited
            )
            return f"({a_expr} || {b_expr})"

        if tag == "not":
            a_expr = self._find_and_trace_input(
                el["id"], "a", el_map, fwd_map, visited
            )
            return f"!{a_expr}"

        # ── Concat ──
        if tag == "concat":
            a_expr = self._find_and_trace_input(
                el["id"], "a", el_map, fwd_map, visited
            )
            b_expr = self._find_and_trace_input(
                el["id"], "b", el_map, fwd_map, visited
            )
            return f"({a_expr} + {b_expr})"

        # ── branch (condition → true/false) ──
        if tag == "branch":
            condition = self._find_and_trace_input(
                el["id"], "condition", el_map, fwd_map, visited
            )
            # branch's true/false are outputs — they feed downstream nodes
            # For now, just return the condition expression
            return f"(branch_condition: {condition})"

        # ── sequence / forEach ──
        if tag == "sequence":
            # sequence passes input through — trace input
            return self._find_and_trace_input(
                el["id"], "input", el_map, fwd_map, visited
            )

        if tag == "forEach":
            return self._find_and_trace_input(
                el["id"], "collection", el_map, fwd_map, visited
            )

        # ── fn node (inline code block) ──
        if tag == "fn":
            source = (el.get("source") or "").strip()
            if source:
                return source
            return "/* fn */"

        # ── Fallback: use node name/type ──
        name = props.get("name", "")
        if name:
            return name
        return tag

    def _find_and_trace_input(
        self,
        to_id: str,
        to_port: str,
        el_map: dict[str, dict],
        fwd_map: dict[str, list[dict]],
        visited: set[str],
    ) -> str:
        """Find the wire feeding into (to_id, to_port), then trace its source expression."""
        # Search all wires for one targeting this port
        # We need a backward lookup: given toId+toPort, find fromId+fromPort
        for from_id, wires in fwd_map.items():
            for w in wires:
                if w.get("toId") == to_id and w.get("toPort") == to_port:
                    return self._trace_expression(
                        from_id,
                        w.get("fromPort", ""),
                        el_map,
                        fwd_map,
                        visited,
                    )
        # No wire found — return a default
        return "0"

    # ── View block generation ──────────────────────────────

    def _emit_view(
        self, elements: list[dict], wires: list[dict]
    ) -> list[str]:
        """Generate the view { ... } block from design elements.

        Builds tree from flat children/parentId arrays, then recursively emits.
        """
        if not elements:
            return []

        # Build element index
        el_map: dict[str, dict] = {el["id"]: el for el in elements}

        # Find root elements (parentId is null/None)
        roots = [el for el in elements if not el.get("parentId")]
        if not roots:
            return []

        # Try rootOrder from the element properties if available
        # (rootOrder is stored separately in the store, not in elements)
        # Default to order as they appear
        lines: list[str] = []
        self._indent_level = 1
        lines.append(f"{self._i()}view {{")
        self._indent_level = 2

        for root in roots:
            lines.extend(self._emit_element(root, el_map))

        self._indent_level = 1
        lines.append(f"{self._i()}}}")
        return lines

    def _emit_element(
        self, el: dict, el_map: dict[str, dict]
    ) -> list[str]:
        """Recursively emit a view element as .omni tag syntax."""
        tag = el.get("tag", "div")
        props = el.get("props", {}) or {}
        child_ids: list[str] = el.get("children", []) or []

        # Build props string
        prop_parts: list[str] = []

        # Props from element.props (skip special/internal props)
        skip_props = {"name", "parentStore", "typeAnnotation", "label"}
        for key, value in props.items():
            if key in skip_props:
                continue
            if key == "label":
                # Label is the inner text content, not a prop
                continue
            if isinstance(value, str):
                prop_parts.append(f'{key}="{value}"')
            elif isinstance(value, bool):
                prop_parts.append(f"{key}={{{str(value).lower()}}}")
            elif isinstance(value, (int, float)):
                prop_parts.append(f'{key}={{{value}}}')
            elif value is None:
                pass  # skip null props
            else:
                prop_parts.append(f'{key}={{{value}}}')

        # Event bindings from config._eventBindings
        config = el.get("config", {}) or {}
        event_bindings = config.get("_eventBindings", {}) or {}
        for event_name, binding in event_bindings.items():
            fn_name = binding.get("fnName", "") if isinstance(binding, dict) else ""
            if fn_name:
                prop_parts.append(f"{event_name}={{{fn_name}}}")

        props_str = (" " + " ".join(prop_parts)) if prop_parts else ""

        # Get inner text from label prop
        label = props.get("label", "")

        lines: list[str] = []
        _i = self._i()

        # Resolve children
        children = []
        for cid in child_ids:
            child_el = el_map.get(cid)
            if child_el:
                children.append(child_el)

        if children or (label and tag != "Button"):
            # Has children or text content
            lines.append(f"{_i}<{tag}{props_str}>")
            self._indent_level += 1
            if label and tag != "Button":
                lines.append(f"{self._i()}{label}")
            for child in children:
                lines.extend(self._emit_element(child, el_map))
            self._indent_level -= 1
            lines.append(f"{_i}</{tag}>")
        elif tag == "Button" and label:
            # Button with label as child text
            lines.append(f"{_i}<{tag}{props_str}>{label}</{tag}>")
        else:
            # Self-closing
            lines.append(f"{_i}<{tag}{props_str} />")

        return lines
