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
import { createLibDefsSlice } from "./slices/lib-defs";
import { createBehaviorSlice } from "./slices/behavior";

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
  debugValues: {},
  debugOverridesEnabled: true,
  debugValueCandidates: {},
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
  lifecycleGraphs: {},
  persistedPositions: {},
  codeFlowGroups: {},
  libDefs: {},

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
  ...createLibDefsSlice(set, get),
  ...createBehaviorSlice(set, get),
}));

// ── HMR State Preservation ──
interface WindowWithPoiesis {
  __POIESIS_HMR_STATE__?: Partial<PoiesisState>;
  __POIESIS_STORE__?: typeof usePoiesisStore;
  __ZUSTAND_STORE__?: unknown;
}

// Vite HMR re-evaluates this module, creating a new Zustand store with empty
// initial state. This breaks the editor: the canvas shows "空画布" and React
// fibers hold stale references to the pre-HMR store (zombie state).
//
// Fix: persist only serializable data via window (which survives HMR).
// On dispose, snapshot current store state (excluding functions and undo stacks).
// On re-evaluation, restore data fields into the new store.
// Must run BEFORE exposing the store on window.
if (typeof window !== "undefined") {
  const win = window as unknown as WindowWithPoiesis;
  const saved = win.__POIESIS_HMR_STATE__;
  if (saved) {
    usePoiesisStore.setState(saved);
  }
}

/** Extract only serializable data from store state for HMR persistence. */
function getSerializableState(state: PoiesisState): Partial<PoiesisState> {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(state) as Array<[string, unknown]>) {
    if (typeof value === "function") continue;
    if (key === "undoStacks") continue;
    try {
      JSON.stringify(value);
      result[key] = value;
    } catch {
      // Skip non-serializable values
    }
  }
  return result as Partial<PoiesisState>;
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    (window as unknown as WindowWithPoiesis).__POIESIS_HMR_STATE__ =
      getSerializableState(usePoiesisStore.getState());
  });
}

// Expose store on window for debugging (Victauri MCP access).
// Always points to current module's store (overwritten on HMR re-evaluation).
if (typeof window !== "undefined") {
  (window as unknown as WindowWithPoiesis).__POIESIS_STORE__ = usePoiesisStore;
}

// ── Selection change subscriber ──
// Fire a CustomEvent when selectedElementIds changes, so the debug window (and
// other listeners) see committed store state rather than stale state from before
// Zustand's set() commits under React 18+ automatic batching.
if (typeof window !== "undefined") {
  usePoiesisStore.subscribe((state, prevState) => {
    const ids = state.selectedElementIds;
    if (ids !== prevState.selectedElementIds) {
      window.dispatchEvent(
        new CustomEvent("poiesis:editor-select", {
          detail: { selectedElementIds: ids },
        }),
      );
    }
  });
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
