// services/persistence/project-json.ts
// Core read/write for the project.json file at project root.

import type { ProjectJson } from "../../core/types/project-json";

/**
 * Write project.json with sorted keys for git merge friendliness.
 */
export function serializeProjectJson(data: ProjectJson): string {
  return JSON.stringify(data, sortedKeysReplacer, 2) + "\n";
}

/**
 * Custom JSON replacer that sorts keys alphabetically.
 */
function sortedKeysReplacer(_key: string, value: unknown): unknown {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    const sorted: Record<string, unknown> = {};
    const keys = Object.keys(value as Record<string, unknown>).sort();
    for (const k of keys) {
      sorted[k] = (value as Record<string, unknown>)[k];
    }
    return sorted;
  }
  return value;
}

/**
 * Parse project.json from raw string.
 */
export function deserializeProjectJson(raw: string): ProjectJson {
  const parsed = JSON.parse(raw);
  validateProjectJson(parsed);
  return parsed as ProjectJson;
}

/**
 * Basic validation of project.json structure.
 */
function validateProjectJson(data: unknown): void {
  if (!data || typeof data !== "object") {
    throw new Error("project.json must be a JSON object");
  }
  const obj = data as Record<string, unknown>;
  if (obj.version !== 2) {
    throw new Error(`Unsupported project.json version: ${obj.version}. Expected: 2`);
  }
  if (obj.nodes !== undefined && typeof obj.nodes !== "object") {
    throw new Error("project.json: 'nodes' must be a Record<string, NodeMeta>");
  }
  if (obj.wires !== undefined && typeof obj.wires !== "object") {
    throw new Error("project.json: 'wires' must be a Record<string, WireMeta>");
  }
}

/**
 * Create an empty project.json with defaults.
 */
export function createEmptyProjectJson(): ProjectJson {
  return {
    version: 2,
    nodes: {},
    wires: {},
    architecture: {
      mode: "overall",
      expandedBubbles: [],
      focusComponent: null,
      camera: { x: 0, y: 0, zoom: 1 },
    },
    canvasStates: {},
    panels: { leftWidth: 240, rightWidth: 300 },
    componentDefs: {},
  };
}
