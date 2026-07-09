// poiesis/src/services/codegen/design-emitter.ts
// Emit design.omni source code from a DesignDefinition object.

import type {
  DesignDefinition,
  DesignToken,
  DesignClass,
  VariantDefinition,
} from "../../core/types/design";
import type { StyleValueSegment } from "../../core/types/style";

// ── Helpers ───────────────────────────────────────────────────────────────

/** Convert camelCase to kebab-case for CSS property name emission. */
function camelToKebab(name: string): string {
  return name.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`);
}

/**
 * Compile StyleValueSegment[] into an OmniLang expression string for design.omni emission.
 *
 * Unlike compileSegments (which emits CSS syntax like `var(--name)`), this emits
 * omni-native syntax:
 *   - text segments → quoted string
 *   - var-ref segments → `token(name)` (omni compile-time resolution)
 */
function compileDesignSegments(segments: StyleValueSegment[]): string {
  if (segments.length === 0) return "";
  if (segments.length === 1) {
    const s = segments[0];
    if (s.type === "text") return `"${s.value}"`;
    if (s.type === "var-ref") return `token(${s.name})`;
    return "";
  }
  return segments
    .map((s) => {
      if (s.type === "text") return `"${s.value}"`;
      if (s.type === "var-ref") return `token(${s.name})`;
      return "";
    })
    .join(" + ");
}

// ── emitDesignSource ──────────────────────────────────────────────────────

/**
 * Generate the .omni source code for a DesignDefinition.
 *
 * Produces output like:
 *
 * ```omni
 * import omniview;
 *
 * design AppDesign {
 *     // @token id=tk_1
 *     token primary: color = "#6366f1";
 *
 *     // @class id=cls_1
 *     class Card {
 *         background: "#ffffff";
 *         &:hover {
 *             box-shadow: "0 4px 12px rgba(0,0,0,0.08)";
 *         }
 *     }
 *
 *     // @variant id=var_1
 *     variant Button {
 *         @dim color: primary | danger;
 *         @dim size: sm | md;
 *         @default { color: primary; size: md; }
 *         base {
 *             display: inline-flex;
 *         }
 *         color.primary { background: "#6366f1"; }
 *         size.sm { padding: "4px 8px"; }
 *     }
 * }
 * ```
 */
export function emitDesignSource(design: DesignDefinition): string {
  const lines: string[] = [];

  lines.push("import omniview;");
  lines.push("");

  // Design block
  const designName = design.name || "AppDesign";
  lines.push(`design ${designName} {`);
  lines.push("");

  // ── Tokens ──
  for (const token of design.tokens) {
    emitToken(lines, token);
  }

  // ── Classes ──
  for (const cls of design.classes) {
    emitClass(lines, cls);
  }

  // ── Variants ──
  for (const variant of design.variants) {
    emitVariant(lines, variant);
  }

  lines.push("}");
  lines.push("");

  return lines.join("\n");
}

// ── Helpers ───────────────────────────────────────────────────────────────

function emitToken(lines: string[], token: DesignToken): void {
  lines.push(`    // @token id=${token.id}`);
  const value = `"${token.value}"`;
  lines.push(`    token ${token.name}: ${token.tokenType} = ${value};`);
  lines.push("");
}

function emitClass(lines: string[], cls: DesignClass): void {
  lines.push(`    // @class id=${cls.id}`);
  lines.push(`    class ${cls.name} {`);

  for (const prop of cls.properties) {
    const val = compileDesignSegments(prop.segments);
    lines.push(`        ${camelToKebab(prop.name)}: ${val};`);
  }

  for (const ps of cls.pseudoStates) {
    lines.push(`        &:${ps.pseudo} {`);
    for (const prop of ps.properties) {
      const val = compileDesignSegments(prop.segments);
      lines.push(`            ${camelToKebab(prop.name)}: ${val};`);
    }
    lines.push(`        }`);
  }

  lines.push(`    }`);
  lines.push("");
}

function emitVariant(lines: string[], variant: VariantDefinition): void {
  lines.push(`    // @variant id=${variant.id}`);
  lines.push(`    variant ${variant.name} {`);

  // Dimensions
  for (const dim of variant.dimensions) {
    lines.push(`        @dim ${dim.name}: ${dim.values.join(" | ")};`);
  }

  // Defaults
  if (Object.keys(variant.defaults).length > 0) {
    lines.push(`        @default {`);
    for (const [key, val] of Object.entries(variant.defaults)) {
      lines.push(`            ${key}: ${val};`);
    }
    lines.push(`        }`);
  }

  // Base properties
  if (variant.baseProperties.length > 0) {
    lines.push(`        base {`);
    for (const prop of variant.baseProperties) {
      const val = compileDesignSegments(prop.segments);
      lines.push(`            ${camelToKebab(prop.name)}: ${val};`);
    }
    lines.push(`        }`);
  }

  // Branch blocks
  for (const branch of variant.branches) {
    lines.push(`        ${branch.dimension}.${branch.value} {`);
    for (const prop of branch.properties) {
      const val = compileDesignSegments(prop.segments);
      lines.push(`            ${camelToKebab(prop.name)}: ${val};`);
    }
    lines.push(`        }`);
  }

  lines.push(`    }`);
  lines.push("");
}

// ── exports for pipeline.ts ───────────────────────────────────────────────

export { emitDesignSource as generateDesignOmniSource };
