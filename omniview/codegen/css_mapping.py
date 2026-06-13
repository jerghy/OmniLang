"""Semantic CSS property mapping — 50+ properties for cross-framework styles.

Each semantic property maps to CSS (React/Vue web), Dioxus (inline style string),
and Flutter (Widget constructor parameters).

Design: OmniLang_v0.3_设计方案.md §七
"""

# ── Layout ────────────────────────────────────────────

LAYOUT = {
    "layout": {
        "css": {"column": "display: flex; flex-direction: column",
                "row": "display: flex; flex-direction: row"},
        "dioxus": {"column": "display: flex; flex-direction: column",
                   "row": "display: flex; flex-direction: row"},
        "flutter": {"column": "Column", "row": "Row"},
    },
    "direction": {
        "css": {"column": "flex-direction: column", "row": "flex-direction: row",
                "column_reverse": "flex-direction: column-reverse",
                "row_reverse": "flex-direction: row-reverse"},
    },
    "wrap": {
        "css": {"true": "flex-wrap: wrap", "false": "flex-wrap: nowrap"},
        "flutter": {"true": "Wrap", "false": None},  # None = use Row/Column
    },
    "align": {
        "css": {"center": "align-items: center", "start": "align-items: flex-start",
                "end": "align-items: flex-end", "stretch": "align-items: stretch",
                "baseline": "align-items: baseline"},
        "flutter": {"center": "CrossAxisAlignment.center",
                    "start": "CrossAxisAlignment.start",
                    "end": "CrossAxisAlignment.end",
                    "stretch": "CrossAxisAlignment.stretch"},
    },
    "justify": {
        "css": {"center": "justify-content: center",
                "start": "justify-content: flex-start",
                "end": "justify-content: flex-end",
                "space_between": "justify-content: space-between",
                "space_around": "justify-content: space-around",
                "space_evenly": "justify-content: space-evenly"},
        "flutter": {"center": "MainAxisAlignment.center",
                    "start": "MainAxisAlignment.start",
                    "end": "MainAxisAlignment.end",
                    "space_between": "MainAxisAlignment.spaceBetween",
                    "space_around": "MainAxisAlignment.spaceAround",
                    "space_evenly": "MainAxisAlignment.spaceEvenly"},
    },
    "align_self": {
        "css": {"center": "align-self: center", "start": "align-self: flex-start",
                "end": "align-self: flex-end", "stretch": "align-self: stretch"},
    },
    "flex": {
        "css": lambda v: f"flex: {v}",
        "flutter": lambda v: f"flex: {v}",
    },
    "overflow": {
        "css": {"hidden": "overflow: hidden", "scroll": "overflow: scroll",
                "visible": "overflow: visible", "auto": "overflow: auto"},
    },
}

# ── Spacing ───────────────────────────────────────────

def _spacing_value(v: str) -> str:
    """Format spacing value: 8 → '8px', '8px 24px' → pass through."""
    if ' ' in str(v):
        return str(v)  # multi-value shorthand, pass through
    return f"{v}px"

SPACING = {
    "gap": {"css": lambda v: f"gap: {_spacing_value(v)}"},
    "padding": {"css": lambda v: f"padding: {_spacing_value(v)}"},
    "padding_top": {"css": lambda v: f"padding-top: {_spacing_value(v)}"},
    "padding_bottom": {"css": lambda v: f"padding-bottom: {_spacing_value(v)}"},
    "padding_left": {"css": lambda v: f"padding-left: {_spacing_value(v)}"},
    "padding_right": {"css": lambda v: f"padding-right: {_spacing_value(v)}"},
    "padding_horizontal": {"css": lambda v: f"padding-left: {v}px; padding-right: {v}px"},
    "padding_vertical": {"css": lambda v: f"padding-top: {v}px; padding-bottom: {v}px"},
    "margin": {"css": lambda v: f"margin: {_spacing_value(v)}"},
    "margin_top": {"css": lambda v: f"margin-top: {_spacing_value(v)}"},
    "margin_bottom": {"css": lambda v: f"margin-bottom: {_spacing_value(v)}"},
    "margin_left": {"css": lambda v: f"margin-left: {_spacing_value(v)}"},
    "margin_right": {"css": lambda v: f"margin-right: {_spacing_value(v)}"},
    "margin_horizontal": {"css": lambda v: f"margin-left: {v}px; margin-right: {v}px"},
    "margin_vertical": {"css": lambda v: f"margin-top: {v}px; margin-bottom: {v}px"},
}

# ── Sizing ────────────────────────────────────────────

SIZING = {
    "width": {"css": lambda v: f"width: {v}px", "flutter": lambda v: f"width: {v}"},
    "height": {"css": lambda v: f"height: {v}px", "flutter": lambda v: f"height: {v}"},
    "min_width": {"css": lambda v: f"min-width: {v}px"},
    "max_width": {"css": lambda v: f"max-width: {v}px"},
    "min_height": {"css": lambda v: f"min-height: {v}px"},
    "max_height": {"css": lambda v: f"max-height: {v}px"},
    "size": {"css": lambda v: f"width: {v}px; height: {v}px"},
}

# ── Typography ────────────────────────────────────────

TYPOGRAPHY = {
    "font_size": {"css": lambda v: f"font-size: {v}px"},
    "font_weight": {
        "css": {"normal": "font-weight: 400", "bold": "font-weight: 700",
                "light": "font-weight: 300", "medium": "font-weight: 500",
                "semibold": "font-weight: 600", "black": "font-weight: 900"},
    },
    "font_family": {"css": lambda v: f"font-family: {v}"},
    "line_height": {"css": lambda v: f"line-height: {v}"},
    "letter_spacing": {"css": lambda v: f"letter-spacing: {v}px"},
    "text_align": {
        "css": {"left": "text-align: left", "center": "text-align: center",
                "right": "text-align: right", "justify": "text-align: justify"},
    },
    "text_decoration": {
        "css": {"none": "text-decoration: none", "underline": "text-decoration: underline",
                "line_through": "text-decoration: line-through"},
    },
    "text_transform": {
        "css": {"none": "text-transform: none", "uppercase": "text-transform: uppercase",
                "lowercase": "text-transform: lowercase", "capitalize": "text-transform: capitalize"},
    },
    "white_space": {
        "css": {"normal": "white-space: normal", "nowrap": "white-space: nowrap",
                "pre": "white-space: pre", "pre_wrap": "white-space: pre-wrap"},
    },
    "word_break": {
        "css": {"normal": "word-break: normal", "break_all": "word-break: break-all",
                "break_word": "word-break: break-word"},
    },
}

# ── Colors ────────────────────────────────────────────

COLORS = {
    "color": {"css": lambda v: f"color: {v}"},
    "background": {"css": lambda v: f"background: {v}"},
    "background_color": {"css": lambda v: f"background-color: {v}"},
    "opacity": {"css": lambda v: f"opacity: {v}"},
}

# ── Borders ───────────────────────────────────────────

BORDERS = {
    "border_radius": {"css": lambda v: f"border-radius: {v}px"},
    "border_radius_top_left": {"css": lambda v: f"border-top-left-radius: {v}px"},
    "border_radius_top_right": {"css": lambda v: f"border-top-right-radius: {v}px"},
    "border_radius_bottom_left": {"css": lambda v: f"border-bottom-left-radius: {v}px"},
    "border_radius_bottom_right": {"css": lambda v: f"border-bottom-right-radius: {v}px"},
    "border_width": {"css": lambda v: f"border-width: {v}px"},
    "border_color": {"css": lambda v: f"border-color: {v}"},
    "border_style": {
        "css": {"solid": "border-style: solid", "dashed": "border-style: dashed",
                "dotted": "border-style: dotted", "none": "border-style: none"},
    },
}

# ── Shadows & Effects ─────────────────────────────────

SHADOWS = {
    "shadow": {
        "css": {"none": "box-shadow: none",
                "sm": "box-shadow: 0 1px 2px rgba(0,0,0,0.1)",
                "md": "box-shadow: 0 4px 6px rgba(0,0,0,0.1)",
                "lg": "box-shadow: 0 10px 15px rgba(0,0,0,0.1)",
                "xl": "box-shadow: 0 20px 25px rgba(0,0,0,0.15)"},
    },
    "shadow_color": {"css": lambda v: f"box-shadow: {v}"},
}

# ── Positioning ───────────────────────────────────────

POSITIONING = {
    "position": {
        "css": {"relative": "position: relative", "absolute": "position: absolute",
                "fixed": "position: fixed", "sticky": "position: sticky"},
    },
    "z_index": {"css": lambda v: f"z-index: {v}"},
    "top": {"css": lambda v: f"top: {v}px"},
    "bottom": {"css": lambda v: f"bottom: {v}px"},
    "left": {"css": lambda v: f"left: {v}px"},
    "right": {"css": lambda v: f"right: {v}px"},
}

# ── Interactivity ─────────────────────────────────────

INTERACTIVITY = {
    "cursor": {
        "css": {"pointer": "cursor: pointer", "default": "cursor: default",
                "text": "cursor: text", "not_allowed": "cursor: not-allowed"},
    },
    "pointer_events": {
        "css": {"none": "pointer-events: none", "auto": "pointer-events: auto"},
    },
    "user_select": {
        "css": {"none": "user-select: none", "text": "user-select: text",
                "all": "user-select: all"},
    },
}


# ── Combined Mapping ──────────────────────────────────

# All 50+ semantic properties organized by category
SEMANTIC_CSS_MAP: dict[str, dict | callable] = {}

def _merge_category(category: dict):
    SEMANTIC_CSS_MAP.update(category)

_merge_category(LAYOUT)
_merge_category(SPACING)
_merge_category(SIZING)
_merge_category(TYPOGRAPHY)
_merge_category(COLORS)
_merge_category(BORDERS)
_merge_category(SHADOWS)
_merge_category(POSITIONING)
_merge_category(INTERACTIVITY)


def lookup_css(property_name: str, value: str) -> str | None:
    """Look up a semantic property → CSS value mapping.

    Returns a CSS declaration string or None if not found.
    """
    mapping = SEMANTIC_CSS_MAP.get(property_name)
    if mapping is None:
        return None

    css_map = mapping.get("css", mapping)
    if callable(css_map):
        return css_map(value)
    if isinstance(css_map, dict):
        return css_map.get(value)
    return None


def lookup_flutter(property_name: str, value: str) -> str | None:
    """Look up a semantic property → Flutter Widget parameter."""
    mapping = SEMANTIC_CSS_MAP.get(property_name)
    if mapping is None:
        return None

    flutter_map = mapping.get("flutter")
    if flutter_map is None:
        return None
    if callable(flutter_map):
        return flutter_map(value)
    if isinstance(flutter_map, dict):
        return flutter_map.get(value)
    return None


__all__ = [
    "SEMANTIC_CSS_MAP",
    "LAYOUT", "SPACING", "SIZING", "TYPOGRAPHY",
    "COLORS", "BORDERS", "SHADOWS", "POSITIONING", "INTERACTIVITY",
    "lookup_css", "lookup_flutter",
]
