// Legacy adapter: codegraph store merged into unified PoiesisStore.
import { usePoiesisStore } from "../../store";
import type { PoiesisState } from "../../store";
import type { ElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import type { CodeTab } from "../../core/types/codegraph";

/** Extended state interface documenting legacy codegraph methods
 *  that callers may rely on. These methods are provided as helper
 *  functions rather than being on PoiesisState. */
export interface CodeGraphStoreState extends PoiesisState {
  getActiveGraph(): { elements: ElementData[]; wires: WireData[] };
  loadFromData(codeTabs: CodeTab[]): void;
  toSerializable(): { tabs: CodeTab[] };
}

// Re-export the unified store directly — the stores have been merged.
export const useCodeGraphStore = usePoiesisStore;

export type CodeEditorState = Record<string, unknown>;

// ── Legacy codegraph helpers ──

/** Helper: get active graph data (replaces getActiveGraph()). */
export function getActiveGraph(): { elements: ElementData[]; wires: WireData[] } {
  const s = usePoiesisStore.getState();
  return { elements: s.elements, wires: s.wires };
}

/** Helper: serialize code tabs (replaces toSerializable()). */
export function serializeCodeTabs(): { tabs: CodeTab[] } {
  const s = usePoiesisStore.getState();
  return {
    tabs: s.designPage.openTabs.map(tab => ({
      id: tab.id,
      type: tab.tabType === "code" ? "event" as const : "logic" as const,
      label: tab.componentName,
      icon: tab.tabType === "code" ? "code" as const : "component" as const,
      graph: s.tabData[tab.id] ?? { elements: s.elements, wires: s.wires },
      variables: [],
    })),
  };
}

/** Helper: load code tabs into store (replaces loadFromData()). */
export function loadCodeTabs(codeTabs: CodeTab[]): void {
  const s = usePoiesisStore.getState();
  for (const tab of codeTabs) {
    s.openTab(`legacy/${tab.id}.omni`, tab.label);
  }
}
