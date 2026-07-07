// poiesis/src/services/codegen/store-extractor.ts
// Two-phase store extraction from code blocks.
// Phase 1: scan for tag==="store" -> StoreDef skeleton
// Phase 2: group signal/computed/action into parent stores

import type { ElementData, CodeblockElementData } from "../../core/types";
import type { StoreDef } from "./types";

export interface StoreExtractResult {
  stores: StoreDef[];
  standaloneSignals: Array<{ name: string; type: string; defaultValue?: unknown }>;
  standaloneComputeds: Array<{ name: string; type: string; body: string }>;
}

/**
 * Two-phase extraction of store definitions and standalone signals
 * from codeblock elements.
 *
 * Phase 1: Build StoreDef skeletons from tag==="store" elements.
 * Phase 2: Assign signal/computed/action elements to their parent store
 * via props.parentStore or parentId chain walk.
 */
export function extractStores(elements: ElementData[]): StoreExtractResult {
  const codeblocks = elements.filter(e => e.nodeType === "codeblock");

  // ── Phase 1: Build store skeletons ──
  const storeDefs = new Map<string, StoreDef>();
  for (const el of codeblocks) {
    if (el.tag === "store") {
      const name = el.props.name as string;
      if (!name) continue;
      storeDefs.set(name, {
        name,
        signals: {},
        getters: {},
        actions: [],
      });
    }
  }

  // Build a lookup from element id -> element for parentId chain walking
  const elById = new Map(elements.map(e => [e.id, e]));

  /**
   * Resolve the parent store name for a codeblock element.
   * Checks props.parentStore first, then walks the parentId chain.
   */
  function resolveParentStore(el: CodeblockElementData): string | undefined {
    // Check explicit parentStore prop
    const explicit = el.props.parentStore as string | undefined;
    if (explicit && storeDefs.has(explicit)) return explicit;

    // Walk parentId chain
    let current = el.parentId;
    while (current) {
      const parent = elById.get(current);
      if (!parent) break;
      if ("tag" in parent && parent.nodeType === "codeblock" && parent.tag === "store") {
        return parent.props.name as string;
      }
      current = parent.parentId;
    }
    return undefined;
  }

  /**
   * Get the return type for a computed element.
   * Checks output ports first, falls back to props.typeAnnotation, then "unknown".
   */
  function getComputedReturnType(el: CodeblockElementData): string {
    if (el.ports) {
      const outPort = el.ports.find((p: { direction: string; type?: string }) => p.direction === "out");
      if (outPort?.type) return outPort.type;
    }
    return (el.props.typeAnnotation as string) || "unknown";
  }

  /**
   * Get the signal type from a signal element.
   */
  function getSignalType(el: CodeblockElementData): string {
    return (el.props.typeAnnotation as string) || "unknown";
  }

  // ── Phase 2: Assign signals/computeds/actions to stores ──
  const standaloneSignals: StoreExtractResult["standaloneSignals"] = [];
  const standaloneComputeds: StoreExtractResult["standaloneComputeds"] = [];

  for (const el of codeblocks) {
    const tag = el.tag;
    const name = el.props.name as string | undefined;
    if (!name) continue;

    const parentStoreName = resolveParentStore(el);

    if (tag === "signal") {
      const signalDef = { type: getSignalType(el), defaultValue: el.props.default as unknown | undefined };
      if (parentStoreName) {
        const store = storeDefs.get(parentStoreName)!;
        store.signals[name] = signalDef;
      } else {
        standaloneSignals.push({ name, type: signalDef.type, defaultValue: signalDef.defaultValue });
      }
    } else if (tag === "computed") {
      const returnType = getComputedReturnType(el);
      const body = (el.source as string) || "";
      const getterDef = { returnType, body };
      if (parentStoreName) {
        const store = storeDefs.get(parentStoreName)!;
        store.getters[name] = getterDef;
      } else {
        standaloneComputeds.push({ name, type: returnType, body });
      }
    } else if (tag === "action") {
      if (parentStoreName) {
        const store = storeDefs.get(parentStoreName)!;
        const params = (el.props.params as Array<{ name: string; type: string }>) || [];
        const body = (el.source as string) || "";
        store.actions.push({ name, params, body });
      }
      // Unmatched actions are silently dropped (no standalone equivalent)
    }
  }

  return {
    stores: [...storeDefs.values()],
    standaloneSignals,
    standaloneComputeds,
  };
}
