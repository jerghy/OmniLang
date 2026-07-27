// core/types/style.ts
// Shared style type definitions — redesigned for OmniLang language fidelity.

// ── Value segments (fude model) ──

export type StyleValueSegment =
  | { type: "text"; value: string }
  | { type: "signal"; name: string; label: string }
  | { type: "var-ref"; name: string };

// ── CSS Variable ──

export interface CssVariable {
  id: string;
  name: string;                       // "--primary-color"
  segments: StyleValueSegment[];
}

// ── Style Class ──

export interface StyleClass {
  id: string;
  name: string;                       // "wrapper" | "card.accent" | "btn:hover"
  properties: StyleProperty[];
}

export interface StyleProperty {
  id: string;
  name: string;                       // "padding" | "--radius" | "background"
  segments: StyleValueSegment[];
}

// ── Component-level registry (SSOT) ──

export interface StyleRegistry {
  variables: CssVariable[];
  classes: StyleClass[];
}

// ── Per-element style (references only) ──

/** Per-element style references (NOT CSS values — those come from WASM resolve_styles).
 *  Kept for backward compat; new code should use classRefs/variantRef directly on ViewElementData. */
export interface ElementStyle {
  classRefs: string[];                // ["wrapper", "card"]
  inlineProperties: StyleProperty[];  // inline props (may contain signal bindings)
}
