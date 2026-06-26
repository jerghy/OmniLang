// core/state/project.ts
// Project management state atom — extracted from store/editor.ts v2.

import { create } from "zustand";
import type { ProjectConfig } from "../types/project";

interface ProjectState {
  projectDir: string | null;
  projectPath: string | null;
  projectName: string;
  sourceCode: string;
  isDirty: boolean;
  activeFilePath: string | null;
  activeFileContent: string | null;
  activeMode: "design" | "code";
  config: ProjectConfig;
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;
  files: Array<{ name: string; path: string; type: "file" | "directory" }>;

  setProjectDir: (dir: string) => void;
  loadFiles: (entries: Array<{ name: string; path: string; type: "file" | "directory" }>) => void;
  openFile: (path: string, content: string) => void;
  setActiveFileContent: (content: string) => void;
  setSourceCode: (code: string) => void;
  setCompileErrors: (errors: Array<{ code: string; message: string; line: number }>) => void;
  setPreviewUrl: (url: string | null) => void;
  setActiveMode: (mode: "design" | "code") => void;
  setConfig: (patch: Partial<ProjectConfig>) => void;
  markDirty: () => void;
  markClean: () => void;
  newProject: () => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  projectDir: null,
  projectPath: null,
  projectName: "未命名项目",
  sourceCode: "",
  isDirty: false,
  activeFilePath: null,
  activeFileContent: null,
  activeMode: "design",
  config: { target: "react", autoSave: false },
  compileErrors: [],
  previewUrl: null,
  files: [],

  setProjectDir: (dir) => set({ projectDir: dir }),
  loadFiles: (entries) => set({ files: entries }),
  openFile: (path, content) => set({ activeFilePath: path, activeFileContent: content }),
  setActiveFileContent: (content) => set({ activeFileContent: content }),
  setSourceCode: (code) => set({ sourceCode: code, isDirty: true }),
  setCompileErrors: (errors) => set({ compileErrors: errors }),
  setPreviewUrl: (url) => set({ previewUrl: url }),
  setActiveMode: (mode) => set({ activeMode: mode }),
  setConfig: (patch) => set((s) => ({ config: { ...s.config, ...patch }, isDirty: true })),
  markDirty: () => set({ isDirty: true }),
  markClean: () => set({ isDirty: false }),
  newProject: () => set({ projectPath: null, projectName: "未命名项目", sourceCode: "", isDirty: false }),
}));
