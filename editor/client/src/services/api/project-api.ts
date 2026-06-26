// services/api/project-api.ts
// Project filesystem operations (list, read, write, project info, project dir management)

import type { FsEntry, ProjectInfo } from "../../api/types";

export function createProjectApi(
  post: <T>(url: string, options?: { body?: BodyInit | null }) => Promise<T>,
  fetchFn: typeof fetch,
) {
  return {
    async listDir(
      dirPath = ".",
    ): Promise<{ entries: FsEntry[]; error?: string }> {
      const res = await fetchFn("/api/fs/list", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ dirPath }),
      });
      return res.json();
    },

    async readFile(
      filePath: string,
    ): Promise<{ content: string; error?: string }> {
      const res = await fetchFn("/api/fs/read", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ filePath }),
      });
      return res.json();
    },

    async writeFile(
      filePath: string,
      content: string,
    ): Promise<{ ok: boolean; error?: string }> {
      const res = await fetchFn("/api/fs/write", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ filePath, content }),
      });
      return res.json();
    },

    async getProject(): Promise<ProjectInfo> {
      const res = await fetchFn("/api/project");
      return res.json();
    },

    async listDirectory(
      path: string,
    ): Promise<{
      ok: boolean;
      current?: string;
      entries?: Array<{ name: string; isDir: boolean }>;
      error?: string;
    }> {
      const res = await fetchFn("/api/fs/list-directory", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path }),
      });
      return res.json();
    },

    async setProjectDir(
      dir: string,
    ): Promise<{ ok: boolean; dir?: string; error?: string }> {
      return post("/api/project/set-dir", {
        body: JSON.stringify({ dir }),
      });
    },

    async createProject(
      dir: string,
      name: string,
    ): Promise<{ ok: boolean; dir?: string; error?: string }> {
      return post("/api/project/create", {
        body: JSON.stringify({ dir, name }),
      });
    },
  };
}
