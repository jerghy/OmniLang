// commands/annotation-api.ts
// Annotation + component definitions + editor TOML operations
// Replaces Python FastAPI endpoints: /api/annotated-*, /api/component-definitions/*, /api/editor-toml/*
// Implemented via Tauri plugin-fs for file I/O

import { readTextFile, writeTextFile, readDir, remove, mkdir } from "@tauri-apps/plugin-fs";
import { Store } from "@tauri-apps/plugin-store";
import type {
  AnnotatedLoadResponse,
  AnnotatedSaveResponse,
  EditorTomlState,
  AnnotationFile,
} from "../core/types";

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

    // ── Annotated save/load ──
    // Saves an envelope JSON alongside the .omni source that includes
    // annotations (viewMarkers, chains, etc.) and the raw source.
    async annotatedSave(
      filePath: string,
      annotations: AnnotationFile,
      source: string,
    ): Promise<AnnotatedSaveResponse> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { success: false, error: "未设置项目目录" };

        // Save the raw .omni source file
        const omniPath = joinPath(dir, filePath);
        await writeTextFile(omniPath, source);

        // Save the annotation envelope
        const annoDir = joinPath(dir, ".poiesis/annotations");
        await mkdir(annoDir, { recursive: true });
        const annoPath = joinPath(dir, `.poiesis/annotations/${filePath.replace(/[/\\:]/g, "_")}.json`);
        await writeTextFile(annoPath, JSON.stringify({ annotations, source, savedAt: Date.now() }, null, 2));

        return { success: true };
      } catch (e) {
        return { success: false, error: String(e) };
      }
    },

    async annotatedLoad(filePath: string): Promise<AnnotatedLoadResponse> {
      try {
        const dir = await resolveProjectDir();
        if (!dir) return { success: false, error: "未设置项目目录" };

        // Try loading annotation envelope first
        const annoPath = joinPath(dir, `.poiesis/annotations/${filePath.replace(/[/\\:]/g, "_")}.json`);
        try {
          const annoContent = await readTextFile(annoPath);
          const envelope = JSON.parse(annoContent);
          return {
            success: true,
            source: envelope.source as string,
            annotations: envelope.annotations as Record<string, unknown>,
          };
        } catch {
          // Fallback: read raw .omni file without annotations
          const omniPath = joinPath(dir, filePath);
          const source = await readTextFile(omniPath);
          return { success: true, source };
        }
      } catch (e) {
        return { success: false, error: String(e) };
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
