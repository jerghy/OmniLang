import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import type * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import styles from "./CodeEditorModal.module.css";
import { initOmniLangLanguage } from "../../../services/monaco/omnilang-language";
import { initOmniLangSnippets } from "../../../services/monaco/omnilang-snippets";
import { injectOmniLangTypes } from "../../../services/monaco/omnilang-types";
import {
  getOmniCompiler,
  runDiagnostics,
  applyDiagnostics,
} from "../../../services/monaco/diagnostics";
import type { StoreDefinition } from "../../../core/types/store";

// Configure Monaco workers for Vite
(self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
  getWorker(_: string, label: string) {
    if (label === "typescript" || label === "javascript") return new tsWorker();
    if (label === "css") return new cssWorker();
    if (label === "html") return new htmlWorker();
    if (label === "json") return new jsonWorker();
    return new editorWorker();
  },
};

// ── One-time initialization (guards prevent duplicate calls) ────────
let monacoServicesInitialized = false;

async function initMonacoServices(storeDefs?: StoreDefinition[]): Promise<void> {
  if (monacoServicesInitialized) {
    // Still refresh dynamic store types on each call
    if (storeDefs) await injectOmniLangTypes(storeDefs);
    return;
  }
  monacoServicesInitialized = true;
  initOmniLangLanguage();
  initOmniLangSnippets();
  await injectOmniLangTypes(storeDefs);
}

interface CodeEditorModalProps {
  elementId: string;
  initialSource: string;
  initialDescription: string;
  /** Monaco language mode — defaults to "typescript"; set "omnilang" for .omni syntax highlighting */
  language?: "typescript" | "omnilang";
  /** Store definitions for dynamic type injection into TS IntelliSense */
  storeDefs?: StoreDefinition[];
  /** Full .omni source for WASM diagnostics (when editing within a known file context) */
  omniSourcePath?: string;
  omniSource?: string;
  /** All project .omni sources for multi-file cross-file diagnostics */
  allSources?: Record<string, string>;
  onSave: (source: string, description: string) => void;
  onClose: () => void;
}

export function CodeEditorModal({
  initialSource,
  initialDescription,
  language = "typescript",
  storeDefs,
  omniSourcePath,
  omniSource,
  allSources,
  onSave,
  onClose,
}: CodeEditorModalProps) {
  const [localDescription, setLocalDescription] = useState(initialDescription);
  const sourceRef = useRef(initialSource);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const savedSourceRef = useRef(initialSource);

  // ── Initialize Monaco services (once per session) ─────────────
  // Use a stable key derived from store names to control re-injection
  const storeNamesKey = useMemo(
    () => storeDefs?.map((s) => s.name).sort().join(",") ?? "",
    [storeDefs],
  );

  useEffect(() => {
    initMonacoServices(storeDefs);
    // Only re-inject when the set of stores changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [storeNamesKey]);

  // ── WASM diagnostics via OmniCompiler ─────────────────────────
  useEffect(() => {
    if (!omniSource || !omniSourcePath) return;
    const editor = editorRef.current;
    if (!editor) return;

    let timer: ReturnType<typeof setTimeout>;

    const runDiagnosticsDebounced = async () => {
      const compiler = await getOmniCompiler();
      if (!compiler) return;

      // Build the full sources map for multi-file cross-file analysis.
      // The current editor content (unsaved) takes precedence over allSources.
      const currentSource = editor.getValue();
      const mergedSources: Record<string, string> = { ...allSources };
      if (omniSourcePath) {
        mergedSources[omniSourcePath] = currentSource;
      }

      compiler.upsert_source(omniSourcePath, currentSource);
      const diagnostics = runDiagnostics(compiler, mergedSources);
      const model = editor.getModel();
      if (model) {
        applyDiagnostics(model, diagnostics, "omnilang-wasm");
      }
    };

    // Debounce — run diagnostics 500ms after last content change
    const disposable = editor.onDidChangeModelContent(() => {
      clearTimeout(timer);
      timer = setTimeout(runDiagnosticsDebounced, 500);
    });

    // Run once on mount
    runDiagnosticsDebounced();

    return () => {
      clearTimeout(timer);
      disposable.dispose();
    };
  }, [omniSource, omniSourcePath, allSources]);

  // Initialize Monaco editor (mount once)
  useEffect(() => {
    if (!containerRef.current) return;
    let cancelled = false;
    let editor: monaco.editor.IStandaloneCodeEditor | null = null;

    import("monaco-editor").then((monacoModule) => {
      if (cancelled || !containerRef.current) return;
      editor = monacoModule.editor.create(containerRef.current, {
        value: initialSource,
        language,
        theme: "vs-dark",
        fontSize: 13,
        minimap: { enabled: false },
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        automaticLayout: true,
      });
      editorRef.current = editor;
      sourceRef.current = initialSource;

      // Debug: expose editor globally for Victauri testing
      (window as unknown as Record<string, unknown>).__monacoEditor = editor;

      editor.onDidChangeModelContent(() => {
        sourceRef.current = editor!.getValue();
      });
    }).catch((err: unknown) => {
      console.error("CodeEditorModal: failed to load Monaco", err);
    });

    return () => {
      cancelled = true;
      (window as unknown as Record<string, unknown>).__monacoEditor = undefined;
      if (editor) {
        editor.dispose();
        editorRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Sync external value changes into the editor
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) return;
    if (initialSource !== savedSourceRef.current) {
      savedSourceRef.current = initialSource;
      editor.setValue(initialSource);
    }
  }, [initialSource]);

  // Escape to close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  const handleSave = useCallback(() => {
    onSave(sourceRef.current, localDescription);
  }, [localDescription, onSave]);

  // Ctrl+Enter / Cmd+Enter to save
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        handleSave();
      }
    },
    [handleSave],
  );

  return (
    <>
      {/* Backdrop */}
      <button
        type="button"
        tabIndex={0}
        aria-label="Close modal"
        onClick={onClose}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
        className={styles.backdrop}
      />
      {/* Dialog */}
      <dialog
        open
        aria-label="Code Editor Modal"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
        className={styles.dialog}
      >
        {/* Title bar */}
        <div className={styles.titleBar}>
          <span className={styles.titleText}>编辑代码块</span>
          <button type="button"
            onClick={onClose}
            className={styles.closeBtn}
          >
            ✕
          </button>
        </div>

        {/* Monaco Editor area */}
        <div ref={containerRef} style={{ flex: 1, minHeight: 0 }} />

        {/* Description field */}
        <div className={styles.descBar}>
          <label className={styles.descLabel} htmlFor="code-description-input">
            功能说明
          </label>
          <input
            id="code-description-input"
            type="text"
            value={localDescription}
            onChange={(e) => setLocalDescription(e.target.value)}
            placeholder="描述此代码块的功能..."
            className={styles.descInput}
          />
        </div>

        {/* Action buttons */}
        <div className={styles.actionBar}>
          <button type="button"
            onClick={onClose}
            className={styles.cancelBtn}
          >
            取消
          </button>
          <button type="button"
            onClick={handleSave}
            className={styles.saveBtn}
          >
            保存
          </button>
        </div>
      </dialog>
    </>
  );
}
