"""OmniLang editor — .omni.json serializer (to_dict / from_dict)."""

from __future__ import annotations

import json
from dataclasses import fields, is_dataclass
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any

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

# Schema version this serializer handles
CURRENT_SCHEMA_VERSION = "1.0.0"
SUPPORTED_VERSIONS = {CURRENT_SCHEMA_VERSION}


class OmniJsonSerializer:
    """Serialize/deserialize OmniJsonDocument to/from plain dicts and JSON strings."""

    # ── Public API ──────────────────────────────────────────

    def to_dict(self, doc: OmniJsonDocument) -> dict[str, Any]:
        """Convert an OmniJsonDocument to a plain JSON-serializable dict."""
        doc.meta.updated_at = datetime.now(timezone.utc).isoformat()
        return self._dataclass_to_dict(doc)

    def from_dict(self, data: dict[str, Any]) -> OmniJsonDocument:
        """Parse and validate a dict into an OmniJsonDocument."""
        version = data.get("schema_version", data.get("schemaVersion", "1.0.0"))
        if version not in SUPPORTED_VERSIONS:
            raise ValueError(
                f"Unsupported schema version: {version}. "
                f"Supported: {sorted(SUPPORTED_VERSIONS)}"
            )
        # Normalize camelCase keys to snake_case for internal processing
        normalized = self._normalize_keys(data)
        return self._dict_to_doc(normalized)

    def _normalize_keys(self, obj: Any) -> Any:
        """Recursively convert camelCase dict keys to snake_case for deserialization."""
        if isinstance(obj, dict):
            result: dict[str, Any] = {}
            for k, v in obj.items():
                # Preserve _kind discriminator and other underscore-prefixed keys
                if k.startswith("_"):
                    result[k] = self._normalize_keys(v)
                else:
                    snake_key = self._to_snake_case(k) if k and k[0].islower() else k
                    result[snake_key] = self._normalize_keys(v)
            return result
        if isinstance(obj, list):
            return [self._normalize_keys(item) for item in obj]
        return obj

    def to_json(self, doc: OmniJsonDocument, indent: int = 2) -> str:
        """Serialize document to JSON string."""
        return json.dumps(self.to_dict(doc), indent=indent, ensure_ascii=False)

    def from_json(self, text: str) -> OmniJsonDocument:
        """Parse JSON string into an OmniJsonDocument."""
        data = json.loads(text)
        return self.from_dict(data)

    def save(self, doc: OmniJsonDocument, filepath: str | Path) -> None:
        """Atomically write an OmniJsonDocument to disk."""
        path = Path(filepath)
        tmp_path = path.parent / f".{path.name}.tmp"
        tmp_path.write_text(self.to_json(doc), encoding="utf-8")
        tmp_path.replace(path)

    def load(self, filepath: str | Path) -> OmniJsonDocument:
        """Read an OmniJsonDocument from disk."""
        text = Path(filepath).read_text(encoding="utf-8")
        return self.from_json(text)

    # ── Internal: dict → dataclass ──────────────────────────

    def _dict_to_doc(self, d: dict[str, Any]) -> OmniJsonDocument:
        return OmniJsonDocument(
            schema_version=d.get("schema_version", "1.0.0"),
            meta=self._dict_to_meta(d.get("meta", {})),
            components=[self._dict_to_component(c) for c in d.get("components", [])],
            stores=[self._dict_to_store(s) for s in d.get("stores", [])],
            themes=[self._dict_to_theme(t) for t in d.get("themes", [])],
            imports=d.get("imports", []),
            elements=[self._dict_to_element(e) for e in d.get("elements", [])],
            wires=[self._dict_to_wire(w) for w in d.get("wires", [])],
            ownership_regions=[self._dict_to_ownership(o) for o in d.get("ownership_regions", [])],
            search_index=self._dict_to_search_index(d.get("search_index", {})),
            canvas_layout=self._dict_to_canvas_layout(d.get("canvas_layout", {})),
        )

    def _dict_to_meta(self, d: dict[str, Any]) -> OmniMetadata:
        return OmniMetadata(
            project_name=d.get("project_name", ""),
            omni_version=d.get("omni_version", "0.6.0"),
            created_at=d.get("created_at", ""),
            updated_at=d.get("updated_at", ""),
            is_hand_written=d.get("is_hand_written", False),
            conversion_warnings=d.get("conversion_warnings", []),
        )

    def _dict_to_component(self, d: dict[str, Any]) -> ComponentSnapshot:
        view = None
        if d.get("view"):
            view = self._dict_to_view_node(d["view"])
        return ComponentSnapshot(
            name=d.get("name", ""),
            generic_params=d.get("generic_params", []),
            props=d.get("props", {}),
            signals=d.get("signals", {}),
            computeds=d.get("computeds", {}),
            events=d.get("events", []),
            refs=d.get("refs", []),
            store_refs=d.get("store_refs", []),
            lifecycles=d.get("lifecycles", {}),
            fns=d.get("fns", []),
            view=view,
            style=d.get("style", {}),
        )

    def _dict_to_store(self, d: dict[str, Any]) -> StoreSnapshot:
        return StoreSnapshot(
            name=d.get("name", ""),
            signals=d.get("signals", {}),
            signal_defaults=d.get("signal_defaults", {}),
            getters=d.get("getters", {}),
            actions=d.get("actions", []),
        )

    def _dict_to_theme(self, d: dict[str, Any]) -> ThemeSnapshot:
        return ThemeSnapshot(
            name=d.get("name", ""),
            categories=d.get("categories", {}),
        )

    # Known _kind values for forward-compatibility check
    _KNOWN_KINDS = frozenset({
        "element", "text", "interpolation", "if", "for",
        "slot", "native", "fragment",
    })

    def _dict_to_view_node(self, d: dict[str, Any]) -> ViewNodeSnapshot:
        kind = d.get("_kind", "text")
        # Forward-compatibility: unknown _kind → treat as pass-through fragment
        # (children are preserved, rendering falls back gracefully)
        if kind not in self._KNOWN_KINDS:
            kind = "fragment"

        children = [self._dict_to_view_node(c) for c in d.get("children", [])]
        fallback = None
        if d.get("fallback") is not None:
            fallback = [self._dict_to_view_node(f) for f in d["fallback"]]
        empty_body = None
        if d.get("empty_body") is not None:
            empty_body = [self._dict_to_view_node(eb) for eb in d["empty_body"]]
        return ViewNodeSnapshot(
            _kind=kind,
            test_id=d.get("test_id", ""),
            tag=d.get("tag", ""),
            props=d.get("props", {}),
            bindings=d.get("bindings", {}),
            events=d.get("events", {}),
            style_classes=d.get("style_classes", []),
            inline_style=d.get("inline_style", {}),
            ref_name=d.get("ref_name", ""),
            children=children,
            text=d.get("text", ""),
            expression=d.get("expression", ""),
            condition=d.get("condition", ""),
            branches=d.get("branches", []),
            fallback=fallback,
            iterable=d.get("iterable", ""),
            item_binding=d.get("item_binding", ""),
            key_expr=d.get("key_expr", ""),
            empty_body=empty_body,
            slot_name=d.get("slot_name", ""),
            target=d.get("target", ""),
            native_code=d.get("native_code", ""),
            native_ports=d.get("native_ports", {}),
        )

    def _dict_to_element(self, d: dict[str, Any]) -> ElementEntry:
        return ElementEntry(
            id=d.get("id", ""),
            node_type=d.get("node_type", d.get("nodeType", "view")),
            tag=d.get("tag", ""),
            props=d.get("props", {}),
            children=d.get("children", []),
            parent_id=d.get("parent_id", d.get("parentId")),
            ports=d.get("ports", []),
            render_mode=d.get("render_mode", d.get("renderMode", "dom")),
            is_folded=d.get("is_folded", d.get("isFolded", False)),
            type_annotation=d.get("type_annotation", d.get("typeAnnotation", "")),
            store_name=d.get("store_name", d.get("storeName", "")),
        )

    def _dict_to_wire(self, d: dict[str, Any]) -> WireEntry:
        return WireEntry(
            id=d.get("id", ""),
            from_id=d.get("from_id", d.get("fromId", "")),
            from_port=d.get("from_port", d.get("fromPort", "")),
            to_id=d.get("to_id", d.get("toId", "")),
            to_port=d.get("to_port", d.get("toPort", "")),
            is_collapsed=d.get("is_collapsed", d.get("isCollapsed", False)),
            collapse_group_id=d.get("collapse_group_id", d.get("collapseGroupId", "")),
        )

    def _dict_to_ownership(self, d: dict[str, Any]) -> OwnershipRegionEntry:
        return OwnershipRegionEntry(
            region_type=d.get("region_type", d.get("regionType", "managed")),
            node_id=d.get("node_id", d.get("nodeId", "")),
            node_type=d.get("node_type", d.get("nodeType", "")),
            field_path=d.get("field_path", d.get("fieldPath", "")),
            source_line_start=d.get("source_line_start", d.get("sourceLineStart", 0)),
            source_line_end=d.get("source_line_end", d.get("sourceLineEnd", 0)),
        )

    def _dict_to_search_index(self, d: dict[str, Any]) -> SearchIndexSnapshot:
        entries = []
        for e in d.get("entries", []):
            entries.append(SearchIndexEntry(
                id=e.get("id", ""),
                source=e.get("source", ""),
                title=e.get("title", ""),
                subtitle=e.get("subtitle", ""),
                keywords=e.get("keywords", []),
                icon=e.get("icon", ""),
                action_type=e.get("action_type", e.get("actionType", "navigate")),
                action_payload=e.get("action_payload", e.get("actionPayload", "")),
            ))
        return SearchIndexSnapshot(
            entries=entries,
            version=d.get("version", 1),
            last_built=d.get("last_built", d.get("lastBuilt", "")),
        )

    def _dict_to_canvas_layout(self, d: dict[str, Any]) -> CanvasLayoutSnapshot:
        return CanvasLayoutSnapshot(positions=d.get("positions", {}))

    # ── Internal: dataclass → dict ──────────────────────────

    def _dataclass_to_dict(self, obj: Any) -> Any:
        """Recursively convert dataclass/Enum/list/dict to plain Python types."""
        if obj is None:
            return None
        if isinstance(obj, (str, int, float, bool)):
            return obj
        if isinstance(obj, Enum):
            return obj.value
        if isinstance(obj, dict):
            return {k: self._dataclass_to_dict(v) for k, v in obj.items()}
        if isinstance(obj, (list, tuple)):
            return [self._dataclass_to_dict(v) for v in obj]
        if is_dataclass(obj) and not isinstance(obj, type):
            result: dict[str, Any] = {}
            for f in fields(obj):
                value = getattr(obj, f.name)
                # Use camelCase keys for JSON output (consistent with frontend)
                key = self._to_camel_case(f.name)
                # Preserve _kind discriminator field name as-is
                if f.name == "_kind":
                    key = "_kind"
                # Omit None values and empty defaults for cleaner JSON
                if value is None:
                    continue
                if isinstance(value, (list, dict, str)) and not value:
                    # Keep lists/dicts only if they have content (except for required structural fields)
                    if f.name in ("children", "elements", "wires", "components", "stores",
                                  "themes", "imports", "entries", "branches", "ownership_regions"):
                        result[key] = self._dataclass_to_dict(value)
                    continue
                result[key] = self._dataclass_to_dict(value)
            return result
        return obj

    @staticmethod
    def _to_camel_case(snake: str) -> str:
        """Convert snake_case to camelCase. Preserves leading underscores."""
        if not snake:
            return snake
        # Count leading underscores
        leading = ""
        rest = snake
        while rest.startswith("_"):
            leading += "_"
            rest = rest[1:]
        parts = rest.split("_")
        result = parts[0] + "".join(p.capitalize() for p in parts[1:])
        return leading + result

    @staticmethod
    def _to_snake_case(camel: str) -> str:
        """Convert camelCase back to snake_case (for deserialization)."""
        if not camel:
            return camel
        result = []
        for ch in camel:
            if ch.isupper() and result:
                result.append("_")
                result.append(ch.lower())
            elif ch.isupper():
                result.append(ch.lower())
            else:
                result.append(ch)
        return "".join(result)
