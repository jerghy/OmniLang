// core/state/tabs.ts
// Multi-canvas tab/canvas management state atom — extracted from store/editor.ts v2.
// Coordinates with element, wire, and selection stores for tab switching.

import { create } from "zustand";
import type { ElementData, WireData } from "../types";
import { useElementStore } from "./elements";
import { useWireStore } from "./wires";
import { useSelectionStore } from "./selection";

interface TabState {
  activeDocId: string | null;
  docBackup: { elements: ElementData[]; wires: WireData[] } | null;
  openTabs: string[];
  tabStates: Record<string, { elements: ElementData[]; wires: WireData[] }>;
  /** Monotonically increasing version counter; bumped when custom component definitions
   *  are saved on exit from a component canvas. LayoutView includes this in its
   *  recompile-change-detection key so that the preview refreshes with the new definition. */
  compVersion: number;

  /** Enter a custom component's internal canvas. Updates elements/wires to component state. */
  enterComponentCanvas: (
    tabKey: string,
    compElements: ElementData[],
    compWires: WireData[],
  ) => void;

  /** Exit component canvas, restoring the parent canvas state. */
  exitComponentCanvas: () => void;

  /** Switch to a different tab, saving/restoring per-tab state. */
  switchTab: (docId: string | null) => void;

  /** Close a tab. */
  closeTab: (docId: string) => void;

  /** Open a file tab (no switch). */
  openFileTab: (tabKey: string, elements: ElementData[], wires: WireData[]) => void;
}

export const useTabStore = create<TabState>((set, get) => ({
  activeDocId: null,
  docBackup: null,
  openTabs: [],
  tabStates: {},
  compVersion: 0,

  enterComponentCanvas: (tabKey, compElements, compWires) => {
    const elState = useElementStore.getState();
    const wireState = useWireStore.getState();
    const backup = { elements: elState.elements, wires: wireState.wires };

    const s = get();
    const newTabStates = { ...s.tabStates };
    if (s.activeDocId && s.activeDocId.startsWith("file:")) {
      newTabStates[s.activeDocId] = { elements: elState.elements, wires: wireState.wires };
    }

    // Update elements/wires to component canvas state
    elState.setElementsAndWires(compElements, compWires);
    wireState.setWires(compWires);
    useSelectionStore.getState().clearSelection();

    const newOpenTabs = s.openTabs.includes(tabKey) ? s.openTabs : [...s.openTabs, tabKey];

    set({
      docBackup: backup,
      activeDocId: tabKey,
      openTabs: newOpenTabs,
      tabStates: newTabStates,
    });
  },

  exitComponentCanvas: () => {
    const s = get();
    if (!s.activeDocId || !s.activeDocId.startsWith("comp:")) return;

    const elState = useElementStore.getState();
    const wireState = useWireStore.getState();

    // The caller should save the component state before calling exitComponentCanvas
    // via useComponentRegistry.saveCanvas()

    // Restore from backup
    const backup = s.docBackup;
    if (backup) {
      elState.setElementsAndWires(backup.elements, backup.wires);
      wireState.setWires(backup.wires);
    }
    useSelectionStore.getState().clearSelection();

    set({
      docBackup: null,
      activeDocId: null,
      compVersion: s.compVersion + 1,
    });
  },

  switchTab: (docId) => {
    const s = get();
    if (s.activeDocId === docId) return;

    const elState = useElementStore.getState();
    const wireState = useWireStore.getState();
    const newTabStates = { ...s.tabStates };
    const currentKey = s.activeDocId;

    // Save current tab state
    if (currentKey && currentKey.startsWith("file:")) {
      newTabStates[currentKey] = { elements: elState.elements, wires: wireState.wires };
    }

    let newElements: ElementData[] | undefined;
    let newWires: WireData[] | undefined;

    let newDocBackup = s.docBackup;

    if (docId === null) {
      // Switch to main canvas
      if (s.docBackup) {
        newElements = s.docBackup.elements;
        newWires = s.docBackup.wires;
      }
      newDocBackup = null;
    } else if (docId.startsWith("comp:")) {
      // Component canvas tab — elements/wires set externally via enterComponentCanvas
      return; // Must use enterComponentCanvas instead
    } else if (docId.startsWith("file:")) {
      const cached = newTabStates[docId];
      if (!cached) return; // Not yet loaded
      newElements = cached.elements;
      newWires = cached.wires;
      if (!currentKey) {
        newDocBackup = { elements: elState.elements, wires: wireState.wires };
      }
    } else {
      return;
    }

    const shouldBump = currentKey?.startsWith("comp:") || currentKey?.startsWith("file:");

    if (newElements !== undefined && newWires !== undefined) {
      elState.setElementsAndWires(newElements, newWires);
      wireState.setWires(newWires);
    }
    useSelectionStore.getState().clearSelection();

    set({
      activeDocId: docId,
      docBackup: newDocBackup,
      tabStates: newTabStates,
      compVersion: shouldBump ? s.compVersion + 1 : s.compVersion,
    });
  },

  closeTab: (docId) => {
    const s = get();
    const newOpenTabs = s.openTabs.filter((t) => t !== docId);
    const newTabStates = { ...s.tabStates };
    delete newTabStates[docId];

    if (s.activeDocId === docId) {
      const elState = useElementStore.getState();
      const wireState = useWireStore.getState();

      let wasDirty = false;

      if (docId.startsWith("file:")) {
        newTabStates[docId] = { elements: elState.elements, wires: wireState.wires };
      }

      // Restore backup or clear
      if (s.docBackup) {
        elState.setElementsAndWires(s.docBackup.elements, s.docBackup.wires);
        wireState.setWires(s.docBackup.wires);
      }
      useSelectionStore.getState().clearSelection();

      set({
        docBackup: null,
        activeDocId: null,
        openTabs: newOpenTabs,
        tabStates: newTabStates,
        compVersion: wasDirty ? s.compVersion + 1 : s.compVersion,
      });
    } else {
      set({ openTabs: newOpenTabs, tabStates: newTabStates });
    }
  },

  openFileTab: (tabKey, elements, wires) => {
    const s = get();
    if (s.openTabs.includes(tabKey)) return;
    set({
      openTabs: [...s.openTabs, tabKey],
      tabStates: { ...s.tabStates, [tabKey]: { elements, wires } },
    });
  },
}));
