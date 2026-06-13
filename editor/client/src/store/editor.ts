/** 编辑器核心 Zustand store。*/

import { create } from "zustand";

// ── Data Types (R2 upgraded schema) ─────────────────

export interface ElementData {
  id: string;
  nodeType: "view" | "codeblock" | "group";
  tag: string;
  props: Record<string, unknown>;
  children: string[];
  parentId: string | null;
  ports?: { name: string; direction: "in" | "out"; type: string; label?: string }[];
  renderMode?: "dom" | "canvas";
  isFolded?: boolean;
  typeAnnotation?: string;
  storeName?: string;
  /** Per-block source code (for code blocks) */
  source?: string;
  /** Runtime-computed output value (for code blocks) */
  outputValue?: unknown;
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

// ── Store ──────────────────────────────────────────

interface EditorState {
  projectDir: string | null;
  previewUrl: string | null;
  files: { name: string; path: string; type: "file" | "directory" }[];
  activeFilePath: string | null;
  activeFileContent: string | null;
  compileErrors: { code: string; message: string; line: number }[];

  elements: ElementData[];
  wires: WireData[];
  selectedElementId: string | null;

  // M6: Read-only mode for hand-written files
  isHandWritten: boolean;
  readOnlyMode: boolean;
  handWrittenWarnings: string[];
  setHandWritten: (is: boolean, warnings: string[]) => void;
  setReadOnly: (readonly: boolean) => void;

  // M6: .omni.json document path
  omniJsonPath: string | null;
  setOmniJsonPath: (path: string | null) => void;

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
  selectElement: (id: string | null) => void;

  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;

  setElementSource: (id: string, source: string) => void;
  setElementPorts: (id: string, ports: ElementData["ports"]) => void;
  setBlockValue: (id: string, value: unknown) => void;
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
  selectedElementId: null,

  // M6
  isHandWritten: false,
  readOnlyMode: false,
  handWrittenWarnings: [],
  setHandWritten: (is, warnings) => set({ isHandWritten: is, handWrittenWarnings: warnings, readOnlyMode: is }),
  setReadOnly: (readonly) => set({ readOnlyMode: readonly }),

  omniJsonPath: null,
  setOmniJsonPath: (path) => set({ omniJsonPath: path }),

  setProjectDir: (dir) => set({ projectDir: dir }),

  loadFiles: (entries) => set({ files: entries }),

  openFile: (path, content) =>
    set({ activeFilePath: path, activeFileContent: content }),

  setActiveFileContent: (content) => set({ activeFileContent: content }),

  setCompileErrors: (errors) => set({ compileErrors: errors }),

  setPreviewUrl: (url) => set({ previewUrl: url }),

  addElement: (el) =>
    set((s) => ({ elements: [...s.elements, el] })),

  removeElement: (id) =>
    set((s) => ({ elements: s.elements.filter((e) => e.id !== id) })),

  setProp: (id, prop, value) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e
      ),
    })),

  getProp: (id, prop) => get().elements.find((e) => e.id === id)?.props[prop],

  selectElement: (id) => set({ selectedElementId: id }),

  addWire: (w) => set((s) => ({ wires: [...s.wires, w] })),

  removeWire: (id) =>
    set((s) => ({ wires: s.wires.filter((w) => w.id !== id) })),

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

  setBlockValue: (id, value) =>
    set((s) => {
      const existing = s.elements.find(e => e.id === id);
      // Skip update if value hasn't changed (prevents render loops)
      if (existing && JSON.stringify(existing.outputValue) === JSON.stringify(value)) return s;
      return {
        elements: s.elements.map((e) =>
          e.id === id ? { ...e, outputValue: value } : e
        ),
      };
    }),
}));

// Expose store for E2E tests (Playwright page.evaluate)
if (typeof window !== "undefined") {
  (window as any).__ZUSTAND_EDITOR_STORE__ = useEditorStore;
}
