import { create } from "zustand";
import type { ElementData, PortData } from "../core/types/element";
import type { WireData } from "../core/types/wire";
import type { ArchitectureState, BubbleNode, BubbleEdge } from "../core/types/architecture";
import type { DesignPageState, DesignTab, Breadcrumb } from "../core/types/design-page";
import { detectTabType } from "../core/types/design-page";
import type { PidEntry } from "../core/types/pid";
import type { ComponentDef, ProjectJson, NodeMeta, WireMeta } from "../core/types/project-json";
import type { StyleProperty } from "../core/types/style";
import { DEFAULT_PROJECT_NAME, DEFAULT_PANEL_LAYOUT } from "../constants";
import type { UndoAction, UndoStack } from "./undo-slice";
import { skipUndoGuard, buildUndoUpdate, createUndoSlice } from "./undo-slice";
import { createTabSlice } from "./tab-slice";

// ── Store Interface ──

export interface PoiesisState {
  // ── Page routing ──
  currentPage: "architecture" | "design";

  // ── Architecture page ──
  architecture: ArchitectureState;

  // ── Design page ──
  designPage: DesignPageState;

  // ── Current tab's canvas data (active tab's working copy) ──
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
  selectedElementIds: string[];

  /** Per-tab canvas data for persistence across tab switches.
   *  Keyed by tab ID. Null entries will be loaded from .omni parse on first switch. */
  tabData: Record<string, {
    elements: ElementData[];
    wires: WireData[];
    rootOrder: string[];
    selectedElementIds: string[];
  }>;

  // ── Component definitions (parsed from .omni, cached) ──
  componentDefs: Record<string, ComponentDef>;

  /** Legacy definitions field — set at runtime via componentRegistry adapter.
   *  May overlap with componentDefs during migration. */
  definitions?: Record<string, Record<string, unknown>>;

  // ── PID index ──
  pidIndex: Record<string, PidEntry>;

  // ── Project ──
  projectDir: string | null;
  projectName: string;
  isDirty: boolean;
  activeFilePath: string | null;
  activeFileContent: string | null;

  // ── Mode ──
  mode: "design" | "code";
  compileTarget: string;
  switchMode(mode: "design" | "code"): void;

  // ── Compilation ──
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;

  // ── File explorer (runtime-only, not persisted) ──
  files?: Array<{ name: string; path: string; type: "file" | "directory"; ext?: string }>;

  // ── Legacy runtime fields (backward compat with driver/service code) ──

  /** @deprecated Legacy — use designPage.activeTabId instead */
  activeDocId?: string;

  /** @deprecated Legacy — use activeFileContent instead */
  sourceCode?: string;

  /** @deprecated Legacy — component canvas backup for save */
  docBackup?: { elements: unknown[]; wires: unknown[] };

  /** @deprecated Legacy — per-doc tab states */
  tabStates?: Record<string, unknown>;

  /** @deprecated Legacy — use designPage.openTabs instead */
  openTabs: string[];

  // ── Editor metadata (for project.json save/load) ──
  nodeMeta: Record<string, NodeMeta>;
  wireMeta: Record<string, WireMeta>;
  canvasStates: Record<string, { zoom: number; panX: number; panY: number }>;
  panels: { leftWidth: number; rightWidth: number };

  // ── Undo/redo (keyed by tabId) ──
  undoStacks: Record<string, UndoStack>;

  // ── Actions ──

  // Architecture
  setArchMode: (mode: "overall" | "local") => void;
  expandBubble: (bubbleId: string) => void;
  collapseBubble: (bubbleId: string) => void;
  focusComponent: (componentId: string | null) => void;
  selectBubbleByPid: (pid: string | null) => void;
  setArchCamera: (camera: { x: number; y: number; zoom: number }) => void;
  setBubblesAndEdges: (bubbles: BubbleNode[], edges: BubbleEdge[]) => void;

  // Design page
  openTab: (filePath: string, componentName: string, tabType?: import("../core/types/design-page").TabType) => void;
  closeTab: (tabId: string) => void;
  switchTab: (tabId: string) => void;
  reorderTabs: (tabOrder: string[]) => void;
  markTabDirty: (tabId: string, dirty: boolean) => void;
  toggleCodeFlow: (elementId: string | null) => void;
  setBreadcrumbs: (breadcrumbs: Breadcrumb[]) => void;

  // Element/wire CRUD
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  selectElement: (id: string | null, multi?: boolean) => void;
  clearSelection: () => void;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setElementSource: (id: string, source: string) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
  setElementClassRefs: (elementId: string, classRefs: string[]) => void;
  setElementInlineStyles: (elementId: string, properties: StyleProperty[]) => void;
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;

  // Undo/redo
  undo: () => void;
  redo: () => void;

  // Component defs
  setComponentDef: (name: string, def: ComponentDef) => void;
  removeComponentDef: (name: string) => void;
  getComponentDef: (name: string) => ComponentDef | undefined;

  // Custom component lifecycle (used by ComponentPalette)
  defineComponent: (name: string, def: Record<string, unknown>) => void;
  removeComponent: (name: string) => void;
  enterComponentCanvas: (name: string) => void;

  // Project
  setProjectDir: (dir: string) => void;
  setProjectName: (name: string) => void;
  resetEditorState: () => void;
  openFile: (path: string, content: string) => void;
  setActiveFileContent: (content: string) => void;
  markDirty: () => void;
  markClean: () => void;
  setCompileErrors: (errors: Array<{ code: string; message: string; line: number }>) => void;
  setPreviewUrl: (url: string | null) => void;

  // PID index
  setPidIndex: (index: Record<string, PidEntry>) => void;
  updatePidEntry: (pid: string, entry: PidEntry) => void;

  // Node/wire metadata
  setNodeMeta: (pid: string, meta: NodeMeta) => void;
  setWireMeta: (wireId: string, meta: WireMeta) => void;
  removeNodeMeta: (pid: string) => void;
  removeWireMeta: (wireId: string) => void;
  setCanvasState: (filePath: string, state: { zoom: number; panX: number; panY: number }) => void;
  setPanelSize: (side: "left" | "right", width: number) => void;

  // Navigation
  navigateTo: (page: "architecture" | "design") => void;

  // New project
  newProject: () => void;

  // Project.json load/save helpers
  toProjectJson: () => ProjectJson;
  loadFromProjectJson: (json: ProjectJson) => void;

  // ── Legacy runtime methods (backward compat with driver/service code) ──

  /** @deprecated Legacy — use setProp + proper element update instead */
  setElementPorts: (id: string, ports: PortData[] | undefined) => void;

  /** @deprecated Legacy — event binding management will be refactored */
  setEventBindings: (id: string, eventName: string, bindings: unknown) => void;

  /** @deprecated Legacy — event binding management will be refactored */
  removeEventBindings: (id: string, eventName: string) => void;

  /** @deprecated Legacy — use activeFileContent + generateOmniSource instead */
  setSourceCode: (content: string) => void;

  /** @deprecated Legacy — file loading will be consolidated */
  loadFiles: (files: Array<{ name: string; path: string; type: string }>) => void;
}

// ── Initial State ──

function createInitialArchitectureState(): ArchitectureState {
  return {
    mode: "overall",
    bubbles: [],
    edges: [],
    focusComponentId: null,
    expandedBubbleIds: [],
    selectedPid: null,
    camera: { x: 0, y: 0, zoom: 1 },
  };
}

function createInitialDesignPageState(): DesignPageState {
  return {
    openTabs: [],
    activeTabId: null,
    tabOrder: [],
    codeFlowVisible: false,
    codeFlowElementId: null,
    breadcrumbs: [],
  };
}

// ── Store Implementation ──

export const usePoiesisStore = create<PoiesisState>((set, get) => ({
  // ── Initial values ──
  currentPage: "design",
  architecture: createInitialArchitectureState(),
  designPage: createInitialDesignPageState(),
  elements: [],
  wires: [],
  rootOrder: [],
  selectedElementIds: [],
  tabData: {},
  componentDefs: {},
  pidIndex: {},
  projectDir: null,
  projectName: DEFAULT_PROJECT_NAME,
  isDirty: false,
  activeFilePath: null,
  activeFileContent: null,
  openTabs: [],
  mode: "design",
  compileTarget: "react",
  compileErrors: [],
  previewUrl: null,
  nodeMeta: {},
  wireMeta: {},
  canvasStates: {},
  panels: { ...DEFAULT_PANEL_LAYOUT },
  undoStacks: {},

  // ── Architecture actions ──
  setArchMode: (mode) => set((s) => ({ architecture: { ...s.architecture, mode } })),

  expandBubble: (bubbleId) => set((s) => {
    const linkedIds = s.architecture.bubbles
      .filter(b => b.id === bubbleId)
      .flatMap(b => {
        const linked = s.architecture.bubbles.filter(
          o => o.depth === b.depth && o.type !== b.type && o.parentId === b.parentId
        );
        return [b.id, ...linked.map(l => l.id)];
      });
    const unique = [...new Set([...s.architecture.expandedBubbleIds, ...linkedIds])];
    return { architecture: { ...s.architecture, expandedBubbleIds: unique } };
  }),

  collapseBubble: (bubbleId) => set((s) => ({
    architecture: {
      ...s.architecture,
      expandedBubbleIds: s.architecture.expandedBubbleIds.filter(id => id !== bubbleId),
    },
  })),

  focusComponent: (componentId) => set((s) => ({
    architecture: {
      ...s.architecture,
      mode: componentId ? "local" : "overall",
      focusComponentId: componentId,
    },
  })),

  selectBubbleByPid: (pid) => set((s) => ({
    architecture: { ...s.architecture, selectedPid: pid },
  })),

  setArchCamera: (camera) => set((s) => ({
    architecture: { ...s.architecture, camera },
  })),

  setBubblesAndEdges: (bubbles, edges) => set((s) => ({
    architecture: { ...s.architecture, bubbles, edges },
  })),

  // ── Design page actions ──
  ...createTabSlice(set, get),

  toggleCodeFlow: (elementId) => set((s) => {
    const visible = elementId !== null;
    const breadcrumbs = visible
      ? [...s.designPage.breadcrumbs, { type: "code-layer" as const, label: "代码", id: elementId }]
      : s.designPage.breadcrumbs.filter(b => b.type !== "code-layer");
    return {
      designPage: { ...s.designPage, codeFlowVisible: visible, codeFlowElementId: elementId, breadcrumbs },
    };
  }),

  setBreadcrumbs: (breadcrumbs) => set((s) => ({
    designPage: { ...s.designPage, breadcrumbs },
  })),

  // ── Element/wire CRUD ──
  addElement: (el) => set((s) => {
    const normalized = { ...el, props: el.props || {} };
    const elements = [...s.elements, normalized];
    const undoUpdate = buildUndoUpdate(
      s.undoStacks,
      s.designPage.activeTabId,
      { type: "removeElement", id: normalized.id },
    );
    if (normalized.parentId) {
      return {
        ...undoUpdate,
        elements: elements.map(e =>
          e.id === normalized.parentId ? { ...e, children: [...e.children, normalized.id] } : e
        ),
        isDirty: true,
      };
    }
    return { ...undoUpdate, elements, rootOrder: [...s.rootOrder, normalized.id], isDirty: true };
  }),

  removeElement: (id) => set((s) => {
    // Recursively collect all descendant IDs for proper wire cleanup
    const removedIds = new Set<string>();
    function collectDescendants(eid: string): void {
      removedIds.add(eid);
      s.elements.filter(e => e.parentId === eid).forEach(c => collectDescendants(c.id));
    }
    const el = s.elements.find(e => e.id === id);
    if (el) collectDescendants(el.id);

    // Save element snapshot for undo
    const undoPayload = el ? { element: el, rootOrder: s.rootOrder } : null;
    let elements = s.elements.filter(e => !removedIds.has(e.id));
    if (el?.parentId) {
      elements = elements.map(e =>
        e.id === el.parentId ? { ...e, children: e.children.filter(cid => !removedIds.has(cid)) } : e
      );
    }
    const undoAction = undoPayload
      ? { type: "addElement" as const, element: undoPayload.element }
      : null;
    const undoUpdate = undoAction
      ? buildUndoUpdate(s.undoStacks, s.designPage.activeTabId, undoAction)
      : {};
    return {
      ...undoUpdate,
      elements,
      rootOrder: s.rootOrder.filter(rid => !removedIds.has(rid)),
      wires: s.wires.filter(w => !removedIds.has(w.fromId) && !removedIds.has(w.toId)),
      isDirty: true,
    };
  }),

  setProp: (id, prop, value) => set((s) => {
    const elements = s.elements.map(e => e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e);
    const oldEl = s.elements.find(e => e.id === id);
    const oldValue = oldEl?.props[prop];
    const undoUpdate = buildUndoUpdate(
      s.undoStacks,
      s.designPage.activeTabId,
      { type: "setProp", id, prop, oldValue },
    );
    return { ...undoUpdate, elements, isDirty: true };
  }),

  getProp: (id, prop) => get().elements.find(e => e.id === id)?.props[prop],

  selectElement: (id, multi = false) => set((s) => {
    if (id === null) return { selectedElementIds: [] };
    if (multi) {
      const exists = s.selectedElementIds.includes(id);
      return {
        selectedElementIds: exists
          ? s.selectedElementIds.filter(x => x !== id)
          : [...s.selectedElementIds, id],
      };
    }
    return { selectedElementIds: [id] };
  }),

  clearSelection: () => set({ selectedElementIds: [] }),

  addWire: (w) => set((s) => {
    const wires = [...s.wires, w];
    const undoUpdate = buildUndoUpdate(
      s.undoStacks,
      s.designPage.activeTabId,
      { type: "removeWire", id: w.id },
    );
    return { ...undoUpdate, wires, isDirty: true };
  }),

  removeWire: (id) => set((s) => {
    const wire = s.wires.find(w => w.id === id);
    const wires = s.wires.filter(w => w.id !== id);
    if (!wire) return { wires, isDirty: true };
    const undoUpdate = buildUndoUpdate(
      s.undoStacks,
      s.designPage.activeTabId,
      { type: "addWire", wire },
    );
    return { ...undoUpdate, wires, isDirty: true };
  }),

  setElementSource: (id, source) => set((s) => ({
    elements: s.elements.map(e => e.id === id ? { ...e, source } : e),
    isDirty: true,
  })),

  setElementPosition: (id, pos) => set((s) => ({
    elements: s.elements.map(e => e.id === id ? { ...e, position: { ...pos } } : e),
    isDirty: true,
  })),

  setElementsAndWires: (elements, wires) => set({
    elements,
    wires,
    rootOrder: elements.filter(e => !e.parentId).map(e => e.id),
    selectedElementIds: [],
  }),

  moveElement: (elementId, newParentId, targetSiblingId, position) => set((s) => {
    const el = s.elements.find(e => e.id === elementId);
    if (!el) return s;
    const oldParentId = el.parentId;
    let elements = s.elements.map(e => e.id === elementId ? { ...e, parentId: newParentId } : e);
    let rootOrder = s.rootOrder;

    // Remove from old parent
    if (oldParentId) {
      elements = elements.map(e =>
        e.id === oldParentId ? { ...e, children: e.children.filter(cid => cid !== elementId) } : e
      );
    } else {
      rootOrder = rootOrder.filter(id => id !== elementId);
    }

    // Add to new parent
    if (newParentId) {
      const newParent = elements.find(e => e.id === newParentId);
      if (newParent) {
        const newChildren = [...newParent.children];
        if (targetSiblingId && newChildren.includes(targetSiblingId)) {
          const idx = newChildren.indexOf(targetSiblingId);
          newChildren.splice(position === "before" ? idx : idx + 1, 0, elementId);
        } else {
          newChildren.push(elementId);
        }
        elements = elements.map(e => e.id === newParentId ? { ...e, children: newChildren } : e);
      }
    } else {
      // Moving to root level — insert into rootOrder
      if (targetSiblingId && rootOrder.includes(targetSiblingId)) {
        const targetIdx = rootOrder.indexOf(targetSiblingId);
        const insertAt = position === "before" ? targetIdx : targetIdx + 1;
        rootOrder = [...rootOrder];
        rootOrder.splice(insertAt, 0, elementId);
      } else {
        rootOrder = [...rootOrder, elementId];
      }
    }

    const undoUpdate = buildUndoUpdate(
      s.undoStacks,
      s.designPage.activeTabId,
      { type: "moveElement", elementId, oldParentId },
    );
    return { ...undoUpdate, elements, rootOrder, isDirty: true };
  }),

  setElementClassRefs: (elementId, classRefs) => set((s) => ({
    elements: s.elements.map(e => e.id === elementId
      ? { ...e, style: { ...(e.style ?? { classRefs: [], inlineProperties: [] }), classRefs } }
      : e
    ),
    isDirty: true,
  })),

  setElementInlineStyles: (elementId, properties) => set((s) => ({
    elements: s.elements.map(e => e.id === elementId
      ? { ...e, style: { ...(e.style ?? { classRefs: [], inlineProperties: [] }), inlineProperties: properties } }
      : e
    ),
    isDirty: true,
  })),

  reorderElement: (elementId, targetSiblingId, position) => set((s) => {
    const el = s.elements.find(e => e.id === elementId);
    if (!el) return s;
    const parentId = el.parentId;
    const parent = parentId ? s.elements.find(e => e.id === parentId) : undefined;
    if (parentId && !parent) {
      console.warn('[store] reorderElement: parent not found for', elementId);
      return s;
    }
    const siblings = parent ? parent.children : s.rootOrder;
    const idx = siblings.indexOf(elementId);
    if (idx === -1) return s;
    const targetIdx = siblings.indexOf(targetSiblingId);
    if (targetIdx === -1) return s;
    const newSiblings = [...siblings];
    newSiblings.splice(idx, 1);
    const insertAt = position === "before" ? targetIdx : targetIdx + 1;
    newSiblings.splice(insertAt, 0, elementId);
    if (parentId) {
      return {
        elements: s.elements.map(e =>
          e.id === parentId ? { ...e, children: newSiblings } : e
        ),
        isDirty: true,
      };
    }
    return { rootOrder: newSiblings, isDirty: true };
  }),

  // ── Undo/redo ──
  ...createUndoSlice(set, get),

  // ── Component defs ──
  setComponentDef: (name, def) => set((s) => ({
    componentDefs: { ...s.componentDefs, [name]: def },
  })),
  removeComponentDef: (name) => set((s) => {
    const { [name]: _, ...rest } = s.componentDefs;
    return { componentDefs: rest };
  }),
  getComponentDef: (name) => get().componentDefs[name],

  // ── Custom component lifecycle (used by ComponentPalette) ──
  defineComponent: (name, def) => {
    const s = get();
    // Register the component definition so it appears in the palette
    s.setComponentDef(name, def as unknown as ComponentDef);
    // Open the component in a new tab for editing
    // The .omni file is persisted when the user saves (Ctrl+S)
    const filePath = `src/components/${name}.omni`;
    s.openTab(filePath, name, detectTabType(filePath));
  },

  removeComponent: (name) => set((s) => {
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
    const saveEntry = { elements: s.elements, wires: s.wires, rootOrder: s.rootOrder, selectedElementIds: s.selectedElementIds };
    const tabDataWithSave = { ...s.tabData, [tabId]: saveEntry };
    const { [tabId]: _tab, ...restTabData } = tabDataWithSave;
    // LRU eviction: keep at most 20 entries to prevent unbounded growth.
    const TAB_DATA_MAX = 20;
    let prunedTabData = restTabData;
    const keys = Object.keys(prunedTabData);
    if (keys.length > TAB_DATA_MAX) {
      const toRemove = keys.length - TAB_DATA_MAX;
      const openIds = new Set(newTabs.map((t) => t.id));
      const removable = keys.filter((k) => !openIds.has(k)).sort();
      for (let i = 0; i < Math.min(toRemove, removable.length); i++) {
        const { [removable[i]]: _, ...remaining } = prunedTabData;
        prunedTabData = remaining;
      }
    }
    const restoreEntry = activeTabId ? prunedTabData[activeTabId] : undefined;
    return {
      componentDefs: restDefs,
      designPage: { ...s.designPage, openTabs: newTabs, tabOrder: newOrder, activeTabId },
      tabData: prunedTabData,
      elements: restoreEntry?.elements ?? (activeTabId ? [] : s.elements),
      wires: restoreEntry?.wires ?? (activeTabId ? [] : s.wires),
      rootOrder: restoreEntry?.rootOrder ?? (activeTabId ? [] : s.rootOrder),
      selectedElementIds: restoreEntry?.selectedElementIds ?? [],
    };
  }),

  enterComponentCanvas: (name) => {
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

  // ── Project ──
  setProjectDir: (dir) => set({ projectDir: dir }),
  setProjectName: (name) => set({ projectName: name }),
  openFile: (path, content) => set({ activeFilePath: path, activeFileContent: content }),
  setActiveFileContent: (content) => set({ activeFileContent: content }),
  markDirty: () => set({ isDirty: true }),
  markClean: () => set({ isDirty: false }),
  setCompileErrors: (errors) => set({ compileErrors: errors }),
  setPreviewUrl: (url) => set({ previewUrl: url }),

  // ── Mode ──
  switchMode: (mode) => set({ mode }),

  // ── PID index ──
  setPidIndex: (index) => set({ pidIndex: index }),
  updatePidEntry: (pid, entry) => set((s) => ({
    pidIndex: { ...s.pidIndex, [pid]: entry },
  })),

  // ── Node/wire metadata ──
  setNodeMeta: (pid, meta) => set((s) => ({
    nodeMeta: { ...s.nodeMeta, [pid]: meta },
  })),
  setWireMeta: (wireId, meta) => set((s) => ({
    wireMeta: { ...s.wireMeta, [wireId]: meta },
  })),
  removeNodeMeta: (pid) => set((s) => {
    const { [pid]: _, ...rest } = s.nodeMeta;
    return { nodeMeta: rest };
  }),
  removeWireMeta: (wireId) => set((s) => {
    const { [wireId]: _, ...rest } = s.wireMeta;
    return { wireMeta: rest };
  }),
  setCanvasState: (filePath, state) => set((s) => ({
    canvasStates: { ...s.canvasStates, [filePath]: state },
  })),
  setPanelSize: (side, width) => set((s) => ({
    panels: { ...s.panels, [side === "left" ? "leftWidth" : "rightWidth"]: width },
  })),

  // ── Navigation ──
  navigateTo: (page) => set({ currentPage: page }),

  // ── Reset editor state (keep project dir/name) ──
  resetEditorState: () => set({
    currentPage: "design",
    architecture: createInitialArchitectureState(),
    designPage: createInitialDesignPageState(),
    elements: [],
    wires: [],
    rootOrder: [],
    selectedElementIds: [],
    tabData: {},
    componentDefs: {},
    pidIndex: {},
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
    panels: { ...DEFAULT_PANEL_LAYOUT },
    undoStacks: {},
  }),

  // ── New project ──
  newProject: () => set({
    currentPage: "design",
    architecture: createInitialArchitectureState(),
    designPage: createInitialDesignPageState(),
    elements: [],
    wires: [],
    rootOrder: [],
    selectedElementIds: [],
    tabData: {},
    componentDefs: {},
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
    panels: { ...DEFAULT_PANEL_LAYOUT },
    undoStacks: {},
  }),

  // ── Project.json ──
  toProjectJson: (): ProjectJson => {
    const s = get();
    return {
      version: 2,
      nodes: s.nodeMeta,
      wires: s.wireMeta,
      architecture: {
        mode: s.architecture.mode,
        expandedBubbles: [...s.architecture.expandedBubbleIds].sort(),
        focusComponent: s.architecture.focusComponentId,
        camera: s.architecture.camera,
      },
      canvasStates: s.canvasStates,
      panels: s.panels,
    };
  },

  loadFromProjectJson: (json) => set((s) => ({
    nodeMeta: json.nodes || {},
    wireMeta: json.wires || {},
    canvasStates: json.canvasStates || {},
    panels: json.panels || s.panels,
    // componentDefs kept in runtime state, not persisted — loaded from .omni parsing
    architecture: {
      ...s.architecture,
      mode: json.architecture?.mode || "overall",
      expandedBubbleIds: json.architecture?.expandedBubbles || [],
      focusComponentId: json.architecture?.focusComponent || null,
      camera: json.architecture?.camera || { x: 0, y: 0, zoom: 1 },
    },
  })),

  // ── Legacy runtime methods (backward compat; minimal no-ops) ──
  setElementPorts: (id, ports) => set((s) => ({
    elements: s.elements.map(e => e.id === id ? { ...e, ports } : e),
  })),
  setEventBindings: (id, eventName, bindings) => set((s) => ({
    elements: s.elements.map(e =>
      e.id === id
        ? { ...e, config: { ...(e.config || {}), _eventBindings: { ...((e.config?._eventBindings as Record<string, unknown>) || {}), [eventName]: bindings } } }
        : e
    ),
  })),
  removeEventBindings: (id, eventName) => set((s) => ({
    elements: s.elements.map(e =>
      e.id === id && e.config?._eventBindings
        ? { ...e, config: { ...e.config, _eventBindings: Object.fromEntries(Object.entries(e.config._eventBindings as Record<string, unknown>).filter(([k]) => k !== eventName)) } }
        : e
    ),
  })),
  setSourceCode: (content) => set({ sourceCode: content }),
  loadFiles: (files) => set({ files: files as PoiesisState["files"] }),
}));

// ── Window exposure for E2E / Victauri ──
if (typeof window !== "undefined" && typeof (window as unknown as Record<string, unknown>).__ZUSTAND_STORE__ === "undefined") {
  (window as unknown as Record<string, unknown>).__ZUSTAND_STORE__ = usePoiesisStore;
}
