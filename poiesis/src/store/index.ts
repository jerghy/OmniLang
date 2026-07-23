// ── Poiesis Store ──
// Created by composing slice factories into a single Zustand store.

import { create } from "zustand";
import type { PoiesisState } from "./types";
import { EMPTY_DESIGN } from "../core/types/design";
import { DEFAULT_PROJECT_NAME } from "../constants";
import { COMPONENT_PROTOCOL } from "../data/componentProtocol";
import type { ComponentProtocolEntry } from "../data/componentProtocol";
import { createArchitectureSlice, createInitialArchitectureState } from "./slices/architecture";
import { createDesignSystemSlice } from "./slices/design-system";
import { createElementCrudSlice } from "./slices/element-crud";
import { createProjectSlice } from "./slices/project";
import { createMetadataSlice } from "./slices/metadata";
import { createComponentLifecycleSlice } from "./slices/component-lifecycle";
import { createNavigationSlice } from "./slices/navigation";
import { createSerializationSlice } from "./slices/serialization";
import { createStoreCrudSlice } from "./slices/store-crud";
import { createTabSlice, createInitialDesignPageState } from "./operations/tab-lifecycle";
import { createUndoSlice } from "./undo-slice";
import { createThemeSlice } from "./slices/theme";

export const usePoiesisStore = create<PoiesisState>((set, get) => ({
  // ── Initial state values ──
  currentPage: "design",
  architecture: createInitialArchitectureState(),
  activeDesign: { ...EMPTY_DESIGN },
  elementVariants: {},
  designPage: createInitialDesignPageState(),
  elements: [],
  wires: [],
  rootOrder: [],
  selectedElementIds: [],
  tabData: {},
  componentDefs: {},
  debugSignalValues: {},
  pidIndex: {},
  projectDir: null,
  projectName: DEFAULT_PROJECT_NAME,
  isDirty: false,
  activeFilePath: null,
  activeFileContent: null,
  mode: "design",
  compileTarget: "react",
  compileErrors: [],
  previewUrl: null,
  nodeMeta: {},
  wireMeta: {},
  canvasStates: {},
  panels: { leftWidth: 240, rightWidth: 300 },
  undoStacks: {},
  codeFlowGraphs: {},

  // ── Actions (composed from slice factories) ──
  ...createThemeSlice(set, get),
  ...createArchitectureSlice(set, get),
  ...createTabSlice(set, get),
  ...createDesignSystemSlice(set, get),
  ...createElementCrudSlice(set, get),
  ...createUndoSlice(set, get),
  ...createComponentLifecycleSlice(set, get),
  ...createProjectSlice(set, get),
  ...createMetadataSlice(set, get),
  ...createNavigationSlice(set, get),
  ...createSerializationSlice(set, get),
  ...createStoreCrudSlice(set, get),
}));

// ── HMR State Preservation ──
// Vite HMR re-evaluates this module, creating a new Zustand store with empty
// initial state. This breaks the editor: the canvas shows "空画布" and React
// fibers hold stale references to the pre-HMR store (zombie state).
//
// Fix: persist state via window (which survives HMR). On dispose, snapshot
// current store state. On re-evaluation, restore it into the new store.
// Must run BEFORE exposing the store on window.
const HMR_STATE_KEY = "__POIESIS_HMR_STATE__" as const;
if (typeof window !== "undefined") {
  const saved = (window as unknown as Record<string, unknown>)[HMR_STATE_KEY] as PoiesisState | undefined;
  if (saved) {
    usePoiesisStore.setState(saved);
  }
}
if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    (window as unknown as Record<string, unknown>)[HMR_STATE_KEY] = usePoiesisStore.getState();
  });
}

// Expose store on window for debugging (Victauri MCP access).
// Always points to current module's store (overwritten on HMR re-evaluation).
if (typeof window !== "undefined") {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (window as any).__POIESIS_STORE__ = usePoiesisStore;
}

// ── Utility accessors (used by codegen services and UI components) ──

/**
 * Safe accessor for component definitions — reads from componentDefs.
 */
export function getComponentDefinitions(): Record<string, Record<string, unknown>> {
  const state = usePoiesisStore.getState();
  return (state?.componentDefs ?? {}) as unknown as Record<string, Record<string, unknown>>;
}

/** Return unified component metadata (built-in + custom). */
export function getUnifiedComponentMeta(): Record<string, ComponentProtocolEntry> {
  const state = usePoiesisStore.getState();
  const customDefs = state.componentDefs ?? {};

  const unified: Record<string, ComponentProtocolEntry> = COMPONENT_PROTOCOL
    ? { ...COMPONENT_PROTOCOL }
    : {};

  for (const [name, def] of Object.entries(customDefs)) {
    const typedDef = def as unknown as Record<string, unknown>;
    // Skip non-custom components — only those in src/components/ are user-defined custom components
    const filePath = (typedDef.filePath as string) || "";
    if (!filePath.startsWith("src/components/")) continue;
    unified[name] = {
      component: name,
      category: (typedDef.category as string) || "Custom",
      description: (typedDef.description as string) || "",
      props: {},
      events: {},
      styleProperties: (typedDef.styleProperties as string[]) || [],
      children: (typedDef.children as boolean) ?? true,
    };
  }

  return unified;
}

export type { PoiesisState } from "./types";

// ── Window exposure for E2E / Victauri ──
// Always update to current module's store (survives HMR re-evaluation).
if (typeof window !== "undefined") {
  (window as unknown as Record<string, unknown>).__ZUSTAND_STORE__ = usePoiesisStore;
}
