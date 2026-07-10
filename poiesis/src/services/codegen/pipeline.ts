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
import { collectStylesFromEntries, applyStyles } from "./collect-styles";
import { compileCodeFlowGraph } from "./graph-compiler";
import type { ElementData, WireData, CodeTab } from "../../core/types";
import type { CodeblockElementData } from "../../core/types/element";
// import type { PidMap } from "../../core/types/pid";
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
import type { StoreDefinition } from "../../core/types/store";
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

function defaultStyleCollection(): StyleCollection {
  return { componentStyle: {}, elementClassMap: new Map(), elementInlineMap: new Map() };
}

// ── Phase Functions ─────────────────────────────────────────────────

function runPhaseA(
  options: GenerateOmniSourceOptions,
  elements: ElementData[],
): { ctx: PhaseAContext; errors: PipelineError[] } {
  const errors: PipelineError[] = [];

  const a1 = runPhase("A", "view-tree-builder", () =>
    buildViewTree(elements, options.componentName, options.rootOrder),
  );
  errors.push(...a1.errors);
  const viewTreeResult = a1.value ?? defaultViewTreeResult();
  const { roots, pascalCaseTags } = viewTreeResult;
  // All PascalCase tags kept — they're component references that need imports.
  // (No filtering of registry components — expansion no longer substitutes for imports.)

  const a2 = runPhase("A", "wire-analyzer", () =>
    analyzeWires(options.designElements, options.designWires),
  );
  errors.push(...a2.errors);
  const bindings = a2.value ?? defaultBindingResult();

  // Check if any elements reference design classes (for conditional design import)
  // classRefs may be at the top level (el.classRefs) or nested in style (el.style.classRefs)
  const hasDesignClassUsage = options.designElements.some(
    (el) =>
      ("classRefs" in el && Array.isArray(el.classRefs) && el.classRefs.length > 0) ||
      (el.style && "classRefs" in el.style && Array.isArray(el.style.classRefs) && el.style.classRefs.length > 0) ||
      ("variantRef" in el && el.variantRef != null && typeof el.variantRef === "object" && "designName" in el.variantRef && typeof el.variantRef.designName === "string"),
  );

  const a3 = runPhase("A", "import-collector", () =>
    collectImports(pascalCaseTags, [...bindings.storeRefs.keys()], options.codeTabs, options.designName, hasDesignClassUsage),
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
  // No expansion — components stay as references, styles only from flat elements

  const c1 = runPhase("C", "style-resolver", () => {
    applyStyles(phaseA.roots);
  });
  errors.push(...c1.errors);

  // ── Collect signal bindings from wire analysis ──
  // These are passed directly to SourceEmitter via elementBindings,
  // rather than walking the tree to merge into resolvedStyle.bindings.
  const elBindings = new Map<string, Record<string, string>>();
  if (c1.errors.length === 0) {
    for (const [sigName, targets] of phaseA.bindings.signalBindings) {
      for (const t of targets) {
        const existing = elBindings.get(t.elementId) ?? {};
        existing[t.propName] = sigName;
        elBindings.set(t.elementId, existing);
      }
    }
  }

  const styleCollection = c1.errors.length > 0
    ? defaultStyleCollection()
    : collectStylesFromEntries(
        options.designElements,
        { variables: [], classes: [] }, // registry no longer needed — WASM handles CSS
        phaseA.roots,
        options.designName,
      );

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
      elementBindings: elBindings,
    },
    errors,
  };
}

// ── generateOmniSource ──────────────────────────────────────────────

export function generateOmniSource(options: GenerateOmniSourceOptions): StageResult<string> {
  // No expansion — components stay as references. Expansion is a canvas concern.
  const phaseA = runPhaseA(options, options.designElements);

  // Short-circuit: if Phase A has fatal errors, phases B/C would produce
  // meaningless results from default empty values — skip them.
  if (phaseA.errors.some((e) => e.severity === "fatal")) {
    return { value: null, errors: phaseA.errors };
  }

  const phaseB = runPhaseB(options);

  // ── Inject GraphCompiler output into function bodies ──
  // Replaces function bodies generated by expression-lowering with compiled
  // code flow graph output that includes // @pid:xxx annotations for stability.
  const codeFlowElements = options.designElements.filter(
    (e): e is CodeblockElementData => e.nodeType === "codeblock"
  );
  if (codeFlowElements.length > 0) {
    for (const fnEl of codeFlowElements) {
      if (fnEl.tag !== "fn") continue;
      const compiledFn = compileCodeFlowGraph(codeFlowElements, options.designWires, fnEl.id);
      if (!compiledFn) continue;

      const fnName = (fnEl.props?.name as string) || "unnamed";
      const existingFn = phaseB.ctx.lowered.functions.find((f) => f.name === fnName);
      if (existingFn) {
        // compileCodeFlowGraph returns a full fn definition like:
        //   // @pid:xxx\nfn name(params) {\n  body\n}
        // Extract just the inner body content (between { and }).
        const openBrace = compiledFn.indexOf("{");
        const closeBrace = compiledFn.lastIndexOf("}");
        if (openBrace !== -1 && closeBrace > openBrace) {
          existingFn.body = compiledFn.slice(openBrace + 1, closeBrace).trim();
        } else {
          existingFn.body = compiledFn;
        }
      }
    }
  }

  const phaseC = runPhaseC(options, phaseA.ctx);

  const mainTab = options.codeTabs.find((t) => t.id === "main") ?? options.codeTabs[0];
  const viewWrapperId = computeViewWrapperId(options);
  const componentPart = buildComponentPart(options, phaseA.ctx, phaseB.ctx, viewWrapperId, phaseC.ctx);
  const emitComponentMeta = buildComponentMeta(options);

  const emitInput: EmitInput = {
    codeTabType: (mainTab.type as "logic" | "event") ?? "logic",
    imports: phaseA.ctx.imports,
    stores: phaseB.ctx.stores.map(storeDefToStoreDefinition),
    component: componentPart,
    componentMeta: emitComponentMeta,
    existingPidMap: options.existingPidMap,
  };

  const c3 = runPhase("C", "source-emitter", () => {
    const elBindings = phaseC.ctx.elementBindings;
    const emitter = new SourceEmitter(
      phaseC.ctx.annotationStrategy ?? undefined,
      phaseC.ctx.styleCollection.elementClassMap,
      phaseC.ctx.styleCollection.elementInlineMap,
      elBindings.size > 0 ? elBindings : undefined,
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
  /** Target file path relative to project root (e.g. "src/components/MyCard.omni"). Default: "src/main.omni". */
  filePath?: string;
  designName?: string;
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
  /** When true, elements already belong to this component (from ComponentDef). Skip filtering. */
  fromDefinition?: boolean;
}): StageResult<string> {
  const { componentName, designElements, designWires, codeTabs, rootOrder, fromDefinition } = options;

  // If elements come from a ComponentDef, they already belong to this component — no filter
  const componentElements = fromDefinition
    ? designElements
    : designElements.filter((el) => {
        const ref = "componentRef" in el ? el.componentRef : undefined;
        if (componentName === "App" || componentName === "main") {
          return !ref || ref === componentName;
        }
        return ref === componentName;
      });

  if (componentElements.length === 0) {
    return { value: "", errors: [] };
  }

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
 * Convert a legacy StoreDef (from store-extractor) to the new StoreDefinition format.
 * Synthetic PIDs are generated since StoreDef has no PID data.
 */
function storeDefToStoreDefinition(sd: StoreDef): StoreDefinition {
  return {
    pid: `gen_${sd.name}`,
    name: sd.name,
    filePath: `src/stores/${sd.name}.omni`,
    imports: [],
    signals: Object.entries(sd.signals).map(([name, sig], i) => ({
      pid: `gen_${sd.name}_sig_${i}`,
      name,
      type: sig.type,
      defaultValue: sig.defaultValue,
    })),
    getters: Object.entries(sd.getters).map(([name, g], i) => ({
      pid: `gen_${sd.name}_get_${i}`,
      name,
      returnType: g.returnType,
      expression: g.body,
    })),
    actions: sd.actions.map((a, i) => ({
      pid: `gen_${sd.name}_act_${i}`,
      name: a.name,
      params: a.params,
      returnType: undefined,
      isAsync: false,
      elements: [],
      wires: [],
    })),
  };
}

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

