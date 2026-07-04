// services/persistence/save-manager.ts
// Orchestrates the full save pipeline.
// Reads store state, writes .omni files via codegen pipeline, writes project.json.

import { usePoiesisStore } from "../../store";
import { serializeProjectJson } from "./project-json";

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
 * Collect the full project state suitable for save.
 * In the full implementation, this also triggers the codegen pipeline
 * to write .omni files. For Phase 6, we handle the project.json side.
 */
export async function saveProject(projectDir: string): Promise<{ success: boolean; error?: string; jsonStr?: string }> {
  try {
    const jsonStr = buildProjectJsonFromStore();

    // TODO: wire Tauri fs writeFile:
    // await writeTextFile(`${projectDir}/project.json`, jsonStr);

    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    void projectDir;

    return { success: true, jsonStr };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}
