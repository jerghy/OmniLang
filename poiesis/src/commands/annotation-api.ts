// services/api/annotation-api.ts
// Component definitions (save/load/delete) and annotation operations (save/load, editor TOML)

import type {
  AnnotatedLoadResponse,
  AnnotatedSaveResponse,
  EditorTomlState,
  AnnotationFile,
} from "../core/types";

export function createAnnotationApi(
  post: <T>(url: string, options?: { body?: BodyInit | null }) => Promise<T>,
  fetchFn: typeof fetch,
) {
  return {
    async saveComponentDefinitions(
      definitions: Record<string, unknown>,
    ): Promise<{ ok: boolean; error?: string }> {
      return post("/api/component-definitions/save", {
        body: JSON.stringify({ definitions }),
      });
    },

    async loadComponentDefinitions(): Promise<{
      definitions: Record<string, unknown>;
      error?: string;
    }> {
      const res = await fetchFn("/api/component-definitions/load");
      return res.json();
    },

    async deleteComponentDefinitions(
      names: string[],
    ): Promise<{ ok: boolean; error?: string }> {
      return post("/api/component-definitions/delete", {
        body: JSON.stringify({ names }),
      });
    },

    async annotatedSave(
      filePath: string,
      annotations: AnnotationFile,
      source: string,
    ): Promise<AnnotatedSaveResponse> {
      return post("/api/annotated-save", {
        body: JSON.stringify({ filePath, annotations, source }),
      });
    },

    async annotatedLoad(filePath: string): Promise<AnnotatedLoadResponse> {
      return post("/api/annotated-load", {
        body: JSON.stringify({ filePath }),
      });
    },

    async saveEditorToml(
      state: EditorTomlState,
    ): Promise<{ ok: boolean; error?: string }> {
      return post("/api/editor-toml/save", {
        body: JSON.stringify({ state }),
      });
    },

    async loadEditorToml(): Promise<{
      state: EditorTomlState | null;
      error?: string;
    }> {
      const res = await fetchFn("/api/editor-toml/load");
      return res.json();
    },
  };
}
