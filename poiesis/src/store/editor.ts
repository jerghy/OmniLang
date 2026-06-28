/** 编辑器核心 Zustand store — v3: 原子存储菊花链。
 *
 *  Backward-compatible combined store + atom re-exports.
 *  State is owned by atomic stores in core/state/; this store delegates to them
 *  and mirrors their state for backward compatibility.
 */

import { create } from "zustand";
import type { StyleRegistry, StyleProperty } from "../core/types";
import { parseOmniSource } from "../services/codegen";

// ── Atom re-exports (preferred imports for refactored consumers) ──
export { useElementStore } from "../core/state/elements";
export { useWireStore } from "../core/state/wires";
export { useSelectionStore } from "../core/state/selection";
export { useProjectStore } from "../core/state/project";
export { useTabStore } from "../core/state/tabs";

// ── Type re-exports ──
export type { ElementData, WireData, PortData, ProjectConfig } from "../core/types";

// ── Atom imports (canonical state owners) ──
import { useElementStore, useWireStore, useSelectionStore, useProjectStore, useTabStore, useStyleRegistryStore } from "../core/state";

// ── Data Types (re-exported from core/types) ────────
export type { ElementData as _ElementData, WireData as _WireData } from "../core/types";
// ── Store ──────────────────────────────────────────

export interface EditorState {
  projectDir: string | null;
  previewUrl: string | null;
  files: { name: string; path: string; type: "file" | "directory" }[];
  activeFilePath: string | null;
  activeFileContent: string | null;
  compileErrors: { code: string; message: string; line: number }[];

  // ── Design canvas (drag-drop UI) ──
  elements: import("../core/types/element").ElementData[];
  wires: import("../core/types/wire").WireData[];
  rootOrder: string[];  // display order of root elements (parentId === null)

  // ── Active mode ──
  activeMode: "design" | "code";

  selectedElementIds: string[];
  /** Monotonically increasing version counter; bumped when custom component definitions
   *  are saved on exit from a component canvas. LayoutView includes this in its
   *  recompile-change-detection key so that the preview refreshes with the new definition. */
  compVersion: number;

  // ── Project management ──
  projectPath: string | null;       // .omni.json file path
  projectName: string;              // "未命名项目" or from filename
  sourceCode: string;               // embedded .omni source
  isDirty: boolean;                 // unsaved changes flag
  config: import("../core/types/project").ProjectConfig;            // { target, autoSave }

  // Actions
  setProjectDir: (dir: string) => void;
  loadFiles: (entries: { name: string; path: string; type: "file" | "directory" }[]) => void;
  openFile: (path: string, content: string) => void;
  setActiveFileContent: (content: string) => void;
  setCompileErrors: (errors: { code: string; message: string; line: number }[]) => void;
  setPreviewUrl: (url: string | null) => void;

  addElement: (el: import("../core/types/element").ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  selectElement: (id: string | null, multi?: boolean) => void;
  selectAll: () => void;
  clearSelection: () => void;

  addWire: (w: import("../core/types/wire").WireData) => void;
  removeWire: (id: string) => void;

  setElementSource: (id: string, source: string) => void;
  setElementPorts: (id: string, ports: import("../core/types/element").ElementData["ports"]) => void;
  setBlockValue: (id: string, value: unknown) => void;
  setEventBindings: (id: string, eventName: string, binding: { fnName: string; paramBindings: Record<string, { sourceType: string; value: string; expectedType: string }> }) => void;
  removeEventBindings: (id: string, eventName: string) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  /** 重命名元素 ID（自动更新子元素的 parentId 引用） */
  setElementId: (oldId: string, newId: string) => void;

  setElementsAndWires: (elements: import("../core/types/element").ElementData[], wires: import("../core/types/wire").WireData[]) => void;

  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;

  // ── Style management ──
  styleRegistry: StyleRegistry;

  // ── Element style (per-element references) ──
  setElementClassRefs: (elementId: string, classRefs: string[]) => void;
  addElementClassRef: (elementId: string, className: string) => void;
  removeElementClassRef: (elementId: string, className: string) => void;
  setElementInlineStyles: (elementId: string, properties: StyleProperty[]) => void;
  addElementInlineProperty: (elementId: string, property: StyleProperty) => void;
  removeElementInlineProperty: (elementId: string, propertyId: string) => void;
  updateElementInlineProperty: (elementId: string, propertyId: string, patch: Partial<StyleProperty>) => void;

  // ── Active mode switching ──
  setActiveMode: (mode: "design" | "code") => void;

  // ── Project management ──
  /** Create a new empty project. */
  newProject: () => void;
  /**
   * 打开项目文件。
   * - `.omni` 文件：传入源码字符串，解析出视觉元素
   * - `.omni.json` 文件（旧格式）：传入解析后的 JSON 文档对象
   */
  openProject: (path: string, sourceOrDocument: string | Record<string, unknown>) => void;
  /** Mark project as dirty (unsaved changes). */
  markDirty: () => void;
  /** Mark project as clean (saved). */
  markClean: () => void;
  setConfig: (patch: Partial<import("../core/types/project").ProjectConfig>) => void;
  setSourceCode: (code: string) => void;

  // ── Multi-Canvas Support (design mode tabs) ──
  activeDocId: string | null;
  docBackup: { elements: import("../core/types/element").ElementData[]; wires: import("../core/types/wire").WireData[] } | null;
  enterComponentCanvas: (componentName: string) => void;
  exitComponentCanvas: () => void;

  // ── Tab System ──
  openTabs: string[];
  tabStates: Record<string, { elements: import("../core/types/element").ElementData[]; wires: import("../core/types/wire").WireData[] }>;
  switchTab: (docId: string | null) => void;
  closeTab: (docId: string) => void;
  /** 打开 .omni.json 文件作为项目（替换当前项目）。 */
  openFileTab: (path: string, name: string, elements: import("../core/types/element").ElementData[], wires: import("../core/types/wire").WireData[], customComponents?: unknown[]) => void;

  /** 内部：打开 .omni 源码文件，解析为视觉元素 */
  _openOmniFile: (path: string, source: string) => void;
}

export const useEditorStore = create<EditorState>((set, get) => {
  /** Push current monolith state to all atomic stores. */
  const syncToAtoms = () => {
    const s = get();
    useElementStore.setState({ elements: s.elements as import("../core/types/element").ElementData[], rootOrder: s.rootOrder });
    useWireStore.setState({ wires: s.wires as import("../core/types/wire").WireData[] });
    useSelectionStore.setState({ selectedElementIds: s.selectedElementIds });
    useProjectStore.setState({
      projectDir: s.projectDir, projectPath: s.projectPath, projectName: s.projectName,
      sourceCode: s.sourceCode, isDirty: s.isDirty, activeFilePath: s.activeFilePath,
      activeFileContent: s.activeFileContent, activeMode: s.activeMode,
      config: s.config as import("../core/types/project").ProjectConfig,
      compileErrors: s.compileErrors, previewUrl: s.previewUrl,
      files: s.files as Array<{ name: string; path: string; type: "file" | "directory" }>,
    });
    useTabStore.setState({
      activeDocId: s.activeDocId,
      docBackup: s.docBackup as { elements: import("../core/types/element").ElementData[]; wires: import("../core/types/wire").WireData[] } | null,
      openTabs: s.openTabs,
      tabStates: s.tabStates as Record<string, { elements: import("../core/types/element").ElementData[]; wires: import("../core/types/wire").WireData[] }>,
      compVersion: s.compVersion,
    });
  };

  return {
    // ── Initial state ──
    projectDir: null, previewUrl: null, files: [],
    activeFilePath: null, activeFileContent: null, compileErrors: [],
    elements: [], wires: [], rootOrder: [],
    activeMode: "design", selectedElementIds: [], compVersion: 0,
    projectPath: null, projectName: "未命名项目", sourceCode: "",
    isDirty: false, config: { target: "react", autoSave: false },
    activeDocId: null, docBackup: null, openTabs: [], tabStates: {},

    // ── Project state setters ──
    setProjectDir: (dir) => { set({ projectDir: dir }); syncToAtoms(); },
    loadFiles: (entries) => { set({ files: entries }); syncToAtoms(); },
    openFile: (path, content) => { set({ activeFilePath: path, activeFileContent: content }); syncToAtoms(); },
    setActiveFileContent: (content) => { set({ activeFileContent: content }); syncToAtoms(); },
    setCompileErrors: (errors) => { set({ compileErrors: errors }); syncToAtoms(); },
    setPreviewUrl: (url) => { set({ previewUrl: url }); syncToAtoms(); },

    // ── Element actions (monolith computes → pushes to atoms) ──
    addElement: (el) => {
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
      });
      syncToAtoms();
    },

    removeElement: (id) => {
      set((s) => ({
        elements: (() => {
          const el = s.elements.find((e) => e.id === id);
          let elements = s.elements.filter((e) => e.id !== id);
          if (el?.parentId) {
            elements = elements.map((e) =>
              e.id === el.parentId ? { ...e, children: e.children.filter((cid) => cid !== id) } : e
            );
          }
          elements = elements.filter((e) => e.parentId !== id);
          return elements;
        })(),
        rootOrder: s.rootOrder.filter((rid) => rid !== id),
        isDirty: true,
      }));
      syncToAtoms();
    },

    setProp: (id, prop, value) => {
      set((s) => ({
        elements: s.elements.map((e) => e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e),
        isDirty: true,
      }));
      syncToAtoms();
    },

    getProp: (id, prop) => get().elements.find((e) => e.id === id)?.props[prop],

    selectElement: (id, multi = false) => {
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
      });
      syncToAtoms();
    },

    selectAll: () => {
      set((s) => ({ selectedElementIds: s.elements.map((e) => e.id) }));
      syncToAtoms();
    },

    clearSelection: () => { set({ selectedElementIds: [] }); syncToAtoms(); },

    addWire: (w) => { set((s) => ({ wires: [...s.wires, w], isDirty: true })); syncToAtoms(); },
    removeWire: (id) => { set((s) => ({ wires: s.wires.filter((w) => w.id !== id), isDirty: true })); syncToAtoms(); },

    setElementSource: (id, source) => {
      set((s) => ({ elements: s.elements.map((e) => e.id === id ? { ...e, source } : e) }));
      syncToAtoms();
    },

    setElementPorts: (id, ports) => {
      set((s) => ({ elements: s.elements.map((e) => e.id === id ? { ...e, ports } : e) }));
      syncToAtoms();
    },

    setEventBindings: (id, eventName, binding) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== id) return e;
          const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
          return { ...e, config: { ...e.config, _eventBindings: { ...currentBindings, [eventName]: binding } } };
        }),
      }));
      syncToAtoms();
    },

    removeEventBindings: (id, eventName) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== id) return e;
          const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
          const updated = { ...currentBindings };
          delete updated[eventName];
          return { ...e, config: { ...e.config, _eventBindings: updated } };
        }),
      }));
      syncToAtoms();
    },

    setBlockValue: (id, value) => {
      set((s) => {
        const existing = s.elements.find(e => e.id === id);
        if (existing && JSON.stringify(existing.outputValue) === JSON.stringify(value)) return s;
        return { elements: s.elements.map((e) => e.id === id ? { ...e, outputValue: value } : e) };
      });
      syncToAtoms();
    },

    setElementPosition: (id, pos) => {
      set((s) => ({ elements: s.elements.map((e) => e.id === id ? { ...e, position: { ...pos } } : e) }));
      syncToAtoms();
    },

    setElementId: (oldId, newId) => {
      set((s) => {
        if (s.elements.some((e) => e.id === newId)) return s;
        const updatedElements = s.elements.map((e) => {
          if (e.id === oldId) return { ...e, id: newId };
          if (e.parentId === oldId) return { ...e, parentId: newId };
          return e;
        });
        return {
          elements: updatedElements,
          rootOrder: s.rootOrder.map((id) => (id === oldId ? newId : id)),
          wires: s.wires.map((w) => ({
            ...w, fromId: w.fromId === oldId ? newId : w.fromId, toId: w.toId === oldId ? newId : w.toId,
          })),
          isDirty: true,
        };
      });
      syncToAtoms();
    },

    setElementsAndWires: (elements, wires) => {
      set({ elements, wires, selectedElementIds: [] });
      syncToAtoms();
    },

    reorderElement: (elementId, targetSiblingId, position) => {
      set((s) => {
        const el = s.elements.find((e) => e.id === elementId);
        const target = s.elements.find((e) => e.id === targetSiblingId);
        if (!el || !target) return {};
        if (el.parentId === null || target.parentId === null) {
          if (el.parentId !== target.parentId) return {};
          const order = [...s.rootOrder];
          const fromIdx = order.indexOf(elementId);
          const toIdx = order.indexOf(targetSiblingId);
          if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return {};
          order.splice(fromIdx, 1);
          const newToIdx = order.indexOf(targetSiblingId);
          const insertIdx = position === "before" ? newToIdx : newToIdx + 1;
          order.splice(insertIdx, 0, elementId);
          const elements = [...s.elements];
          const [movedEl] = elements.splice(fromIdx, 1);
          elements.splice(insertIdx, 0, movedEl);
          return { rootOrder: order, elements, isDirty: true };
        }
        if (el.parentId !== target.parentId) return {};
        const parentId = el.parentId;
        const parent = s.elements.find((e) => e.id === parentId);
        if (!parent) return {};
        const children = [...parent.children];
        const fromIdx = children.indexOf(elementId);
        const toIdx = children.indexOf(targetSiblingId);
        if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return {};
        children.splice(fromIdx, 1);
        const insertIdx = position === "before" ? children.indexOf(targetSiblingId) : children.indexOf(targetSiblingId) + 1;
        children.splice(insertIdx, 0, elementId);
        return { elements: s.elements.map((e) => e.id === parentId ? { ...e, children } : e), isDirty: true };
      });
      syncToAtoms();
    },

    moveElement: (elementId, newParentId, targetSiblingId, position) => {
      set((s) => {
        const el = s.elements.find((e) => e.id === elementId);
        if (!el) return {};
        const oldParentId = el.parentId;
        let elements = s.elements.map((e) => e.id === elementId ? { ...e, parentId: newParentId } : e);
        if (oldParentId) {
          elements = elements.map((e) =>
            e.id === oldParentId ? { ...e, children: e.children.filter((cid) => cid !== elementId) } : e
          );
        }
        if (newParentId) {
          const newParent = elements.find((e) => e.id === newParentId);
          if (!newParent) return {};
          const newChildren = [...newParent.children];
          if (targetSiblingId && newChildren.includes(targetSiblingId)) {
            const targetIdx = newChildren.indexOf(targetSiblingId);
            const insertIdx = position === "before" ? targetIdx : targetIdx + 1;
            newChildren.splice(insertIdx, 0, elementId);
          } else {
            newChildren.push(elementId);
          }
          elements = elements.map((e) => e.id === newParentId ? { ...e, children: newChildren } : e);
        }
        return { elements, isDirty: true };
      });
      syncToAtoms();
    },

    // ── Style Registry (compute active registry locally) ──
    get styleRegistry(): StyleRegistry {
      const activeDocId = get().activeDocId;
      const componentName = activeDocId?.startsWith("comp:")
        ? activeDocId.replace("comp:", "")
        : "App";
      return useStyleRegistryStore.getState().getRegistry(componentName);
    },

    // ── Element style mutations ──

    setElementClassRefs: (elementId, classRefs) => {
      set((s) => ({
        elements: s.elements.map((e) =>
          e.id === elementId
            ? { ...e, style: { ...(e.style ?? { classRefs: [], inlineProperties: [] }), classRefs } }
            : e
        ) as import("../core/types/element").ElementData[],
        isDirty: true,
        compVersion: s.compVersion + 1,
      }));
      syncToAtoms();
    },

    addElementClassRef: (elementId, className) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const es = e.style ?? { classRefs: [], inlineProperties: [] };
          return { ...e, style: { ...es, classRefs: [...es.classRefs, className] } };
        }),
        isDirty: true,
        compVersion: s.compVersion + 1,
      }));
      syncToAtoms();
    },

    removeElementClassRef: (elementId, className) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const es = e.style ?? { classRefs: [], inlineProperties: [] };
          return { ...e, style: { ...es, classRefs: es.classRefs.filter((c) => c !== className) } };
        }),
        isDirty: true,
        compVersion: s.compVersion + 1,
      }));
      syncToAtoms();
    },

    setElementInlineStyles: (elementId, properties) => {
      set((s) => ({
        elements: s.elements.map((e) =>
          e.id === elementId
            ? { ...e, style: { ...(e.style ?? { classRefs: [], inlineProperties: [] }), inlineProperties: properties } }
            : e
        ),
        isDirty: true,
        compVersion: s.compVersion + 1,
      }));
      syncToAtoms();
    },

    addElementInlineProperty: (elementId, property) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const es = e.style ?? { classRefs: [], inlineProperties: [] };
          return { ...e, style: { ...es, inlineProperties: [...es.inlineProperties, property] } };
        }),
        isDirty: true,
      }));
    },

    removeElementInlineProperty: (elementId, propertyId) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const es = e.style ?? { classRefs: [], inlineProperties: [] };
          return { ...e, style: { ...es, inlineProperties: es.inlineProperties.filter((p) => p.id !== propertyId) } };
        }),
        isDirty: true,
      }));
    },

    updateElementInlineProperty: (elementId, propertyId, patch) => {
      set((s) => ({
        elements: s.elements.map((e) => {
          if (e.id !== elementId) return e;
          const es = e.style ?? { classRefs: [], inlineProperties: [] };
          return {
            ...e,
            style: {
              ...es,
              inlineProperties: es.inlineProperties.map((p) =>
                p.id === propertyId ? { ...p, ...patch } : p
              ),
            },
          };
        }),
        isDirty: true,
      }));
    },

    // ── Active mode ──
    setActiveMode: (mode) => { set({ activeMode: mode }); syncToAtoms(); },

    // ── Project management ──
    newProject: () => {
      set({
        projectPath: null, projectName: "未命名项目", sourceCode: "",
        isDirty: false, elements: [], wires: [], rootOrder: [],
        activeDocId: null, docBackup: null, openTabs: [], tabStates: {},
        selectedElementIds: [], compVersion: 0,
      });
      syncToAtoms();
    },

    openProject: (path, sourceOrDocument) => {
      set({
        projectPath: path,
        projectDir: path ? path.substring(0, path.lastIndexOf('/')) : null,
        sourceCode: typeof sourceOrDocument === 'string' ? sourceOrDocument : '',
        activeFilePath: path,
        isDirty: false,
      });
      syncToAtoms();
    },

    markDirty: () => {
      useProjectStore.getState().markDirty();
      set({ isDirty: true });
    },

    markClean: () => {
      useProjectStore.getState().markClean();
      set({ isDirty: false });
    },

    setConfig: (patch) => {
      useProjectStore.getState().setConfig(patch);
      set((s) => ({ config: { ...s.config, ...patch }, isDirty: true }));
    },

    setSourceCode: (code) => {
      useProjectStore.getState().setSourceCode(code);
      set({ sourceCode: code, isDirty: true });
    },

    // ── .omni 源码文件解析 ──

    _openOmniFile: (path, source) => {
      let designElements: import("../core/types/element").ElementData[] = [];
      try {
        const parsed = parseOmniSource(source);
        if (parsed.designElements) designElements = parsed.designElements;
      } catch (e) {
        console.warn("[editor] Failed to parse .omni source:", e);
      }
      const projectName = path.replace(/^.*[/\\]/, "").replace(/\.omni$/, "");
      const result = set({
        projectPath: path,
        projectName,
        sourceCode: source,
        isDirty: false,
        elements: designElements,
        wires: [],
        rootOrder: designElements.filter((e) => !e.parentId).map((e) => e.id),
        config: { target: "react", autoSave: false },
        activeDocId: null,
        docBackup: null,
        openTabs: ["main"],
        tabStates: {},
        selectedElementIds: [],
      });
      syncToAtoms();
      return result;
    },

    // ── Multi-Canvas Support ──
    enterComponentCanvas: (componentName) => {
      set((s) => {
        const backup = { elements: s.elements, wires: s.wires };
        const compDef = useComponentRegistry.getState().definitions[componentName];
        if (!compDef) return {};
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
          rootOrder: (compDef.elements || []).filter((e) => !e.parentId).map((e) => e.id),
        };
      });
      syncToAtoms();
    },

    exitComponentCanvas: () => {
      set((s) => {
        if (!s.activeDocId || !s.activeDocId.startsWith("comp:")) return {};
        const compName = s.activeDocId.replace("comp:", "");
        useComponentRegistry.getState().saveCanvas(compName, s.elements, s.wires);
        import("./undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});
        return {
          elements: s.docBackup?.elements || [],
          wires: s.docBackup?.wires || [],
          docBackup: null,
          activeDocId: null,
          selectedElementIds: [],
          rootOrder: (s.docBackup?.elements || []).filter((e) => !e.parentId).map((e) => e.id),
          compVersion: s.compVersion + 1,
        };
      });
      syncToAtoms();
    },

    // ── Tab System ──
    switchTab: (docId) => {
      set((s) => {
        if (s.activeDocId === docId) return {};
        const newTabStates = { ...s.tabStates };
        const currentKey = s.activeDocId;
        let leavingComponentFile = false;

        if (currentKey && currentKey.startsWith("file:")) {
          newTabStates[currentKey] = { elements: s.elements, wires: s.wires };
          const filePath = currentKey.replace("file:", "");
          const compMatch = filePath.match(/src[/\\]components[/\\](.+)\.omni$/);
          if (compMatch) {
            const compName = compMatch[1];
            const registry = useComponentRegistry.getState();
            if (registry.definitions[compName]) {
              registry.saveCanvas(compName, s.elements, s.wires);
              leavingComponentFile = true;
            }
          }
        } else if (currentKey && currentKey.startsWith("comp:")) {
          const compName = currentKey.replace("comp:", "");
          useComponentRegistry.getState().saveCanvas(compName, s.elements, s.wires);
        }

        let newElements: import("../core/types/element").ElementData[];
        let newWires: import("../core/types/wire").WireData[];
        let newDocBackup = s.docBackup;

        if (docId === null) {
          newElements = s.docBackup?.elements || [];
          newWires = s.docBackup?.wires || [];
          newDocBackup = null;
        } else if (docId.startsWith("comp:")) {
          const compName = docId.replace("comp:", "");
          const compDef = useComponentRegistry.getState().definitions[compName];
          if (!compDef) return {};
          newDocBackup = s.activeDocId === null ? { elements: s.elements, wires: s.wires } : s.docBackup;
          newElements = compDef.elements;
          newWires = compDef.wires;
        } else if (docId.startsWith("file:")) {
          const cached = newTabStates[docId];
          if (!cached) return {};
          newElements = cached.elements;
          newWires = cached.wires;
          if (s.activeDocId === null) newDocBackup = { elements: s.elements, wires: s.wires };
        } else {
          return {};
        }

        import("./undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});
        const shouldBump = currentKey?.startsWith("comp:") || leavingComponentFile;
        const newRootOrder = newElements.filter((e) => !e.parentId).map((e) => e.id);

        return {
          elements: newElements,
          wires: newWires,
          rootOrder: newRootOrder,
          activeDocId: docId,
          docBackup: newDocBackup,
          tabStates: newTabStates,
          selectedElementIds: [],
          compVersion: shouldBump ? s.compVersion + 1 : s.compVersion,
          activeFilePath: docId?.startsWith("file:") ? docId.replace("file:", "") : s.activeFilePath,
        };
      });
      syncToAtoms();
    },

  closeTab: (docId) => {
    set((s) => {
      const newOpenTabs = s.openTabs.filter((t) => t !== docId);
      const newTabStates = { ...s.tabStates };
      delete newTabStates[docId];
      return {
        openTabs: newOpenTabs,
        tabStates: newTabStates,
      };
    });
    syncToAtoms();
  },

  /** @deprecated Use openProject() for .omni.json files instead. */
  openFileTab: (path, _name, elements, wires, customComponents) =>
    set((s) => {
      const tabKey = `file:${path}`;
      if (s.openTabs.includes(tabKey)) return s;
      const newTabStates = {
        ...s.tabStates,
        [tabKey]: { elements, wires },
      };
      if (customComponents && Array.isArray(customComponents) && customComponents.length > 0) {
        try {
          const componentDefs = customComponents as Array<Record<string, unknown>>;
          const defs: Record<string, Record<string, unknown>> = {};
          for (const cc of componentDefs) {
            const entryName = cc.name;
            if (typeof entryName === "string") {
              defs[entryName] = cc;
            }
          }
          if (Object.keys(defs).length > 0) {
            import("./componentRegistry").then((m) => {
              m.useComponentRegistry.setState({
                definitions: defs as unknown as Record<string, import("./componentRegistry").ComponentDefinition>,
              });
            }).catch(() => {});
          }
        } catch { /* non-critical */ }
      }
      return {
        openTabs: [...s.openTabs, tabKey],
        tabStates: newTabStates,
      };
    }),
}});

import { useComponentRegistry } from "./componentRegistry";

// Expose store for E2E tests (Playwright page.evaluate)
if (typeof window !== "undefined") {
  window.__ZUSTAND_EDITOR_STORE__ = useEditorStore;
}
