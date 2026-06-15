"""OmniLang editor native format (.omni.json) — v2 schema definitions.

v2 changes:
  - sourceCode: embedded .omni source for code mode text editor
  - design/code split: separate elements/wires for design canvas and code canvas
  - components[]: unified array with source marker ("embedded" or path reference)
  - tabState: persisted tab layout (openTabs, activeTab)
  - config: project-level configuration
  - Removed: stores, themes, imports, ownershipRegions, searchIndex (dead weight)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


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


@dataclass
class ProjectConfig:
    """Project-level configuration."""
    target: str = "react"       # "react" | "vue" | "flutter" | "omniview"
    auto_save: bool = False


@dataclass
class CanvasData:
    """Separate elements + wires for a single canvas mode."""
    elements: list["ElementEntry"] = field(default_factory=list)
    wires: list["WireEntry"] = field(default_factory=list)


@dataclass
class TabState:
    """Persisted tab layout."""
    open_tabs: list[str] = field(default_factory=lambda: ["main"])
    active_tab: str = "main"


@dataclass
class ComponentRefEntry:
    """A component reference in the project's component list.

    source = "embedded" → definition is stored inline
    source = "./path/to/lib.omni-components.json" → definition is loaded from external file
    """
    name: str
    source: str = "embedded"                        # "embedded" | path to .omni-components.json
    definition: dict[str, Any] | None = None        # inline definition (when embedded)
    id: str = ""
    category: str = "Custom"
    description: str = ""
    props: dict[str, dict[str, Any]] = field(default_factory=dict)
    events: dict[str, dict[str, Any]] = field(default_factory=dict)
    style_properties: list[str] = field(default_factory=list)
    children: bool = True
    elements: list[dict[str, Any]] = field(default_factory=list)
    wires: list[dict[str, Any]] = field(default_factory=list)
    created_at: str = ""
    updated_at: str = ""
    storage: str = "config"


@dataclass
class OmniJsonDocument:
    """Root document — the full contents of a .omni.json project file (v2)."""
    schema_version: str = "2.0.0"
    meta: OmniMetadata = field(default_factory=OmniMetadata)

    # Embedded .omni source code (for code mode text editor)
    source_code: str = ""

    # Split canvas data
    design: CanvasData = field(default_factory=CanvasData)
    code: CanvasData = field(default_factory=CanvasData)

    # Multi-tab code graph state (v2.1 — for CodeEditorStore)
    code_tabs: CodeTabsSnapshot = field(default_factory=lambda: CodeTabsSnapshot())

    # Component references (unified array with source marker)
    components: list[ComponentRefEntry] = field(default_factory=list)

    # Canvas element positions
    canvas_layout: "CanvasLayoutSnapshot" = field(default_factory=lambda: CanvasLayoutSnapshot())

    # Persisted tab state
    tab_state: TabState = field(default_factory=TabState)

    # Project configuration
    config: ProjectConfig = field(default_factory=ProjectConfig)


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
    parent_id: str | None = None
    ports: list[dict[str, Any]] = field(default_factory=list)     # [{name, direction, type, portType}]
    render_mode: str = "dom"
    is_folded: bool = False
    type_annotation: str = ""
    store_name: str = ""
    source: str = ""            # Per-block source code
    node_category: str = ""     # "flow" | "event" | "action" | "data" | "variable" | "code"
    config: dict[str, Any] = field(default_factory=dict)
    component_ref: str = ""     # Custom component reference name


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
class CodeTabEntry:
    """A single code graph tab in the multi-tab visual programming editor."""
    id: str = ""                              # "main", "comp:Name", "fn:Name", or generated
    type: str = "freeform"                    # "freeform" | "component" | "function"
    label: str = ""
    icon: str = "📄"
    graph: dict[str, Any] = field(default_factory=lambda: {"elements": [], "wires": []})
    component_id: str = ""                    # For "component" tabs
    fn_name: str = ""                          # For "function" tabs
    fn_params: list[dict[str, Any]] = field(default_factory=list)
    fn_return_type: str = ""
    variables: list[dict[str, Any]] = field(default_factory=list)


@dataclass
class CodeTabsSnapshot:
    """Container for multi-tab code graph state."""
    open_tabs: list[str] = field(default_factory=lambda: ["main"])
    active_tab: str = "main"
    tabs: list[CodeTabEntry] = field(default_factory=list)


@dataclass
class CanvasLayoutSnapshot:
    """Per-element canvas positions/sizes (editor metadata)."""
    positions: dict[str, dict[str, float]] = field(default_factory=dict)
    # {elementId: {x: 0, y: 0, w: 200, h: 60}}
