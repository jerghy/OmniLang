// commands/annotation-api.ts
// PID annotation + component definitions + editor TOML operations
// Replaces the old annotatedLoad/annotatedSave with v2 PID-based versions.
// Implemented via Tauri plugin-fs for file I/O

import { readTextFile, writeTextFile, mkdir, remove } from "@tauri-apps/plugin-fs";
import { Store } from "@tauri-apps/plugin-store";
import type { EditorTomlState } from "../core/types/annotation";
import type { PidMap, PidEntry } from "../core/types/pid";
import { buildProjectJsonFromStore, loadProjectFromJson } from "../services/persistence";
import { createLogger } from "../lib/utils/logger";

const log = createLogger("annotation");

const STORE_PATH = "project.json";
const COMPONENT_DEFS_FILE = ".poiesis/components.json";
const EDITOR_TOML_FILE = ".poiesis/editor.toml.json";

async function resolveProjectDir(): Promise<string | null> {
  try {
    const store = await Store.load(STORE_PATH);
    const val = await store.get<string>("projectDir");
    return val ?? null;
  } catch (e) {
    log.warn("Failed to resolve project dir:", e);
    return null;
  }
}

function joinPath(base: string, rel: string): string {
  // Reject path traversal attempts
  if (rel.split("/").some(seg => seg === ".." || seg === "~")) {
    throw new Error(`Path traversal rejected: ${rel}`);
  }
  const b = base.replace(/\\/g, "/").replace(/\/$/, "");
  const r = rel.replace(/\\/g, "/");
  return `${b}/${r}`;
}

async function ensurePoiesisDir(projectDir: string): Promise<string> {
  const dir = joinPath(projectDir, ".poiesis");
  try {
    await mkdir(dir, { recursive: true });
  } catch (e) {
    log.warn("Failed to create .poiesis dir:", e);
  }
  return dir;
}

/**
 * Parse `// @pid:xxx` comments from raw .omni source text.
 * Returns a PidMap with the parsed PIDs.
 */
function parsePidComments(source: string): PidMap {
  const entries: PidEntry[] = [];
  const lines = source.split("\n");
  const pidRe = /^\s*\/\/\s*@pid:(.+)$/;

  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(pidRe);
    if (match) {
      const pid = match[1].trim();
      entries.push({
        pid,
        blockType: "view",
        blockName: "",
        sourceFile: "",
        startLine: i + 1, // 1-based
        endLine: i + 1,
      });
    }
  }

  const pidMap: PidMap = { entries: {}, lineToPid: {} };
  for (const entry of entries) {
    pidMap.entries[entry.pid] = entry;
    for (let line = entry.startLine; line <= entry.endLine; line++) {
      pidMap.lineToPid[line] = entry.pid;
    }
  }

  return pidMap;
}

export function createAnnotationApi() {
  return {
    // ── Component definitions ──
    async saveComponentDefinitions(
      definitions: Record<string, unknown>,
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        await ensurePoiesisDir(dir);
        const path = joinPath(dir, COMPONENT_DEFS_FILE);
        await writeTextFile(path, JSON.stringify(definitions, null, 2));
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    async loadComponentDefinitions(): Promise<{
      definitions: Record<string, unknown>;
      error?: string;
    }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { definitions: {}, error: "未设置项目目录" };
        const path = joinPath(dir, COMPONENT_DEFS_FILE);
        const content = await readTextFile(path);
        return { definitions: JSON.parse(content) };
      } catch (e) {
        log.warn("Failed to load component definitions:", e);
        return { definitions: {}, error: String(e) };
      }
    },

    async deleteComponentDefinitions(
      names: string[],
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        const path = joinPath(dir, COMPONENT_DEFS_FILE);
        let defs: Record<string, unknown> = {};
        try {
          const content = await readTextFile(path);
          defs = JSON.parse(content) as Record<string, unknown>;
        } catch (e) {
          log.debug("Could not read component definitions file (expected on first use):", e);
        }
        for (const name of names) {
          delete defs[name];
        }
        await ensurePoiesisDir(dir);
        await writeTextFile(path, JSON.stringify(defs, null, 2));
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    /** Delete the .omni source file for a custom component. */
    async deleteComponentFile(
      componentName: string,
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        const filePath = joinPath(dir, `src/components/${componentName}.omni`);
        await remove(filePath);
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    // ── PID annotation API (v2, replaces old annotatedSave/annotatedLoad) ──

    /**
     * Read a .omni file and extract its PID map from `// @pid:xxx` comments.
     *
     * @param filePath  Relative path from project root (e.g. "src/main.omni").
     * @returns The parsed PidMap, or an empty PidMap if no pid comments found.
     */
    async readPidMap(filePath: string): Promise<PidMap> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { entries: {}, lineToPid: {} };

        const fullPath = joinPath(dir, filePath);
        const source = await readTextFile(fullPath);
        return parsePidComments(source);
      } catch (e) {
        log.warn("Failed to read PID map:", e);
        return { entries: {}, lineToPid: {}, error: String(e) };
      }
    },

    /**
     * Write the .omni source file and persist its PidMap to the annotation store.
     *
     * @param filePath  Relative path from project root.
     * @param source    The .omni source text to write.
     * @param pidMap    The PidMap to persist as annotation metadata.
     */
    async writePidAnnotations(
      filePath: string,
      source: string,
      pidMap: PidMap,
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };

        // Save the raw .omni source file (truth source)
        const omniPath = joinPath(dir, filePath);
        await writeTextFile(omniPath, source);

        // PidMap is embedded in .omni via // @pid:xxx comments — no sidecar needed
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    // ── Editor TOML state ──
    async saveEditorToml(
      state: EditorTomlState,
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        await ensurePoiesisDir(dir);
        const path = joinPath(dir, EDITOR_TOML_FILE);
        await writeTextFile(path, JSON.stringify(state, null, 2));
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    async loadEditorToml(): Promise<{
      state: EditorTomlState | null;
      error?: string;
    }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { state: null, error: "未设置项目目录" };
        const path = joinPath(dir, EDITOR_TOML_FILE);
        const content = await readTextFile(path);
        return { state: JSON.parse(content) };
      } catch (e) {
        log.warn("Failed to load editor.toml:", e);
        return { state: null, error: String(e) };
      }
    },

    // ── Project save/load (via persistence layer) ──

    /**
     * Save the full project state to project.json.
     * Builds the ProjectJson from the store via buildProjectJsonFromStore()
     * and writes it to `project.json` in the project directory.
     */
    async saveProject(): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        const jsonStr = buildProjectJsonFromStore();
        const path = joinPath(dir, "project.json");
        await writeTextFile(path, jsonStr);
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    /**
     * Load project state from project.json.
     * Reads `project.json` from the project directory and populates
     * the store via loadProjectFromJson().
     */
    async loadProject(): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        const path = joinPath(dir, "project.json");
        const content = await readTextFile(path);
        const result = loadProjectFromJson(content);
        return result.success ? { ok: true } : { ok: false, error: result.error };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },
  };
}
