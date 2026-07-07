// ── Tab lifecycle slice for Poiesis store ──

import type { DesignTab } from "../core/types/design-page";
import type { PoiesisState } from "./index";
import { buildTabSaveEntry, tabRestoreCanvasPartial, tabRestoreDesignPagePartial } from "./tab-helpers";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createTabSlice(set: SetFn, _get?: GetFn) {
  let _tabIdCounter = 0;
  function generateTabId(): string {
    return `tab_${Date.now()}_${_tabIdCounter++}`;
  }
  return {
    openTab: (filePath: string, componentName: string, tabType: import("../core/types/design-page").TabType = "component") => set((s) => {
      const existing = s.designPage.openTabs.find(t => t.filePath === filePath);
      if (existing) {
        const oldTabId = s.designPage.activeTabId;
        const td = s.tabData[existing.id];
        // Save current tab's data before switching (if different tab)
        const saveEntry = oldTabId && oldTabId !== existing.id
          ? buildTabSaveEntry({
              elements: s.elements,
              wires: s.wires,
              rootOrder: s.rootOrder,
              selectedElementIds: s.selectedElementIds,
              designPage: s.designPage,
            })
          : undefined;
        const updatedTabData = saveEntry
          ? { ...s.tabData, [oldTabId!]: { ...s.tabData[oldTabId!], ...saveEntry } }
          : s.tabData;

        const canvasPartial = tabRestoreCanvasPartial(td, s.elements);
        const codeFlowPartial = tabRestoreDesignPagePartial(td);
        return {
          currentPage: "design",
          designPage: {
            ...s.designPage,
            activeTabId: existing.id,
            ...codeFlowPartial,
          },
          tabData: updatedTabData,
          ...canvasPartial,
        };
      }
      const id = generateTabId();
      const newTab: DesignTab = {
        id, filePath, componentName,
        isDirty: false,
        pidMap: { status: "ok", entries: {}, lineToPid: {} },
        tabType,
      };
      // Save current tab's data before switching to new tab (including code flow view state)
      const oldTabId = s.designPage.activeTabId;
      const saveEntry = oldTabId
        ? buildTabSaveEntry({
            elements: s.elements,
            wires: s.wires,
            rootOrder: s.rootOrder,
            selectedElementIds: s.selectedElementIds,
            designPage: s.designPage,
          })
        : undefined;
      const updatedTabData = saveEntry
        ? { ...s.tabData, [oldTabId!]: saveEntry }
        : s.tabData;
      return {
        currentPage: "design",
        designPage: {
          ...s.designPage,
          openTabs: [...s.designPage.openTabs, newTab],
          tabOrder: [...s.designPage.tabOrder, id],
          activeTabId: id,
          codeFlowVisible: false,
          codeFlowElementId: null,
          breadcrumbs: [{ type: "architecture", label: "架构", id: "arch" }, { type: "component", label: componentName, id, filePath }],
        },
        tabData: updatedTabData,
        elements: [],
        wires: [],
        rootOrder: [],
        selectedElementIds: [],
      };
    }),

    /** Maximum number of tabData entries to prevent unbounded growth (LRU eviction). */
    closeTab: (tabId: string) => set((s) => {
      const idx = s.designPage.openTabs.findIndex(t => t.id === tabId);
      if (idx === -1) return s;
      const newTabs = s.designPage.openTabs.filter(t => t.id !== tabId);
      const newOrder = s.designPage.tabOrder.filter(id => id !== tabId);
      let activeTabId = s.designPage.activeTabId;
      if (activeTabId === tabId) {
        const neighborIdx = Math.min(idx, newTabs.length - 1);
        activeTabId = newTabs[neighborIdx]?.id ?? null;
      }
      // Save current canvas state for the tab being closed BEFORE removing its data.
      // This ensures unsaved canvas state is persisted in tabData for undo/redo history.
      const saveEntry = buildTabSaveEntry({
        elements: s.elements,
        wires: s.wires,
        rootOrder: s.rootOrder,
        selectedElementIds: s.selectedElementIds,
        designPage: s.designPage,
      });
      const tabDataWithSave = { ...s.tabData, [tabId]: saveEntry };
      // Clean up the closed tab's data
      const { [tabId]: _, ...restTabData } = tabDataWithSave;
      // LRU eviction: keep at most 20 entries to prevent unbounded growth.
      const TAB_DATA_MAX = 20;
      let prunedTabData = restTabData;
      const keys = Object.keys(prunedTabData);
      if (keys.length > TAB_DATA_MAX) {
        const toRemove = keys.length - TAB_DATA_MAX;
        // Remove oldest (not currently open) entries — simple FIFO approximation
        const openIds = new Set(newTabs.map(t => t.id));
        const removable = keys.filter(k => !openIds.has(k)).sort();
        for (let i = 0; i < Math.min(toRemove, removable.length); i++) {
          const { [removable[i]]: _, ...remaining } = prunedTabData;
          prunedTabData = remaining;
        }
      }
      // If a neighbor tab becomes active, restore its data
      const restoreEntry = activeTabId ? prunedTabData[activeTabId] : undefined;
      const canvasPartial = tabRestoreCanvasPartial(restoreEntry, activeTabId ? undefined : s.elements);
      const codeFlowPartial = tabRestoreDesignPagePartial(restoreEntry);
      return {
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

    switchTab: (tabId: string) => set((s) => {
      const oldTabId = s.designPage.activeTabId;
      if (oldTabId === tabId) return s;
      // Save current tab's data into tabData (including code flow view state)
      const saveEntry = buildTabSaveEntry({
        elements: s.elements,
        wires: s.wires,
        rootOrder: s.rootOrder,
        selectedElementIds: s.selectedElementIds,
        designPage: s.designPage,
      });
      const savedTabData = oldTabId
        ? { [oldTabId]: saveEntry }
        : {};
      // Restore target tab's data (or empty if not yet loaded)
      const restoreEntry = s.tabData[tabId];
      const canvasPartial = tabRestoreCanvasPartial(restoreEntry);
      const codeFlowPartial = tabRestoreDesignPagePartial(restoreEntry);
      return {
        designPage: {
          ...s.designPage,
          activeTabId: tabId,
          ...codeFlowPartial,
        },
        tabData: { ...s.tabData, ...savedTabData },
        ...canvasPartial,
      };
    }),

    reorderTabs: (tabOrder: string[]) => set((s) => ({
      designPage: { ...s.designPage, tabOrder },
    })),

    markTabDirty: (tabId: string, dirty: boolean) => set((s) => ({
      designPage: {
        ...s.designPage,
        openTabs: s.designPage.openTabs.map(t => t.id === tabId ? { ...t, isDirty: dirty } : t),
      },
      isDirty: dirty,
    })),
  };
}
