"""OmniLang editor — .omni.json exporter (`.omni` source → `.omni.json` document).

The exporter is the "save" direction of the editor: it takes .omni source text
and produces a full OmniJsonDocument snapshot.
"""

from __future__ import annotations

import hashlib
import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

from editor.server.omni_json.schema import (
    CanvasLayoutSnapshot,
    ComponentSnapshot,
    ElementEntry,
    OmniJsonDocument,
    OmniMetadata,
    OwnershipRegionEntry,
    SearchIndexEntry,
    SearchIndexSnapshot,
    StoreSnapshot,
    ThemeSnapshot,
    ViewNodeSnapshot,
    WireEntry,
)


class OmniJsonExporter:
    """Convert .omni source to .omni.json document snapshot."""

    def __init__(self):
        self._last_source: str = ""

    def export(
        self,
        source: str,
        filepath: str = "<input>",
        target: str = "react",
        existing_elements: list[dict[str, Any]] | None = None,
        existing_wires: list[dict[str, Any]] | None = None,
        existing_canvas: dict[str, Any] | None = None,
    ) -> OmniJsonDocument:
        """Export .omni source → full OmniJsonDocument.

        Args:
            source: .omni source text
            filepath: source file path (for metadata)
            target: target framework
            existing_elements: previously saved element layout (preserved across export)
            existing_wires: previously saved wires (preserved)
            existing_canvas: previously saved canvas positions (preserved)
        """
        self._last_source = source
        from omniview.api import compile_omni, parse_view_tree, parse_code_blocks

        # Compile to verify and get component/store data
        compile_result = compile_omni(source, filepath=filepath, target=target)

        # Parse view tree for elements
        view_trees: list[dict[str, Any]] = []
        try:
            parsed = parse_view_tree(source, filepath=filepath)
            view_trees = [_view_tree_node_to_dict(vt) for vt in parsed]
        except Exception as e:
            logger.warning("View tree parsing failed (best-effort): %s", e)
            view_trees = []

        # Parse code blocks for store/signal/action nodes
        code_blocks: list[dict[str, Any]] = []
        try:
            blocks = parse_code_blocks(source, filepath=filepath)
            code_blocks = [
                {
                    "id": b.id,
                    "blockType": b.blockType,
                    "name": b.name,
                    "parentStore": b.parentStore,
                    "typeAnnotation": b.typeAnnotation,
                    "sourceLine": b.sourceLine,
                }
                for b in blocks
            ]
        except Exception as e:
            logger.warning("Code block parsing failed (best-effort): %s", e)
            code_blocks = []

        # Build component snapshots from compile output
        components = self._build_components(view_trees)

        # Build store snapshots from code blocks
        stores = self._build_stores(code_blocks)

        # Build elements from view trees + code blocks (preserving layout from existing)
        elements = self._build_elements(view_trees, code_blocks, existing_elements or [])

        # Build wires (preserve existing if provided, otherwise empty)
        wires = self._build_wires(elements, existing_wires or [])

        # Build ownership regions
        ownership = self._build_ownership(source)

        # Build search index
        search_index = self._build_search_index(components, stores, elements, wires, code_blocks)

        # Build canvas layout (preserve existing positions)
        canvas = CanvasLayoutSnapshot(
            positions=(existing_canvas or {}).get("positions", {})
        )

        # Build metadata
        meta = OmniMetadata(
            project_name=Path(filepath).stem if filepath != "<input>" else "",
            omni_version="0.6.0",
            created_at=datetime.now(timezone.utc).isoformat(),
            updated_at=datetime.now(timezone.utc).isoformat(),
            is_hand_written=self._detect_hand_written(source),
            conversion_warnings=[],
        )

        return OmniJsonDocument(
            schema_version="1.0.0",
            meta=meta,
            components=components,
            stores=stores,
            themes=[],
            imports=[],
            elements=elements,
            wires=wires,
            ownership_regions=ownership,
            search_index=search_index,
            canvas_layout=canvas,
        )

    # ── Component/Store Building ──────────────────────────────

    def _build_components(self, view_trees: list[dict[str, Any]]) -> list[ComponentSnapshot]:
        """Build ComponentSnapshot list from parsed view trees + AST component declarations."""
        components: list[ComponentSnapshot] = []

        # Also parse full AST to get component declarations (signals, fns, store-refs)
        ast_declarations = self._parse_component_declarations()

        for vt in view_trees:
            comp_name = vt.get("props", {}).get("data-component", vt.get("id", "unknown"))
            # Merge with AST declarations
            ast_info = ast_declarations.get(comp_name, {})
            signals = ast_info.get("signals", {})
            fns = ast_info.get("fns", [])
            store_refs = ast_info.get("store_refs", [])
            computeds = ast_info.get("computeds", {})

            # Also extract from view tree bindings if AST didn't have them
            for key, val in vt.get("props", {}).items():
                if isinstance(val, str) and val.startswith("{bind:"):
                    sig_name = val.replace("{bind:", "").rstrip("}")
                    if sig_name not in signals:
                        signals[sig_name] = {"type": "i32", "init": 0}

            components.append(ComponentSnapshot(
                name=comp_name,
                signals=signals,
                computeds=computeds,
                fns=fns,
                store_refs=store_refs,
                view=_dict_to_view_snapshot(vt),
            ))
        return components

    def _parse_component_declarations(self) -> dict[str, dict[str, Any]]:
        """Parse OmniView AST to extract component-level declarations (signals, fns, store-refs).

        Returns dict keyed by component name.
        """
        result: dict[str, dict[str, Any]] = {}
        try:
            from omniview.preprocessor import preprocess
            from omnilang.lexer import create_omniview_parser
            from omniview.parser import OmniViewTransformer

            pp_result = preprocess(self._last_source or "")
            tree = create_omniview_parser().parse(pp_result.processed_source)
            transformer = OmniViewTransformer(block_map=pp_result.blocks)
            program = transformer.transform(tree)

            for comp in program.components:
                name = comp.name
                signals: dict[str, dict[str, Any]] = {}
                fns: list[dict[str, Any]] = []
                store_refs: list[str] = []
                computeds: dict[str, dict[str, Any]] = {}

                for sig in getattr(comp, 'signals', []) or []:
                    sig_type = str(sig.type_annotation) if hasattr(sig, 'type_annotation') and sig.type_annotation else "i32"
                    sig_init = "0"
                    if hasattr(sig, 'init_value') and sig.init_value:
                        sig_init = str(sig.init_value)
                    signals[sig.name] = {"type": sig_type, "init": sig_init}

                for fn in getattr(comp, 'fns', []) or []:
                    params = []
                    for p in getattr(fn, 'params', []) or []:
                        p_type = str(p.type_annotation) if hasattr(p, 'type_annotation') and p.type_annotation else "i32"
                        params.append({"name": p.name, "type": p_type})
                    ret = str(fn.return_type) if hasattr(fn, 'return_type') and fn.return_type else "void"
                    fns.append({"name": fn.name, "params": params, "returnType": ret, "body": ""})

                for sr in getattr(comp, 'stores', []) or []:
                    if hasattr(sr, 'store_name'):
                        store_refs.append(str(sr.store_name))
                    elif hasattr(sr, 'name'):
                        store_refs.append(str(sr.name))
                    else:
                        store_refs.append(str(sr))

                for c in getattr(comp, 'computeds', []) or []:
                    c_type = str(c.type_annotation) if hasattr(c, 'type_annotation') and c.type_annotation else "i32"
                    deps = list(c.deps) if hasattr(c, 'deps') and c.deps else []
                    computeds[c.name] = {"type": c_type, "body": "", "deps": deps}

                result[name] = {
                    "signals": signals,
                    "fns": fns,
                    "store_refs": store_refs,
                    "computeds": computeds,
                }
        except Exception as e:
            logger.warning(
                "Failed to parse component declarations from AST: %s. "
                "Falling back to view-tree-only extraction. "
                "Signals, fns, store-refs, and computeds may be incomplete.",
                e,
            )
        return result

    def _build_stores(self, code_blocks: list[dict[str, Any]]) -> list[StoreSnapshot]:
        """Build StoreSnapshot list from code blocks."""
        store_map: dict[str, dict[str, Any]] = {}
        for cb in code_blocks:
            if cb["blockType"] == "store":
                store_name = cb["name"]
                if store_name not in store_map:
                    store_map[store_name] = {
                        "signals": {},
                        "signal_defaults": {},
                        "getters": {},
                        "actions": [],
                    }
            elif cb["blockType"] == "signal" and cb.get("parentStore"):
                s = store_map.setdefault(cb["parentStore"], {
                    "signals": {}, "signal_defaults": {}, "getters": {}, "actions": [],
                })
                s["signals"][cb["name"]] = cb.get("typeAnnotation", "i32")
            elif cb["blockType"] == "computed" and cb.get("parentStore"):
                s = store_map.setdefault(cb["parentStore"], {
                    "signals": {}, "signal_defaults": {}, "getters": {}, "actions": [],
                })
                s["getters"][cb["name"]] = {
                    "returnType": cb.get("typeAnnotation", "i32"),
                    "body": "",
                }
            elif cb["blockType"] == "action" and cb.get("parentStore"):
                s = store_map.setdefault(cb["parentStore"], {
                    "signals": {}, "signal_defaults": {}, "getters": {}, "actions": [],
                })
                s["actions"].append({"name": cb["name"], "params": [], "body": ""})

        return [
            StoreSnapshot(name=name, **fields)
            for name, fields in store_map.items()
        ]

    # ── Elements Building ──────────────────────────────────────

    def _build_elements(
        self,
        view_trees: list[dict[str, Any]],
        code_blocks: list[dict[str, Any]],
        existing: list[dict[str, Any]],
    ) -> list[ElementEntry]:
        """Build ElementEntry list, merging view trees + code blocks + preserved layout."""
        # Build ID→existing mapping for layout preservation
        existing_by_id: dict[str, dict[str, Any]] = {}
        for ex in existing:
            eid = ex.get("id", "")
            if eid:
                existing_by_id[eid] = ex

        elements: list[ElementEntry] = []

        # Add view tree elements
        for vt in view_trees:
            elements.extend(_flatten_view_elements(vt, existing_by_id))

        # Add code block elements
        for cb in code_blocks:
            cb_id = cb.get("id", "")
            existing_entry = existing_by_id.get(cb_id, {})
            elements.append(ElementEntry(
                id=cb_id or f"cb-{cb.get('name', 'unknown')}",
                node_type="codeblock",
                tag=cb.get("blockType", "signal"),
                props={
                    "name": cb.get("name", ""),
                    "parentStore": cb.get("parentStore", ""),
                    "typeAnnotation": cb.get("typeAnnotation", ""),
                },
                children=existing_entry.get("children", []),
                parent_id=existing_entry.get("parentId"),
                ports=_build_codeblock_ports(cb),
                is_folded=existing_entry.get("isFolded", False),
                type_annotation=cb.get("typeAnnotation", ""),
                store_name=cb.get("parentStore", ""),
            ))

        return elements

    # ── Wires Building ─────────────────────────────────────────

    def _build_wires(
        self,
        elements: list[ElementEntry],
        existing_wires: list[dict[str, Any]],
    ) -> list[WireEntry]:
        """Preserve existing wires (connections are editor-only metadata)."""
        wires: list[WireEntry] = []
        seen_ids: set[str] = set()

        for ew in existing_wires:
            wid = ew.get("id", "")
            if not wid or wid in seen_ids:
                continue
            seen_ids.add(wid)

            # Validate wire endpoints still exist
            from_ok = any(el.id == ew.get("fromId") for el in elements)
            to_ok = any(el.id == ew.get("toId") for el in elements)
            # Even if endpoints are gone, preserve the wire (it will show as broken)
            wires.append(WireEntry(
                id=wid,
                from_id=ew.get("fromId", ""),
                from_port=ew.get("fromPort", ""),
                to_id=ew.get("toId", ""),
                to_port=ew.get("toPort", ""),
                is_collapsed=ew.get("isCollapsed", False),
                collapse_group_id=ew.get("collapseGroupId", ""),
            ))

        return wires

    # ── Ownership Building ─────────────────────────────────────

    def _build_ownership(self, source: str) -> list[OwnershipRegionEntry]:
        """Build AST-anchored ownership regions from line-based markers."""
        from editor.server.api.code_ownership import parse_ownership_regions

        try:
            regions = parse_ownership_regions(source)
        except Exception:
            return []

        entries: list[OwnershipRegionEntry] = []
        for r in regions:
            entries.append(OwnershipRegionEntry(
                region_type=r.type,
                node_id=f"region-l{r.start_line}-l{r.end_line}",
                node_type="view",  # Default — refined when we have full AST
                field_path=f"source[{r.start_line}:{r.end_line}]",
                source_line_start=r.start_line,
                source_line_end=r.end_line,
            ))
        return entries

    # ── Search Index Building ──────────────────────────────────

    def _build_search_index(
        self,
        components: list[ComponentSnapshot],
        stores: list[StoreSnapshot],
        elements: list[ElementEntry],
        wires: list[WireEntry],
        code_blocks: list[dict[str, Any]],
    ) -> SearchIndexSnapshot:
        """Precompute search index entries for fast client-side search."""
        entries: list[SearchIndexEntry] = []

        # Component entries
        for comp in components:
            keywords = [comp.name.lower()]
            keywords.extend(comp.name.lower().split("_"))
            for sig_name in comp.signals:
                keywords.append(sig_name.lower())
            entries.append(SearchIndexEntry(
                id=f"comp-{comp.name}",
                source="component",
                title=comp.name,
                subtitle=f"Component — {len(comp.signals)} signals, {len(comp.fns)} fns",
                keywords=keywords,
                icon="🧩",
                action_type="select-element",
                action_payload=f"comp-{comp.name}",
            ))

        # Store entries
        for store in stores:
            keywords = [store.name.lower()]
            for sig_name in store.signals:
                keywords.append(sig_name.lower())
            for action in store.actions:
                keywords.append(action.get("name", "").lower())
            entries.append(SearchIndexEntry(
                id=f"store-{store.name}",
                source="component",
                title=store.name,
                subtitle=f"Store — {len(store.signals)} signals, {len(store.actions)} actions",
                keywords=keywords,
                icon="🗄️",
                action_type="select-element",
                action_payload=f"cb-store-{store.name}",
            ))

        # Wire entries
        for w in wires:
            entries.append(SearchIndexEntry(
                id=f"wire-{w.id}",
                source="wire",
                title=f"{w.from_id}:{w.from_port} → {w.to_id}:{w.to_port}",
                subtitle="Wire connection",
                keywords=[w.from_id.lower(), w.to_id.lower(), w.from_port.lower(), w.to_port.lower()],
                icon="🔗",
                action_type="focus-wire",
                action_payload=w.id,
            ))

        # Code block entries (signals/actions in stores)
        for cb in code_blocks:
            name = cb.get("name", "")
            bt = cb.get("blockType", "")
            keywords = [name.lower(), bt.lower()]
            if cb.get("parentStore"):
                keywords.append(cb["parentStore"].lower())
            entries.append(SearchIndexEntry(
                id=f"idx-{cb.get('id', name)}",
                source="component",
                title=f"{bt}: {name}",
                subtitle=f"{bt.capitalize()} in {cb.get('parentStore', 'component')}",
                keywords=keywords,
                icon=_block_type_icon(bt),
                action_type="select-element",
                action_payload=cb.get("id", ""),
            ))

        return SearchIndexSnapshot(
            entries=entries,
            version=1,
            last_built=datetime.now(timezone.utc).isoformat(),
        )

    # ── Hand-written Detection ─────────────────────────────────

    @staticmethod
    def _detect_hand_written(source: str) -> bool:
        """Heuristic: no @editor:managed markers + has comments → likely hand-written.

        Uses regex to detect actual @editor:managed marker comments (not substring matches
        inside other comments or code).
        """
        import re
        # Match lines that are actual @editor:managed marker comments
        _MARKER_RE = re.compile(
            r"^\s*(?://|#|/\*|\*|<!--)?\s*@editor:\s*managed"
        )
        has_managed = bool(_MARKER_RE.search(source))
        # Comments: // or /* style
        has_comments = "//" in source or "/*" in source
        # If there are comments but no editor markers, it was likely hand-written
        return bool((not has_managed) and has_comments)


# ── Internal Helpers ────────────────────────────────────────────


def _view_tree_node_to_dict(vt) -> dict[str, Any]:
    """Convert a ViewTreeNode to a plain dict, including directive branches."""
    result: dict[str, Any] = {
        "id": getattr(vt, "id", ""),
        "tag": getattr(vt, "tag", ""),
        "props": dict(getattr(vt, "props", {})),
        "test_id": getattr(vt, "test_id", None),
        "source_line": getattr(vt, "source_line", 0),
        "children": [_view_tree_node_to_dict(c) for c in getattr(vt, "children", [])],
    }
    # Capture directive branches (@if/@else-if/@else bodies)
    if hasattr(vt, "branches") and vt.branches:
        result["branches"] = [
            {
                "condition": getattr(b, "condition", ""),
                "body": [_view_tree_node_to_dict(c) for c in getattr(b, "children", [])],
            }
            for b in vt.branches
        ]
    if hasattr(vt, "fallback") and vt.fallback:
        result["fallback"] = [_view_tree_node_to_dict(f) for f in vt.fallback]
    if hasattr(vt, "empty_body") and vt.empty_body:
        result["empty_body"] = [_view_tree_node_to_dict(eb) for eb in vt.empty_body]
    return result


def _dict_to_view_snapshot(d: dict[str, Any]) -> ViewNodeSnapshot:
    """Convert a view tree dict to a ViewNodeSnapshot."""
    children = [_dict_to_view_snapshot(c) for c in d.get("children", [])]
    tag = d.get("tag", "")
    props = d.get("props", {})

    # Detect _kind from tag/props
    kind = "element"
    directive = props.get("data-directive", "")
    if directive:
        if directive.startswith("@if"):
            kind = "if"
        elif directive.startswith("@for"):
            kind = "for"
    if tag == "Text":
        label = props.get("label", "")
        if label.startswith("{") and "}" in label:
            kind = "interpolation"
        else:
            kind = "text"

    # Extract bindings and events from props
    bindings: dict[str, str] = {}
    events: dict[str, dict[str, Any]] = {}
    clean_props: dict[str, Any] = {}
    for key, val in props.items():
        if isinstance(val, str):
            if val.startswith("{bind:"):
                bindings[key] = val.replace("{bind:", "").rstrip("}")
            elif val.startswith("→"):
                events[key] = {"kind": "fn_ref", "ref": val[1:]}
            else:
                clean_props[key] = val
        else:
            clean_props[key] = val

    return ViewNodeSnapshot(
        _kind=kind,
        test_id=d.get("test_id", ""),
        tag=tag,
        props=clean_props,
        bindings=bindings,
        events=events,
        children=children,
        text=props.get("label", "") if kind in ("text", "interpolation") else "",
        expression=props.get("label", "") if kind == "interpolation" else "",
        condition=directive if directive.startswith("@if") else "",
        iterable="" if not directive.startswith("@for") else directive,
    )


def _flatten_view_elements(
    vt_dict: dict[str, Any],
    existing_by_id: dict[str, dict[str, Any]],
    parent_id: str | None = None,
) -> list[ElementEntry]:
    """Recursively flatten a view tree into flat ElementEntry list.

    Walks into directive branches (@if/@else-if/@else/@for/@empty) so that
    elements inside conditionals and loops are visible to the canvas.
    """
    elements: list[ElementEntry] = []
    vid = vt_dict.get("id", "")
    existing = existing_by_id.get(vid, {})

    # Collect all child nodes including directive bodies
    all_children: list[dict[str, Any]] = list(vt_dict.get("children", []))

    # @if/@else-if branches
    for branch in vt_dict.get("branches", []):
        all_children.extend(branch.get("body", []))

    # @else fallback
    all_children.extend(vt_dict.get("fallback", []))

    # @for/@empty body
    all_children.extend(vt_dict.get("empty_body", []))

    children_ids = []
    child_elements: list[ElementEntry] = []
    for child in all_children:
        child_els = _flatten_view_elements(child, existing_by_id, vid)
        child_elements.extend(child_els)
        for ce in child_els:
            children_ids.append(ce.id)

    entry = ElementEntry(
        id=vid,
        node_type=existing.get("nodeType", "view"),
        tag=vt_dict.get("tag", ""),
        props=vt_dict.get("props", {}),
        children=children_ids,
        parent_id=existing.get("parentId", parent_id),
        ports=existing.get("ports", []),
        is_folded=existing.get("isFolded", False),
    )

    elements.append(entry)
    elements.extend(child_elements)
    return elements


def _build_codeblock_ports(cb: dict[str, Any]) -> list[dict[str, str]]:
    """Build port definitions for a code block element."""
    bt = cb.get("blockType", "")
    ports: list[dict[str, str]] = []
    type_ann = cb.get("typeAnnotation", "unknown")

    if bt == "store":
        ports.append({"name": "items", "direction": "out", "type": type_ann})
    elif bt == "signal":
        ports.append({"name": "value", "direction": "out", "type": type_ann})
        ports.append({"name": "set", "direction": "in", "type": type_ann})
    elif bt == "computed":
        ports.append({"name": "value", "direction": "out", "type": type_ann})
        ports.append({"name": "deps", "direction": "in", "type": "any"})
    elif bt == "action":
        ports.append({"name": "call", "direction": "in", "type": "any"})

    return ports


def _block_type_icon(block_type: str) -> str:
    """Map block type to emoji icon."""
    return {
        "store": "🗄️",
        "signal": "⚡",
        "computed": "🔄",
        "action": "▶️",
    }.get(block_type, "📦")


def _source_hash(source: str) -> str:
    """Short hash of source text for change detection."""
    return hashlib.sha256(source.encode()).hexdigest()[:16]
