/** 编辑器核心 Zustand store — v2: design/code 分叉 + 项目管理。*/

import { create } from "zustand";
import type { StyleEntry } from "../constants/style-properties";

// ── Data Types (R2 upgraded schema) ─────────────────

export interface PortData {
  name: string;
  direction: "in" | "out";
  portType?: "flow" | "data";
  type: string;
  label?: string;
  configurable?: boolean;
  defaultValue?: unknown;
}

export interface ElementData {
  id: string;
  nodeType: "view" | "codeblock" | "group";
  tag: string;
  props: Record<string, unknown>;
  styles?: StyleEntry[];
  children: string[];
  parentId: string | null;
  ports?: PortData[];
  renderMode?: "dom" | "canvas";
  isFolded?: boolean;
  typeAnnotation?: string;
  storeName?: string;
  source?: string;
  outputValue?: unknown;
  position?: { x: number; y: number };
  nodeCategory?: "flow" | "event" | "action" | "data" | "variable" | "code";
  config?: Record<string, unknown>;
  flowOrder?: number;
  componentRef?: string;
}

export interface WireData {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  isCollapsed?: boolean;
  collapseGroupId?: string;
}

export interface ProjectConfig {
  target: string;       // "react" | "vue" | "flutter" | "omniview"
  autoSave: boolean;
}

// ── Store ──────────────────────────────────────────

interface EditorState {
  projectDir: string | null;
  previewUrl: string | null;
  files: { name: string; path: string; type: "file" | "directory" }[];
  activeFilePath: string | null;
  activeFileContent: string | null;
  compileErrors: { code: string; message: string; line: number }[];

  // ── Design canvas (drag-drop UI) ──
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];  // display order of root elements (parentId === null)

  // ── Active mode ──
  activeMode: "design" | "code";

  selectedElementIds: string[];

  // ── Project management ──
  projectPath: string | null;       // .omni.json file path
  projectName: string;              // "未命名项目" or from filename
  sourceCode: string;               // embedded .omni source
  isDirty: boolean;                 // unsaved changes flag
  config: ProjectConfig;            // { target, autoSave }

  // Actions
  setProjectDir: (dir: string) => void;
  loadFiles: (entries: { name: string; path: string; type: "file" | "directory" }[]) => void;
  openFile: (path: string, content: string) => void;
  setActiveFileContent: (content: string) => void;
  setCompileErrors: (errors: { code: string; message: string; line: number }[]) => void;
  setPreviewUrl: (url: string | null) => void;

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
  setBlockValue: (id: string, value: unknown) => void;
  setEventBindings: (id: string, eventName: string, binding: { fnName: string; paramBindings: Record<string, { sourceType: string; value: string; expectedType: string }> }) => void;
  removeEventBindings: (id: string, eventName: string) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;

  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;

  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;

  // ── Style management ──
  addStyleEntry: (elementId: string, entry: StyleEntry) => void;
  removeStyleEntry: (elementId: string, entryId: string) => void;
  updateStyleEntry: (elementId: string, entryId: string, patch: Partial<StyleEntry>) => void;
  reorderStyleEntries: (elementId: string, entryIds: string[]) => void;
  getStyles: (elementId: string) => StyleEntry[];

  // ── Active mode switching ──
  setActiveMode: (mode: "design" | "code") => void;

  // ── Project management ──
  /** Create a new empty project. */
  newProject: () => void;
  /** Load an existing .omni.json project. */
  openProject: (path: string, document: Record<string, unknown>) => void;
  /** Mark project as dirty (unsaved changes). */
  markDirty: () => void;
  /** Mark project as clean (saved). */
  markClean: () => void;
  setConfig: (patch: Partial<ProjectConfig>) => void;
  setSourceCode: (code: string) => void;

  // ── Multi-Canvas Support (design mode tabs) ──
  activeDocId: string | null;
  docBackup: { elements: ElementData[]; wires: WireData[] } | null;
  enterComponentCanvas: (componentName: string) => void;
  exitComponentCanvas: () => void;

  // ── Tab System ──
  openTabs: string[];
  tabStates: Record<string, { elements: ElementData[]; wires: WireData[] }>;
  switchTab: (docId: string | null) => void;
  closeTab: (docId: string) => void;
  /** Open a .omni.json file as project (replaces current project). */
  openFileTab: (path: string, name: string, elements: ElementData[], wires: WireData[], customComponents?: unknown[]) => void;
}

export const useEditorStore = create<EditorState>((set, get) => ({
  projectDir: null,
  previewUrl: null,
  files: [],
  activeFilePath: null,
  activeFileContent: null,
  compileErrors: [],
  elements: [],
  wires: [],
  rootOrder: [],
  activeMode: "design",
  selectedElementIds: [],

  // ── Project management ──
  projectPath: null,
  projectName: "未命名项目",
  sourceCode: "",
  isDirty: false,
  config: { target: "react", autoSave: false },

  setProjectDir: (dir) => set({ projectDir: dir }),

  loadFiles: (entries) => set({ files: entries }),

  openFile: (path, content) =>
    set({ activeFilePath: path, activeFileContent: content }),

  setActiveFileContent: (content) => set({ activeFileContent: content }),

  setCompileErrors: (errors) => set({ compileErrors: errors }),

  setPreviewUrl: (url) => set({ previewUrl: url }),

  addElement: (el) =>
    set((s) => {
      const normalized = { ...el, props: el.props || {} };
      const elements = [...s.elements, normalized];
      if (normalized.parentId) {
        return {
          elements: elements.map((e) =>
            e.id === normalized.parentId ? { ...e, children: [...e.children, normalized.id] } : e
          ),
          isDirty: true,
        };
      }
      return { elements, rootOrder: [...s.rootOrder, normalized.id], isDirty: true };
    }),

  removeElement: (id) =>
    set((s) => ({ elements: (() => {
      const el = s.elements.find((e) => e.id === id);
      let elements = s.elements.filter((e) => e.id !== id);
      if (el?.parentId) {
        elements = elements.map((e) =>
          e.id === el.parentId
            ? { ...e, children: e.children.filter((cid) => cid !== id) }
            : e
        );
      }
      elements = elements.filter((e) => e.parentId !== id);
      return elements;
    })(), rootOrder: s.rootOrder.filter((rid) => rid !== id), isDirty: true })),

  setProp: (id, prop, value) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e
      ),
      isDirty: true,
    })),

  getProp: (id, prop) => get().elements.find((e) => e.id === id)?.props[prop],

  selectElement: (id, multi = false) =>
    set((s) => {
      if (id === null) return { selectedElementIds: [] };
      if (multi) {
        const exists = s.selectedElementIds.includes(id);
        return {
          selectedElementIds: exists
            ? s.selectedElementIds.filter((x) => x !== id)
            : [...s.selectedElementIds, id],
        };
      }
      return { selectedElementIds: [id] };
    }),

  selectAll: () =>
    set((s) => ({
      selectedElementIds: s.elements.map((e) => e.id),
    })),

  clearSelection: () => set({ selectedElementIds: [] }),

  addWire: (w) => set((s) => ({ wires: [...s.wires, w], isDirty: true })),

  removeWire: (id) =>
    set((s) => ({ wires: s.wires.filter((w) => w.id !== id), isDirty: true })),

  setElementSource: (id, source) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, source } : e
      ),
    })),

  setElementPorts: (id, ports) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, ports } : e
      ),
    })),

  setEventBindings: (id, eventName, binding) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        return {
          ...e,
          config: { ...e.config, _eventBindings: { ...currentBindings, [eventName]: binding } },
        };
      }),
    })),

  removeEventBindings: (id, eventName) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        const updated = { ...currentBindings };
        delete updated[eventName];
        return { ...e, config: { ...e.config, _eventBindings: updated } };
      }),
    })),

  setBlockValue: (id, value) =>
    set((s) => {
      const existing = s.elements.find(e => e.id === id);
      if (existing && JSON.stringify(existing.outputValue) === JSON.stringify(value)) return s;
      return {
        elements: s.elements.map((e) =>
          e.id === id ? { ...e, outputValue: value } : e
        ),
      };
    }),

  setElementPosition: (id, pos) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, position: { ...pos } } : e
      ),
    })),

  setElementsAndWires: (elements, wires) =>
    set({ elements, wires, selectedElementIds: [] }),

  reorderElement: (elementId, targetSiblingId, position) =>
    set((s) => {
      const el = s.elements.find((e) => e.id === elementId);
      const target = s.elements.find((e) => e.id === targetSiblingId);
      if (!el || !target) return s;

      // Root-level reorder: use rootOrder
      if (el.parentId === null || target.parentId === null) {
        if (el.parentId !== target.parentId) return s; // must be same level
        const order = [...s.rootOrder];
        const fromIdx = order.indexOf(elementId);
        const toIdx = order.indexOf(targetSiblingId);
        if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return s;
        order.splice(fromIdx, 1);
        const newToIdx = order.indexOf(targetSiblingId);
        const insertIdx = position === "before" ? newToIdx : newToIdx + 1;
        order.splice(insertIdx, 0, elementId);
        return { rootOrder: order, isDirty: true };
      }

      // Container-level reorder: use parent.children
      if (el.parentId !== target.parentId) return s;
      const parentId = el.parentId;
      const parent = s.elements.find((e) => e.id === parentId);
      if (!parent) return s;
      const children = [...parent.children];
      const fromIdx = children.indexOf(elementId);
      const toIdx = children.indexOf(targetSiblingId);
      if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return s;
      children.splice(fromIdx, 1);
      const insertIdx = position === "before" ? children.indexOf(targetSiblingId) : children.indexOf(targetSiblingId) + 1;
      children.splice(insertIdx, 0, elementId);
      return {
        elements: s.elements.map((e) =>
          e.id === parentId ? { ...e, children } : e
        ),
        isDirty: true,
      };
    }),

  moveElement: (elementId, newParentId, targetSiblingId, position) =>
    set((s) => {
      const el = s.elements.find((e) => e.id === elementId);
      if (!el) return s;
      const oldParentId = el.parentId;

      let elements = s.elements.map((e) =>
        e.id === elementId ? { ...e, parentId: newParentId } : e
      );

      if (oldParentId) {
        elements = elements.map((e) =>
          e.id === oldParentId
            ? { ...e, children: e.children.filter((cid) => cid !== elementId) }
            : e
        );
      }

      if (newParentId) {
        const newParent = elements.find((e) => e.id === newParentId);
        if (!newParent) return s;
        const newChildren = [...newParent.children];
        if (targetSiblingId && newChildren.includes(targetSiblingId)) {
          const targetIdx = newChildren.indexOf(targetSiblingId);
          const insertIdx = position === "before" ? targetIdx : targetIdx + 1;
          newChildren.splice(insertIdx, 0, elementId);
        } else {
          newChildren.push(elementId);
        }
        elements = elements.map((e) =>
          e.id === newParentId ? { ...e, children: newChildren } : e
        );
      }

      return { elements, isDirty: true };
    }),

  // ── Style management ──
  addStyleEntry: (elementId, entry) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? { ...e, styles: [...(e.styles || []), entry] }
          : e
      ),
      isDirty: true,
    })),

  removeStyleEntry: (elementId, entryId) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? { ...e, styles: (e.styles || []).filter((s) => s.id !== entryId) }
          : e
      ),
      isDirty: true,
    })),

  updateStyleEntry: (elementId, entryId, patch) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === elementId
          ? { ...e, styles: (e.styles || []).map((s) =>
              s.id === entryId ? { ...s, ...patch } : s
            )}
          : e
      ),
      isDirty: true,
    })),

  reorderStyleEntries: (elementId, entryIds) =>
    set((s) => {
      const el = s.elements.find((e) => e.id === elementId);
      if (!el?.styles) return { elements: s.elements };
      const byId = new Map(el.styles.map((s) => [s.id, s]));
      // Validate all IDs exist before reordering
      if (!entryIds.every((id) => byId.has(id))) return { elements: s.elements };
      const reordered = entryIds.map((id) => byId.get(id)!);
      return {
        elements: s.elements.map((e) =>
          e.id === elementId ? { ...e, styles: reordered } : e
        ),
        isDirty: true,
      };
    }),

  getStyles: (elementId) => {
    return get().elements.find((e) => e.id === elementId)?.styles || [];
  },

  // ── Active mode switching ──
  setActiveMode: (mode) => set({ activeMode: mode }),

  // ── Project management ──
  newProject: () =>
    set({
      projectPath: null,
      projectName: "未命名项目",
      sourceCode: "",
      isDirty: false,
      elements: [],
      wires: [],
      rootOrder: [],
      activeDocId: null,
      docBackup: null,
      openTabs: [],
      tabStates: {},
      selectedElementIds: [],
    }),

  openProject: (path, document) => {
    const doc = document as Record<string, unknown>;
    const meta = (doc.meta || doc.meta || {}) as Record<string, unknown>;
    const design = (doc.design || {}) as Record<string, unknown>;
    const code = (doc.code || {}) as Record<string, unknown>;
    const tabState = (doc.tabState || {}) as Record<string, unknown>;
    const config = (doc.config || {}) as Record<string, unknown>;
    const canvasLayout = (doc.canvasLayout || {}) as Record<string, unknown>;
    const compRefs = (doc.components || []) as Array<Record<string, unknown>>;

    const designElements = (design.elements || []) as ElementData[];
    const designWires = (design.wires || []) as WireData[];

    // Restore component definitions
    if (compRefs.length > 0) {
      import("./componentRegistry").then((m) => {
        const defs: Record<string, unknown> = {};
        for (const cr of compRefs) {
          const name = cr.name as string;
          const source = cr.source as string;
          if (source === "embedded" && cr.definition) {
            defs[name] = { ...cr.definition, source };
          } else if (source !== "embedded") {
            // External reference — mark for lazy loading
            defs[name] = { name, source, _external: true };
          }
        }
        if (Object.keys(defs).length > 0) {
          m.useComponentRegistry.setState((s: any) => ({
            definitions: { ...s.definitions, ...defs },
          }));
        }
      }).catch(() => {});
    }

    // Parse project name from path
    const projectName = (meta.projectName as string) ||
      path.replace(/^.*[\\/]/, "").replace(".omni.json", "") ||
      "未命名项目";

    const rootOrder = designElements
      .filter((e) => !e.parentId)
      .map((e) => e.id);

    return set({
      projectPath: path,
      projectName,
      sourceCode: (doc.sourceCode as string) || "",
      isDirty: false,
      elements: designElements,
      wires: designWires,
      rootOrder,
      config: {
        target: (config.target as string) || "react",
        autoSave: (config.autoSave as boolean) || false,
      },
      activeDocId: null,
      docBackup: null,
      openTabs: (tabState.openTabs as string[]) || ["main"],
      tabStates: {},
      selectedElementIds: [],
      // Switch to the active tab from saved state
    });
  },

  markDirty: () => set({ isDirty: true }),
  markClean: () => set({ isDirty: false }),

  setConfig: (patch) =>
    set((s) => ({ config: { ...s.config, ...patch }, isDirty: true })),

  setSourceCode: (code) =>
    set({ sourceCode: code, isDirty: true }),

  // ── Multi-Canvas Support ──
  activeDocId: null,
  docBackup: null,

  enterComponentCanvas: (componentName) =>
    set((s) => {
      const backup = { elements: s.elements, wires: s.wires };
      const compDef = useComponentRegistry.getState().definitions[componentName];
      if (!compDef) return s;
      const newTabStates = { ...s.tabStates };
      if (s.activeDocId && s.activeDocId.startsWith("file:")) {
        newTabStates[s.activeDocId] = { elements: s.elements, wires: s.wires };
      }
      import("./undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});
      const tabKey = `comp:${componentName}`;
      const newOpenTabs = s.openTabs.includes(tabKey) ? s.openTabs : [...s.openTabs, tabKey];
      return {
        docBackup: backup,
        elements: compDef.elements,
        wires: compDef.wires,
        activeDocId: tabKey,
        openTabs: newOpenTabs,
        tabStates: newTabStates,
        selectedElementIds: [],
      };
    }),

  exitComponentCanvas: () =>
    set((s) => {
      if (!s.activeDocId || !s.activeDocId.startsWith("comp:")) return s;
      const compName = s.activeDocId.replace("comp:", "");
      useComponentRegistry.getState().saveCanvas(compName, s.elements, s.wires);
      import("./undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});
      return {
        elements: s.docBackup?.elements || [],
        wires: s.docBackup?.wires || [],
        docBackup: null,
        activeDocId: null,
        selectedElementIds: [],
      };
    }),

  // ── Tab System ──
  openTabs: [],
  tabStates: {},

  switchTab: (docId) =>
    set((s) => {
      if (s.activeDocId === docId) return s;
      const newTabStates = { ...s.tabStates };
      const currentKey = s.activeDocId;
      if (currentKey && currentKey.startsWith("file:")) {
        newTabStates[currentKey] = { elements: s.elements, wires: s.wires };
      } else if (currentKey && currentKey.startsWith("comp:")) {
        const compName = currentKey.replace("comp:", "");
        useComponentRegistry.getState().saveCanvas(compName, s.elements, s.wires);
      }

      let newElements: ElementData[];
      let newWires: WireData[];
      let newDocBackup = s.docBackup;

      if (docId === null) {
        newElements = s.docBackup?.elements || [];
        newWires = s.docBackup?.wires || [];
        newDocBackup = null;
      } else if (docId.startsWith("comp:")) {
        const compName = docId.replace("comp:", "");
        const compDef = useComponentRegistry.getState().definitions[compName];
        if (!compDef) return s;
        newDocBackup = s.activeDocId === null
          ? { elements: s.elements, wires: s.wires }
          : s.docBackup;
        newElements = compDef.elements;
        newWires = compDef.wires;
      } else if (docId.startsWith("file:")) {
        const cached = newTabStates[docId];
        if (cached) {
          newElements = cached.elements;
          newWires = cached.wires;
        } else {
          return s;
        }
        if (s.activeDocId === null) {
          newDocBackup = { elements: s.elements, wires: s.wires };
        }
      } else {
        return s;
      }

      import("./undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});

      return {
        elements: newElements,
        wires: newWires,
        activeDocId: docId,
        docBackup: newDocBackup,
        tabStates: newTabStates,
        selectedElementIds: [],
      };
    }),

  closeTab: (docId) =>
    set((s) => {
      const newOpenTabs = s.openTabs.filter((t) => t !== docId);
      const newTabStates = { ...s.tabStates };
      delete newTabStates[docId];

      if (s.activeDocId === docId) {
        if (docId.startsWith("comp:")) {
          const compName = docId.replace("comp:", "");
          useComponentRegistry.getState().saveCanvas(compName, s.elements, s.wires);
        } else if (docId.startsWith("file:")) {
          newTabStates[docId] = { elements: s.elements, wires: s.wires };
        }
        import("./undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});
        return {
          elements: s.docBackup?.elements || [],
          wires: s.docBackup?.wires || [],
          docBackup: null,
          activeDocId: null,
          openTabs: newOpenTabs,
          tabStates: newTabStates,
          selectedElementIds: [],
        };
      }

      return { openTabs: newOpenTabs, tabStates: newTabStates };
    }),

  /** @deprecated Use openProject() for .omni.json files instead. */
  openFileTab: (path, name, elements, wires, customComponents) =>
    set((s) => {
      const tabKey = `file:${path}`;
      if (s.openTabs.includes(tabKey)) return s;
      const newTabStates = {
        ...s.tabStates,
        [tabKey]: { elements, wires },
      };
      if (customComponents && Array.isArray(customComponents) && customComponents.length > 0) {
        try {
          const defs: Record<string, unknown> = {};
          for (const cc of customComponents) {
            if ((cc as Record<string, unknown>).name) {
              defs[(cc as Record<string, unknown>).name as string] = cc;
            }
          }
          if (Object.keys(defs).length > 0) {
            import("./componentRegistry").then((m) => {
              m.useComponentRegistry.setState({ definitions: defs as any });
            }).catch(() => {});
          }
        } catch { /* non-critical */ }
      }
      return {
        openTabs: [...s.openTabs, tabKey],
        tabStates: newTabStates,
      };
    }),
}));

import { useComponentRegistry } from "./componentRegistry";

// Expose store for E2E tests (Playwright page.evaluate)
if (typeof window !== "undefined") {
  (window as any).__ZUSTAND_EDITOR_STORE__ = useEditorStore;
}
