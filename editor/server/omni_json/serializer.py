"""OmniLang editor — .omni.json serializer v2 (to_dict / from_dict)."""

from __future__ import annotations

import json
from dataclasses import fields, is_dataclass
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any

from editor.server.omni_json.schema import (
    CanvasData,
    CanvasLayoutSnapshot,
    CodeTabEntry,
    CodeTabsSnapshot,
    ComponentRefEntry,
    ElementEntry,
    OmniJsonDocument,
    OmniMetadata,
    ProjectConfig,
    TabState,
    WireEntry,
)

CURRENT_SCHEMA_VERSION = "2.0.0"
SUPPORTED_VERSIONS = {CURRENT_SCHEMA_VERSION}


class OmniJsonSerializer:
    """Serialize/deserialize OmniJsonDocument v2 to/from plain dicts and JSON strings."""

    # ── Public API ──────────────────────────────────────────

    def to_dict(self, doc: OmniJsonDocument) -> dict[str, Any]:
        """Convert an OmniJsonDocument to a plain JSON-serializable dict."""
        doc.meta.updated_at = datetime.now(timezone.utc).isoformat()
        return self._dataclass_to_dict(doc)

    def from_dict(self, data: dict[str, Any]) -> OmniJsonDocument:
        """Parse and validate a dict into an OmniJsonDocument."""
        version = data.get("schemaVersion", data.get("schema_version", "2.0.0"))
        if version not in SUPPORTED_VERSIONS:
            raise ValueError(
                f"Unsupported schema version: {version}. "
                f"Supported: {sorted(SUPPORTED_VERSIONS)}"
            )
        normalized = self._normalize_keys(data)
        return self._dict_to_doc(normalized)

    def _normalize_keys(self, obj: Any) -> Any:
        """Recursively convert camelCase dict keys to snake_case for deserialization."""
        if isinstance(obj, dict):
            result: dict[str, Any] = {}
            for k, v in obj.items():
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
            schema_version=d.get("schema_version", "2.0.0"),
            meta=self._dict_to_meta(d.get("meta", {})),
            source_code=d.get("source_code", d.get("sourceCode", "")),
            design=self._dict_to_canvas_data(d.get("design", {})),
            code=self._dict_to_canvas_data(d.get("code", {})),
            components=[self._dict_to_component_ref(c) for c in d.get("components", [])],
            canvas_layout=self._dict_to_canvas_layout(d.get("canvas_layout", d.get("canvasLayout", {}))),
            tab_state=self._dict_to_tab_state(d.get("tab_state", d.get("tabState", {}))),
            config=self._dict_to_config(d.get("config", {})),
        )

    def _dict_to_meta(self, d: dict[str, Any]) -> OmniMetadata:
        return OmniMetadata(
            project_name=d.get("project_name", d.get("projectName", "")),
            omni_version=d.get("omni_version", d.get("omniVersion", "0.6.0")),
            created_at=d.get("created_at", d.get("createdAt", "")),
            updated_at=d.get("updated_at", d.get("updatedAt", "")),
        )

    def _dict_to_canvas_data(self, d: dict[str, Any]) -> CanvasData:
        if not d:
            return CanvasData()
        return CanvasData(
            elements=[self._dict_to_element(e) for e in d.get("elements", [])],
            wires=[self._dict_to_wire(w) for w in d.get("wires", [])],
        )

    def _dict_to_component_ref(self, d: dict[str, Any]) -> ComponentRefEntry:
        definition = d.get("definition")
        return ComponentRefEntry(
            name=d.get("name", ""),
            source=d.get("source", "embedded"),
            definition=definition,
            id=d.get("id", ""),
            category=d.get("category", "Custom"),
            description=d.get("description", ""),
            props=d.get("props", {}),
            events=d.get("events", {}),
            style_properties=d.get("style_properties", d.get("styleProperties", [])),
            children=d.get("children", True),
            elements=d.get("elements", []),
            wires=d.get("wires", []),
            created_at=d.get("created_at", d.get("createdAt", "")),
            updated_at=d.get("updated_at", d.get("updatedAt", "")),
            storage=d.get("storage", "config"),
        )

    def _dict_to_tab_state(self, d: dict[str, Any]) -> TabState:
        if not d:
            return TabState()
        return TabState(
            open_tabs=d.get("open_tabs", d.get("openTabs", ["main"])),
            active_tab=d.get("active_tab", d.get("activeTab", "main")),
        )

    def _dict_to_config(self, d: dict[str, Any]) -> ProjectConfig:
        if not d:
            return ProjectConfig()
        return ProjectConfig(
            target=d.get("target", "react"),
            auto_save=d.get("auto_save", d.get("autoSave", False)),
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
            source=d.get("source", ""),
            node_category=d.get("node_category", d.get("nodeCategory", "")),
            config=d.get("config", {}),
            component_ref=d.get("component_ref", d.get("componentRef", "")),
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
                key = self._to_camel_case(f.name)
                # Preserve _kind discriminator field name as-is
                if f.name == "_kind":
                    key = "_kind"
                if value is None:
                    continue
                if isinstance(value, (list, dict, str)) and not value:
                    # Keep lists/dicts for structural fields, skip empty strings
                    if f.name in ("elements", "wires", "components", "open_tabs", "children",
                                  "ports", "branches", "positions", "style_properties"):
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
        leading = ""
        rest = snake
        while rest.startswith("_"):
            leading += "_"
            rest = rest[1:]
        parts = rest.split("_")
        result = leading + parts[0] + "".join(p.capitalize() for p in parts[1:])
        return result

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
