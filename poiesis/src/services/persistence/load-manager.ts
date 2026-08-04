// services/persistence/load-manager.ts
// Orchestrates the full load pipeline.
// Reads .omni files, reads project.json, reconstructs store state.

import { usePoiesisStore } from "../../store";
import { deserializeProjectJson } from "./project-json";

/**
 * Load project.json into the store (in-memory, no .omni parsing).
 * Useful when the .omni files are already loaded and you just
 * need to restore editor metadata.
 */
export function loadProjectFromJson(raw: string): { success: boolean; error?: string } {
  try {
    const projectJson = deserializeProjectJson(raw);
    const store = usePoiesisStore.getState();
    store.newProject();
    store.loadFromProjectJson(projectJson);
    // v3 持久化形状（pid/nodes/wires）不恢复进 store.codeFlowGraphs——解析侧图
    // 由 App.tsx loadOmniFiles 物化重建（见 store/slices/serialization.ts）。
    return { success: true };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}
