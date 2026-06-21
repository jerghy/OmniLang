// editor/client/src/omni-gen/pipeline.ts
// Orchestrates the 3-phase omni-gen pipeline:
//   Phase A: ViewTreeBuilder + WireAnalyzer + ImportCollector
//   Phase B: STUB (StoreExtractor + ExpressionLowering + ChainEmitter)
//   Phase C: StyleResolver + SourceEmitter

import { buildViewTree } from "./view-tree-builder";
import { analyzeWires } from "./wire-analyzer";
import { collectImports } from "./import-collector";
import { resolveStyle } from "./style-resolver";
import { SourceEmitter } from "./source-emitter";
import type { ElementData, WireData } from "../store/editor";
import type { CodeTab } from "../codegraph/types";
import type { StageResult, EmitInput } from "./types";

export function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
}): StageResult<string> {
  const errors: StageResult<string>["errors"] = [];

  // ── Phase A: Build view tree, analyze wires, collect imports ──

  const { roots, pascalCaseTags } = buildViewTree(
    options.designElements,
    options.componentName,
  );
  const bindings = analyzeWires(options.designElements, options.designWires);
  const imports = collectImports(
    pascalCaseTags,
    [...bindings.storeRefs.keys()],
    options.codeTabs,
  );

  // ── Phase B (stub) ──
  // Will be filled by StoreExtractor + ExpressionLowering + ChainEmitter (Tasks 12-14)

  const mainTab = options.codeTabs.find(t => t.id === "main") || options.codeTabs[0];

  // ── Phase C: Apply styles, collect style rules, emit source ──

  applyStyles(roots);

  const componentStyle: Record<string, Record<string, string>> = {};
  collectStyles(roots, componentStyle);

  const emitInput: EmitInput = {
    codeTabType: mainTab.type as "logic" | "event",
    imports,
    stores: [],
    component: {
      name: options.componentName,
      signals: {},
      computeds: {},
      storeRefs: [...bindings.storeRefs.keys()],
      fns: [],
      view: roots[0] || null,
      style: componentStyle,
    },
  };

  const emitter = new SourceEmitter();
  const source = emitter.emitDocument(emitInput, new Map());

  return { value: source, errors };
}

// ── Helpers ──

function applyStyles(roots: import("./types").ViewNode[]): void {
  for (const r of roots) {
    walkApply(r);
  }
  function walkApply(n: import("./types").ViewNode): void {
    n.resolvedStyle = resolveStyle(n);
    for (const c of n.children) walkApply(c);
  }
}

function collectStyles(
  roots: import("./types").ViewNode[],
  out: Record<string, Record<string, string>>,
): void {
  for (const r of roots) {
    walkCollect(r);
  }
  function walkCollect(n: import("./types").ViewNode): void {
    if (n.resolvedStyle && Object.keys(n.resolvedStyle.cssRules).length > 0) {
      out[n.resolvedStyle.cssClassName] = n.resolvedStyle.cssRules;
    }
    for (const c of n.children) walkCollect(c);
  }
}
