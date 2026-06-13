"""OmniLang editor native format (.omni.json) — v1 schema definitions.

The .omni.json document stores a snapshot of the editor's full state:
  - Component/Store/Theme definitions (compiled from .omni AST)
  - Elements (canvas view nodes with layout positions)
  - Wires (port-to-port connections)
  - Ownership regions (AST-anchored managed/handwritten markers)
  - Search index (precomputed for fast lookup)
  - Canvas layout (per-element position/size metadata)

Design: snapshot format with `_kind` discriminators (same pattern as ir_to_dict() in lowering.py).
The snapshots are JSON-friendly dicts, NOT frozen dataclass AST — they survive schema evolution
without coupling to parser internals.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


# ═══════════════════════════════════════════════════════════════
# Core Document
# ═══════════════════════════════════════════════════════════════


@dataclass
class OmniMetadata:
    """Top-level document metadata."""
    project_name: str = ""
    omni_version: str = "0.6.0"
    created_at: str = ""        # ISO 8601
    updated_at: str = ""        # ISO 8601
    is_hand_written: bool = False
    conversion_warnings: list[str] = field(default_factory=list)


@dataclass
class OmniJsonDocument:
    """Root document — the full contents of a .omni.json file."""
    schema_version: str = "1.0.0"
    meta: OmniMetadata = field(default_factory=OmniMetadata)
    components: list[ComponentSnapshot] = field(default_factory=list)
    stores: list[StoreSnapshot] = field(default_factory=list)
    themes: list[ThemeSnapshot] = field(default_factory=list)
    imports: list[str] = field(default_factory=list)
    elements: list[ElementEntry] = field(default_factory=list)
    wires: list[WireEntry] = field(default_factory=list)
    ownership_regions: list[OwnershipRegionEntry] = field(default_factory=list)
    search_index: SearchIndexSnapshot = field(default_factory=lambda: SearchIndexSnapshot())
    canvas_layout: CanvasLayoutSnapshot = field(default_factory=lambda: CanvasLayoutSnapshot())


# ═══════════════════════════════════════════════════════════════
# Component / Store / Theme Snapshots
# ═══════════════════════════════════════════════════════════════


@dataclass
class ComponentSnapshot:
    """A single component's compiled snapshot."""
    name: str
    generic_params: list[str] = field(default_factory=list)
    props: dict[str, str] = field(default_factory=dict)           # name → type
    signals: dict[str, dict[str, Any]] = field(default_factory=dict)  # name → {type, init}
    computeds: dict[str, dict[str, Any]] = field(default_factory=dict) # name → {type, body, deps}
    events: list[str] = field(default_factory=list)
    refs: list[str] = field(default_factory=list)
    store_refs: list[str] = field(default_factory=list)           # @store names
    lifecycles: dict[str, list[str]] = field(default_factory=dict) # {mount: [body], unmount: [body]}
    fns: list[dict[str, Any]] = field(default_factory=list)       # [{name, params, return_type, body}]
    view: Optional[ViewNodeSnapshot] = None                       # Root view tree
    style: dict[str, dict[str, str]] = field(default_factory=dict) # {className: {prop: value}}


@dataclass
class StoreSnapshot:
    """A single store's compiled snapshot."""
    name: str
    signals: dict[str, str] = field(default_factory=dict)         # name → type
    signal_defaults: dict[str, Any] = field(default_factory=dict)  # name → default value
    getters: dict[str, dict[str, Any]] = field(default_factory=dict) # name → {return_type, body}
    actions: list[dict[str, Any]] = field(default_factory=list)   # [{name, params, body}]


@dataclass
class ThemeSnapshot:
    """Design tokens theme definition."""
    name: str
    categories: dict[str, dict[str, str]] = field(default_factory=dict)
    # { "colors": {"primary": "#4F46E5"}, "spacing": {"md": "16px"}, ... }


# ═══════════════════════════════════════════════════════════════
# View Tree Nodes (recursive, _kind-discriminated)
# ═══════════════════════════════════════════════════════════════

@dataclass
class ViewNodeSnapshot:
    """A node in the view tree. _kind discriminator selects the specific shape."""
    _kind: str = "text"         # "element" | "text" | "interpolation" | "if" | "for" | "slot" | "native" | "fragment"
    # Shared
    test_id: str = ""
    # Element-specific
    tag: str = ""
    props: dict[str, Any] = field(default_factory=dict)
    bindings: dict[str, str] = field(default_factory=dict)        # prop → signalName
    events: dict[str, dict[str, Any]] = field(default_factory=dict)  # eventName → {kind, ref}
    style_classes: list[str] = field(default_factory=list)
    inline_style: dict[str, str] = field(default_factory=dict)
    ref_name: str = ""
    # Children (recursive)
    children: list[ViewNodeSnapshot] = field(default_factory=list)
    # Text-specific
    text: str = ""
    # Interpolation
    expression: str = ""
    # If-specific
    condition: str = ""
    branches: list[dict[str, Any]] = field(default_factory=list)  # [{condition, body: [ViewNodeSnapshot]}]
    fallback: Optional[list[ViewNodeSnapshot]] = None
    # For-specific
    iterable: str = ""
    item_binding: str = ""
    key_expr: str = ""
    empty_body: Optional[list[ViewNodeSnapshot]] = None
    # Slot-specific
    slot_name: str = ""
    # Native-specific
    target: str = ""
    native_code: str = ""
    native_ports: dict[str, str] = field(default_factory=dict)    # {portName: type}


# ═══════════════════════════════════════════════════════════════
# Editor Runtime State
# ═══════════════════════════════════════════════════════════════


@dataclass
class ElementEntry:
    """Canvas element — mirrors ElementData in the Zustand store."""
    id: str
    node_type: str = "view"     # "view" | "codeblock" | "group"
    tag: str = ""               # "Button", "Flex", etc. — or codeblock type like "signal"/"store"
    props: dict[str, Any] = field(default_factory=dict)
    children: list[str] = field(default_factory=list)             # child element IDs
    parent_id: Optional[str] = None
    ports: list[dict[str, str]] = field(default_factory=list)    # [{name, direction, type}]
    render_mode: str = "dom"
    is_folded: bool = False
    type_annotation: str = ""
    store_name: str = ""


@dataclass
class WireEntry:
    """Canvas wire — mirrors WireData in the Zustand store."""
    id: str
    from_id: str = ""
    from_port: str = ""
    to_id: str = ""
    to_port: str = ""
    is_collapsed: bool = False
    collapse_group_id: str = ""


@dataclass
class CanvasLayoutSnapshot:
    """Per-element canvas positions/sizes (editor metadata, not in .omni)."""
    positions: dict[str, dict[str, float]] = field(default_factory=dict)
    # {elementId: {x: 0, y: 0, w: 200, h: 60}}


# ═══════════════════════════════════════════════════════════════
# Ownership Regions (AST-anchored)
# ═══════════════════════════════════════════════════════════════


@dataclass
class OwnershipRegionEntry:
    """Ownership marker anchored to an AST node ID (not line numbers)."""
    region_type: str = "managed"    # "managed" | "handwritten"
    node_id: str = ""               # AST node ID e.g. "comp-Foo" or "el-Button-0x..."
    node_type: str = ""             # "component" | "store" | "view" | "signal" | "action" | "fn" | "style"
    field_path: str = ""            # e.g. "view.children[0].props.onPress"
    source_line_start: int = 0      # 0-based, fallback for display
    source_line_end: int = 0


# ═══════════════════════════════════════════════════════════════
# Search Index
# ═══════════════════════════════════════════════════════════════


@dataclass
class SearchIndexEntry:
    """A precomputed search index entry."""
    id: str = ""
    source: str = ""            # "file" | "component" | "snippet" | "connector" | "token" | "wire" | "command"
    title: str = ""
    subtitle: str = ""
    keywords: list[str] = field(default_factory=list)  # Pre-extracted for fast matching
    icon: str = ""
    action_type: str = "navigate"       # "open-file" | "insert-snippet" | "select-element" | "focus-wire"
    action_payload: str = ""


@dataclass
class SearchIndexSnapshot:
    """Precomputed search index stored in .omni.json."""
    entries: list[SearchIndexEntry] = field(default_factory=list)
    version: int = 1
    last_built: str = ""        # ISO 8601 timestamp
