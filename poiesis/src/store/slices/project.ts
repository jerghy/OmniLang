// ── Project slice for Poiesis store ──

import type { PoiesisState } from "../types";
import { DEFAULT_PROJECT_NAME, DEFAULT_PANEL_LAYOUT } from "../../constants";
import { createInitialArchitectureState } from "./architecture";
import { createInitialDesignPageState } from "../operations/tab-lifecycle";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

/** Shared clean state factory used by resetEditorState and newProject. */
function createCleanState(): Partial<PoiesisState> {
  return {
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
    // 跨项目隔离：pid 可跨项目重复，重置时必须清空旧项目的位置缓存，
    // 否则新项目同名 pid 物化时套用旧项目坐标（与 nodeMeta/canvasStates 同源模式）
    persistedPositions: {},
    // 跨项目隔离：调试值（prop/signal/store 覆盖）不随项目存活
    debugValues: {},
    panels: { ...DEFAULT_PANEL_LAYOUT },
    undoStacks: {},
  };
}

export function createProjectSlice(set: SetFn, _get?: GetFn) {
  return {
    setProjectDir: (dir: string) => set({ projectDir: dir }),
    setProjectName: (name: string) => set({ projectName: name }),
    openFile: (path: string, content: string) => set({ activeFilePath: path, activeFileContent: content }),
    setActiveFileContent: (content: string) => set({ activeFileContent: content }),
    markDirty: () => set({ isDirty: true }),
    markClean: () => set({ isDirty: false }),
    setCompileErrors: (errors: Array<{ code: string; message: string; line: number }>) => set({ compileErrors: errors }),
    setPreviewUrl: (url: string | null) => set({ previewUrl: url }),

    // ── Mode ──
    switchMode: (mode: "design" | "code") => set({ mode }),

    // ── Reset editor state (keep project dir/name) ──
    resetEditorState: () => {
      set(createCleanState());
    },

    // ── New project ──
    newProject: () => {
      set({ ...createCleanState(), projectDir: null, projectName: DEFAULT_PROJECT_NAME });
    },
  };
}
