// editor/client/src/omni-gen/pipeline.ts
// Orchestrates the 3-phase omni-gen pipeline:
//   Phase A: ViewTreeBuilder + WireAnalyzer + ImportCollector
//   Phase B: StoreExtractor + ExpressionLowering + ChainEmitter
//   Phase C: StyleResolver + AnnotationData + SourceEmitter

import { buildViewTree } from "./view-tree-builder";
import { analyzeWires } from "./wire-analyzer";
import { collectImports } from "./import-collector";
import { resolveStyle } from "./style-resolver";
import { extractStores } from "./store-extractor";
import { lowerCode } from "./expression-lowering";
import { emitChains } from "./chain-emitter";
import { buildAnnotationData, createAnnotationStrategy } from "./annotation-data";
import { SourceEmitter, CodeWriter } from "./source-emitter";
import type { StyleEntry } from "../constants/style-properties";
import { variableToClassName } from "../constants/style-properties";
import type { ElementData, WireData } from "../store/editor";
import type { CodeTab } from "../codegraph/types";
import type {
  StageResult,
  PipelineError,
  EmitInput,
  ViewNode,
  StoreDef,
  ChainBlock,
} from "./types";

// ── runPhase helper ─────────────────────────────────────────────────

/**
 * Wrap a phase module call in try/catch.
 * Returns StageResult<T> with the value on success or a fatal PipelineError on throw.
 */
function runPhase<T>(
  phase: "A" | "B" | "C",
  module: string,
  fn: () => T,
): StageResult<T> {
  try {
    const value = fn();
    return { value, errors: [] };
  } catch (e) {
    const error: PipelineError = {
      phase,
      module,
      severity: "fatal",
      code: "E_PHASE",
      message: e instanceof Error ? e.message : String(e),
    };
    return { value: null, errors: [error] };
  }
}

// ── generateOmniSource ──────────────────────────────────────────────

export function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
}): StageResult<string> {
  const allErrors: PipelineError[] = [];

  // ── Phase A: Build view tree, analyze wires, collect imports ──

  const a1 = runPhase("A", "view-tree-builder", () =>
    buildViewTree(options.designElements, options.componentName, options.rootOrder),
  );
  allErrors.push(...a1.errors);
  const { roots, pascalCaseTags } = a1.value ?? {
    roots: [] as ViewNode[],
    pascalCaseTags: [] as string[],
  };

  const a2 = runPhase("A", "wire-analyzer", () =>
    analyzeWires(options.designElements, options.designWires),
  );
  allErrors.push(...a2.errors);
  const bindings = a2.value ?? {
    signalBindings: new Map<
      string,
      Array<{ elementId: string; propName: string }>
    >(),
    storeRefs: new Map<string, string[]>(),
    computedDeps: new Map<string, string[]>(),
    unwiredProps: [] as Array<{
      elementId: string;
      tag: string;
      propName: string;
    }>,
  };

  const a3 = runPhase("A", "import-collector", () =>
    collectImports(
      pascalCaseTags,
      [...bindings.storeRefs.keys()],
      options.codeTabs,
    ),
  );
  allErrors.push(...a3.errors);
  const imports = a3.value ?? [];

  // ── Phase B: StoreExtractor + ExpressionLowering + ChainEmitter ──

  const b1 = runPhase("B", "store-extractor", () =>
    extractStores(options.designElements, options.designWires),
  );
  allErrors.push(...b1.errors);
  const { stores, standaloneSignals, standaloneComputeds } = b1.value ?? {
    stores: [] as StoreDef[],
    standaloneSignals: [] as Array<{
      name: string;
      type: string;
      default?: unknown;
    }>,
    standaloneComputeds: [] as Array<{ name: string; type: string; body: string }>,
  };

  const b2 = runPhase("B", "expression-lowering", () =>
    lowerCode(options.designElements, options.designWires),
  );
  allErrors.push(...b2.errors);
  const lowered = b2.value ?? {
    expressions: new Map<string, string>(),
    functions: [] as Array<{
      name: string;
      params: Array<{ name: string; type: string }>;
      returnType: string;
      body: string;
    }>,
    actions: [] as Array<{
      storeName: string;
      name: string;
      params: Array<{ name: string; type: string }>;
      body: string;
    }>,
    getters: [] as Array<{
      name: string;
      returnType: string;
      body: string;
      deps: string[];
    }>,
  };

  const mainTab =
    options.codeTabs.find((t) => t.id === "main") ?? options.codeTabs[0];

  // Merge chains from all tabs (main + event tabs)
  const allChains = new Map<string, ChainBlock>();
  for (const tab of options.codeTabs) {
    if (tab.graph.elements.length === 0) continue;
    const tabChains = runPhase("B", "chain-emitter", () =>
      emitChains(tab, lowered.functions),
    );
    allErrors.push(...tabChains.errors);
    if (tabChains.value) {
      for (const [k, v] of tabChains.value) allChains.set(k, v);
    }
  }

  // ── Phase C: Apply styles, build annotations, emit source ──

  const c1 = runPhase("C", "style-resolver", () => {
    applyStyles(roots);
  });
  allErrors.push(...c1.errors);

  // Build componentStyle and per-element maps from StyleEntry[] on designElements
  const { componentStyle, elementClassMap, elementInlineMap } = collectStylesFromEntries(options.designElements);

  // Build component-level signals from standaloneSignals (signals not in a store)
  const componentSignals: Record<
    string,
    { type: string; default?: unknown }
  > = {};
  for (const s of standaloneSignals) {
    componentSignals[s.name] = { type: s.type, default: s.default };
  }

  // Build component-level computeds from standaloneComputeds
  const componentComputeds: Record<string, { type: string; body: string }> = {};
  for (const c of standaloneComputeds) {
    componentComputeds[c.name] = { type: c.type, body: c.body };
  }

  // Build annotation data and strategy
  const c2 = runPhase("C", "annotation-data", () => {
    const annData = buildAnnotationData(
      roots,
      (mainTab.type as "logic" | "event") ?? "logic",
    );
    return createAnnotationStrategy(annData);
  });
  allErrors.push(...c2.errors);
  const annotationStrategy = c2.value;

  // Assemble the full EmitInput
  const emitInput: EmitInput = {
    codeTabType: (mainTab.type as "logic" | "event") ?? "logic",
    imports,
    stores,
    component: {
      name: options.componentName,
      signals: componentSignals,
      computeds: componentComputeds,
      storeRefs: [...bindings.storeRefs.keys()],
      fns: lowered.functions,
      view: roots.length > 0 ? {
        id: `${options.componentName}.view`,
        path: `${options.componentName}.view`,
        tag: "div",
        props: {},
        children: roots,
        kind: "element" as const,
        resolvedStyle: {
          htmlTag: "div",
          cssClassName: `${options.componentName.toLowerCase()}_view`,
          contentText: undefined,
          htmlAttrs: {},
          eventBindings: {},
          bindings: {},
          cssRules: {},
          usedClassNames: [],
        },
      } : null,
      style: componentStyle,
    },
  };

  // Emit the source document
  const c3 = runPhase("C", "source-emitter", () => {
    const emitter = new SourceEmitter(annotationStrategy, elementClassMap, elementInlineMap);
    return emitter.emitDocument(emitInput, allChains);
  });
  allErrors.push(...c3.errors);

  // If any fatal error occurred, return null value
  const hasFatal = allErrors.some((e) => e.severity === "fatal");

  return {
    value: hasFatal || c3.value === null ? null : c3.value,
    errors: allErrors,
  };
}

// ── generateOmniFiles: multi-file output ────────────────────────────

/**
 * Generate multi-file OmniLang output.
 *
 * Runs the full pipeline then maps the result to:
 *   - src/main.omni            — component body (stores + component + view + style)
 *   - src/stores/<Name>.omni   — each store as a standalone file
 *   - src/components/<Name>.omni  — each custom component (future)
 */
export function generateOmniFiles(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
}): StageResult<Record<string, string>> {
  // Run the full pipeline for the main source
  const mainResult = generateOmniSource(options);
  const allErrors = [...mainResult.errors];

  if (mainResult.value === null) {
    return { value: null, errors: allErrors };
  }

  const files: Record<string, string> = {
    "src/main.omni": mainResult.value,
  };

  // Re-extract stores (fast — just iterates elements)
  const storeResult = runPhase("B", "store-extractor", () =>
    extractStores(options.designElements, options.designWires),
  );
  allErrors.push(...storeResult.errors);

  if (storeResult.value) {
    for (const store of storeResult.value.stores) {
      const storeSource = emitStoreBlock(store);
      files[`src/stores/${store.name}.omni`] = storeSource;
    }
  }

  return { value: files, errors: allErrors };
}

// ── generateOmniFragment: single-component output ────────────────────

/**
 * Generate .omni source fragment for a single component.
 * Filters `designElements` to only those belonging to the specified component,
 * then runs the full pipeline on the filtered set.
 *
 * Used for incremental compilation — only re-generates the changed component's source.
 *
 * NOTE: This is a simplified approach for Phase 1. In a future phase, this will
 * use a true sub-pipeline that only processes one component without the overhead
 * of the full pipeline entry point.
 */
export function generateOmniFragment(options: {
  componentName: string;
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  rootOrder?: string[];
}): StageResult<string> {
  const { componentName, designElements, designWires, codeTabs, rootOrder } = options;

  // Filter elements belonging to this component
  const componentElements = designElements.filter((el) => {
    if (componentName === "App" || componentName === "main") {
      return !el.componentRef || el.componentRef === componentName;
    }
    return el.componentRef === componentName;
  });

  // If no elements for this component, return empty
  if (componentElements.length === 0) {
    return { value: "", errors: [] };
  }

  // Re-run the full pipeline but only for these elements
  return generateOmniSource({
    designElements: componentElements,
    designWires,
    codeTabs,
    componentName,
    rootOrder,
  });
}

// ── Helpers ─────────────────────────────────────────────────────────

/**
 * Emit a single store block as standalone OmniLang source.
 */
function emitStoreBlock(store: StoreDef): string {
  const w = new CodeWriter();
  w.emit(`store ${store.name} {`);
  w.indent();
  for (const [name, sig] of Object.entries(store.signals)) {
    const def =
      sig.default !== undefined
        ? ` = ${JSON.stringify(sig.default)}`
        : "";
    w.emit(`signal ${name}: ${sig.type}${def};`);
  }
  for (const [name, g] of Object.entries(store.getters)) {
    w.emit(`getter ${name}: ${g.returnType} = ${g.body};`);
  }
  for (const a of store.actions) {
    const params = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
    w.emit(`action ${a.name}(${params}) {`);
    w.indent();
    for (const line of a.body.split("\n")) w.emit(line);
    w.dedent();
    w.emit(`}`);
  }
  w.dedent();
  w.emit(`}`);
  return w.toString();
}

function applyStyles(roots: ViewNode[]): void {
  for (const r of roots) {
    walkApply(r);
  }
  function walkApply(n: ViewNode): void {
    n.resolvedStyle = resolveStyle(n);
    for (const c of n.children) walkApply(c);
  }
}

export function collectStylesFromEntries(
  elements: Array<{ id: string; styles?: StyleEntry[] }>,
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
} {
  const componentStyle: Record<string, Record<string, string>> = {};
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

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
          componentStyle[entry.name] = rules;
          classNames.push(entry.name);
          break;
        }
        case "variable": {
          const varName = entry.name;
          const className = variableToClassName(varName);
          componentStyle[className] = { [varName]: entry.properties[0]?.value ?? "" };
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

    if (classNames.length > 0) {
      elementClassMap.set(el.id, classNames);
    }
    if (Object.keys(inlineStyles).length > 0) {
      elementInlineMap.set(el.id, inlineStyles);
    }
  }

  return { componentStyle, elementClassMap, elementInlineMap };
}
