// ── Tab lifecycle slice for Poiesis store ──

import type { DesignTab, DesignPageState } from "../../core/types/design-page";
import type { PoiesisState } from "../types";
import { buildTabSaveEntry, tabRestoreCanvasPartial, tabRestoreDesignPagePartial } from "../tab-helpers";
import { lruEvictTabData } from "./lru-evict";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createInitialDesignPageState(): DesignPageState {
  return {
    openTabs: [],
    activeTabId: null,
    tabOrder: [],
    codeFlowVisible: false,
    codeFlowElementId: null,
    codeFlowFocusPoint: null,
    previewOutput: null,
    breadcrumbs: [],
    designView: "tokens",
    storePanelVisible: false,
    storePanelActiveStore: null,
  };
}

export function createTabSlice(set: SetFn, _get?: GetFn) {
  let _tabIdCounter = 0;
  function generateTabId(): string {
    return `tab_${Date.now()}_${_tabIdCounter++}`;
  }
  return {
    openTab: (filePath: string, componentName: string, tabType: import("../../core/types/design-page").TabType = "component") => set((s) => {
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
        // Preserve selectedActionPid from existing tabData
        if (saveEntry && oldTabId && s.tabData[oldTabId]?.selectedActionPid) {
          saveEntry.selectedActionPid = s.tabData[oldTabId].selectedActionPid;
        }
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

      // ── Recover data from a previously-closed tab with the same filePath ──
      let restoredTabData = s.tabData;
      let restoredElements: import("../../core/types/element").ElementData[] = [];
      let restoredWires: import("../../core/types/wire").WireData[] = [];
      let restoredRootOrder: string[] = [];
      for (const [tid, entry] of Object.entries(s.tabData)) {
        if (entry.filePath === filePath && entry.elements.length > 0) {
          // Found a closed tab's data — reuse it with the new tab ID
          const { [tid]: _old, ...rest } = s.tabData;
          restoredTabData = { ...rest, [id]: entry };
          restoredElements = entry.elements;
          restoredWires = entry.wires;
          restoredRootOrder = entry.rootOrder;
          break;
        }
      }

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
      // Preserve selectedActionPid from existing tabData
      if (saveEntry && oldTabId && s.tabData[oldTabId]?.selectedActionPid) {
        saveEntry.selectedActionPid = s.tabData[oldTabId].selectedActionPid;
      }
      const updatedTabData = saveEntry
        ? { ...restoredTabData, [oldTabId!]: saveEntry }
        : restoredTabData;
      return {
        currentPage: "design",
        designPage: {
          ...s.designPage,
          openTabs: [...s.designPage.openTabs, newTab],
          tabOrder: [...s.designPage.tabOrder, id],
          activeTabId: id,
          codeFlowVisible: false,
          codeFlowElementId: null,
          codeFlowFocusPoint: null,
          previewOutput: null,
          breadcrumbs: [{ type: "architecture", label: "架构", id: "arch" }, { type: "component", label: componentName, id, filePath }],
        },
        tabData: updatedTabData,
        elements: restoredElements,
        wires: restoredWires,
        rootOrder: restoredRootOrder,
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
      // Save the CLOSED tab's canvas state before removing it.
      // If the closed tab is currently active, save the live canvas state.
      // If it's a background tab, save its per-tab tabData (not the active canvas).
      const closedTab = s.designPage.openTabs.find(t => t.id === tabId);
      const closedTabData = s.tabData[tabId];
      const isActive = s.designPage.activeTabId === tabId;
      const saveEntry = buildTabSaveEntry({
        elements: isActive ? s.elements : (closedTabData?.elements ?? []),
        wires: isActive ? s.wires : (closedTabData?.wires ?? []),
        rootOrder: isActive ? s.rootOrder : (closedTabData?.rootOrder ?? []),
        selectedElementIds: isActive ? s.selectedElementIds : (closedTabData?.selectedElementIds ?? []),
        filePath: closedTab?.filePath,
        designPage: {
          codeFlowVisible: isActive ? s.designPage.codeFlowVisible : (closedTabData?.codeFlowVisible ?? false),
          codeFlowElementId: isActive ? s.designPage.codeFlowElementId : (closedTabData?.codeFlowElementId ?? null),
        },
      });
      // Preserve selectedActionPid from existing tabData
      if (closedTabData?.selectedActionPid) {
        saveEntry.selectedActionPid = closedTabData.selectedActionPid;
      }
      const tabDataWithSave = { ...s.tabData, [tabId]: saveEntry };
      // LRU eviction: keep at most 20 entries (evicts least-recently-used, not the closed tab)
      const prunedTabData = lruEvictTabData(tabDataWithSave, new Set(newTabs.map(t => t.id)));
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
      // Preserve selectedActionPid from existing tabData
      if (oldTabId && s.tabData[oldTabId]?.selectedActionPid) {
        saveEntry.selectedActionPid = s.tabData[oldTabId].selectedActionPid;
      }
      const savedTabData = oldTabId
        ? { [oldTabId]: saveEntry }
        : {};

      // ── Sync componentDefs when leaving a custom component tab ──
      // The main canvas expands custom components from componentDefs,
      // not from tabData. Without this sync, edits made on a custom
      // component tab (e.g. MyCard) won't appear in main.omni.
      const oldTab = oldTabId
        ? s.designPage.openTabs.find((t) => t.id === oldTabId)
        : null;
      const updatedComponentDefs =
        oldTab && oldTab.componentName !== "main" && oldTab.componentName !== "design"
          ? {
              ...s.componentDefs,
              [oldTab.componentName]: {
                ...(s.componentDefs[oldTab.componentName] || {}),
                name: oldTab.componentName,
                filePath: oldTab.filePath,
                elements: s.elements,
                wires: s.wires,
                rootOrder: s.rootOrder,
              },
            }
          : s.componentDefs;

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
        componentDefs: updatedComponentDefs,
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

    toggleCodeFlow: (elementId: string | null, focusPoint?: { x: number; y: number }) => set((s) => {
      const visible = elementId !== null;
      const breadcrumbs = visible
        ? [...s.designPage.breadcrumbs, { type: "code-layer" as const, label: "代码", id: elementId }]
        : s.designPage.breadcrumbs.filter(b => b.type !== "code-layer");
      return {
        designPage: {
          ...s.designPage,
          codeFlowVisible: visible,
          codeFlowElementId: elementId,
          codeFlowFocusPoint: visible ? (focusPoint ?? null) : null,
          breadcrumbs,
        },
      };
    }),

    setBreadcrumbs: (breadcrumbs: import("../../core/types/design-page").Breadcrumb[]) => set((s) => ({
      designPage: { ...s.designPage, breadcrumbs },
    })),

    setPreviewOutput: (output: import("../../components/design/useWasmCompile").OmniViewOutput | null) => set((s) => ({
      designPage: { ...s.designPage, previewOutput: output },
    })),

    setStorePanelVisible: (visible: boolean) => set((s) => ({
      designPage: { ...s.designPage, storePanelVisible: visible },
    })),
    setStorePanelActiveStore: (name: string | null) => set((s) => ({
      designPage: { ...s.designPage, storePanelActiveStore: name },
    })),
  };
}
