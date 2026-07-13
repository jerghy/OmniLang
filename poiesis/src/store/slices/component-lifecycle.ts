// ── Component lifecycle slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { ComponentDef, DefEntry, ComputedEntry } from "../../core/types/project-json";
import { detectTabType } from "../../core/types/design-page";
import { buildTabSaveEntry, tabRestoreCanvasPartial, tabRestoreDesignPagePartial } from "../tab-helpers";
import { lruEvictTabData } from "../operations/lru-evict";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createComponentLifecycleSlice(set: SetFn, get: GetFn) {
  return {
    // ── Component defs ──
    setComponentDef: (name: string, def: ComponentDef) => set((s) => ({
      componentDefs: { ...s.componentDefs, [name]: def },
    })),
    removeComponentDef: (name: string) => set((s) => {
      const { [name]: _, ...rest } = s.componentDefs;
      return { componentDefs: rest };
    }),
    getComponentDef: (name: string) => get().componentDefs[name],

    // ── Custom component lifecycle (used by ComponentPalette) ──
    defineComponent: (name: string, def: Record<string, unknown>) => {
      const s = get();
      // Register the component definition so it appears in the palette
      if (def && typeof def === 'object' && 'name' in def) {
        s.setComponentDef(name, def as unknown as ComponentDef);
      }
      // Open the component in a new tab for editing
      // The .omni file is persisted when the user saves (Ctrl+S)
      const filePath = `src/components/${name}.omni`;
      s.openTab(filePath, name, detectTabType(filePath));
    },

    removeComponent: (name: string) => set((s) => {
      const { [name]: _, ...restDefs } = s.componentDefs;
      // Close any tabs for this component (inline closeTab logic for consistency)
      const tab = s.designPage.openTabs.find((t) => t.componentName === name);
      if (!tab) return { componentDefs: restDefs };
      const tabId = tab.id;
      const idx = s.designPage.openTabs.findIndex((t) => t.id === tabId);
      const newTabs = s.designPage.openTabs.filter((t) => t.id !== tabId);
      const newOrder = s.designPage.tabOrder.filter((id) => id !== tabId);
      let activeTabId = s.designPage.activeTabId;
      if (activeTabId === tabId) {
        const neighborIdx = Math.min(idx, newTabs.length - 1);
        activeTabId = newTabs[neighborIdx]?.id ?? null;
      }
      // Use tab helpers for save/restore
      const saveEntry = buildTabSaveEntry({
        elements: s.elements,
        wires: s.wires,
        rootOrder: s.rootOrder,
        selectedElementIds: s.selectedElementIds,
        designPage: s.designPage,
      });
      const tabDataWithSave = { ...s.tabData, [tabId]: saveEntry };
      const { [tabId]: _tab, ...restTabData } = tabDataWithSave;
      // LRU eviction: keep at most 20 entries to prevent unbounded growth.
      const prunedTabData = lruEvictTabData(restTabData, new Set(newTabs.map((t) => t.id)));
      const restoreEntry = activeTabId ? prunedTabData[activeTabId] : undefined;
      const canvasPartial = tabRestoreCanvasPartial(restoreEntry, activeTabId ? [] : undefined);
      const codeFlowPartial = tabRestoreDesignPagePartial(restoreEntry);
      return {
        componentDefs: restDefs,
        designPage: {
          ...s.designPage,
          openTabs: newTabs,
          tabOrder: newOrder,
          activeTabId,
          ...codeFlowPartial,
        },
        tabData: prunedTabData,
        ...canvasPartial,
      };
    }),

    enterComponentCanvas: (name: string) => {
      const s = get();
      const filePath = `src/components/${name}.omni`;
      // Check if tab already open
      const existing = s.designPage.openTabs.find((t) => t.componentName === name);
      if (existing) {
        s.switchTab(existing.id);
      } else {
        s.openTab(filePath, name, detectTabType(filePath));
      }
    },

    // ── Component signal CRUD ──
    addComponentSignal: (compName: string, signal: DefEntry) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const signals = [...(def.signals || []), signal];
      s.setComponentDef(compName, { ...def, signals });
      return {};
    }),

    updateComponentSignal: (compName: string, pid: string, updates: Partial<DefEntry>) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const signals = (def.signals || []).map((sig) =>
        sig.pid === pid ? { ...sig, ...updates } : sig
      );
      s.setComponentDef(compName, { ...def, signals });
      return {};
    }),

    removeComponentSignal: (compName: string, pid: string) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const signals = (def.signals || []).filter((sig) => sig.pid !== pid);
      s.setComponentDef(compName, { ...def, signals });
      return {};
    }),

    // ── Component prop CRUD ──
    addComponentProp: (compName: string, prop: DefEntry) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const props = [...(def.props || []), prop];
      s.setComponentDef(compName, { ...def, props });
      return {};
    }),

    updateComponentProp: (compName: string, pid: string, updates: Partial<DefEntry>) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const props = (def.props || []).map((p) =>
        p.pid === pid ? { ...p, ...updates } : p
      );
      s.setComponentDef(compName, { ...def, props });
      return {};
    }),

    removeComponentProp: (compName: string, pid: string) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const props = (def.props || []).filter((p) => p.pid !== pid);
      s.setComponentDef(compName, { ...def, props });
      return {};
    }),

    // ── Component computed CRUD ──
    addComponentComputed: (compName: string, computed: ComputedEntry) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const computeds = [...(def.computeds || []), computed];
      s.setComponentDef(compName, { ...def, computeds });
      return {};
    }),

    updateComponentComputed: (compName: string, pid: string, updates: Partial<ComputedEntry>) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const computeds = (def.computeds || []).map((c) =>
        c.pid === pid ? { ...c, ...updates } : c
      );
      s.setComponentDef(compName, { ...def, computeds });
      return {};
    }),

    removeComponentComputed: (compName: string, pid: string) => set((s) => {
      const def = s.componentDefs[compName];
      if (!def) return {};
      const computeds = (def.computeds || []).filter((c) => c.pid !== pid);
      s.setComponentDef(compName, { ...def, computeds });
      return {};
    }),
  };
}
