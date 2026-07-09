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
 * Build the final style maps from ElementData (with ElementStyle) and a
 * StyleRegistry (canonical source of class/variable definitions).
 *
 * Returns three data structures consumed by SourceEmitter:
 *   - componentStyle:   class name → { property: value } (for the style {} block)
 *   - elementClassMap:  element ID → class names (for class: attrib on view nodes)
 *   - elementInlineMap: element ID → { property: value } (for inline style: on view nodes)
 */
export function collectStylesFromEntries(
  elements: Array<{ id: string; style?: { classRefs?: string[]; inlineProperties?: Array<{ name: string; segments: StyleValueSegment[] }> } }>,
  _styleRegistry: StyleRegistry,
  roots?: ViewNode[],
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
} {
  const componentStyle: Record<string, Record<string, string>> = {};
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

  // Phase 1 (REMOVED): Design classes and token variables are emitted only in
  // design.omni via emitDesignSource. They should NOT be duplicated into every
  // component's style { } block. Component-local styles are added by Phase 3 below.

  // Phase 2: Element classRefs → elementClassMap, inlineProperties → elementInlineMap
  // Filter out auto-generated CSS class names (e.g. "MyCard_p_0") that come from
  // the view tree path. Everything else — design-system classes, component-local
  // classes, user-bound classes — passes through into the emitted source.
  const autoClassNames = new Set<string>();
  if (roots) {
    walkViewNodes(roots, (n) => {
      if (n.kind === "element" && n.resolvedStyle?.cssClassName) {
        autoClassNames.add(n.resolvedStyle.cssClassName);
      }
    });
  }

  for (const el of elements) {
    const classRefs = (el.style?.classRefs ?? []).filter(
      (name) => !autoClassNames.has(name),
    );
    if (classRefs.length > 0) {
      elementClassMap.set(el.id, [...new Set(classRefs)]);
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

  // Phase 3: Merge resolvedStyle.cssRules from ViewNode tree.
  // applyStyles(roots) computed resolvedStyle on each ViewNode with cssRules from
  // snake_case props (e.g. background_color -> background).  Make sure those rules
  // end up in componentStyle under the correct class name so they appear in the
  // emitted style block, not just as inline HTML attrs.
  if (roots) {
    walkViewNodes(roots, (n) => {
      if (n.kind !== "element" || !n.resolvedStyle) return;
      const { cssRules, cssClassName } = n.resolvedStyle;
      if (!cssRules || Object.keys(cssRules).length === 0) return;

      if (componentStyle[cssClassName]) {
        componentStyle[cssClassName] = { ...componentStyle[cssClassName], ...cssRules };
      } else {
        componentStyle[cssClassName] = cssRules;
      }
    });
  }

  return { componentStyle, elementClassMap, elementInlineMap };
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
