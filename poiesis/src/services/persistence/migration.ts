// poiesis/src/services/persistence/migration.ts
// v2 -> v3 project.json migration.

import type { ProjectJson, CodeFlowGraph, NodeMeta, WireMeta, CanvasViewport } from "../../core/types/project-json";

/**
 * Check if project.json needs migration.
 */
export function needsMigration(json: { version: number }): boolean {
  return json.version < 3;
}

/**
 * Migrate v2 project.json -> v3.
 *
 * Changes:
 * - version: 2 -> 3
 * - + codeFlowGraphs: {} (empty -- populated on first save from chain emitter)
 * - nodes[pid] left as-is (nodeType inferred during parse if missing)
 */
export function migrateV2ToV3(
  v2: Record<string, unknown> & { version: 2 },
): ProjectJson {
  return {
    version: 3,
    nodes: (v2.nodes as unknown as Record<string, NodeMeta>) || {},
    wires: (v2.wires as unknown as Record<string, WireMeta>) || {},
    architecture: (v2.architecture as ProjectJson["architecture"]) || {
      mode: "overall",
      expandedBubbles: [],
      focusComponent: null,
      camera: { x: 0, y: 0, zoom: 1 },
    },
    canvasStates: (v2.canvasStates as unknown as Record<string, CanvasViewport>) || {},
    panels: (v2.panels as ProjectJson["panels"]) || { leftWidth: 240, rightWidth: 300 },
    codeFlowGraphs: (v2.codeFlowGraphs as unknown as Record<string, CodeFlowGraph>) || {},
  };
}
