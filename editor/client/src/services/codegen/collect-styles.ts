// editor/client/src/services/codegen/collect-styles.ts
// Collect and merge style data from design-element StyleEntry[] entries and
// ViewNode resolvedStyle trees into the componentStyle map, elementClassMap,
// and elementInlineMap consumed by SourceEmitter.

import { resolveStyle } from "./style-resolver";
import { variableToClassName } from "../../core/types";
import type { StyleEntry } from "../../core/types";
import type { ViewNode } from "./types";

// ── applyStyles ───────────────────────────────────────────────────────

/**
 * Walk the ViewNode tree and call resolveStyle on each node,
 * setting the `resolvedStyle` property in place.
 */
export function applyStyles(roots: ViewNode[]): void {
  for (const r of roots) {
    walkApply(r);
  }
  function walkApply(n: ViewNode): void {
    n.resolvedStyle = resolveStyle(n);
    for (const c of n.children) walkApply(c);
  }
}

// ── collectStylesFromEntries ──────────────────────────────────────────

/**
 * Build the final style maps from ElementData StyleEntry[] and an optional
 * ViewNode tree.
 *
 * Returns three data structures consumed by SourceEmitter:
 *   - componentStyle:   class name → { property: value } (for the style {} block)
 *   - elementClassMap:  element ID → class names (for class: attrib on view nodes)
 *   - elementInlineMap: element ID → { property: value } (for inline style: on view nodes)
 */
export function collectStylesFromEntries(
  elements: Array<{ id: string; styles?: StyleEntry[] }>,
  roots?: ViewNode[],
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
} {
  const componentStyle: Record<string, Record<string, string>> = {};
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

  // Phase 1: Process StyleEntry[] from design elements (new style-panel path)
  for (const el of elements) {
    if (!el.styles?.length) continue;
    const classNames: string[] = [];
    const inlineStyles: Record<string, string> = {};

    for (const entry of el.styles) {
      switch (entry.type) {
        case "class": {
          const rules: Record<string, string> = {};
          for (const prop of entry.properties) {
            rules[prop.name] = prop.value;
          }
          // Issue 2: merge instead of overwrite on class name collision
          if (componentStyle[entry.name]) {
            componentStyle[entry.name] = { ...componentStyle[entry.name], ...rules };
          } else {
            componentStyle[entry.name] = rules;
          }
          classNames.push(entry.name);
          break;
        }
        case "variable": {
          const varName = entry.name;
          const className = variableToClassName(varName);
          const rules = { [varName]: entry.properties[0]?.value ?? "" };
          // Issue 2: merge instead of overwrite on class name collision
          if (componentStyle[className]) {
            componentStyle[className] = { ...componentStyle[className], ...rules };
          } else {
            componentStyle[className] = rules;
          }
          classNames.push(className);
          break;
        }
        case "inline": {
          for (const prop of entry.properties) {
            inlineStyles[prop.name] = prop.value;
          }
          break;
        }
      }
    }

    // Issue 3: deduplicate class names before storing
    if (classNames.length > 0) {
      elementClassMap.set(el.id, [...new Set(classNames)]);
    }
    if (Object.keys(inlineStyles).length > 0) {
      elementInlineMap.set(el.id, inlineStyles);
    }
  }

  // Phase 2: Merge resolvedStyle.cssRules from ViewNode tree (Issue 1).
  // applyStyles(roots) computed resolvedStyle on each ViewNode with cssRules from
  // snake_case props (e.g. background_color -> background).  Make sure those rules
  // end up in componentStyle under the correct class name so they appear in the
  // emitted style block, not just as inline HTML attrs.
  if (roots) {
    walkViewNodes(roots, (n) => {
      if (!n.resolvedStyle) return;
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

// ── walkViewNodes ──────────────────────────────────────────────────────

/** Walk a ViewNode tree breadth-first, invoking callback on every node. */
export function walkViewNodes(nodes: ViewNode[], callback: (node: ViewNode) => void): void {
  for (const n of nodes) {
    callback(n);
    walkViewNodes(n.children, callback);
  }
}
