// editor/client/src/omni-gen/custom-component-emitter.ts
// Phase C: Emit custom component definitions as standalone OmniLang source files.
//
// Produces per-spec format:
//   // @code-tab type=logic
//   import omniview;
//   component MyCard {
//       prop title: String = "";
//       event onAction;
//       view {
//           // @edit
//           <div id="card" class: card>...</div>
//       }
//       style { ... }
//   }

import { buildViewTree } from "./view-tree-builder";
import { resolveStyle } from "./style-resolver";
import { SourceEmitter } from "./source-emitter";
import { buildAnnotationData, createAnnotationStrategy } from "./annotation-data";
import { collectStylesFromEntries } from "./pipeline";
import type { ElementData } from "../core/types";
import type { StageResult, EmitInput, ViewNode } from "./types";
import type { StyleEntry, StyleProperty } from "../core/types";

/**
 * Emit standalone OmniLang source files for each custom component.
 *
 * Given an array of custom component definitions (name + elements + optional meta),
 * produces a map of file path → OmniLang source in the spec format:
 *
 *   src/components/{Name}.omni
 *
 * The output is self-contained: `// @code-tab type=logic`, `import omniview;`,
 * `component { prop / event / view / style }`. No separate `// @component-meta` comment.
 */
export function emitCustomComponents(
  customComponents: Array<{ name: string; elements: ElementData[]; meta?: Record<string, unknown> }>
): StageResult<Record<string, string>> {
  const result: Record<string, string> = {};
  const errors: StageResult<Record<string, string>>["errors"] = [];

  for (const cc of customComponents) {
    try {
      const { roots } = buildViewTree(cc.elements, cc.name);
      if (roots.length > 1) {
        errors.push({
          phase: "C",
          module: "CustomComponentEmitter",
          severity: "warning",
          code: "W_MULTI_ROOT",
          message: `Component "${cc.name}" has ${roots.length} root elements; only the first is emitted`,
        });
      }
      applyStylesToRoots(roots);

      // Process StyleEntry[] from elements + merge cssRules from ViewNode tree
      const { componentStyle, elementClassMap, elementInlineMap } =
        collectStylesFromEntries(cc.elements, roots);

      // Build annotation strategy for // @code-tab type=logic and // @edit
      const annData = buildAnnotationData(roots, "logic");
      const annotationStrategy = createAnnotationStrategy(annData);

      // Extract props/events from meta (converted from ComponentDefinition format)
      const meta = cc.meta;
      const props = meta?.props
        ? Object.entries(meta.props as Record<string, unknown>).map(([name, schema]) => ({
            name,
            type: ((schema as Record<string, unknown>).type as string) || "String",
            default: (schema as Record<string, unknown>).default,
          }))
        : [];
      const events = meta?.events
        ? Object.keys(meta.events as Record<string, unknown>).map((name) => ({ name }))
        : [];

      const emitter = new SourceEmitter(annotationStrategy, elementClassMap, elementInlineMap);

      // Build componentMeta from meta (replaces old *.meta.json sidecar)
      const componentMeta: EmitInput["componentMeta"] = meta
        ? {
            id: (meta.id as string) || cc.name,
            category: (meta.category as string) || "Custom",
            description: (meta.description as string) || "",
            children: (meta.children as boolean) ?? true,
            styleProperties: (meta.styleProperties as string[]) || [],
            propSchema: meta.props
              ? Object.fromEntries(
                  Object.entries(meta.props as Record<string, Record<string, unknown>>).map(([name, p]) => [
                    name,
                    { editor: p.editor as string, values: p.values as string[] },
                  ])
                )
              : undefined,
            eventSchema: meta.events
              ? Object.fromEntries(
                  Object.entries(meta.events as Record<string, Record<string, unknown>>).map(([name, e]) => [
                    name,
                    { payload: e.payload as string, description: e.description as string },
                  ])
                )
              : undefined,
          }
        : undefined;

      const emitInput: EmitInput = {
        codeTabType: "logic",
        imports: ["import omniview;"],
        stores: [],
        component: {
          name: cc.name,
          signals: {},
          computeds: {},
          storeRefs: [],
          fns: [],
          props,
          events,
          view: roots[0] || null,
          style: componentStyle,
        },
        componentMeta,
      };

      const omniSource = emitter.emitDocument(emitInput, new Map());
      result[`src/components/${cc.name}.omni`] = omniSource;
    } catch (e) {
      errors.push({
        phase: "C",
        module: "CustomComponentEmitter",
        severity: "fatal",
        code: "E_CC_EMIT",
        message: e instanceof Error ? e.message : String(e),
      });
    }
  }

  return { value: result, errors };
}

// ── Helpers ──

function applyStylesToRoots(roots: ViewNode[]): void {
  for (const r of roots) {
    walkApply(r);
  }
  function walkApply(n: ViewNode): void {
    n.resolvedStyle = resolveStyle(n);
    for (const c of n.children) walkApply(c);
  }
}

function collectStylesFromRoots(
  roots: ViewNode[],
  out: Record<string, Record<string, string>>,
): void {
  for (const r of roots) walkCollect(r);
  function walkCollect(n: ViewNode): void {
    if (n.resolvedStyle && Object.keys(n.resolvedStyle.cssRules).length > 0) {
      out[n.resolvedStyle.cssClassName] = n.resolvedStyle.cssRules;
    }
    for (const c of n.children) walkCollect(c);
  }
}

