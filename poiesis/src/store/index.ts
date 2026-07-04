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

  // ── Current tab's canvas data ──
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
  selectedElementIds: string[];

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

interface UndoStack {
  undo: Array<{ type: string; payload: unknown }>;
  redo: Array<{ type: string; payload: unknown }>;
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
  currentPage: "architecture",
  architecture: createInitialArchitectureState(),
  designPage: createInitialDesignPageState(),
  elements: [],
  wires: [],
  rootOrder: [],
  selectedElementIds: [],
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
    return {
      designPage: { ...s.designPage, openTabs: newTabs, tabOrder: newOrder, activeTabId },
      elements: activeTabId ? s.elements : [],
      wires: activeTabId ? s.wires : [],
    };
  }),

  switchTab: (tabId) => set((s) => ({
    designPage: { ...s.designPage, activeTabId: tabId },
  })),

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
    if (normalized.parentId) {
      return {
        elements: elements.map(e =>
          e.id === normalized.parentId ? { ...e, children: [...e.children, normalized.id] } : e
        ),
        isDirty: true,
      };
    }
    return { elements, rootOrder: [...s.rootOrder, normalized.id], isDirty: true };
  }),

  removeElement: (id) => set((s) => {
    const el = s.elements.find(e => e.id === id);
    let elements = s.elements.filter(e => e.id !== id);
    if (el?.parentId) {
      elements = elements.map(e =>
        e.id === el.parentId ? { ...e, children: e.children.filter(cid => cid !== id) } : e
      );
    }
    elements = elements.filter(e => e.parentId !== id);
    return {
      elements,
      rootOrder: s.rootOrder.filter(rid => rid !== id),
      wires: s.wires.filter(w => w.fromId !== id && w.toId !== id),
      isDirty: true,
    };
  }),

  setProp: (id, prop, value) => set((s) => ({
    elements: s.elements.map(e => e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e),
    isDirty: true,
  })),

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

  addWire: (w) => set((s) => ({ wires: [...s.wires, w], isDirty: true })),
  removeWire: (id) => set((s) => ({ wires: s.wires.filter(w => w.id !== id), isDirty: true })),

  setElementSource: (id, source) => set((s) => ({
    elements: s.elements.map(e => e.id === id ? { ...e, source } : e),
  })),

  setElementPosition: (id, pos) => set((s) => ({
    elements: s.elements.map(e => e.id === id ? { ...e, position: { ...pos } } : e),
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
    return { elements, isDirty: true };
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
      componentDefs: s.componentDefs,
    };
  },

  loadFromProjectJson: (json) => set((s) => ({
    nodeMeta: json.nodes || {},
    wireMeta: json.wires || {},
    canvasStates: json.canvasStates || {},
    panels: json.panels || s.panels,
    componentDefs: json.componentDefs || {},
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
