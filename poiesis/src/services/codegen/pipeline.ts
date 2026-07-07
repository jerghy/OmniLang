// poiesis/src/services/codegen/pipeline.ts
// Orchestrates the 3-phase omni-gen pipeline:
//   Phase A: ViewTreeBuilder + WireAnalyzer + ImportCollector
//   Phase B: StoreExtractor + ExpressionLowering + ChainEmitter
//   Phase C: StyleResolver + AnnotationData + SourceEmitter

import { buildViewTree } from "./view-tree-builder";
import { analyzeWires } from "./wire-analyzer";
import { collectImports } from "./import-collector";
import { extractStores } from "./store-extractor";
import { lowerCode } from "./expression-lowering";
import { emitChains } from "./chain-emitter";
import { buildPidAnnotations, createAnnotationStrategy } from "./annotation-data";
import { SourceEmitter, CodeWriter } from "./source-emitter";
import { expandCustomComponents } from "./expand-custom-components";
import { collectStylesFromEntries, applyStyles } from "./collect-styles";
import type { ElementData, WireData, CodeTab } from "../../core/types";
// import type { PidMap } from "../../core/types/pid";
import { getComponentDefinitions } from "../../store";
import type {
  StageResult,
  PipelineError,
  EmitInput,
  ViewNode,
  StoreDef,
  ChainBlock,
  BindingResult,
  LoweredCode,
} from "./types";
import type { GenerateOmniSourceOptions } from "./pipeline-types";
// import { parseStylesFromAST } from "./style-parser";
import type { PhaseAContext, PhaseBContext, PhaseCContext, StyleCollection } from "./component-builder";
import { buildComponentPart, buildComponentMeta, computeViewWrapperId } from "./component-builder";

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

// ── Default Factory Functions ───────────────────────────────────────

function defaultViewTreeResult(): { roots: ViewNode[]; pascalCaseTags: string[] } {
  return { roots: [], pascalCaseTags: [] };
}

function defaultBindingResult(): BindingResult {
  return {
    signalBindings: new Map(),
    storeRefs: new Map(),
    computedDeps: new Map(),
    unwiredProps: [],
  };
}

interface StoreExtractResult {
  stores: StoreDef[];
  standaloneSignals: Array<{ name: string; type: string; defaultValue?: unknown }>;
  standaloneComputeds: Array<{ name: string; type: string; body: string }>;
}

function defaultStoreExtractResult(): StoreExtractResult {
  return { stores: [], standaloneSignals: [], standaloneComputeds: [] };
}

function defaultLoweredCode(): LoweredCode {
  return {
    expressions: new Map(),
    functions: [],
    actions: [],
    getters: [],
  };
}

const EMPTY_REGISTRY = { variables: [], classes: [] };

function defaultStyleCollection(): StyleCollection {
  return { componentStyle: {}, elementClassMap: new Map(), elementInlineMap: new Map() };
}

// ── Phase Functions ─────────────────────────────────────────────────

function runPhaseA(
  options: GenerateOmniSourceOptions,
  expandedElements: ElementData[],
): { ctx: PhaseAContext; errors: PipelineError[] } {
  const errors: PipelineError[] = [];
  const shouldExpand = !options.forSave;

  const a1 = runPhase("A", "view-tree-builder", () =>
    buildViewTree(expandedElements, options.componentName, options.rootOrder),
  );
  errors.push(...a1.errors);
  const viewTreeResult = a1.value ?? defaultViewTreeResult();
  const { roots } = viewTreeResult;
  let { pascalCaseTags } = viewTreeResult;

  const registryDefs = getComponentDefinitions();
  if (shouldExpand) {
    pascalCaseTags = pascalCaseTags.filter((tag) => !(tag in registryDefs));
  }

  const a2 = runPhase("A", "wire-analyzer", () =>
    analyzeWires(options.designElements, options.designWires),
  );
  errors.push(...a2.errors);
  const bindings = a2.value ?? defaultBindingResult();

  const a3 = runPhase("A", "import-collector", () =>
    collectImports(pascalCaseTags, [...bindings.storeRefs.keys()], options.codeTabs),
  );
  errors.push(...a3.errors);
  const imports = a3.value ?? [];

  return { ctx: { roots, pascalCaseTags, bindings, imports }, errors };
}

function runPhaseB(
  options: GenerateOmniSourceOptions,
): { ctx: PhaseBContext; errors: PipelineError[] } {
  const errors: PipelineError[] = [];

  const b1 = runPhase("B", "store-extractor", () =>
    extractStores(options.designElements),
  );
  errors.push(...b1.errors);
  const storeResult = b1.value ?? defaultStoreExtractResult();

  const b2 = runPhase("B", "expression-lowering", () =>
    lowerCode(options.designElements, options.designWires),
  );
  errors.push(...b2.errors);
  const lowered = b2.value ?? defaultLoweredCode();

  // Merge chains from all tabs (main + event tabs)
  const allChains = new Map<string, ChainBlock>();
  for (const tab of options.codeTabs) {
    if (tab.graph.elements.length === 0) continue;
    const tabChains = runPhase("B", "chain-emitter", () =>
      emitChains(tab, lowered.functions),
    );
    errors.push(...tabChains.errors);
    if (tabChains.value) {
      for (const [k, v] of tabChains.value) allChains.set(k, v);
    }
  }

  return {
    ctx: {
      stores: storeResult.stores,
      standaloneSignals: storeResult.standaloneSignals,
      standaloneComputeds: storeResult.standaloneComputeds,
      lowered,
      allChains,
    },
    errors,
  };
}

function runPhaseC(
  options: GenerateOmniSourceOptions,
  phaseA: PhaseAContext,
): { ctx: PhaseCContext; errors: PipelineError[] } {
  const errors: PipelineError[] = [];
  const shouldExpand = !options.forSave;
  const expandedElements = shouldExpand
    ? expandCustomComponents(options.designElements)
    : options.designElements;

  const c1 = runPhase("C", "style-resolver", () => {
    applyStyles(phaseA.roots);
  });
  errors.push(...c1.errors);

  // ── Plumb signal bindings from wire analysis into ViewNode trees ──
  if (c1.errors.length === 0) {
    const elBindings = new Map<string, Record<string, string>>();
    for (const [sigName, targets] of phaseA.bindings.signalBindings) {
      for (const t of targets) {
        const existing = elBindings.get(t.elementId) ?? {};
        existing[t.propName] = sigName;
        elBindings.set(t.elementId, existing);
      }
    }
    if (elBindings.size > 0) {
      for (const root of phaseA.roots) {
        walkAndApplyBindings(root, elBindings);
      }
    }
  }

  const registry = EMPTY_REGISTRY;
  const styleCollection = c1.errors.length > 0
    ? defaultStyleCollection()
    : collectStylesFromEntries(expandedElements, registry, phaseA.roots);

  const mainTab = options.codeTabs.find((t) => t.id === "main") ?? options.codeTabs[0];

  const c2 = runPhase("C", "annotation-data", () => {
    const annData = buildPidAnnotations(
      phaseA.roots,
      (mainTab.type as "logic" | "event") ?? "logic",
      1,
      options.existingPidMap,
    );
    return createAnnotationStrategy(annData);
  });
  errors.push(...c2.errors);

  return {
    ctx: {
      annotationStrategy: c2.value,
      styleCollection,
    },
    errors,
  };
}

/**
 * Walk a ViewNode tree and merge signal bindings into resolvedStyle.bindings.
 */
function walkAndApplyBindings(
  node: ViewNode,
  elBindings: Map<string, Record<string, string>>,
): void {
  if (node.resolvedStyle) {
    const bindings = elBindings.get(node.id);
    if (bindings) {
      node.resolvedStyle.bindings = { ...node.resolvedStyle.bindings, ...bindings };
    }
  }
  for (const child of node.children) {
    walkAndApplyBindings(child, elBindings);
  }
}

// ── generateOmniSource ──────────────────────────────────────────────

export function generateOmniSource(options: GenerateOmniSourceOptions): StageResult<string> {
  const shouldExpand = !options.forSave;
  const expandedElements = shouldExpand
    ? expandCustomComponents(options.designElements)
    : options.designElements;

  const phaseA = runPhaseA(options, expandedElements);

  // Short-circuit: if Phase A has fatal errors, phases B/C would produce
  // meaningless results from default empty values — skip them.
  if (phaseA.errors.some((e) => e.severity === "fatal")) {
    return { value: null, errors: phaseA.errors };
  }

  const phaseB = runPhaseB(options);
  const phaseC = runPhaseC(options, phaseA.ctx);

  const mainTab = options.codeTabs.find((t) => t.id === "main") ?? options.codeTabs[0];
  const viewWrapperId = computeViewWrapperId(options);
  const componentPart = buildComponentPart(options, phaseA.ctx, phaseB.ctx, viewWrapperId, phaseC.ctx);
  const emitComponentMeta = buildComponentMeta(options);

  const emitInput: EmitInput = {
    codeTabType: (mainTab.type as "logic" | "event") ?? "logic",
    imports: phaseA.ctx.imports,
    stores: phaseB.ctx.stores,
    component: componentPart,
    componentMeta: emitComponentMeta,
    existingPidMap: options.existingPidMap,
  };

  const c3 = runPhase("C", "source-emitter", () => {
    const emitter = new SourceEmitter(
      phaseC.ctx.annotationStrategy ?? undefined,
      phaseC.ctx.styleCollection.elementClassMap,
      phaseC.ctx.styleCollection.elementInlineMap,
    );
    return emitter.emitDocument(emitInput, phaseB.ctx.allChains);
  });

  const allErrors = [
    ...phaseA.errors,
    ...phaseB.errors,
    ...phaseC.errors,
    ...c3.errors,
  ];

  return {
    value: allErrors.some((e) => e.severity === "fatal") || c3.value === null ? null : c3.value,
    errors: allErrors,
  };
}

// ── generateOmniFiles: multi-file output ────────────────────────────

/**
 * Generate multi-file OmniLang output.
 *
 * Runs the full pipeline then maps the result to:
 *   - <targetPath>                — component body (stores + component + view + style)
 *   - src/stores/<Name>.omni      — each store as a standalone file
 *   - src/components/<Name>.omni  — each custom component (future)
 */
export function generateOmniFiles(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
  forSave?: boolean;
  /** Target file path relative to project root (e.g. "src/components/MyCard.omni"). Default: "src/main.omni". */
  filePath?: string;
}): StageResult<Record<string, string>> {
  // Run the full pipeline for the main source
  const mainResult = generateOmniSource(options);
  const allErrors = [...mainResult.errors];

  if (mainResult.value === null) {
    return { value: null, errors: allErrors };
  }

  const targetPath = options.filePath ?? "src/main.omni";
  const files: Record<string, string> = {
    [targetPath]: mainResult.value,
  };

  // Re-extract stores (fast — just iterates elements)
  const storeResult = runPhase("B", "store-extractor", () =>
    extractStores(options.designElements),
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
      sig.defaultValue !== undefined
        ? ` = ${JSON.stringify(sig.defaultValue)}`
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
