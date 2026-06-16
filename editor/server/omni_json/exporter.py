"""OmniLang editor — .omni.json exporter v2 (.omni source → code canvas snapshot).

The exporter regenerates the code canvas (elements + wires) from .omni source.
Design canvas data is preserved from the existing document (user drag-drop UI).
"""

from __future__ import annotations

import hashlib
import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

from editor.server.omni_json.schema import (
    CanvasData,
    CanvasLayoutSnapshot,
    ComponentRefEntry,
    ElementEntry,
    OmniJsonDocument,
    OmniMetadata,
    ProjectConfig,
    TabState,
    WireEntry,
)


class OmniJsonExporter:
    """Regenerate code canvas from .omni source, preserving design canvas."""

    def __init__(self):
        self._last_source: str = ""

    def regenerate_code_canvas(
        self,
        source: str,
        filepath: str = "<input>",
        target: str = "react",
        existing_code_elements: list[dict[str, Any]] | None = None,
        existing_code_wires: list[dict[str, Any]] | None = None,
    ) -> CanvasData:
        """Parse .omni source → new code.elements + code.wires.

        Design canvas is NOT touched by the exporter — it's managed by the drag-drop UI.
        Caller merges the returned CanvasData into the document.
        """
        self._last_source = source
        from omnilang.api import parse_view_tree, parse_code_blocks

        # Parse view tree for elements (UI structure from .omni source)
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

        # Build elements from view trees + code blocks
        existing_by_id: dict[str, dict[str, Any]] = {}
        for ex in (existing_code_elements or []):
            eid = ex.get("id", "")
            if eid:
                existing_by_id[eid] = ex

        elements: list[ElementEntry] = []

        # Flatten view tree elements
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
                node_category=_block_type_to_category(cb.get("blockType", "")),
            ))

        # Preserve existing wires
        wires = self._preserve_wires(elements, existing_code_wires or [])

        return CanvasData(elements=elements, wires=wires)

    def _preserve_wires(
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

    @staticmethod
    def detect_hand_written(source: str) -> bool:
        """Heuristic: no @editor:managed markers + has comments → likely hand-written."""
        import re
        _MARKER_RE = re.compile(
            r"^\s*(?://|#|/\*|\*|<!--)?\s*@editor:\s*managed"
        )
        has_managed = bool(_MARKER_RE.search(source))
        has_comments = "//" in source or "/*" in source
        return bool((not has_managed) and has_comments)


# ── Internal Helpers ────────────────────────────────────────────


def _view_tree_node_to_dict(vt) -> dict[str, Any]:
    """Convert a ViewTreeNode to a plain dict."""
    result: dict[str, Any] = {
        "id": getattr(vt, "id", ""),
        "tag": getattr(vt, "tag", ""),
        "props": dict(getattr(vt, "props", {})),
        "test_id": getattr(vt, "test_id", None),
        "source_line": getattr(vt, "source_line", 0),
        "children": [_view_tree_node_to_dict(c) for c in getattr(vt, "children", [])],
    }
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


def _flatten_view_elements(
    vt_dict: dict[str, Any],
    existing_by_id: dict[str, dict[str, Any]],
    parent_id: str | None = None,
) -> list[ElementEntry]:
    """Recursively flatten a view tree into flat ElementEntry list."""
    elements: list[ElementEntry] = []
    vid = vt_dict.get("id", "")
    existing = existing_by_id.get(vid, {})

    all_children: list[dict[str, Any]] = list(vt_dict.get("children", []))
    for branch in vt_dict.get("branches", []):
        all_children.extend(branch.get("body", []))
    all_children.extend(vt_dict.get("fallback", []))
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


def _build_codeblock_ports(cb: dict[str, Any]) -> list[dict[str, Any]]:
    """Build port definitions for a code block element."""
    bt = cb.get("blockType", "")
    ports: list[dict[str, Any]] = []
    type_ann = cb.get("typeAnnotation", "unknown")

    if bt == "store":
        ports.append({"name": "items", "direction": "out", "type": type_ann, "portType": "data"})
    elif bt == "signal":
        ports.append({"name": "value", "direction": "out", "type": type_ann, "portType": "data"})
        ports.append({"name": "set", "direction": "in", "type": type_ann, "portType": "data"})
    elif bt == "computed":
        ports.append({"name": "value", "direction": "out", "type": type_ann, "portType": "data"})
        ports.append({"name": "deps", "direction": "in", "type": "any", "portType": "data"})
    elif bt == "action":
        ports.append({"name": "call", "direction": "in", "type": "any", "portType": "flow"})
        ports.append({"name": "result", "direction": "out", "type": type_ann, "portType": "data"})

    return ports


def _block_type_to_category(block_type: str) -> str:
    """Map block type to nodeCategory."""
    return {
        "store": "variable",
        "signal": "variable",
        "computed": "data",
        "action": "action",
    }.get(block_type, "code")


def _source_hash(source: str) -> str:
    """Short hash of source text for change detection."""
    return hashlib.sha256(source.encode()).hexdigest()[:16]
