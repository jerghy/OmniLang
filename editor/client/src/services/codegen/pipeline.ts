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
import { buildAnnotationData, createAnnotationStrategy } from "./annotation-data";
import { SourceEmitter, CodeWriter } from "./source-emitter";
import { expandCustomComponents } from "./expand-custom-components";
import { collectStylesFromEntries, applyStyles } from "./collect-styles";
import { uid } from "../../core/utils";
import { classNameToVariable } from "../../core/types";
import type { StyleProperty, StyleValueSegment, ElementData, WireData, CodeTab } from "../../core/types";
import { useComponentRegistry } from "../../core/state/componentRegistry";
import { useStyleRegistryStore } from "../../core/state/styleRegistry";
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

// ── parseStylesFromAST ─────────────────────────────────────────────
// Reconstruct StyleEntry[] from a component AST node (view tree + style blocks).
// Called when loading .omni files to populate element styles before storing.

function generateId(): string {
  return uid("s");
}

function findViewNode(view: any, elId: string): any | null {
  if (!view) return null;
  if (view.id === elId) return view;
  for (const child of view.children || []) {
    const found = findViewNode(child, elId);
    if (found) return found;
  }
  return null;
}

function exprToString(expr: any): string {
  if (!expr) return "";
  if (typeof expr === "string") return expr;
  if (typeof expr === "number") return String(expr);
  if (expr.value !== undefined) return String(expr.value);
  if (expr.literal !== undefined) return String(expr.literal);
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
export function parseStylesFromAST(
  componentNode: any,
  viewElements: Map<string, ElementData>,
): void {
  const componentName = componentNode?.name ?? "unknown";
  const store = useStyleRegistryStore.getState();

  // 1. Parse style { } blocks → populate StyleRegistry
  const styleBlock = componentNode?.style;
  const hasStyleBlock = Array.isArray(styleBlock);
  if (hasStyleBlock) {
    for (const rule of styleBlock) {
      const className: string = rule.class_name;
      const properties: StyleProperty[] = Object.entries(rule.properties).map(
        ([name, valueExpr]: [string, any]) => ({
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
      const classAttr = viewNode.props?.["class"];
      if (classAttr && typeof classAttr === "string") {
        classRefs.push(...classAttr.split(/\s+/));
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

// ── generateOmniSource ──────────────────────────────────────────────

export function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
  forSave?: boolean;  // true: keep <MyCard> tags + generate imports; false (default): expand inline
  componentMeta?: {
    id?: string;
    category?: string;
    description?: string;
    props?: Record<string, unknown>;
    events?: Record<string, unknown>;
    styleProperties?: string[];
    children?: boolean;
    propSchema?: Record<string, { editor?: string; values?: string[] }>;
    eventSchema?: Record<string, { payload?: string | null; description?: string }>;
    createdAt?: string;
  };
}): StageResult<string> {
  const allErrors: PipelineError[] = [];

  // ── Custom component handling ──
  // Two strategies:
  //   1. "expand" (default) — for preview/compile. Inline custom component internals
  //      so the WASM/Rust compiler can render them in a single-source context.
  //   2. "import" (forSave) — for saving to disk. Keep <MyCard> tags in the view tree
  //      and generate `import { MyCard } from "./components/MyCard"` statements.
  //      The custom component definitions are stored separately in src/components/*.omni.
  const shouldExpand = !options.forSave;
  const expandedElements = shouldExpand
    ? expandCustomComponents(options.designElements)
    : options.designElements;

  // ── Phase A: Build view tree, analyze wires, collect imports ──

  const a1 = runPhase("A", "view-tree-builder", () =>
    buildViewTree(expandedElements, options.componentName, options.rootOrder),
  );
  allErrors.push(...a1.errors);
  let { roots, pascalCaseTags } = a1.value ?? {
    roots: [] as ViewNode[],
    pascalCaseTags: [] as string[],
  };
  const registryDefs = useComponentRegistry.getState().definitions;
  if (shouldExpand) {
    // For preview/compile: custom components were expanded inline, so filter them
    // out of pascalCaseTags — no imports needed.
    pascalCaseTags = pascalCaseTags.filter((tag) => !(tag in registryDefs));
  }
  // For save: keep custom component tags in pascalCaseTags so collectImports
  // generates `import { MyCard } from "./components/MyCard"`.

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

  // If applyStyles failed, roots may be partially modified (some nodes got
  // resolvedStyle, others didn't).  Skip style collection and let the fatal
  // error check below short-circuit the pipeline with null value.
  const activeRegistry = useStyleRegistryStore.getState().getRegistry(options.componentName);
  const { componentStyle, elementClassMap, elementInlineMap } = c1.errors.length > 0
    ? { componentStyle: {}, elementClassMap: new Map<string, string[]>(), elementInlineMap: new Map<string, Record<string, string>>() }
    : collectStylesFromEntries(expandedElements, activeRegistry, roots);

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

  // Build annotation data and strategy (for both preview and save modes)
  const c2 = runPhase("C", "annotation-data", () => {
    const annData = buildAnnotationData(
      roots,
      (mainTab.type as "logic" | "event") ?? "logic",
    );
    return createAnnotationStrategy(annData);
  });
  allErrors.push(...c2.errors);
  const annotationStrategy = c2.value;

  // Determine view wrapper ID — must NOT conflict with any user element ID
  let viewWrapperId = `${options.componentName}.view`;
  if (options.designElements.some((el) => el.id === viewWrapperId)) {
    // Append a suffix to avoid duplicate ID that would corrupt the parse tree
    viewWrapperId = `${options.componentName}.view-wrapper`;
  }

  // ── Custom component metadata conversion ──
  // When componentMeta is provided, extract DSL prop/event declarations and
  // convert to EmitInput.componentMeta format for // @component / @prop / @event / @style comments.

  let componentProps: EmitInput["component"]["props"];
  let componentEvents: EmitInput["component"]["events"];
  let emitComponentMeta: EmitInput["componentMeta"];

  if (options.componentMeta) {
    const meta = options.componentMeta;
    const rawProps = meta.props as Record<string, Record<string, unknown>> | undefined;
    const rawEvents = meta.events as Record<string, Record<string, unknown>> | undefined;

    // DSL prop declarations (e.g. `prop title: String = ""`)
    componentProps = rawProps
      ? Object.entries(rawProps).map(([name, schema]) => ({
          name,
          type: (schema.type as string) || "String",
          default: schema.default,
        }))
      : undefined;

    // DSL event declarations (e.g. `event onAction`)
    componentEvents = rawEvents
      ? Object.keys(rawEvents).map((name) => ({ name }))
      : undefined;

    // Editor metadata comments (replaces old *.meta.json sidecar)
    emitComponentMeta = {
      id: (meta.id as string) || options.componentName,
      category: (meta.category as string) || "Custom",
      description: (meta.description as string) || "",
      children: (meta.children as boolean) ?? true,
      styleProperties: (meta.styleProperties as string[]) || [],
      propSchema: rawProps
        ? Object.fromEntries(
            Object.entries(rawProps).map(([name, p]) => [
              name,
              { editor: p.editor as string, values: p.values as string[] },
            ])
          )
        : undefined,
      eventSchema: rawEvents
        ? Object.fromEntries(
            Object.entries(rawEvents).map(([name, e]) => [
              name,
              { payload: e.payload as string, description: e.description as string },
            ])
          )
        : undefined,
    };
  }

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
      props: componentProps,
      events: componentEvents,
      view: roots.length > 0 ? {
        id: viewWrapperId,
        path: viewWrapperId,
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
    componentMeta: emitComponentMeta,
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

