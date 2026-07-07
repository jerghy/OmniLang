// poiesis/src/services/codegen/component-builder.ts
// Build ComponentPart and ComponentMeta from pipeline phase outputs.
// These functions assemble the final component data structure from the
// results of Phase A (view tree + bindings), Phase B (stores + chains),
// and Phase C (styles).

import type {
  ComponentPart,
  ComponentMeta,
  ViewNode,
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
  // Build component-level signals from standaloneSignals (signals not in a store)
  const componentSignals: Record<string, { type: string; defaultValue?: unknown }> = {};
  for (const s of phaseB.standaloneSignals) {
    componentSignals[s.name] = { type: s.type, defaultValue: s.defaultValue };
  }

  // Build component-level computeds from standaloneComputeds
  const componentComputeds: Record<string, { type: string; body: string }> = {};
  for (const c of phaseB.standaloneComputeds) {
    componentComputeds[c.name] = { type: c.type, body: c.body };
  }

  // Read props/events from the component registry so they appear
  // as `prop name: Type = default;` and `event name;` in the output.
  const registryDef: unknown = getComponentDefinitions()[options.componentName];
  const registryRecord = registryDef && typeof registryDef === "object"
    ? (registryDef as Record<string, unknown>)
    : null;
  const registryProps: Record<string, unknown> = registryRecord && "props" in registryRecord
    ? (registryRecord.props as Record<string, unknown>)
    : {};
  const registryEvents: Record<string, unknown> = registryRecord && "events" in registryRecord
    ? (registryRecord.events as Record<string, unknown>)
    : {};
  const componentProps: ComponentPart["props"] = Object.keys(registryProps).length > 0
    ? Object.entries(registryProps).map(([name, schema]) => ({
        name,
        type: (schema as { type: string }).type,
        default: (schema as { default?: unknown }).default,
      }))
    : undefined;
  const componentEvents: ComponentPart["events"] = Object.keys(registryEvents).length > 0
    ? Object.entries(registryEvents).map(([name]) => ({
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
    // When there's exactly one root, use it directly — don't wrap it in an extra div.
    // When there are multiple roots, wrap them in a container div for single-root view block.
    view: phaseA.roots.length === 1
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
          }
        : null,
    style: phaseC.styleCollection.componentStyle,
  };
}

// ── Helper: Build ComponentMeta ─────────────────────────────────────

export function buildComponentMeta(options: GenerateOmniSourceOptions): ComponentMeta | undefined {
  return options.componentMeta;
}
