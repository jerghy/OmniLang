// ─── Property → Unit mapping (kebab-case keys) ────────
export const PROPERTY_UNIT_MAP: Record<string, { default: string; common: string[]; advanced?: string[] }> = {
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
export function getUnitGroup(propName: string): { default: string; common: string[]; advanced?: string[] } | null {
  return PROPERTY_UNIT_MAP[propName] ?? null;
}

/** Get dropdown options for a property, or null if text input */
export function getDropdownOptions(propName: string): string[] | null {
  const opts = DROPDOWN_PROPERTIES[propName];
  if (opts === undefined) return null;   // not in dropdown list
  return opts;                            // string[] or null (free text)
}

