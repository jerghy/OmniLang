import { useEffect, useMemo, useRef, useState } from "react";
import type * as monaco from "monaco-editor";
import type { StoreDefinition } from "../../core/types/store";
import {
  initOmniLangLanguage,
  initOmniLangSnippets,
  injectOmniLangTypes,
} from "./index";

// ── One-time initialization (guards prevent duplicate calls) ────────
let monacoServicesInitialized = false;

function initMonacoServices(storeDefs?: StoreDefinition[]): void {
  if (monacoServicesInitialized) {
    // Still refresh dynamic store types on each call
    if (storeDefs) injectOmniLangTypes(storeDefs);
    return;
  }
  monacoServicesInitialized = true;
  initOmniLangLanguage();
  initOmniLangSnippets();
  injectOmniLangTypes(storeDefs);
}

export interface UseMonacoEditorConfig {
  containerRef: React.RefObject<HTMLDivElement | null>;
  initialValue: string;
  language: "typescript" | "omnilang";
  storeDefs?: StoreDefinition[];
  /** Called on every content change with current editor value */
  onChange?: (value: string) => void;
}

export interface UseMonacoEditorResult {
  editor: monaco.editor.IStandaloneCodeEditor | null;
  getValue: () => string;
  setValue: (value: string) => void;
}

/**
 * React hook that encapsulates Monaco editor creation and lifecycle.
 * Creates exactly ONE Monaco editor instance (via dynamic import) and
 * cleans up on unmount.
 */
export function useMonacoEditor(
  config: UseMonacoEditorConfig,
): UseMonacoEditorResult {
  const { containerRef, initialValue, language, storeDefs, onChange } = config;
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const [, forceUpdate] = useState(0);

  // Derive a stable key from store names to control re-injection of types
  const storeNamesKey = useMemo(
    () => storeDefs?.map((s) => s.name).sort().join(",") ?? "",
    [storeDefs],
  );

  // ── Initialize Monaco services (once per session) ─────────────
  useEffect(() => {
    initMonacoServices(storeDefs);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [storeNamesKey]);

  // ── Create Monaco editor (once) ───────────────────────────────
  useEffect(() => {
    if (!containerRef.current) return;
    let cancelled = false;

    import("monaco-editor").then((monacoModule) => {
      if (cancelled || !containerRef.current) return;
      const editor = monacoModule.editor.create(containerRef.current, {
        value: initialValue,
        language,
        theme: "vs-dark",
        fontSize: 13,
        minimap: { enabled: false },
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        automaticLayout: true,
      });
      editorRef.current = editor;
      // Trigger re-render so consumers see the non-null editor
      forceUpdate((n) => n + 1);

      if (onChange) {
        editor.onDidChangeModelContent(() => onChange(editor.getValue()));
      }
    });

    return () => {
      cancelled = true;
      editorRef.current?.dispose();
      editorRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    editor: editorRef.current,
    getValue: () => editorRef.current?.getValue() ?? "",
    setValue: (value) => editorRef.current?.setValue(value),
  };
}
