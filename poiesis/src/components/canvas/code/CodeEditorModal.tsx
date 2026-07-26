import { useState, useEffect, useCallback, useRef } from "react";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import {
  getOmniCompiler,
  runDiagnostics,
  applyDiagnostics,
} from "../../../services/monaco";
import { useMonacoEditor } from "../../../services/monaco/useMonacoEditor";
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
  const containerRef = useRef<HTMLDivElement>(null);
  const savedSourceRef = useRef(initialSource);

  // ── Monaco editor via shared hook ──────────────────────────────
  const { editor } = useMonacoEditor({
    containerRef,
    initialValue: initialSource,
    language,
    storeDefs,
    onChange: (value) => {
      sourceRef.current = value;
    },
  });

  // Debug: expose editor globally for Victauri testing
  useEffect(() => {
    (window as unknown as Record<string, unknown>).__monacoEditor = editor;
    return () => {
      (window as unknown as Record<string, unknown>).__monacoEditor = undefined;
    };
  }, [editor]);

  // ── WASM diagnostics via OmniCompiler ─────────────────────────
  useEffect(() => {
    if (!editor || !omniSource || !omniSourcePath) return;

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
  }, [editor, omniSource, omniSourcePath, allSources]);

  // Sync external value changes into the editor
  useEffect(() => {
    if (!editor) return;
    if (initialSource !== savedSourceRef.current) {
      savedSourceRef.current = initialSource;
      editor.setValue(initialSource);
    }
  }, [editor, initialSource]);

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
      <div
        onClick={onClose}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 9998,
          background: "rgba(0,0,0,0.4)",
        }}
      />
      {/* Dialog */}
      <div
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
        style={{
          position: "fixed",
          top: "50%",
          left: "50%",
          transform: "translate(-50%,-50%)",
          zIndex: 9999,
          background: "#fff",
          borderRadius: 12,
          boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          width: "80vw",
          height: "80vh",
          maxWidth: 1200,
          maxHeight: 900,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Title bar */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "14px 20px",
            borderBottom: "1px solid #e2e8f0",
          }}
        >
          <span style={{ fontSize: 14, fontWeight: 600, color: "#1e293b" }}>
            编辑代码块
          </span>
          <button
            onClick={onClose}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              fontSize: 18,
              color: "#94a3b8",
              padding: "4px 8px",
            }}
          >
            ✕
          </button>
        </div>

        {/* Monaco Editor area */}
        <div ref={containerRef} style={{ flex: 1, minHeight: 0 }} />

        {/* Description field */}
        <div
          style={{
            padding: "12px 20px",
            borderTop: "1px solid #e2e8f0",
          }}
        >
          <label
            style={{
              display: "block",
              fontSize: 11,
              fontWeight: 600,
              color: "#64748b",
              marginBottom: 4,
            }}
          >
            功能说明
          </label>
          <input
            type="text"
            value={localDescription}
            onChange={(e) => setLocalDescription(e.target.value)}
            placeholder="描述此代码块的功能..."
            style={{
              width: "100%",
              padding: "8px 12px",
              fontSize: 13,
              border: "1px solid #d1d5db",
              borderRadius: 6,
              fontFamily: "inherit",
              boxSizing: "border-box",
            }}
          />
        </div>

        {/* Action buttons */}
        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: 8,
            padding: "12px 20px",
            borderTop: "1px solid #e2e8f0",
          }}
        >
          <button
            onClick={onClose}
            style={{
              padding: "8px 20px",
              fontSize: 13,
              border: "1px solid #d1d5db",
              borderRadius: 6,
              background: "#fff",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            取消
          </button>
          <button
            onClick={handleSave}
            style={{
              padding: "8px 20px",
              fontSize: 13,
              fontWeight: 600,
              border: "none",
              borderRadius: 6,
              background: "#6366F1",
              color: "#fff",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            保存
          </button>
        </div>
      </div>
    </>
  );
}
