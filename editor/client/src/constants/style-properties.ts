// ─── Types ───────────────────────────────────────────
export interface StyleProperty {
  name: string;   // kebab-case CSS property name, e.g. "font-size", "--primary-color"
  value: string;  // CSS value, e.g. "16px", "#333", "var(--primary-color)"
}

export interface StyleEntry {
  id: string;
  type: "class" | "variable" | "inline";
  name: string;  // class name | variable name (--xxx) | "" (inline)
  properties: StyleProperty[];
}

// ─── Unit group type ──────────────────────────────────
export interface UnitGroup {
  default: string;        // default unit ("" = unitless)
  common: string[];       // always-visible units
  advanced?: string[];    // hidden behind "more" toggle
}

// ─── Style property name set (for migration + parsing) ─
export const STYLE_PROPERTY_NAMES = new Set([
  // From STYLE_PROP_CSS_MAP legacy
  "background_color", "border_radius", "font_size", "font_weight",
  "text_align", "shadow", "padding", "margin", "gap",
  "width", "height", "color",
  // Additional CSS properties (kebab-case + camelCase aliases)
  "display", "position", "opacity", "overflow", "overflow-x", "overflow_x", "overflowX",
  "overflow-y", "overflow_y", "overflowY", "z-index", "z_index", "zIndex",
  "box-shadow", "box_shadow", "boxShadow",
  "border", "border-color", "border_color", "borderColor",
  "font-family", "font_family", "fontFamily",
  "line-height", "line_height", "lineHeight",
  "letter-spacing", "letter_spacing", "letterSpacing",
  "text-decoration", "text_decoration", "textDecoration",
  "text-transform", "text_transform", "textTransform",
  "white-space", "white_space", "whiteSpace",
  "word-break", "word_break", "wordBreak",
  "min-width", "min_width", "minWidth",
  "max-width", "max_width", "maxWidth",
  "min-height", "min_height", "minHeight",
  "max-height", "max_height", "maxHeight",
  "transform", "transition", "animation", "cursor",
  "box-sizing", "box_sizing", "boxSizing",
  "flex-direction", "flex_direction", "flexDirection",
  "flex-wrap", "flex_wrap", "flexWrap",
  "flex-grow", "flex_grow", "flexGrow",
  "flex-shrink", "flex_shrink", "flexShrink",
  "flex-basis", "flex_basis", "flexBasis",
  "justify-content", "justify_content", "justifyContent",
  "align-items", "align_items", "alignItems",
  "align-self", "align_self", "alignSelf",
  "grid-template-columns", "grid_template_columns", "gridTemplateColumns",
  "grid-template-rows", "grid_template_rows", "gridTemplateRows",
  "grid-column", "grid_column", "gridColumn",
  "grid-row", "grid_row", "gridRow",
  "background-image", "background_image", "backgroundImage",
  "transition-property", "transition_property", "transitionProperty",
  "transition-duration", "transition_duration", "transitionDuration",
  "transition-timing-function", "transition_timing_function", "transitionTimingFunction",
  "animation-name", "animation_name", "animationName",
  "animation-duration", "animation_duration", "animationDuration",
  "animation-iteration-count", "animation_iteration_count", "animationIterationCount",
  "animation-direction", "animation_direction", "animationDirection",
  "animation-fill-mode", "animation_fill_mode", "animationFillMode",
  "animation-timing-function", "animation_timing_function", "animationTimingFunction",
]);

// ─── Property → Unit mapping (kebab-case keys) ────────
export const PROPERTY_UNIT_MAP: Record<string, UnitGroup> = {
  // Length
  width:        { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "vmin", "vmax", "auto"] },
  height:       { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "vmin", "vmax", "auto"] },
  "min-width":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh"] },
  "max-width":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "none"] },
  "min-height": { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh"] },
  "max-height": { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "none"] },
  padding:      { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-top":    { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-right":  { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-bottom": { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-left":   { default: "px", common: ["px", "rem", "em", "%"] },
  margin:       { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-top":    { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-right":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-bottom": { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-left":   { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  gap:          { default: "px", common: ["px", "rem", "em", "%"] },

  // Typography
  "font-size":      { default: "px", common: ["px", "rem", "em", "%"], advanced: ["pt", "ch"] },
  "line-height":    { default: "", common: ["", "px", "rem", "em", "%"], advanced: ["ch"] },
  "letter-spacing": { default: "px", common: ["px", "rem", "em"] },

  // Borders & effects
  "border-radius":  { default: "px", common: ["px", "rem", "em", "%"] },
  "box-shadow":     { default: "px", common: ["px"] },

  // Time
  "transition-duration": { default: "ms", common: ["ms", "s"] },
  "animation-duration":  { default: "ms", common: ["ms", "s"] },
};

// ─── Dropdown-only properties (kebab-case keys) ───────
// null = free text input; string[] = dropdown options
export const DROPDOWN_PROPERTIES: Record<string, string[] | null> = {
  display:            ["block", "flex", "grid", "inline", "inline-block", "inline-flex", "none"],
  position:           ["static", "relative", "absolute", "fixed", "sticky"],
  "font-weight":      ["100", "200", "300", "400", "500", "600", "700", "800", "900", "normal", "bold", "lighter", "bolder"],
  "text-align":       ["left", "center", "right", "justify", "start", "end"],
  overflow:           ["visible", "hidden", "scroll", "auto"],
  "overflow-x":       ["visible", "hidden", "scroll", "auto"],
  "overflow-y":       ["visible", "hidden", "scroll", "auto"],
  cursor:             ["auto", "default", "pointer", "text", "move", "not-allowed", "grab", "grabbing", "crosshair", "zoom-in", "zoom-out"],
  "box-sizing":       ["content-box", "border-box"],
  "text-transform":   ["none", "uppercase", "lowercase", "capitalize"],
  "text-decoration":  ["none", "underline", "line-through", "overline"],
  "white-space":      ["normal", "nowrap", "pre", "pre-wrap", "pre-line"],
  "word-break":       ["normal", "break-all", "break-word", "keep-all"],
  "flex-direction":   ["row", "row-reverse", "column", "column-reverse"],
  "flex-wrap":        ["nowrap", "wrap", "wrap-reverse"],
  "justify-content":  ["flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly"],
  "align-items":      ["stretch", "flex-start", "flex-end", "center", "baseline"],
  "align-self":       ["auto", "stretch", "flex-start", "flex-end", "center", "baseline"],
  "transition-timing-function": ["ease", "ease-in", "ease-out", "ease-in-out", "linear", "step-start", "step-end"],
  "animation-iteration-count":  ["infinite", "1", "2", "3"],
  "animation-direction":        ["normal", "reverse", "alternate", "alternate-reverse"],
  "animation-fill-mode":        ["none", "forwards", "backwards", "both"],
  "animation-timing-function":  ["ease", "ease-in", "ease-out", "ease-in-out", "linear"],
  // Free-text properties (null = text input)
  "grid-template-columns": null,
  "grid-template-rows": null,
  "grid-column": null,
  "grid-row": null,
  "transition-property": null,
  "animation-name": null,
  "flex-grow": null,
  "flex-shrink": null,
  "flex-basis": null,
  "font-family": null,
  "background-image": null,
  "box-shadow": null,
  transform: null,
};

// ─── Helpers ──────────────────────────────────────────

/** Detect if a property name is a CSS custom property */
export function isCustomProperty(name: string): boolean {
  return name.startsWith("--");
}

/** Detect if a property is color-like (needs color picker) */
export function isColorProperty(name: string): boolean {
  return name.includes("color") && !isCustomProperty(name);
}

/** Get the unit group for a property, or null if unitless */
export function getUnitGroup(propName: string): UnitGroup | null {
  return PROPERTY_UNIT_MAP[propName] ?? null;
}

/** Get dropdown options for a property, or null if text input */
export function getDropdownOptions(propName: string): string[] | null {
  const opts = DROPDOWN_PROPERTIES[propName];
  if (opts === undefined) return null;   // not in dropdown list
  return opts;                            // string[] or null (free text)
}

/** Variable name "--primary-color" → class name "_v-primary-color" */
export function variableToClassName(varName: string): string {
  return "_v-" + varName.slice(2); // strip --, prefix _v-
}

/** Class name "_v-primary-color" → variable name "--primary-color" */
export function classNameToVariable(className: string): string | null {
  if (className.startsWith("_v-")) {
    return "--" + className.slice(3);
  }
  return null;
}
