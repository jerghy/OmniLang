"""IR JSON Reader — validates and enriches Rust-generated IR JSON for Python codegen."""
from __future__ import annotations

import json
from typing import Any


class IRReader:
    """Read a Rust-generated IR JSON and validate/enrich it for codegen."""

    def __init__(self, ir_input: str | dict):
        if isinstance(ir_input, str):
            try:
                self.ir: dict = json.loads(ir_input)
            except json.JSONDecodeError as e:
                self.ir = {}
                self._errors = [f"Invalid JSON: {e}"]
                return
        else:
            self.ir = ir_input
        self._errors: list[str] = []
        self._validate()

    def _validate(self) -> None:
        """Check version and top-level structure."""
        if not self.ir:
            self._errors.append("Empty IR document")
            return

        v = self.ir.get("version")
        if v is None:
            self._errors.append("IR missing 'version' field")
        elif v != "1.0":
            self._errors.append(f"Unsupported IR version: {v} (expected '1.0')")

        has_components = "components" in self.ir and self.ir["components"]
        has_stores = "stores" in self.ir and self.ir["stores"]
        if not has_components and not has_stores:
            self._errors.append("IR must contain 'components' or 'stores' with at least one entry")

    @property
    def errors(self) -> list[str]:
        return self._errors

    @property
    def is_valid(self) -> bool:
        return len(self._errors) == 0

    def components(self) -> dict[str, dict]:
        """Return {name: ComponentIR} dict."""
        return self.ir.get("components", {})

    def stores(self) -> dict[str, dict]:
        """Return {name: StoreIR} dict."""
        return self.ir.get("stores", {})

    def source_map(self) -> dict:
        """Return {node_id: SourceMapEntry} dict."""
        return self.ir.get("source_map", {})

    @staticmethod
    def ir_kind(obj: Any) -> str:
        """Read _kind field for dispatch. e.g. 'ElementIR', 'StateIntent'."""
        if isinstance(obj, dict):
            return obj.get("_kind", "unknown")
        return "unknown"
