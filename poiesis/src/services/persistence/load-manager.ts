// services/persistence/load-manager.ts
// Orchestrates the full load pipeline.
// Reads .omni files, reads project.json, reconstructs store state.

import { usePoiesisStore } from "../../store";
import { deserializeProjectJson, createEmptyProjectJson } from "./project-json";
import type { ProjectJson } from "../../core/types/project-json";

/**
 * Load project from disk. Pipeline:
 * 1. Scan .omni files in project directory
 * 2. Read project.json (if exists, else create empty)
 * 3. Parse .omni sources to extract elements/wires/componentDefs
 * 4. Merge with project.json metadata (positions, fold state, canvas state)
 * 5. Populate store
 */
export async function loadProject(projectDir: string, omniFiles: string[]): Promise<{ success: boolean; error?: string }> {
  try {
    // 1. Try to load project.json
    let projectJson: ProjectJson;
    try {
      // In Tauri: const raw = await readTextFile(`${projectDir}/project.json`);
      // For now, start with empty
      projectJson = createEmptyProjectJson();
    } catch {
      projectJson = createEmptyProjectJson();
    }

    // 2. Load .omni files (delegated to Phase 1's parseOmniSource)
    // For each .omni file, parse elements/wires/componentDefs
    // This is wired in Phase 1 via the codegen pipeline

    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    void omniFiles;

    // 3. Populate store
    const store = usePoiesisStore.getState();
    store.newProject();
    store.setProjectDir(projectDir);
    store.loadFromProjectJson(projectJson);

    // 4. Build PID index from parsed .omni files
    // store.setPidIndex(pidIndex);

    return { success: true };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

/**
 * Load project.json into the store (in-memory, no .omni parsing).
 * Useful when the .omni files are already loaded and you just
 * need to restore editor metadata.
 */
export function loadProjectFromJson(raw: string): { success: boolean; error?: string } {
  try {
    const projectJson = deserializeProjectJson(raw);
    const store = usePoiesisStore.getState();
    store.loadFromProjectJson(projectJson);
    return { success: true };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

/**
 * Validate that project.json PIDs match the .omni files.
 * Returns mismatched PIDs that need regeneration.
 */
export function validatePidConsistency(
  projectJson: ProjectJson,
  pidIndex: Record<string, { sourceFile: string }>,
): string[] {
  const mismatched: string[] = [];
  for (const pid of Object.keys(projectJson.nodes)) {
    if (!(pid in pidIndex)) {
      mismatched.push(pid);
    }
  }
  return mismatched;
}
