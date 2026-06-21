// editor/client/src/omni-gen/custom-component-emitter.ts
// Phase C: Emit custom component definitions as standalone OmniLang source files.
//
// Reuses the existing pipeline modules:
//   - ViewTreeBuilder: convert ElementData → ViewNode tree
//   - StyleResolver: resolve styles on each ViewNode
//   - SourceEmitter: emit the full component document

import { buildViewTree } from "./view-tree-builder";
import { resolveStyle } from "./style-resolver";
import { SourceEmitter } from "./source-emitter";
import type { ElementData } from "../store/editor";
import type { StageResult, EmitInput, ViewNode } from "./types";

/**
 * Emit standalone OmniLang source files for each custom component.
 *
 * Given an array of custom component definitions (name + elements),
 * produces a map of file path → OmniLang source.
 */
export function emitCustomComponents(
  customComponents: Array<{ name: string; elements: ElementData[] }>
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

      const componentStyle: Record<string, Record<string, string>> = {};
      collectStylesFromRoots(roots, componentStyle);

      const emitter = new SourceEmitter();
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
          view: roots[0] || null,
          style: componentStyle,
        },
      };

      result[`src/components/${cc.name}.omni`] = emitter.emitDocument(emitInput, new Map());
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
