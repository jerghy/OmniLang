import { create } from "zustand";
import type { ElementData } from "../core/types/element";
import type { WireData } from "../core/types/wire";
import type { ArchitectureState, BubbleNode, BubbleEdge } from "../core/types/architecture";
import type { DesignPageState, DesignTab, Breadcrumb } from "../core/types/design-page";
import type { PidEntry } from "../core/types/pid";
import type { ComponentDef, ProjectJson, NodeMeta, WireMeta } from "../core/types/project-json";
import type { StyleProperty } from "../core/types/style";

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

  // ── PID index ──
  pidIndex: Record<string, PidEntry>;

  // ── Project ──
  projectDir: string | null;
  projectName: string;
  isDirty: boolean;
  activeFilePath: string | null;
  activeFileContent: string | null;

  // ── Compilation ──
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;

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
  openTab: (filePath: string, componentName: string) => void;
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

  // Project
  setProjectDir: (dir: string) => void;
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
}

type UndoAction =
  | { type: "addElement"; element: ElementData }
  | { type: "removeElement"; id: string }
  | { type: "setProp"; id: string; prop: string; oldValue: unknown }
  | { type: "addWire"; wire: WireData }
  | { type: "removeWire"; id: string }
  | { type: "moveElement"; elementId: string; oldParentId: string | null };

interface UndoStack {
  undo: UndoAction[];
  redo: UndoAction[];
}

/** Skip undo recording during undo/redo operations to prevent stack contamination. */
let __skipUndo = false;

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
  currentPage: "architecture",
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
  projectName: "未命名项目",
  isDirty: false,
  activeFilePath: null,
  activeFileContent: null,
  compileErrors: [],
  previewUrl: null,
  nodeMeta: {},
  wireMeta: {},
  canvasStates: {},
  panels: { leftWidth: 240, rightWidth: 300 },
  undoStacks: {},

  // ── Architecture actions ──
  setArchMode: (mode) => set((s) => ({ architecture: { ...s.architecture, mode } })),

  expandBubble: (bubbleId) => set((s) => {
    const linkedIds = s.architecture.bubbles
      .filter(b => b.id === bubbleId)
      .flatMap(b => {
        // Find linked bubbles (same depth, sibling types)
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
  openTab: (filePath, componentName) => set((s) => {
    const existing = s.designPage.openTabs.find(t => t.filePath === filePath);
    if (existing) {
      return {
        currentPage: "design",
        designPage: { ...s.designPage, activeTabId: existing.id },
      };
    }
    const id = `tab_${Date.now()}`;
    const newTab: DesignTab = {
      id, filePath, componentName,
      isDirty: false,
      pidMap: { entries: {}, lineToPid: {} },
    };
    return {
      currentPage: "design",
      designPage: {
        ...s.designPage,
        openTabs: [...s.designPage.openTabs, newTab],
        tabOrder: [...s.designPage.tabOrder, id],
        activeTabId: id,
        breadcrumbs: [{ type: "architecture", label: "架构", id: "arch" }, { type: "component", label: componentName, id, filePath }],
      },
    };
  }),

  closeTab: (tabId) => set((s) => {
    const idx = s.designPage.openTabs.findIndex(t => t.id === tabId);
    if (idx === -1) return s;
    const newTabs = s.designPage.openTabs.filter(t => t.id !== tabId);
    const newOrder = s.designPage.tabOrder.filter(id => id !== tabId);
    let activeTabId = s.designPage.activeTabId;
    if (activeTabId === tabId) {
      const neighborIdx = Math.min(idx, newTabs.length - 1);
      activeTabId = newTabs[neighborIdx]?.id ?? null;
    }
    // Save current canvas state for the tab being closed BEFORE removing its data
    const saveEntry = { elements: s.elements, wires: s.wires, rootOrder: s.rootOrder, selectedElementIds: s.selectedElementIds };
    const tabDataWithSave = { ...s.tabData, [tabId]: saveEntry };
    // Clean up the closed tab's data
    const { [tabId]: _, ...restTabData } = tabDataWithSave;
    // If a neighbor tab becomes active, restore its data
    const restoreEntry = activeTabId ? restTabData[activeTabId] : undefined;
    return {
      designPage: { ...s.designPage, openTabs: newTabs, tabOrder: newOrder, activeTabId },
      tabData: restTabData,
      elements: restoreEntry?.elements ?? (activeTabId ? [] : s.elements),
      wires: restoreEntry?.wires ?? (activeTabId ? [] : s.wires),
      rootOrder: restoreEntry?.rootOrder ?? (activeTabId ? [] : s.rootOrder),
      selectedElementIds: restoreEntry?.selectedElementIds ?? [],
    };
  }),

  switchTab: (tabId) => set((s) => {
    const oldTabId = s.designPage.activeTabId;
    if (oldTabId === tabId) return s;
    // Save current tab's data into tabData
    const saveEntry = { elements: s.elements, wires: s.wires, rootOrder: s.rootOrder, selectedElementIds: s.selectedElementIds };
    const savedTabData = oldTabId
      ? { [oldTabId]: saveEntry }
      : {};
    // Restore target tab's data (or empty if not yet loaded)
    const restoreEntry = s.tabData[tabId];
    return {
      designPage: { ...s.designPage, activeTabId: tabId },
      tabData: { ...s.tabData, ...savedTabData },
      elements: restoreEntry?.elements ?? [],
      wires: restoreEntry?.wires ?? [],
      rootOrder: restoreEntry?.rootOrder ?? [],
      selectedElementIds: restoreEntry?.selectedElementIds ?? [],
    };
  }),

  reorderTabs: (tabOrder) => set((s) => ({
    designPage: { ...s.designPage, tabOrder },
  })),

  markTabDirty: (tabId, dirty) => set((s) => ({
    designPage: {
      ...s.designPage,
      openTabs: s.designPage.openTabs.map(t => t.id === tabId ? { ...t, isDirty: dirty } : t),
    },
    isDirty: dirty,
  })),

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

    // Skip undo recording during undo/redo operations
    if (__skipUndo) {
      if (normalized.parentId) {
        return {
          elements: elements.map(e =>
            e.id === normalized.parentId ? { ...e, children: [...e.children, normalized.id] } : e
          ),
          isDirty: true,
        };
      }
      return { elements, rootOrder: [...s.rootOrder, normalized.id], isDirty: true };
    }

    const tabId = s.designPage.activeTabId;
    // Push inverse for undo
    const undoAction = { type: "removeElement" as const, id: normalized.id };
    const existingStack = tabId ? s.undoStacks[tabId] : undefined;
    const undoStack = tabId
      ? { undo: [...(existingStack?.undo ?? []), undoAction], redo: [] }
      : undefined;
    const undoUpdate = undoStack && tabId
      ? { undoStacks: { ...s.undoStacks, [tabId]: undoStack } }
      : {};
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
    const el = s.elements.find(e => e.id === id);
    // Save element snapshot for undo
    const undoPayload = el ? { element: el, rootOrder: s.rootOrder } : null;
    let elements = s.elements.filter(e => e.id !== id);
    if (el?.parentId) {
      elements = elements.map(e =>
        e.id === el.parentId ? { ...e, children: e.children.filter(cid => cid !== id) } : e
      );
    }
    elements = elements.filter(e => e.parentId !== id);

    // Skip undo recording during undo/redo operations
    if (__skipUndo) {
      return {
        elements,
        rootOrder: s.rootOrder.filter(rid => rid !== id),
        wires: s.wires.filter(w => w.fromId !== id && w.toId !== id),
        isDirty: true,
      };
    }

    const tabId = s.designPage.activeTabId;
    // Push inverse for undo
    const undoAction = undoPayload ? { type: "addElement" as const, element: undoPayload.element } : null;
    const existingStack = tabId ? s.undoStacks[tabId] : undefined;
    const undoStack = tabId && undoAction
      ? { undo: [...(existingStack?.undo ?? []), undoAction], redo: [] }
      : undefined;
    const undoUpdate = undoStack && tabId
      ? { undoStacks: { ...s.undoStacks, [tabId]: undoStack } }
      : {};
    return {
      ...undoUpdate,
      elements,
      rootOrder: s.rootOrder.filter(rid => rid !== id),
      wires: s.wires.filter(w => w.fromId !== id && w.toId !== id),
      isDirty: true,
    };
  }),

  setProp: (id, prop, value) => set((s) => {
    const elements = s.elements.map(e => e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e);

    if (__skipUndo) {
      return { elements, isDirty: true };
    }

    const oldEl = s.elements.find(e => e.id === id);
    const oldValue = oldEl?.props[prop];

    const tabId = s.designPage.activeTabId;
    const existingStack = tabId ? s.undoStacks[tabId] : undefined;
    const undoAction = { type: "setProp" as const, id, prop, oldValue };
    const undoStack = tabId
      ? { undo: [...(existingStack?.undo ?? []), undoAction], redo: [] }
      : undefined;
    const undoUpdate = undoStack && tabId
      ? { undoStacks: { ...s.undoStacks, [tabId]: undoStack } }
      : {};

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

    if (__skipUndo) {
      return { wires, isDirty: true };
    }

    const tabId = s.designPage.activeTabId;
    const existingStack = tabId ? s.undoStacks[tabId] : undefined;
    const undoAction = { type: "removeWire" as const, id: w.id };
    const undoStack = tabId
      ? { undo: [...(existingStack?.undo ?? []), undoAction], redo: [] }
      : undefined;
    const undoUpdate = undoStack && tabId
      ? { undoStacks: { ...s.undoStacks, [tabId]: undoStack } }
      : {};

    return { ...undoUpdate, wires, isDirty: true };
  }),
  removeWire: (id) => set((s) => {
    const wire = s.wires.find(w => w.id === id);
    const wires = s.wires.filter(w => w.id !== id);

    if (__skipUndo || !wire) {
      return { wires, isDirty: true };
    }

    const tabId = s.designPage.activeTabId;
    const existingStack = tabId ? s.undoStacks[tabId] : undefined;
    const undoAction = { type: "addWire" as const, wire };
    const undoStack = tabId
      ? { undo: [...(existingStack?.undo ?? []), undoAction], redo: [] }
      : undefined;
    const undoUpdate = undoStack && tabId
      ? { undoStacks: { ...s.undoStacks, [tabId]: undoStack } }
      : {};

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
    if (oldParentId) {
      elements = elements.map(e =>
        e.id === oldParentId ? { ...e, children: e.children.filter(cid => cid !== elementId) } : e
      );
    }
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
    }

    if (__skipUndo) {
      return { elements, isDirty: true };
    }

    const tabId = s.designPage.activeTabId;
    const existingStack = tabId ? s.undoStacks[tabId] : undefined;
    const undoAction = { type: "moveElement" as const, elementId, oldParentId };
    const undoStack = tabId
      ? { undo: [...(existingStack?.undo ?? []), undoAction], redo: [] }
      : undefined;
    const undoUpdate = undoStack && tabId
      ? { undoStacks: { ...s.undoStacks, [tabId]: undoStack } }
      : {};

    return { ...undoUpdate, elements, isDirty: true };
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

  // Undo/redo: execute the inverse of the last action
  undo: () => {
    const s = get();
    const tabId = s.designPage.activeTabId;
    if (!tabId) return;
    const stack = s.undoStacks[tabId];
    if (!stack || stack.undo.length === 0) return;
    const action = stack.undo[stack.undo.length - 1];
    // Execute inverse (skip undo recording to avoid contamination)
    __skipUndo = true;
    try {
      switch (action.type) {
        case "removeElement":
          s.removeElement(action.id);
          break;
        case "addElement":
          s.addElement(action.element);
          break;
        case "setProp":
          s.setProp(action.id, action.prop, action.oldValue);
          break;
        case "addWire":
          s.addWire(action.wire);
          break;
        case "removeWire":
          s.removeWire(action.id);
          break;
        case "moveElement":
          set((st) => {
            const element = st.elements.find(e => e.id === action.elementId);
            if (!element) return st;
            const currentParent = element.parentId;
            let elements = st.elements.map(e =>
              e.id === action.elementId ? { ...e, parentId: action.oldParentId } : e
            );
            if (currentParent) {
              elements = elements.map(e =>
                e.id === currentParent ? { ...e, children: e.children.filter(c => c !== action.elementId) } : e
              );
            }
            let rootOrder = st.rootOrder.filter(rid => rid !== action.elementId);
            if (action.oldParentId) {
              elements = elements.map(e =>
                e.id === action.oldParentId ? { ...e, children: [...e.children, action.elementId] } : e
              );
            } else {
              rootOrder = [...rootOrder, action.elementId];
            }
            return { elements, rootOrder };
          });
          break;
      }
    } finally {
      __skipUndo = false;
    }
    // Update stacks (after the inverse execution modified the store)
    const s2 = get();
    set({
      undoStacks: {
        ...s2.undoStacks,
        [tabId]: {
          undo: stack.undo.slice(0, -1),
          redo: [...stack.redo, action],
        },
      },
    });
  },

  redo: () => {
    const s = get();
    const tabId = s.designPage.activeTabId;
    if (!tabId) return;
    const stack = s.undoStacks[tabId];
    if (!stack || stack.redo.length === 0) return;
    const action = stack.redo[stack.redo.length - 1];
    // Re-execute the action (skip undo recording)
    __skipUndo = true;
    try {
      switch (action.type) {
        case "removeElement":
          s.removeElement(action.id);
          break;
        case "addElement":
          s.addElement(action.element);
          break;
        case "setProp":
          s.setProp(action.id, action.prop, action.oldValue);
          break;
        case "addWire":
          s.addWire(action.wire);
          break;
        case "removeWire":
          s.removeWire(action.id);
          break;
        case "moveElement":
          set((st) => {
            const element = st.elements.find(e => e.id === action.elementId);
            if (!element) return st;
            const currentParent = element.parentId;
            let elements = st.elements.map(e =>
              e.id === action.elementId ? { ...e, parentId: action.oldParentId } : e
            );
            if (currentParent) {
              elements = elements.map(e =>
                e.id === currentParent ? { ...e, children: e.children.filter(c => c !== action.elementId) } : e
              );
            }
            let rootOrder = st.rootOrder.filter(rid => rid !== action.elementId);
            if (action.oldParentId) {
              elements = elements.map(e =>
                e.id === action.oldParentId ? { ...e, children: [...e.children, action.elementId] } : e
              );
            } else {
              rootOrder = [...rootOrder, action.elementId];
            }
            return { elements, rootOrder };
          });
          break;
      }
    } finally {
      __skipUndo = false;
    }
    // Update stacks
    const s2 = get();
    set({
      undoStacks: {
        ...s2.undoStacks,
        [tabId]: {
          undo: [...stack.undo, action],
          redo: stack.redo.slice(0, -1),
        },
      },
    });
  },

  // ── Component defs ──
  setComponentDef: (name, def) => set((s) => ({
    componentDefs: { ...s.componentDefs, [name]: def },
  })),
  removeComponentDef: (name) => set((s) => {
    const { [name]: _, ...rest } = s.componentDefs;
    return { componentDefs: rest };
  }),
  getComponentDef: (name) => get().componentDefs[name],

  // ── Project ──
  setProjectDir: (dir) => set({ projectDir: dir }),
  openFile: (path, content) => set({ activeFilePath: path, activeFileContent: content }),
  setActiveFileContent: (content) => set({ activeFileContent: content }),
  markDirty: () => set({ isDirty: true }),
  markClean: () => set({ isDirty: false }),
  setCompileErrors: (errors) => set({ compileErrors: errors }),
  setPreviewUrl: (url) => set({ previewUrl: url }),

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

  // ── New project ──
  newProject: () => set({
    currentPage: "architecture",
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
    projectName: "未命名项目",
    isDirty: false,
    activeFilePath: null,
    activeFileContent: null,
    compileErrors: [],
    previewUrl: null,
    nodeMeta: {},
    wireMeta: {},
    canvasStates: {},
    panels: { leftWidth: 240, rightWidth: 300 },
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
}));

// ── Window exposure for E2E / Victauri ──
if (typeof window !== "undefined") {
  (window as unknown as Record<string, unknown>).__ZUSTAND_STORE__ = usePoiesisStore;
}
