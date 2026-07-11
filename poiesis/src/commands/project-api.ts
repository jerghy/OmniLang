// commands/project-api.ts
// Project filesystem operations — implemented via Tauri plugins (fs, store, dialog)
// Replaces Python FastAPI backend endpoints: /api/fs/*, /api/project/*

import { readDir, readTextFile, writeTextFile, mkdir, remove } from "@tauri-apps/plugin-fs";
import { Store } from "@tauri-apps/plugin-store";
import { open } from "@tauri-apps/plugin-dialog";
import type { FsEntry, ProjectInfo } from "./types";
import { usePoiesisStore } from "../store";
import { createLogger } from "../lib/utils/logger";

const log = createLogger("project");

const STORE_PATH = "project.json";
const RECENT_KEY = "recentProjects";
const MAX_RECENT = 10;
let cachedProjectDir: string | null = null;

export interface RecentProject {
  name: string;
  path: string;
  openedAt: number; // timestamp
}

async function getStore(): Promise<Store> {
  return Store.load(STORE_PATH);
}

async function resolveProjectDir(): Promise<string | null> {
  if (cachedProjectDir !== null) return cachedProjectDir;
  try {
    const store = await getStore();
    const val = await store.get<string>("projectDir");
    cachedProjectDir = val ?? null;
    return cachedProjectDir;
  } catch (e) {
    log.warn("Failed to resolve project dir:", e);
    return null;
  }
}

/** Join paths, normalizing slashes and removing double separators. */
function joinPath(base: string, rel: string): string {
  // Reject path traversal attempts
  if (rel.split("/").some(seg => seg === ".." || seg === "~")) {
    throw new Error(`Path traversal rejected: ${rel}`);
  }
  const b = base.replace(/\\/g, "/").replace(/\/$/, "");
  const r = rel.replace(/\\/g, "/").replace(/^\.\//, "");
  if (r === "." || r === "") return b;
  return `${b}/${r}`;
}

export function createProjectApi() {
  return {
    // ── Project file listing (relative to project dir) ──
    async listDir(
      dirPath = ".",
    ): Promise<{ entries: FsEntry[]; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { entries: [], error: "未设置项目目录" };
        const fullPath = dirPath === "." ? dir : joinPath(dir, dirPath);
        const fsEntries = await readDir(fullPath);
        return {
          entries: fsEntries.map((e) => {
            const rel = dirPath === "." ? e.name : `${dirPath.replace(/\\/g, "/")}/${e.name}`;
            return {
              name: e.name,
              path: rel,
              type: e.isDirectory ? "directory" as const : "file" as const,
              ext: e.name.includes(".") ? e.name.split(".").pop() || "" : "",
            };
          }),
        };
      } catch (e) {
        return { entries: [], error: String(e) };
      }
    },

    // ── Read file (relative to project dir) ──
    async readFile(
      filePath: string,
    ): Promise<{ content: string; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { content: "", error: "未设置项目目录" };
        const fullPath = joinPath(dir, filePath);
        const content = await readTextFile(fullPath);
        return { content };
      } catch (e) {
        return { content: "", error: String(e) };
      }
    },

    // ── Write file (relative to project dir) ──
    async writeFile(
      filePath: string,
      content: string,
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        const fullPath = joinPath(dir, filePath);
        // Ensure parent directory exists
        const parentDir = fullPath.substring(0, fullPath.lastIndexOf("/"));
        await mkdir(parentDir, { recursive: true });
        await writeTextFile(fullPath, content);
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    // ── Delete file (relative to project dir) ──
    async deleteFile(
      filePath: string,
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        const fullPath = joinPath(dir, filePath);
        await remove(fullPath);
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    // ── Get current project info ──
    async getProject(): Promise<ProjectInfo> {
      const dir = await resolveProjectDir();
      return { dir, target: "react", mainFile: null };
    },

    // ── Browse any directory (absolute path) — used by DirBrowser ──
    async listDirectory(
      path: string,
    ): Promise<{
      ok: boolean;
      current?: string;
      entries?: Array<{ name: string; isDir: boolean }>;
      error?: string;
    }> {
      try {
        const dir = usePoiesisStore.getState().projectDir;
        if (!dir) throw new Error("No project dir");
        const resolvedPath = path.startsWith(dir) ? path : joinPath(dir, path);
        const normalized = resolvedPath.replace(/\\/g, "/");
        const entries = await readDir(normalized);
        return {
          ok: true,
          current: normalized,
          entries: entries.map((e) => ({
            name: e.name,
            isDir: e.isDirectory,
          })),
        };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    // ── Set current project directory ──
    async setProjectDir(
      dir: string,
    ): Promise<{ ok: boolean; dir?: string; error?: string }> {
      try {
        const normalized = dir.replace(/\\/g, "/");
        const store = await getStore();
        await store.set("projectDir", normalized);
        await store.save();
        cachedProjectDir = normalized;
        return { ok: true, dir: normalized };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    // ── Create new project ──
    async createProject(
      dir: string,
      name: string,
    ): Promise<{ ok: boolean; dir?: string; error?: string }> {
      try {
        const parentDir = dir.replace(/\\/g, "/");
        const projectPath = `${parentDir}/${name}`;
        await mkdir(projectPath, { recursive: true });
        await mkdir(`${projectPath}/src`, { recursive: true });

        const defaultOmni = `// ${name} — OmniLang Project\n\nview Main {\n  \n}\n`;
        await writeTextFile(`${projectPath}/src/main.omni`, defaultOmni);

        const store = await getStore();
        await store.set("projectDir", projectPath);
        await store.save();
        cachedProjectDir = projectPath;
        return { ok: true, dir: projectPath };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    // ── Native folder picker ──
    async openFolderDialog(): Promise<string | null> {
      try {
        const selected = await open({
          directory: true,
          multiple: false,
          title: "选择项目文件夹",
        });
        return selected ?? null;
      } catch (e) {
        log.warn("Failed to open folder dialog:", e);
        return null;
      }
    },

    // ── Recent projects ──
    async getRecentProjects(): Promise<RecentProject[]> {
      try {
        const store = await getStore();
        const val = await store.get<RecentProject[]>(RECENT_KEY);
        return val ?? [];
      } catch (e) {
        log.warn("Failed to get recent projects:", e);
        return [];
      }
    },

    async addRecentProject(name: string, path: string): Promise<void> {
      try {
        const store = await getStore();
        const list = (await store.get<RecentProject[]>(RECENT_KEY)) ?? [];
        // Remove duplicate if exists
        const filtered = list.filter((p) => p.path !== path);
        // Add to front
        filtered.unshift({ name, path, openedAt: Date.now() });
        // Keep max
        const trimmed = filtered.slice(0, MAX_RECENT);
        await store.set(RECENT_KEY, trimmed);
        await store.save();
      } catch (e) {
        log.warn("Failed to add recent project:", e);
      }
    },

    async removeRecentProject(path: string): Promise<void> {
      try {
        const store = await getStore();
        const list = (await store.get<RecentProject[]>(RECENT_KEY)) ?? [];
        const filtered = list.filter((p) => p.path !== path);
        await store.set(RECENT_KEY, filtered);
        await store.save();
      } catch (e) {
        log.warn("Failed to remove recent project:", e);
      }
    },
  };
}
