// services/persistence/save-manager.ts
// Orchestrates the full save pipeline.
// Reads store state, writes .omni files via codegen pipeline, writes project.json.

import { writeTextFile, mkdir } from "@tauri-apps/plugin-fs";
import { usePoiesisStore } from "../../store";
import { serializeProjectJson } from "./project-json";
import { buildCodeFlowGraphs } from "../codegen/chain-emitter";
import type { CodeblockElementData, WireData } from "../../core/types";

/**
 * Save the current project. Pipeline:
 * 1. Read store state
 * 2. Generate .omni source via codegen pipeline (delegated to caller for now)
 * 3. Collect editor metadata
 * 4. Write project.json
 *
 * Returns the serialized project.json string.
 */
export function buildProjectJsonFromStore(): string {
  const store = usePoiesisStore.getState();
  const json = store.toProjectJson();
  return serializeProjectJson(json);
}

/**
 * Update code flow graphs in the store from chain emitter output.
 * Called during the save pipeline before writing project.json.
 */
export function updateCodeFlowGraphsInStore(
  parsedGraphs: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>,
): void {
  const graphs = buildCodeFlowGraphs(parsedGraphs);
  usePoiesisStore.setState({ codeFlowGraphs: graphs });
}

/**
 * Collect the full project state suitable for save.
 * In the full implementation, this also triggers the codegen pipeline
 * to write .omni files. For Phase 6, we handle the project.json side.
 */
export async function saveProject(projectDir: string): Promise<{ success: boolean; error?: string; jsonStr?: string }> {
  try {
    const jsonStr = buildProjectJsonFromStore();

    // Ensure .poiesis directory exists
    const poiesisDir = `${projectDir}/.poiesis`;
    await mkdir(poiesisDir, { recursive: true });

    const filePath = `${poiesisDir}/project.json`;
    await writeTextFile(filePath, jsonStr);

    return { success: true, jsonStr };
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    return { success: false, error: message, jsonStr: "" };
  }
}
