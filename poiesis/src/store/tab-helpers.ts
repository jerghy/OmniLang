// ── Tab Data Helper Utilities ──
// Extracts the duplicated tabData save/restore logic shared across tab-slice.ts,
// store/index.ts, and any other code that manages per-tab canvas persistence.

import type { ElementData } from "../core/types/element";
import type { WireData } from "../core/types/wire";

// ── Types ──

export interface TabDataEntry {
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
  selectedElementIds: string[];
  filePath?: string; // preserved across close/reopen to restore tab state
  codeFlowVisible?: boolean;
  codeFlowElementId?: string | null;
  /** Store tab: currently selected action PID */
  selectedActionPid?: string | null;
}

/**
 * Minimal source shape required to build a save entry.
 * Accepts any object with these fields — avoids coupling to PoiesisState.
 */
export interface TabSaveSource {
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
  selectedElementIds: string[];
  filePath?: string;
  designPage: { codeFlowVisible: boolean; codeFlowElementId: string | null };
}

// ── Helpers ──

/**
 * Build a TabDataEntry from the current store state.
 * Call this before switching/opening/closing tabs to persist canvas data.
 *
 * Usage: const entry = buildTabSaveEntry(state);
 *        const tabData = { ...state.tabData, [tabId]: entry };
 */
export function buildTabSaveEntry(source: TabSaveSource): TabDataEntry {
  return {
    elements: source.elements,
    wires: source.wires,
    rootOrder: source.rootOrder,
    selectedElementIds: source.selectedElementIds,
    filePath: source.filePath,
    codeFlowVisible: source.designPage.codeFlowVisible,
    codeFlowElementId: source.designPage.codeFlowElementId,
    selectedActionPid: (source as { selectedActionPid?: string | null }).selectedActionPid ?? null,
  };
}

/**
 * Build canvas data partial (elements, wires, rootOrder, selectedElementIds)
 * from a saved TabDataEntry with an optional fallback for elements.
 *
 * Usage: const partial = tabRestoreCanvasPartial(s.tabData[tabId], s.elements);
 *        return { ...partial, ...otherState };
 */
export function tabRestoreCanvasPartial(
  entry?: TabDataEntry,
  fallbackElements?: ElementData[],
): {
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
  selectedElementIds: string[];
} {
  return {
    elements: entry?.elements ?? fallbackElements ?? [],
    wires: entry?.wires ?? [],
    rootOrder: entry?.rootOrder ?? [],
    selectedElementIds: entry?.selectedElementIds ?? [],
  };
}

/**
 * Extract code flow design page fields from a saved entry.
 *
 * Usage: const codeFlow = tabRestoreDesignPagePartial(s.tabData[tabId]);
 *        return { designPage: { ...s.designPage, ...codeFlow } };
 */
export function tabRestoreDesignPagePartial(entry?: TabDataEntry): {
  codeFlowVisible: boolean;
  codeFlowElementId: string | null;
} {
  return {
    codeFlowVisible: entry?.codeFlowVisible ?? false,
    codeFlowElementId: entry?.codeFlowElementId ?? null,
  };
}
