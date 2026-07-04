// commands/annotation-api.ts
// PID annotation + component definitions + editor TOML operations
// Replaces the old annotatedLoad/annotatedSave with v2 PID-based versions.
// Implemented via Tauri plugin-fs for file I/O

import { readTextFile, writeTextFile, mkdir } from "@tauri-apps/plugin-fs";
import { Store } from "@tauri-apps/plugin-store";
import type { EditorTomlState } from "../core/types/annotation";
import type { PidMap, PidEntry } from "../core/types/pid";

const STORE_PATH = "project.json";
const COMPONENT_DEFS_FILE = ".poiesis/components.json";
const EDITOR_TOML_FILE = ".poiesis/editor.toml.json";

async function resolveProjectDir(): Promise<string | null> {
  try {
    const store = await Store.load(STORE_PATH);
    const val = await store.get<string>("projectDir");
    return val ?? null;
  } catch {
    return null;
  }
}

function joinPath(base: string, rel: string): string {
  const b = base.replace(/\\/g, "/").replace(/\/$/, "");
  const r = rel.replace(/\\/g, "/");
  return `${b}/${r}`;
}

async function ensurePoiesisDir(projectDir: string): Promise<string> {
  const dir = joinPath(projectDir, ".poiesis");
  try {
    await mkdir(dir, { recursive: true });
  } catch {
    // Directory may already exist
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
      } catch {
        return { definitions: {} };
      }
    },

    async deleteComponentDefinitions(
      names: string[],
    ): Promise<{ ok: boolean; error?: string }> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { ok: false, error: "未设置项目目录" };
        const path = joinPath(dir, COMPONENT_DEFS_FILE);
        const content = await readTextFile(path);
        const defs = JSON.parse(content) as Record<string, unknown>;
        for (const name of names) {
          delete defs[name];
        }
        await writeTextFile(path, JSON.stringify(defs, null, 2));
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
      } catch {
        return { entries: {}, lineToPid: {} };
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

        // Save the raw .omni source file
        const omniPath = joinPath(dir, filePath);
        await writeTextFile(omniPath, source);

        // Save the PidMap annotation envelope
        const annoDir = joinPath(dir, ".poiesis/annotations");
        await mkdir(annoDir, { recursive: true });
        const annoPath = joinPath(
          dir,
          `.poiesis/annotations/${filePath.replace(/[/\\:]/g, "_")}.json`,
        );
        await writeTextFile(
          annoPath,
          JSON.stringify(
            { pidMap, source, savedAt: Date.now() },
            null,
            2,
          ),
        );

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
      } catch {
        return { state: null };
      }
    },
  };
}
