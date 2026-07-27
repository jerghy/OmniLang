// commands/compile-api.ts
// Compilation + runtime lifecycle — WASM compiler for compilation,
// Tauri shell plugin for runtime Vite dev server
// Replaces Python FastAPI endpoints: /api/compile, /api/run/*

import { invoke } from "@tauri-apps/api/core";
import { rustCompiler } from "./rustCompiler";
import type { CompileResponse, CompileError } from "./types";

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
        return this.compileViaTauri(source, target);
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

    // ── Fallback: compile via Tauri IPC when WASM is unavailable ──
    async compileViaTauri(
      source: string,
      target: string,
    ): Promise<CompileResponse> {
      // Check if running inside a Tauri webview
      const isTauri =
        typeof window !== "undefined" &&
        typeof (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !==
          "undefined";

      if (!isTauri) {
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
              message: "OmniLang WASM 编译器未加载，且不在 Tauri 桌面环境中",
              suggestion:
                "请使用桌面版应用，或在浏览器中确认 public/wasm/ WASM 文件存在",
            },
          ],
        };
      }

      try {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const result: Record<string, any> = await invoke("compile_source", {
          source,
          target,
        });

        const errors: CompileError[] = (result.errors || []).map(
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          (e: Record<string, any>) => ({
            file: e.file || "",
            line: typeof e.line === "number" ? e.line : 0,
            col: typeof e.col === "number" ? e.col : 0,
            endLine:
              typeof e.end_line === "number" ? e.end_line : undefined,
            endCol: typeof e.end_col === "number" ? e.end_col : undefined,
            severity: e.severity === "warning" ? "warning" : "error",
            code: e.code || "UNKNOWN",
            message: e.message || "Unknown error",
            suggestion: e.suggestion ?? null,
          }),
        );

        return {
          success: result.success === true,
          files: (result.files as Record<string, string>) || {},
          diff: {},
          errors,
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
              code: "TAURI_COMPILE_ERROR",
              message: `Tauri 编译失败: ${String(e)}`,
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
