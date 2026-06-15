/** Pure function: convert WireData[] to React Flow Edge[].
 *  Data-only edges — flow edges are removed in the new architecture. */

import type { Edge } from "@xyflow/react";
import type { ElementData, WireData } from "../store/editor";
import { getAllPorts } from "./connectionValidators";

export interface WiresToEdgesOptions {
  wires: WireData[];
  elements: ElementData[];
  /** Highlighted wire ID (for PropertyPanel wire selection). */
  highlightedWireId?: string | null;
  /** Active edge IDs for runtime execution highlighting. */
  activeEdgeIds?: Set<string>;
}

export function wiresToEdges(opts: WiresToEdgesOptions): Edge[] {
  const { wires, elements, highlightedWireId, activeEdgeIds } = opts;

  const edges: Edge[] = [];

  for (const w of wires) {
    const fromEl = elements.find((e) => e.id === w.fromId);
    const toEl = elements.find((e) => e.id === w.toId);
    if (!fromEl || !toEl) continue;

    const fromPorts = getAllPorts(fromEl);
    const fromPort = fromPorts.find((p) => p.name === w.fromPort);
    const dataType = fromPort?.type || "any";

    const isActiveEdge = activeEdgeIds?.has(w.id);

    edges.push({
      id: w.id,
      source: w.fromId,
      target: w.toId,
      sourceHandle: w.fromPort,
      targetHandle: w.toPort,
      type: "data", // always data edges
      animated: !!isActiveEdge,
      data: { type: dataType },
      selected: w.id === highlightedWireId,
      style: isActiveEdge
        ? {
            stroke: "#10B981",
            strokeWidth: 2.5,
            filter: "drop-shadow(0 0 4px rgba(16,185,129,0.5))",
          }
        : undefined,
    });
  }

  return edges;
}
