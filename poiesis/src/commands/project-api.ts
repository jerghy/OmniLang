// commands/project-api.ts
// Project filesystem operations — implemented via Tauri plugins (fs, store)
// Replaces Python FastAPI backend endpoints: /api/fs/*, /api/project/*

import { readDir, readTextFile, writeTextFile, mkdir } from "@tauri-apps/plugin-fs";
import { Store } from "@tauri-apps/plugin-store";
import type { FsEntry, ProjectInfo } from "./types";

const STORE_PATH = "project.json";
let cachedProjectDir: string | null = null;

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
  } catch {
    return null;
  }
}

/** Join paths, normalizing slashes and removing double separators. */
function joinPath(base: string, rel: string): string {
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
        await writeTextFile(fullPath, content);
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
        const normalized = path.replace(/\\/g, "/");
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
  };
}
