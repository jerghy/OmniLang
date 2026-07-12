// poiesis/src/services/codegen/component-builder.ts
// Build ComponentPart and ComponentMeta from pipeline phase outputs.
// These functions assemble the final component data structure from the
// results of Phase A (view tree + bindings), Phase B (stores + chains),
// and Phase C (styles).

import type {
  ComponentPart,
  ComponentMeta,
  ViewNode,
  ElementViewNode,
  BindingResult,
  LoweredCode,
  ChainBlock,
  AnnotationStrategy,
  StoreDef,
} from "./types";
import type { GenerateOmniSourceOptions } from "./pipeline-types";
import { getComponentDefinitions } from "../../store";

// ── Shared types between component-builder and pipeline orchestration ──

export interface StyleCollection {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
}

export interface PhaseAContext {
  roots: ViewNode[];
  pascalCaseTags: string[];
  bindings: BindingResult;
  imports: string[];
}

export interface PhaseBContext {
  stores: StoreDef[];
  standaloneSignals: Array<{ name: string; type: string; defaultValue?: unknown }>;
  standaloneComputeds: Array<{ name: string; type: string; body: string }>;
  lowered: LoweredCode;
  allChains: Map<string, ChainBlock>;
}

export interface PhaseCContext {
  annotationStrategy: AnnotationStrategy | null;
  styleCollection: StyleCollection;
  /** Signal bindings keyed by elementId → { propName: signalName } */
  elementBindings: Map<string, Record<string, string>>;
}

// ── Helper: View Wrapper ID ─────────────────────────────────────────

export function computeViewWrapperId(options: GenerateOmniSourceOptions): string {
  let viewWrapperId = `${options.componentName}.view`;
  if (options.designElements.some((el) => el.id === viewWrapperId)) {
    viewWrapperId = `${options.componentName}.view-wrapper`;
  }
  return viewWrapperId;
}

// ── Helper: Build ComponentPart ─────────────────────────────────────

export function buildComponentPart(
  options: GenerateOmniSourceOptions,
  phaseA: PhaseAContext,
  phaseB: PhaseBContext,
  viewWrapperId: string,
  phaseC: PhaseCContext,
): ComponentPart {
  // Read component definition from the registry (populated by parser -> componentDefs)
  const registryDef = getComponentDefinitions()[options.componentName];
  const componentDef = (registryDef && typeof registryDef === "object" && "signals" in registryDef)
    ? registryDef as unknown as {
        signals?: Array<{ name: string; type: string; defaultValue?: unknown }>;
        props?: Array<{ name: string; type: string; defaultValue?: unknown }>;
        computeds?: Array<{ pid: string; name: string; returnType: string; expression: string }>;
        events?: Array<{ name: string }>;
      }
    : null;

  // Props from ComponentDef
  const componentProps: import("./types").PropDef[] = (componentDef?.props || []).map((p) => ({
    name: p.name,
    type: p.type,
    defaultValue: p.defaultValue,
  }));

  // Signals from ComponentDef (no longer from standaloneSignals)
  const componentSignals: Record<string, { type: string; defaultValue?: unknown }> = {};
  for (const s of componentDef?.signals || []) {
    componentSignals[s.name] = { type: s.type, defaultValue: s.defaultValue };
  }

  // Computeds from ComponentDef (no longer from standaloneComputeds)
  const componentComputeds: Record<string, { type: string; body: string }> = {};
  for (const c of componentDef?.computeds || []) {
    componentComputeds[c.name] = { type: c.returnType, body: c.expression };
  }

  // Events from ComponentDef
  const componentEvents: import("./types").EventDef[] = (componentDef?.events || []).map((e) => ({
    name: e.name,
  }));

  // Extract `const` declarations from codeblock elements with tag === "const"
  const consts: import("./types").ConstDef[] = [];
  for (const el of options.designElements) {
    if (el.nodeType === "codeblock" && el.tag === "const") {
      const name = el.props.name as string | undefined;
      if (name) {
        consts.push({ name, value: el.props.value });
      }
    }
  }

  const view =
    phaseA.roots.length === 1
      ? phaseA.roots[0]
      : phaseA.roots.length > 0
        ? {
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
          } as ElementViewNode
        : null;

  if (view) {
    return {
      status: "valid",
      name: options.componentName,
      signals: componentSignals,
      computeds: componentComputeds,
      props: componentProps,
      events: componentEvents,
      consts,
      storeRefs: [...phaseA.bindings.storeRefs.keys()],
      fns: phaseB.lowered.functions,
      view,
      style: phaseC.styleCollection.componentStyle,
    } satisfies import("./types").ValidComponentPart;
  }

  return {
    status: "partial",
    name: options.componentName,
    signals: componentSignals,
    storeRefs: [...phaseA.bindings.storeRefs.keys()],
    computeds: componentComputeds,
    fns: phaseB.lowered.functions,
    style: phaseC.styleCollection.componentStyle,
    view: null,
    consts: consts.length > 0 ? consts : undefined,
    props: componentProps.length > 0 ? componentProps : undefined,
    events: componentEvents.length > 0 ? componentEvents : undefined,
    errors: [],
  } satisfies import("./types").PartialComponentPart;
}

// ── Helper: Build ComponentMeta ─────────────────────────────────────

export function buildComponentMeta(options: GenerateOmniSourceOptions): ComponentMeta | undefined {
  return options.componentMeta;
}
