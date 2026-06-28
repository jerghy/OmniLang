// services/api/compile-api.ts
// Compilation + runtime lifecycle (compile, run start/stop/status, write run files)

import type { CompileResponse } from "./types";

export function createCompileApi(
  post: <T>(url: string, options?: { body?: BodyInit | null }) => Promise<T>,
  fetchFn: typeof fetch,
) {
  return {
    async compile(
      source: string,
      target = "react",
      previousFiles?: Record<string, string>,
      debug = false,
    ): Promise<CompileResponse> {
      const res = await fetchFn("/api/compile", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source, target, previousFiles, debug }),
      });
      return res.json();
    },

    async startRun(
      files: Record<string, string>,
      debug = false,
    ): Promise<{ success: boolean; url?: string; port?: number; error?: string }> {
      return post("/api/run/start", {
        body: JSON.stringify({ files, debug }),
      });
    },

    async stopRun(): Promise<{ success: boolean }> {
      return post("/api/run/stop");
    },

    async getRunStatus(): Promise<{ running: boolean; url: string | null }> {
      const res = await fetchFn("/api/run/status");
      return res.json();
    },

    async writeRunFiles(
      files: Record<string, string>,
    ): Promise<{ success: boolean; count?: number; error?: string }> {
      return post("/api/run/write-files", {
        body: JSON.stringify({ files }),
      });
    },
  };
}
