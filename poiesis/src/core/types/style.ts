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

export interface ElementStyle {
  classRefs: string[];                // ["wrapper", "card"]
  inlineProperties: StyleProperty[];  // inline props (may contain signal bindings)
}

// ── Variable ↔ Class name conversion (for compiler output) ──

export function variableToClassName(varName: string): string {
  return "_v-" + varName.slice(2);
}

export function classNameToVariable(className: string): string | null {
  if (className.startsWith("_v-")) {
    return "--" + className.slice(3);
  }
  return null;
}
