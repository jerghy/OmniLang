// services/project/component-service.ts
// Custom component definition save/load logic — extracted from project-service.ts.

import { generateOmniSource } from "../codegen";
import { api } from "../../commands";
import { uid } from "../../core/utils/id";
import type { ComputedEntry, DefEntry } from "../../core/types/project-json";
import type { ElementData, WireData } from "../../core/types";

/**
 * Convert registry definitions to omni DSL and save to server.
 * Each component's elements/wires are emitted as proper .omni source
 * with metadata stored as // @component / @prop / @event / @style comments
 * (no more .meta.json).
 */
export async function saveComponentDefinitionsToServer(
  defs: Record<string, { id?: string; category?: string; description?: string; props?: unknown; events?: unknown; styleProperties?: string[]; children?: boolean; createdAt?: string; elements?: ElementData[]; wires?: WireData[] }>,
): Promise<void> {
  const payload: Record<string, { omniSource: string }> = {};

  for (const [name, def] of Object.entries(defs)) {
    const result = generateOmniSource({
      designElements: def.elements || [],
      designWires: def.wires || [],
      codeTabs: [],
      componentName: name,
      rootOrder: (def.elements || [])
        .filter((e: { parentId?: string | null }) => !e.parentId)
        .map((e: { id: string }) => e.id),
      // forSave removed — pipeline always emits clean code
      componentMeta: {
        id: def.id,
        category: def.category,
        description: def.description,
        propSchema: def.props as Record<string, { editor?: string; values?: string[] }>,
        eventSchema: def.events as Record<string, { payload?: string; description?: string }>,
        styleProperties: def.styleProperties,
        children: def.children,
      },
    });

    if (result.value) {
      payload[name] = { omniSource: result.value };
    }
  }

  if (Object.keys(payload).length > 0) {
    await api.annotation.saveComponentDefinitions(
      payload as unknown as Record<string, unknown>,
    );
  }
}

/**
 * Load custom component definitions from server.
 * Parses .omni source back to elements/wires; metadata is extracted from
 * // @component / @prop / @event / @style comments in the source.
 */
export async function loadComponentDefinitionsFromServer(): Promise<
  Record<string, unknown>
> {
  const [{ parseOmniSource }, result] = await Promise.all([
    import("../codegen"),
    api.annotation.loadComponentDefinitions(),
  ]);
  const merged: Record<string, unknown> = {};

  for (const [name, raw] of Object.entries(result.definitions)) {
    const data = raw as Record<string, unknown>;
    const source = (data.source as string) || "";
    if (!source) continue;

    const parsed = parseOmniSource(source);
    const meta = parsed.componentMeta || {};

    // Elements/wires come from parsed view tree
    const elements =
      parsed.designElements && parsed.designElements.length > 0
        ? parsed.designElements
        : [];
    const wires = parsed.designWires || [];

    // Props schema from comment metadata (// @prop), fallback to DSL declarations
    const propSchema: Record<string, unknown> =
      (meta.propSchema as Record<string, unknown>) || {};
    const dslProps = parsed.props || [];
    for (const p of dslProps) {
      if (!propSchema[p.name]) {
        propSchema[p.name] = {
          type: p.type,
          default: p.defaultValue,
          editor: "text",
        };
      }
    }

    // Events schema from comment metadata (// @event), fallback to DSL declarations
    const eventSchema: Record<string, unknown> =
      (meta.eventSchema as Record<string, unknown>) || {};
    const dslEvents = parsed.events || [];
    for (const e of dslEvents) {
      if (!eventSchema[e.name]) {
        eventSchema[e.name] = { id: e.name, description: "", payload: null };
      }
    }

    // Signals from DSL declarations
    const dslSignals = parsed.signals || [];
    const mappedSignals = dslSignals.map((s) => ({
      pid: uid("sig"),
      name: s.name,
      type: s.type,
      defaultValue: s.defaultValue,
    }));

    // Computeds from DSL declarations
    const dslComputeds = parsed.computeds || [];
    const mappedComputeds: ComputedEntry[] = dslComputeds.map((c) => ({
      pid: uid("comp"),
      name: c.name,
      returnType: c.type,
      expression: c.expression,
    }));

    // Normalize props to DefEntry[] (server stores as Record, component-builder expects array)
    const propsArray: DefEntry[] = Object.entries(propSchema).map(([pName, schema]) => ({
      pid: uid("p"),
      name: pName,
      type: ((schema as Record<string, unknown>)?.type as string) || "String",
      defaultValue: (schema as Record<string, unknown>)?.default,
    }));

    // Normalize events to array (server stores as Record, component-builder expects array)
    const eventsArray: Array<{ pid: string; name: string; payloadType: string }> =
      Object.entries(eventSchema).map(([eName, schema]) => ({
        pid: uid("evt"),
        name: eName,
        payloadType: ((schema as Record<string, unknown>)?.payload as string) || "",
      }));

    merged[name] = {
      id: (meta.id as string) || name,
      name,
      category: (meta.category as string) || "Custom",
      description: (meta.description as string) || "",
      signals: mappedSignals,
      props: propsArray,
      events: eventsArray,
      computeds: mappedComputeds,
      styleProperties: (meta.styleProperties as string[]) || [],
      children: (meta.children as boolean) ?? true,
      elements,
      wires,
      codeFlowGraphs: parsed.codeFlowGraphs || {},
      createdAt: ((meta as Record<string, unknown>).createdAt as string) || new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      storage: "file",
    };
  }

  return merged;
}
