/** CodeEditorStore — independent Zustand store for the multi-tab visual programming editor.
 *  Each code tab owns a CodeGraph (elements + wires). FlowCanvas renders the active tab's graph.
 *
 *  Design decisions:
 *  - Tabs are fully isolated — no cross-tab wires
 *  - Each tab has its own undo/redo stack (preserved on tab switch)
 *  - Clipboard is store-level (cross-tab)
 *  - Variable scope is per-tab
 */

import { create } from "zustand";
import type { ElementData, WireData } from "../store/editor";
import {
  type CodeTab,
  type CodeTabType,
  type CodeGraph,
  type ClipboardData,
  type SignalDefinition,
  type FnParam,
  emptyGraph,
  createMainTab,
  newTabId,
} from "./types";
import type { Command } from "../commands/types";

// ── Store Interface ──

export interface CodeEditorState {
  // ── Tab management ──
  tabs: CodeTab[];
  activeTabId: string;

  /** Derive: active tab's graph */
  readonly activeGraph: CodeGraph;
  /** Derive: active tab */
  readonly activeTab: CodeTab;

  // ── Clipboard ──
  clipboard: ClipboardData | null;

  // ── Tab CRUD ──
  addTab: (type: CodeTabType, label: string, opts?: {
    componentId?: string;
    fnName?: string;
    fnParams?: FnParam[];
    fnReturnType?: string;
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
  freeform: "📄",
  component: "★",
  function: "📦",
};

function uid(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

// ── Store ──

export const useCodeGraphStore = create<CodeEditorState>((set, get) => ({
  tabs: [createMainTab()],
  activeTabId: "main",

  get activeTab(): CodeTab {
    const { tabs, activeTabId } = get();
    return tabs.find((t) => t.id === activeTabId) || tabs[0];
  },

  get activeGraph(): CodeGraph {
    return get().activeTab.graph;
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
    const id = opts?.componentId
      ? `comp:${opts.componentId}`
      : opts?.fnName
        ? `fn:${opts.fnName}`
        : newTabId();

    const newTab: CodeTab = {
      id,
      type,
      label,
      icon: opts?.icon || TAB_ICONS[type],
      graph: opts?.initialGraph || emptyGraph(),
      componentId: opts?.componentId,
      fnName: opts?.fnName,
      fnParams: opts?.fnParams,
      fnReturnType: opts?.fnReturnType,
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
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === s.activeTabId
          ? { ...t, graph: { ...t.graph, elements: [...t.graph.elements, { ...el, props: el.props || {} }] } }
          : t
      ),
    })),

  removeElement: (id) =>
    set((s) => ({
      tabs: s.tabs.map((t) => {
        if (t.id !== s.activeTabId) return t;
        const el = t.graph.elements.find((e) => e.id === id);
        let elements = t.graph.elements.filter((e) => e.id !== id);
        if (el?.parentId) {
          elements = elements.map((e) =>
            e.id === el.parentId
              ? { ...e, children: e.children.filter((cid) => cid !== id) }
              : e
          );
        }
        elements = elements.filter((e) => e.parentId !== id);
        // Also remove connected wires
        const wires = t.graph.wires.filter(
          (w) => w.fromId !== id && w.toId !== id
        );
        return { ...t, graph: { elements, wires } };
      }),
    })),

  setProp: (id, prop, value) =>
    set((s) => ({
      tabs: s.tabs.map((t) => {
        if (t.id !== s.activeTabId) return t;
        return {
          ...t,
          graph: {
            ...t.graph,
            elements: t.graph.elements.map((e) =>
              e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e
            ),
          },
        };
      }),
    })),

  getProp: (id, prop) => {
    const { activeGraph } = get();
    return activeGraph.elements.find((e) => e.id === id)?.props[prop];
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
    set((s) => ({
      selectedElementIds: s.activeGraph.elements.map((e) => e.id),
    })),

  clearSelection: () => set({ selectedElementIds: [] }),

  addWire: (w) =>
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === s.activeTabId
          ? { ...t, graph: { ...t.graph, wires: [...t.graph.wires, w] } }
          : t
      ),
    })),

  removeWire: (id) =>
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === s.activeTabId
          ? { ...t, graph: { ...t.graph, wires: t.graph.wires.filter((w) => w.id !== id) } }
          : t
      ),
    })),

  setElementSource: (id, source) =>
    set((s) => ({
      tabs: s.tabs.map((t) => {
        if (t.id !== s.activeTabId) return t;
        return {
          ...t,
          graph: {
            ...t.graph,
            elements: t.graph.elements.map((e) =>
              e.id === id ? { ...e, source } : e
            ),
          },
        };
      }),
    })),

  setElementPorts: (id, ports) =>
    set((s) => ({
      tabs: s.tabs.map((t) => {
        if (t.id !== s.activeTabId) return t;
        return {
          ...t,
          graph: {
            ...t.graph,
            elements: t.graph.elements.map((e) =>
              e.id === id ? { ...e, ports } : e
            ),
          },
        };
      }),
    })),

  setElementPosition: (id, pos) =>
    set((s) => ({
      tabs: s.tabs.map((t) => {
        if (t.id !== s.activeTabId) return t;
        return {
          ...t,
          graph: {
            ...t.graph,
            elements: t.graph.elements.map((e) =>
              e.id === id ? { ...e, position: { ...pos } } : e
            ),
          },
        };
      }),
    })),

  setElementsAndWires: (elements, wires) =>
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === s.activeTabId
          ? { ...t, graph: { elements, wires } }
          : t
      ),
    })),

  // ── Clipboard ──

  copySelection: (selectedIds) => {
    const { activeGraph, activeTabId } = get();
    const idSet = new Set(selectedIds);
    const elements = activeGraph.elements.filter((e) => idSet.has(e.id));
    const wireIdSet = new Set(elements.map((e) => e.id));
    const wires = activeGraph.wires.filter(
      (w) => wireIdSet.has(w.fromId) && wireIdSet.has(w.toId)
    );
    set({ clipboard: { elements, wires, sourceTabId: activeTabId } });
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
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === s.activeTabId
          ? {
              ...t,
              graph: {
                elements: [...t.graph.elements, ...newElements],
                wires: [...t.graph.wires, ...newWires],
              },
            }
          : t
      ),
    }));

    return newElements.map((e) => e.id);
  },

  hasClipboard: () => {
    const { clipboard } = get();
    return clipboard !== null && clipboard.elements.length > 0;
  },

  // ── Variables ──

  defineVariable: (variable) =>
    set((s) => ({
      tabs: s.tabs.map((t) => {
        if (t.id !== s.activeTabId) return t;
        const existing = t.variables.findIndex((v) => v.name === variable.name);
        const variables = existing >= 0
          ? t.variables.map((v, i) => i === existing ? variable : v)
          : [...t.variables, variable];
        return { ...t, variables };
      }),
    })),

  removeVariable: (name) =>
    set((s) => ({
      tabs: s.tabs.map((t) =>
        t.id === s.activeTabId
          ? { ...t, variables: t.variables.filter((v) => v.name !== name) }
          : t
      ),
    })),

  getVariableNames: () => {
    const { activeTab } = get();
    const names: string[] = [];
    // Tab-local variables
    for (const v of activeTab.variables) names.push(v.name);
    // Variables from signal/store nodes in the graph
    for (const el of activeTab.graph.elements) {
      if (el.tag === "signal" || el.tag === "store") {
        const name = el.props.name as string;
        if (name && !names.includes(name)) names.push(name);
      }
    }
    // Function params
    if (activeTab.type === "function" && activeTab.fnParams) {
      for (const p of activeTab.fnParams) {
        if (!names.includes(p.name)) names.push(p.name);
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
interface CodeEditorState {
  selectedElementIds: string[];
}

// ── Expose for E2E tests ──

if (typeof window !== "undefined") {
  (window as any).__CODE_GRAPH_STORE__ = useCodeGraphStore;
}
