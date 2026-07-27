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
 * Update code flow graphs in the store from chain emitter output.
 * Called during the save pipeline before writing project.json.
 */
