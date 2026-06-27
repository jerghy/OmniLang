// core/state/index.ts
// Barrel export for all state atoms.

export { useElementStore } from "./elements";
export { useWireStore } from "./wires";
export { useSelectionStore } from "./selection";
export { useProjectStore } from "./project";
export { useTabStore } from "./tabs";
export { useStyleRegistryStore } from "./styleRegistry";

// Migrated stores
export { useUndoStore } from "./undo";
export { useComponentRegistry, getUnifiedComponentMeta } from "./componentRegistry";
export type { PropSchema, EventSchema, ComponentDefinition, ComponentMeta } from "./componentRegistry";

export { useCodeGraphStore } from "./codegraph";
export type { CodeEditorState, CodeGraphCommandContext } from "./codegraph";

export { createCodeGraphContext } from "./codegraph-context";

export { migrateOldCodeGraph } from "./codegraph-migration";
export type { MigrationResult } from "./codegraph-migration";
