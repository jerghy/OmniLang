// @ts-nocheck — Codegen service, needs type alignment with unified store
// editor/client/src/services/codegen/pipeline.ts
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
import { uid } from "../../core/utils";
import { classNameToVariable, variableToClassName } from "../../core/types";
import type { StyleProperty, ElementData, WireData, CodeTab } from "../../core/types";
import { useComponentRegistry } from "../../core/state/componentRegistry";
import { useStyleRegistryStore } from "../../core/state/styleRegistry";
import type {
  StageResult,
  PipelineError,
  EmitInput,
  ViewNode,
  StoreDef,
  ChainBlock,
  BindingResult,
  LoweredCode,
  ComponentPart,
  ComponentMeta,
  AnnotationStrategy,
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

// ── parseStylesFromAST ─────────────────────────────────────────────
// Reconstruct ElementStyle (class refs + inline properties) from a component AST node (view tree + style blocks).
// Called when loading .omni files to populate element styles before storing.

function generateId(): string {
  return uid("s");
}

function findViewNode(view: ViewNode | null | undefined, elId: string): ViewNode | null {
  if (!view) return null;
  if (view.id === elId) return view;
  for (const child of view.children || []) {
    const found = findViewNode(child, elId);
    if (found) return found;
  }
  return null;
}

function exprToString(expr: unknown): string {
  if (!expr) return "";
  if (typeof expr === "string") return expr;
  if (typeof expr === "number") return String(expr);
  if (typeof expr === "object" && expr !== null) {
    const e = expr as Record<string, unknown>;
    if (e.value !== undefined) return String(e.value);
    if (e.literal !== undefined) return String(e.literal);
  }
  return String(expr);
}

/**
 * Parse style { } blocks from a component AST and populate ElementStyle
 * (classRefs + inlineProperties) on the given view elements via el.style.
 *
 * Also populates the StyleRegistry (useStyleRegistryStore) from style { } blocks.
 *
 * Handles:
 *   - class: attributes on view nodes → classRefs
 *   - inline_style on view nodes → inlineProperties
 *   - Backward compatibility: old files without style { } blocks
 *     → extracts known style property keys from element props as inlineProperties
 */
interface StyleRule {
  class_name: string;
  properties: Record<string, string>;
}

interface ComponentAST {
  name?: string;
  style?: StyleRule[];
  view?: ViewNode;
}

export function parseStylesFromAST(
  componentNode: ComponentAST | undefined | null,
  viewElements: Map<string, ElementData>,
): void {
  const componentName = componentNode?.name ?? "App";
  const store = useStyleRegistryStore.getState();

  // 1. Parse style { } blocks → populate StyleRegistry
  const styleBlock = componentNode?.style;
  const hasStyleBlock = Array.isArray(styleBlock);
  if (hasStyleBlock) {
    for (const rule of styleBlock) {
      const className: string = rule.class_name;
      const properties: StyleProperty[] = Object.entries(rule.properties).map(
        ([name, valueExpr]: [string, string]) => ({
          id: generateId(),
          name,
          segments: [{ type: "text" as const, value: exprToString(valueExpr) }],
        })
      );

      const varName = classNameToVariable(className);
      if (varName) {
        store.addVariable(componentName, {
          id: generateId(),
          name: varName,
          segments: properties[0]?.segments ?? [],
        });
      } else {
        store.addClass(componentName, {
          id: generateId(),
          name: className,
          properties,
        });
      }
    }
  }

  // 2. Walk view elements → map class: and style: to ElementStyle
  for (const [elId, el] of viewElements) {
    const classRefs: string[] = [];
    const inlineProperties: StyleProperty[] = [];
    const viewNode = findViewNode(componentNode?.view, elId);

    if (viewNode) {
      // Parse class: attribute → classRefs
      // The class: attribute includes BOTH auto-generated path classes (e.g. "demo1_h1_0")
      // from the style resolver AND user-defined classes from the registry.
      // Only recover user-defined classes (they match the registry) and discard
      // auto-generated path classes that would appear as garbled names in the UI.
      const classAttr = viewNode.props?.["class"];
      if (classAttr && typeof classAttr === "string") {
        const allClasses = classAttr.split(/\s+/);
        if (hasStyleBlock && store.getRegistry(componentName).classes.length > 0) {
          // Recover classRefs from the class: attribute.  The attribute includes
          // BOTH auto-generated path classes (e.g. "Test_div_0") from the style
          // resolver AND user-defined classes from the registry.
          const registry = store.getRegistry(componentName);
          const registryNames = new Set(registry.classes.map((c) => c.name));
          for (const v of registry.variables) {
            registryNames.add(variableToClassName(v.name));
          }
          const compEscaped = componentName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
          const autoGenRe = new RegExp(`^${compEscaped}_[a-z]+(_\\d+)?$`);
          for (const c of allClasses) {
            // Keep classes that are in the registry (user-defined) OR are
            // auto-generated path classes needed for round-trip fidelity.
            if (registryNames.has(c) || autoGenRe.test(c)) classRefs.push(c);
          }
        } else {
          // No style block or empty registry — keep all classes (backward compat)
          classRefs.push(...allClasses);
        }
      }

      // Parse inline_style → inlineProperties
      const inlineStyle = viewNode.inline_style;
      if (inlineStyle && typeof inlineStyle === "object") {
        for (const [name, value] of Object.entries(inlineStyle)) {
          inlineProperties.push({
            id: generateId(),
            name,
            segments: [{ type: "text" as const, value: String(value) }],
          });
        }
      }
    }

    // 3. Backward compatibility: no style { } block and no class:/style: attributes
    //    → extract known style keys from el.props and create inline properties
    if (classRefs.length === 0 && inlineProperties.length === 0 && !hasStyleBlock) {
      for (const key of Object.keys(el.props)) {
        // Common CSS property names that might appear as element props in old files
        if (/^(background|color|padding|margin|border|radius|width|height|display|flex|gap|font|text|align|justify)/i.test(key)) {
          inlineProperties.push({
            id: generateId(),
            name: key,
            segments: [{ type: "text" as const, value: String(el.props[key]) }],
          });
        }
      }
    }

    el.style = { classRefs, inlineProperties };

    // Cleanup: remove old props.style and props.className when style
    // has been set, to avoid duplicate style/class attributes in emitted source
    if ((classRefs.length > 0 || inlineProperties.length > 0) && (el.props?.style || el.props?.className)) {
      const { style: _oldStyle, className: _oldClass, ...restProps } = el.props;
      el.props = restProps;
    }
  }
}

// ── Options type ────────────────────────────────────────────────────

type GenerateOmniSourceOptions = {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
  forSave?: boolean;
  componentMeta?: ComponentMeta;
};

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
  standaloneSignals: Array<{ name: string; type: string; default?: unknown }>;
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

interface StyleCollection {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
}

function defaultStyleCollection(): StyleCollection {
  return { componentStyle: {}, elementClassMap: new Map(), elementInlineMap: new Map() };
}

// ── Phase Context Types ─────────────────────────────────────────────

interface PhaseAContext {
  roots: ViewNode[];
  pascalCaseTags: string[];
  bindings: BindingResult;
  imports: string[];
}

interface PhaseBContext {
  stores: StoreDef[];
  standaloneSignals: Array<{ name: string; type: string; default?: unknown }>;
  standaloneComputeds: Array<{ name: string; type: string; body: string }>;
  lowered: LoweredCode;
  allChains: Map<string, ChainBlock>;
}

interface PhaseCContext {
  annotationStrategy: AnnotationStrategy | null;
  styleCollection: StyleCollection;
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

  const registryDefs = useComponentRegistry.getState().definitions;
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
  // Invert signalBindings (signalName → [{elementId, propName}]) to
  // elementId → {propName: signalName} so the SourceEmitter can emit
  // `bind:prop={signalName}` on each view node.
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

  // If applyStyles failed, roots may be partially modified (some nodes got
  // resolvedStyle, others didn't).  Skip style collection and let the fatal
  // error check below short-circuit the pipeline with null value.
  const registry = useStyleRegistryStore.getState().getRegistry(options.componentName);
  const styleCollection = c1.errors.length > 0
    ? defaultStyleCollection()
    : collectStylesFromEntries(expandedElements, registry, phaseA.roots);

  const mainTab = options.codeTabs.find((t) => t.id === "main") ?? options.codeTabs[0];

  const c2 = runPhase("C", "annotation-data", () => {
    const annData = buildPidAnnotations(
      phaseA.roots,
      (mainTab.type as "logic" | "event") ?? "logic",
      1, // componentIndex — hardcoded for single-component files
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

/** Walk a ViewNode tree and merge signal bindings into resolvedStyle.bindings. */
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

// ── Helper: View Wrapper ID ─────────────────────────────────────────

function computeViewWrapperId(options: GenerateOmniSourceOptions): string {
  let viewWrapperId = `${options.componentName}.view`;
  if (options.designElements.some((el) => el.id === viewWrapperId)) {
    viewWrapperId = `${options.componentName}.view-wrapper`;
  }
  return viewWrapperId;
}

// ── Helper: Build ComponentPart ─────────────────────────────────────

function buildComponentPart(
  options: GenerateOmniSourceOptions,
  phaseA: PhaseAContext,
  phaseB: PhaseBContext,
  viewWrapperId: string,
  phaseC: PhaseCContext,
): ComponentPart {
  // Build component-level signals from standaloneSignals (signals not in a store)
  const componentSignals: Record<string, { type: string; default?: unknown }> = {};
  for (const s of phaseB.standaloneSignals) {
    componentSignals[s.name] = { type: s.type, default: s.default };
  }

  // Build component-level computeds from standaloneComputeds
  const componentComputeds: Record<string, { type: string; body: string }> = {};
  for (const c of phaseB.standaloneComputeds) {
    componentComputeds[c.name] = { type: c.type, body: c.body };
  }

  // Read props/events from the component registry so they appear
  // as `prop name: Type = default;` and `event name;` in the output.
  const registryDef = useComponentRegistry.getState().definitions[options.componentName];
  const componentProps: ComponentPart["props"] = registryDef
    ? Object.entries(registryDef.props).map(([name, schema]) => ({
        name,
        type: schema.type,
        default: schema.default,
      }))
    : undefined;
  const componentEvents: ComponentPart["events"] = registryDef
    ? Object.entries(registryDef.events).map(([name]) => ({
        name,
      }))
    : undefined;

  // Extract `const` declarations from codeblock elements with tag === "const"
  const consts: ComponentPart["consts"] = [];
  for (const el of options.designElements) {
    if (el.nodeType === "codeblock" && el.tag === "const") {
      const name = el.props.name as string | undefined;
      if (name) {
        consts.push({ name, value: el.props.value });
      }
    }
  }

  return {
    name: options.componentName,
    signals: componentSignals,
    computeds: componentComputeds,
    props: componentProps,
    events: componentEvents,
    consts: consts.length > 0 ? consts : undefined,
    storeRefs: [...phaseA.bindings.storeRefs.keys()],
    fns: phaseB.lowered.functions,
    view: phaseA.roots.length > 0 ? {
      id: viewWrapperId,
      path: viewWrapperId,
      tag: "div",
      props: {},
      children: phaseA.roots,
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
    style: phaseC.styleCollection.componentStyle,
  };
}

// ── Helper: Build ComponentMeta ─────────────────────────────────────

function buildComponentMeta(options: GenerateOmniSourceOptions): ComponentMeta | undefined {
  return options.componentMeta;
}

// ── generateOmniSource ──────────────────────────────────────────────

export function generateOmniSource(options: GenerateOmniSourceOptions): StageResult<string> {
  const shouldExpand = !options.forSave;
  const expandedElements = shouldExpand
    ? expandCustomComponents(options.designElements)
    : options.designElements;

  const phaseA = runPhaseA(options, expandedElements);
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

