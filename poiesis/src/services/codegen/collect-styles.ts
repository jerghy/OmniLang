// poiesis/src/services/codegen/collect-styles.ts
// Collect and merge style data from StyleRegistry + ElementStyle entries and
// ViewNode resolvedStyle trees into the componentStyle map, elementClassMap,
// and elementInlineMap consumed by SourceEmitter.

import type { StyleRegistry, StyleValueSegment } from "../../core/types";
import type { ViewNode } from "./types";

// ── applyStyles ───────────────────────────────────────────────────────

/**
 * Stub: style resolution is now performed by WASM resolve_styles().
 * Kept as a no-op to avoid breaking the pipeline call site.
 */
export function applyStyles(_roots: ViewNode[]): void {
  // No-op: WASM handles style resolution
}

// ── collectStylesFromEntries ──────────────────────────────────────────

/**
 * Build the final style maps from ElementData (with ElementStyle).
 *
 * Returns three data structures consumed by SourceEmitter:
 *   - componentStyle:   always empty — CSS comes from WASM resolve_styles()
 *   - elementClassMap:  element ID → design class references (for class: attrib on view nodes)
 *   - elementInlineMap: element ID → { property: value } (for inline style: on view nodes)
 */
export function collectStylesFromEntries(
  elements: Array<{ id: string; style?: { classRefs?: string[]; inlineProperties?: Array<{ name: string; segments: StyleValueSegment[] }> } }>,
  _styleRegistry: StyleRegistry,
  _roots?: ViewNode[],
  designName?: string,
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
} {
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

  // Element classRefs → elementClassMap (design classes from classRefs)
  // inlineProperties → elementInlineMap (inline/dynamic styles)
  for (const el of elements) {
    const classRefs = el.style?.classRefs ?? [];
    if (classRefs.length > 0) {
      const normalized = classRefs.map((ref) => {
        if (ref.includes(".")) return ref;              // already "Design.Class"
        if (designName) return `${designName}.${ref}`;  // prefix bare name
        return ref;                                      // no designName, keep as-is
      });
      elementClassMap.set(el.id, [...new Set(normalized)]);
    }
    const inlineProps = el.style?.inlineProperties ?? [];
    if (inlineProps.length > 0) {
      const inlineMap: Record<string, string> = {};
      for (const prop of inlineProps) {
        inlineMap[prop.name] = compileSegments(prop.segments);
      }
      elementInlineMap.set(el.id, inlineMap);
    }
  }

  // componentStyle is now empty — CSS comes from WASM resolve_styles()
  return { componentStyle: {}, elementClassMap, elementInlineMap };
}

// ── compileSegments ──────────────────────────────────────────────────

/** Ensure a var-ref name has the `--` CSS custom property prefix. */
function normalizeVarRef(name: string): string {
  return name.startsWith("--") ? name : `--${name}`;
}

/**
 * Compile StyleValueSegment[] into an OmniLang expression string.
 *
 * - text segments are wrapped in double quotes
 * - signal segments appear bare (they resolve at runtime)
 * - var-ref segments produce var(--name)
 * - multiple segments are joined with ` + `
 */
export function compileSegments(segments: StyleValueSegment[]): string {
  if (segments.length === 0) return "";
  if (segments.length === 1) {
    const s = segments[0];
    if (s.type === "text") return `"${s.value}"`;
    if (s.type === "signal") return s.name;
    if (s.type === "var-ref") return `var(${normalizeVarRef(s.name)})`;
    return "";
  }
  return segments.map((s) => {
    if (s.type === "text") return `"${s.value}"`;
    if (s.type === "signal") return s.name;
    if (s.type === "var-ref") return `var(${normalizeVarRef(s.name)})`;
    return "";
  }).join(" + ");
}

// ── walkViewNodes ──────────────────────────────────────────────────────

/** Walk a ViewNode tree breadth-first, invoking callback on every node. */
export function walkViewNodes(nodes: ViewNode[], callback: (node: ViewNode) => void): void {
  for (const n of nodes) {
    callback(n);
    switch (n.kind) {
      case "element":
      case "for":
      case "slot":
        walkViewNodes(n.children, callback);
        break;
      case "if":
        for (const branch of n.branches) {
          walkViewNodes(branch.body, callback);
        }
        if (n.fallback) walkViewNodes(n.fallback, callback);
        break;
      case "text":
      case "interpolation":
        break;
    }
  }
}
