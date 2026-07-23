// poiesis/src/core/types/design.ts
// Design system types for design.omni
import type { StyleProperty } from "./style";

// ── Token ────────────────────────────────────────────────────
export type TokenType =
  | "color"
  | "size"
  | "radius"
  | "fontFamily"
  | "fontSize"
  | "fontWeight"
  | "lineHeight"
  | "letterSpacing"
  | "shadow"
  | "duration"
  | "easing"
  | "opacity"
  | "zIndex";

export interface DesignToken {
  id: string;
  name: string;
  category: string;        // "colors", "spacing", "typography"
  tokenType: TokenType;
  value: string;           // "var(--ol-color-primary)", "16px"
}

// ── Design Class ─────────────────────────────────────────────
export interface DesignClass {
  id: string;
  name: string;            // "Card", "BtnPrimary"
  properties: StyleProperty[];
  pseudoStates: PseudoState[];
}

export interface PseudoState {
  id: string;
  pseudo: string;          // "hover", "focus", "active"
  properties: StyleProperty[];
}

// ── Variant ──────────────────────────────────────────────────
export interface VariantDefinition {
  id: string;
  name: string;            // "Button"
  dimensions: VariantDimension[];
  defaults: Record<string, string>;
  baseProperties: StyleProperty[];
  branches: VariantBranch[];
  /** Property names pinned to base — branches inherit them read-only. */
  inheritedProps?: string[];
}

export interface VariantDimension {
  name: string;
  values: string[];
}

export interface VariantBranch {
  id: string;
  dimension: string;
  value: string;
  properties: StyleProperty[];
}

// ── Design Definition ────────────────────────────────────────
export interface DesignDefinition {
  name: string;            // "AppDesign"
  tokens: DesignToken[];
  classes: DesignClass[];
  variants: VariantDefinition[];
}

// ── Element bindings ─────────────────────────────────────────
export interface ElementVariantBinding {
  variantName: string;
  dimensionValues: Record<string, string>;
}

// ── Design annotations (for round-trip stability) ────────────
/**
 * @deprecated Use parsePidAnnotation from annotation-registry instead.
 * Kept for backward compatibility with old // @token/@class/@variant annotations.
 * Remove after all projects have been migrated.
 */
export const DESIGN_ANNOTATION_PREFIX = {
  token: "// @token",
  class: "// @class",
  variant: "// @variant",
} as const;

export function parseDesignAnnotation(
  line: string
): { type: "token" | "class" | "variant"; id: string } | null {
  const trimmed = line.trim();
  for (const [type, prefix] of Object.entries(DESIGN_ANNOTATION_PREFIX)) {
    if (trimmed.startsWith(prefix)) {
      const match = trimmed.match(/id=(\S+)/);
      if (match) {
        return { type: type as "token" | "class" | "variant", id: match[1] };
      }
    }
  }
  return null;
}

// ── Constants ────────────────────────────────────────────────
export const EMPTY_DESIGN: DesignDefinition = {
  name: "",
  tokens: [],
  classes: [],
  variants: [],
};

/** Map a tokenType to the TokenEditor category name for UI grouping. */
export function tokenTypeToCategory(tokenType: string): string {
  switch (tokenType) {
    case "color":         return "colors";
    case "size":          return "spacing";
    case "radius":        return "radii";
    case "fontFamily":    return "typography";
    case "fontSize":      return "typography";
    case "fontWeight":    return "typography";
    case "lineHeight":    return "typography";
    case "letterSpacing": return "typography";
    case "shadow":        return "shadows";
    case "duration":      return "animation";
    case "easing":        return "animation";
    case "opacity":       return "effects";
    case "zIndex":        return "layout";
    default:              return "custom";
  }
}
