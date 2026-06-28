// commands/compile-api.ts
// Compilation + runtime lifecycle — WASM compiler for compilation,
// Tauri shell plugin for runtime Vite dev server
// Replaces Python FastAPI endpoints: /api/compile, /api/run/*

import { rustCompiler } from "./rustCompiler";
import type { CompileResponse } from "./types";

export function createCompileApi() {
  // ── Runtime state ──
  let runState: { running: boolean; url: string | null; port: number | null } = {
    running: false,
    url: null,
    port: null,
  };

  return {
    // ── Compile OmniLang source → target files ──
    async compile(
      source: string,
      target = "react",
      _previousFiles?: Record<string, string>,
      _debug = false,
    ): Promise<CompileResponse> {
      await rustCompiler.init();

      if (!rustCompiler.isReady) {
        return {
          success: false,
          files: {},
          diff: {},
          errors: [
            {
              file: "",
              line: 0,
              col: 0,
              severity: "error",
              code: "COMPILER_UNAVAILABLE",
              message: "OmniLang WASM 编译器未加载",
              suggestion: "请检查 public/wasm/ 下的 WASM 文件",
            },
          ],
        };
      }

      try {
        const result = rustCompiler.compileToOmniView(source, false);

        if (result.success && result.files) {
          return {
            success: true,
            files: {
              "index.html": result.files.html,
              "index.js": result.files.js,
              "index.css": result.files.css,
            },
            diff: {},
            errors: [],
          };
        }

        return {
          success: false,
          files: {},
          diff: {},
          errors: (result.errors || []).map((e) => ({
            file: "",
            line: e.line,
            col: e.col,
            severity: e.severity as "error" | "warning",
            code: e.code,
            message: e.message,
            suggestion: e.suggestion || null,
          })),
        };
      } catch (e) {
        return {
          success: false,
          files: {},
          diff: {},
          errors: [
            {
              file: "",
              line: 0,
              col: 0,
              severity: "error",
              code: "COMPILE_EXCEPTION",
              message: `编译异常: ${String(e)}`,
              suggestion: null,
            },
          ],
        };
      }
    },

    // ── Runtime: start Vite dev server for preview ──
    async startRun(
      _files: Record<string, string>,
      _debug = false,
    ): Promise<{ success: boolean; url?: string; port?: number; error?: string }> {
      // TODO: Implement via @tauri-apps/plugin-shell to spawn Vite
      // For now, return a stub — the preview feature will be wired later
      runState = { running: true, url: "http://localhost:5173", port: 5173 };
      return { success: true, url: "http://localhost:5173", port: 5173 };
    },

    async stopRun(): Promise<{ success: boolean }> {
      runState = { running: false, url: null, port: null };
      return { success: true };
    },

    async getRunStatus(): Promise<{ running: boolean; url: string | null }> {
      return { running: runState.running, url: runState.url };
    },

    async writeRunFiles(
      _files: Record<string, string>,
    ): Promise<{ success: boolean; count?: number; error?: string }> {
      // TODO: Write files to runtime temp directory via fs plugin
      const count = Object.keys(_files).length;
      return { success: true, count };
    },
  };
}
