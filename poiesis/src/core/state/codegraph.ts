/** CodeEditorStore — independent Zustand store for the multi-tab visual programming editor.
 *  Each code tab owns a CodeGraph (elements + wires). FlowCanvas renders the active tab's graph.
 *
 *  Design decisions:
 *  - Tabs are fully isolated — no cross-tab wires
 *  - Each tab has its own undo/redo stack (preserved on tab switch)
 *  - Clipboard is store-level (cross-tab)
 *  - Variable scope is per-tab
 *
 *  Migrated to core/state/
 */

import { create } from "zustand";
import { uid as _uid } from "../utils";
import type { ElementData, WireData } from "../../core/types";
import {
  type CodeTab,
  type CodeTabType,
  type CodeGraph,
  type ClipboardData,
  type SignalDefinition,
  emptyGraph,
  createMainTab,
  newTabId,
} from "../../services/codegraph/types";
import type { Command } from "../../core/commands/types";

// ── Store Interface ──

export interface CodeEditorState {
  // ── Tab management ──
  tabs: CodeTab[];
  activeTabId: string;

  /** Get the active tab's graph (NOT a Zustand getter — call directly). */
  getActiveGraph: () => CodeGraph;
  /** Get the active tab. */
  getActiveTab: () => CodeTab;

  // ── Clipboard ──
  clipboard: ClipboardData | null;

  // ── Tab CRUD ──
  addTab: (type: CodeTabType, label: string, opts?: {
    icon?: string;
    initialGraph?: CodeGraph;
  }) => string;
  removeTab: (tabId: string) => void;
  switchTab: (tabId: string) => void;
  renameTab: (tabId: string, newLabel: string) => void;

  // ── Graph mutations (always on active tab) ──
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  selectElement: (id: string | null, multi?: boolean) => void;
  selectAll: () => void;
  clearSelection: () => void;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setElementSource: (id: string, source: string) => void;
  setElementPorts: (id: string, ports: ElementData["ports"]) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;

  // ── Clipboard ──
  copySelection: (selectedIds: string[]) => void;
  pasteFromClipboard: (offsetX?: number, offsetY?: number) => string[] | null;
  hasClipboard: () => boolean;

  // ── Variables ──
  defineVariable: (variable: SignalDefinition) => void;
  removeVariable: (name: string) => void;
  /** Get all variable names in the active tab's scope. */
  getVariableNames: () => string[];

  // ── Undo / Redo (per-tab stacks) ──
  undo: (ctx: CodeGraphCommandContext) => void;
  redo: (ctx: CodeGraphCommandContext) => void;
  pushUndo: (tabId: string, cmd: Command) => void;
  clearUndo: (tabId?: string) => void;

  // ── Project lifecycle ──
  /** Reset to initial state (new project). */
  reset: () => void;
  /** Load tabs from serialized data (open project). */
  loadFromData: (tabs: CodeTab[], activeTabId?: string) => void;
  /** Get serializable tabs snapshot for save. */
  toSerializable: () => { openTabs: string[]; activeTab: string; tabs: CodeTab[] };
}

// ── CommandContext (for undo/redo system) ──

/** Context passed to Command.execute/undo for code graph operations. */
export interface CodeGraphCommandContext {
  getElement: (id: string) => ElementData | undefined;
  elements: ElementData[];
  wires: WireData[];
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;
  setElementSource: (id: string, source: string) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
}

// ── Helpers ──

const TAB_ICONS: Record<CodeTabType, string> = {
  event: "⚡",
  logic: "📄",
};

function uid(): string {
  return _uid();
}

/** Helper: update the active tab in a Zustand set call. Eliminates repeated
 *  `s.tabs.map(t => t.id === s.activeTabId ? {...} : t)` boilerplate. */
function updateActiveTab(
  set: (fn: (s: CodeEditorState) => Partial<CodeEditorState>) => void,
  get: () => CodeEditorState,
  updater: (tab: CodeTab) => CodeTab,
) {
  set((s) => ({
    tabs: s.tabs.map((t) => (t.id === s.activeTabId ? updater(t) : t)),
  }));
}

// ── Store ──

export const useCodeGraphStore = create<CodeEditorState>((set, get) => ({
  tabs: [createMainTab()],
  activeTabId: "main",

  getActiveTab: () => {
    const { tabs, activeTabId } = get();
    return tabs.find((t) => t.id === activeTabId) || tabs[0];
  },

  getActiveGraph: () => {
    const { tabs, activeTabId } = get();
    const tab = tabs.find((t) => t.id === activeTabId) || tabs[0];
    return tab ? tab.graph : { elements: [], wires: [] };
  },

  clipboard: null,

  // ── Undo stacks (not exposed in state to avoid React re-renders) ──
  undo: (ctx) => {
    const store = _undoStore.get(get().activeTabId);
    if (store.undoStack.length === 0) return;
    const cmd = store.undoStack.pop()!;
    store.redoStack.push(cmd);
    try { cmd.undo(ctx); } catch (e) { console.error("Undo failed:", e); }
  },

  redo: (ctx) => {
    const store = _undoStore.get(get().activeTabId);
    if (store.redoStack.length === 0) return;
    const cmd = store.redoStack.pop()!;
    store.undoStack.push(cmd);
    try { cmd.execute(ctx); } catch (e) { console.error("Redo failed:", e); }
  },

  pushUndo: (tabId, cmd) => {
    const store = _undoStore.get(tabId);
    store.undoStack.push(cmd);
    store.redoStack = []; // Clear redo on new action
    // Cap undo stack at 100
    if (store.undoStack.length > 100) store.undoStack.shift();
  },

  clearUndo: (tabId?: string) => {
    const id = tabId || get().activeTabId;
    const store = _undoStore.get(id);
    store.undoStack = [];
    store.redoStack = [];
  },

  // ── Tab CRUD ──

  addTab: (type, label, opts) => {
    const id = newTabId();

    const newTab: CodeTab = {
      id,
      type,
      label,
      icon: opts?.icon || TAB_ICONS[type],
      graph: opts?.initialGraph || emptyGraph(),
      variables: [],
    };

    set((s) => ({
      tabs: [...s.tabs, newTab],
      activeTabId: id,
    }));
    return id;
  },

  removeTab: (tabId) => {
    const { tabs, activeTabId } = get();
    if (tabId === "main") return; // main tab is permanent
    const idx = tabs.findIndex((t) => t.id === tabId);
    if (idx === -1) return;

    const newTabs = tabs.filter((t) => t.id !== tabId);

    // If closing the active tab, switch to main or the nearest sibling
    let newActiveId = activeTabId;
    if (activeTabId === tabId) {
      if (newTabs.length === 0) newActiveId = "main";
      else {
        const neighborIdx = Math.min(idx, newTabs.length - 1);
        newActiveId = newTabs[neighborIdx].id;
      }
    }

    set({ tabs: newTabs, activeTabId: newActiveId });
  },

  switchTab: (tabId) => {
    const { tabs } = get();
    if (!tabs.find((t) => t.id === tabId)) return;
    set({ activeTabId: tabId });
  },

  renameTab: (tabId, newLabel) =>
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === tabId ? { ...t, label: newLabel } : t
      ),
    })),

  // ── Graph mutations ──

  addElement: (el) =>
    updateActiveTab(set, get, (t) => ({
      ...t,
      graph: { ...t.graph, elements: [...t.graph.elements, { ...el, props: el.props || {} }] },
    })),

  removeElement: (id) =>
    updateActiveTab(set, get, (t) => {
      const el = t.graph.elements.find((e) => e.id === id);
      let elements = t.graph.elements.filter((e) => e.id !== id);
      if (el?.parentId) {
        elements = elements.map((e) =>
          e.id === el.parentId ? { ...e, children: e.children.filter((cid) => cid !== id) } : e
        );
      }
      elements = elements.filter((e) => e.parentId !== id);
      const wires = t.graph.wires.filter((w) => w.fromId !== id && w.toId !== id);
      return { ...t, graph: { elements, wires } };
    }),

  setProp: (id, prop, value) =>
    updateActiveTab(set, get, (t) => ({
      ...t,
      graph: {
        ...t.graph,
        elements: t.graph.elements.map((e) => {
          if (e.id !== id) return e;
          const updated = { ...e, props: { ...e.props, [prop]: value } };
          // Also update top-level `config` field so editors that read element.config stay in sync
          if (prop === "config") {
            updated.config = value as Record<string, unknown>;
          }
          return updated;
        }),
      },
    })),

  getProp: (id, prop) => {
    const graph = get().getActiveGraph();
    return graph.elements.find((e) => e.id === id)?.props[prop];
  },

  selectedElementIds: [],

  selectElement: (id, multi = false) =>
    set((s) => {
      if (id === null) return { selectedElementIds: [] as string[] };
      if (multi) {
        const sel = s.selectedElementIds as string[];
        const exists = sel.includes(id);
        return {
          selectedElementIds: exists
            ? sel.filter((x) => x !== id)
            : [...sel, id],
        };
      }
      return { selectedElementIds: [id] };
    }),

  selectAll: () =>
    set(() => ({
      selectedElementIds: get().getActiveGraph().elements.map((e) => e.id),
    })),

  clearSelection: () => set({ selectedElementIds: [] }),

  addWire: (w) =>
    updateActiveTab(set, get, (t) => ({
      ...t, graph: { ...t.graph, wires: [...t.graph.wires, w] },
    })),

  removeWire: (id) =>
    updateActiveTab(set, get, (t) => ({
      ...t, graph: { ...t.graph, wires: t.graph.wires.filter((w) => w.id !== id) },
    })),

  setElementSource: (id, source) =>
    updateActiveTab(set, get, (t) => ({
      ...t, graph: { ...t.graph, elements: t.graph.elements.map((e) => (e.id === id ? { ...e, source } : e)) },
    })),

  setElementPorts: (id, ports) =>
    updateActiveTab(set, get, (t) => ({
      ...t, graph: { ...t.graph, elements: t.graph.elements.map((e) => (e.id === id ? { ...e, ports } : e)) },
    })),

  setElementPosition: (id, pos) =>
    updateActiveTab(set, get, (t) => ({
      ...t, graph: { ...t.graph, elements: t.graph.elements.map((e) => (e.id === id ? { ...e, position: { ...pos } } : e)) },
    })),

  setElementsAndWires: (elements, wires) =>
    updateActiveTab(set, get, (t) => ({ ...t, graph: { elements, wires } })),

  // ── Clipboard ──

  copySelection: (selectedIds) => {
    const graph = get().getActiveGraph();
    const tabId = get().activeTabId;
    const idSet = new Set(selectedIds);
    const elements = graph.elements.filter((e) => idSet.has(e.id));
    const wireIdSet = new Set(elements.map((e) => e.id));
    const wires = graph.wires.filter(
      (w) => wireIdSet.has(w.fromId) && wireIdSet.has(w.toId)
    );
    set({ clipboard: { elements, wires, sourceTabId: tabId } });
  },

  pasteFromClipboard: (offsetX = 40, offsetY = 40) => {
    const { clipboard } = get();
    if (!clipboard || clipboard.elements.length === 0) return null;

    const idMap = new Map<string, string>();
    for (const el of clipboard.elements) {
      idMap.set(el.id, uid());
    }

    const newElements = clipboard.elements.map((el) => ({
      ...el,
      id: idMap.get(el.id)!,
      position: el.position
        ? { x: el.position.x + offsetX, y: el.position.y + offsetY }
        : undefined,
    }));

    const newWires = clipboard.wires
      .map((w) => {
        const newFrom = idMap.get(w.fromId);
        const newTo = idMap.get(w.toId);
        if (!newFrom || !newTo) return null;
        return { ...w, id: uid(), fromId: newFrom, toId: newTo };
      })
      .filter(Boolean) as WireData[];

    // Add to active tab
    updateActiveTab(set, get, (t) => ({
      ...t,
      graph: {
        elements: [...t.graph.elements, ...newElements],
        wires: [...t.graph.wires, ...newWires],
      },
    }));

    return newElements.map((e) => e.id);
  },

  hasClipboard: () => {
    const { clipboard } = get();
    return clipboard !== null && clipboard.elements.length > 0;
  },

  // ── Variables ──

  defineVariable: (variable) =>
    updateActiveTab(set, get, (t) => {
      const existing = t.variables.findIndex((v) => v.name === variable.name);
      const variables = existing >= 0
        ? t.variables.map((v, i) => i === existing ? variable : v)
        : [...t.variables, variable];
      return { ...t, variables };
    }),

  removeVariable: (name) =>
    updateActiveTab(set, get, (t) => ({
      ...t, variables: t.variables.filter((v) => v.name !== name),
    })),

  getVariableNames: () => {
    const tab = get().getActiveTab();
    const names: string[] = [];
    // Tab-local variables
    for (const v of tab.variables) names.push(v.name);
    // Variables from signal/store nodes in the graph
    for (const el of tab.graph.elements) {
      if (el.tag === "signal" || el.tag === "store" || el.tag === "computed") {
        const name = el.props.name as string;
        if (name && !names.includes(name)) names.push(name);
      }
      // customFn params
      if (el.tag === "customFn" && el.config?.fnParams) {
        for (const p of (el.config.fnParams as Array<{ name: string }>)) {
          if (!names.includes(p.name)) names.push(p.name);
        }
      }
    }
    return names;
  },

  // ── Lifecycle ──

  reset: () =>
    set({
      tabs: [createMainTab()],
      activeTabId: "main",
      clipboard: null,
      selectedElementIds: [],
    }),

  loadFromData: (tabs, activeTabId) => {
    const validTabs = tabs.length > 0 ? tabs : [createMainTab()];
    const validActiveId = activeTabId && validTabs.find((t) => t.id === activeTabId)
      ? activeTabId
      : validTabs[0].id;
    set({
      tabs: validTabs,
      activeTabId: validActiveId,
      clipboard: null,
      selectedElementIds: [],
    });
  },

  toSerializable: () => {
    const { tabs, activeTabId } = get();
    return {
      openTabs: tabs.map((t) => t.id),
      activeTab: activeTabId,
      tabs,
    };
  },
}));

// ── Per-tab undo stacks (module-level, not in Zustand state) ──

interface UndoStoreEntry {
  undoStack: Command[];
  redoStack: Command[];
}

const _undoStore = {
  _map: new Map<string, UndoStoreEntry>(),
  get(tabId: string): UndoStoreEntry {
    if (!this._map.has(tabId)) {
      this._map.set(tabId, { undoStack: [], redoStack: [] });
    }
    return this._map.get(tabId)!;
  },
};

// ── selectedElementIds (tracked in store for React re-renders) ──

// We extend the interface above — Zustand merges automatically
export interface CodeEditorState {
  selectedElementIds: string[];
}

// ── Expose for E2E tests ──

if (typeof window !== "undefined") {
  window.__CODE_GRAPH_STORE__ = useCodeGraphStore;
}
