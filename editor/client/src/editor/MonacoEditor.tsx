/**
 * Monaco Editor wrapper for OmniLang (.omni) files.
 *
 * Registers the OmniLang language on mount, sets up completion provider,
 * and renders the Monaco code editor. Falls back to a plain textarea
 * if Monaco fails to load (CDN unavailable, etc.).
 */

import React, { useState, useRef, useImperativeHandle, forwardRef } from "react";
import Editor from "@monaco-editor/react";
import type * as Monaco from "monaco-editor";
import { OMNILANG_LANGUAGE_ID, conf, language } from "./omniLangTokens";
import { createCompletionProvider } from "./omniCompletion";

export interface MonacoEditorHandle {
  /** Insert text at current cursor position (supports snippet placeholders $1, $2) */
  insertSnippet: (text: string) => void;
  /** Get current cursor offset */
  getCursorOffset: () => number;
  getValue: () => string;
}

interface MonacoEditorProps {
  value: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
}

export const MonacoEditor = forwardRef<MonacoEditorHandle, MonacoEditorProps>(
  function MonacoEditor({ value, onChange, readOnly = false }, ref) {
  const [fallback, setFallback] = useState(false);
  const editorRef = useRef<Monaco.editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<typeof Monaco | null>(null);

  useImperativeHandle(ref, () => ({
    insertSnippet(text: string) {
      const ed = editorRef.current;
      const monaco = monacoRef.current;
      if (!ed || !monaco) return;

      // Expand snippet placeholders
      const snippet = text.replace(/\$(\d+)/g, (_, n) => `\${${n}}`);
      // Use executeEdits for clean insertion with undo support
      const selection = ed.getSelection();
      if (!selection) return;
      const pos = selection.getPosition();
      void { major: 1, minor: 1 };
      ed.executeEdits("snippet", [
        {
          range: new monaco.Range(pos.lineNumber, pos.column, pos.lineNumber, pos.column),
          text: snippet,
          forceMoveMarkers: true,
        },
      ]);
      ed.focus();
    },
    getCursorOffset() {
      const ed = editorRef.current;
      if (!ed) return 0;
      const pos = ed.getPosition();
      if (!pos) return 0;
      const model = ed.getModel();
      if (!model) return 0;
      return model.getOffsetAt(pos);
    },
    getValue() {
      return editorRef.current?.getValue() ?? "";
    },
  }), []);

  // ── Language Registration ──────────────────────

  const handleBeforeMount = (monaco: typeof Monaco) => {
    monacoRef.current = monaco;

    // Register OmniLang language
    monaco.languages.register({ id: OMNILANG_LANGUAGE_ID, extensions: [".omni"], aliases: ["OmniLang", "omni"] });
    monaco.languages.setLanguageConfiguration(OMNILANG_LANGUAGE_ID, conf);
    monaco.languages.setMonarchTokensProvider(OMNILANG_LANGUAGE_ID, language);

    // Register completion provider
    monaco.languages.registerCompletionItemProvider(
      OMNILANG_LANGUAGE_ID,
      createCompletionProvider(),
    );

    // Define a theme that matches the editor aesthetic
    monaco.editor.defineTheme("omni-light", {
      base: "vs",
      inherit: true,
      rules: [
        { token: "keyword", foreground: "7C3AED", fontStyle: "bold" },
        { token: "type", foreground: "2563EB" },
        { token: "string", foreground: "059669" },
        { token: "number", foreground: "D97706" },
        { token: "comment", foreground: "6B7280", fontStyle: "italic" },
        { token: "annotation", foreground: "DC2626" },
        { token: "identifier", foreground: "111827" },
        { token: "delimiter", foreground: "6B7280" },
        { token: "operator", foreground: "4F46E5" },
      ],
      colors: {
        "editor.background": "#fafafa",
        "editor.lineHighlightBackground": "#F3F4F6",
        "editor.selectionBackground": "#DBEAFE",
        "editorCursor.foreground": "#4F46E5",
        "editorLineNumber.foreground": "#D1D5DB",
        "editorLineNumber.activeForeground": "#6B7280",
      },
    });
  };

  const handleMount = (editor: Monaco.editor.IStandaloneCodeEditor) => {
    editorRef.current = editor;
    editor.getModel()?.updateOptions({ tabSize: 4, insertSpaces: true });
    editor.focus();
  };

  // ── Error boundary ─────────────────────────────

  if (fallback) {
    return (
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        readOnly={readOnly}
        style={{ width: "100%", height: "100%", border: "none", resize: "none", padding: 16, fontFamily: "monospace", fontSize: 13, outline: "none", tabSize: 4, background: readOnly ? "#f5f5f5" : "#fafafa", color: readOnly ? "#6B7280" : "inherit", cursor: readOnly ? "default" : "text" }}
        spellCheck={false}
      />
    );
  }

  return (
    <ErrorCatcher onError={() => setFallback(true)}>
      <Editor
        height="100%"
        language={OMNILANG_LANGUAGE_ID}
        value={value}
        onChange={(v) => onChange(v ?? "")}
        beforeMount={handleBeforeMount}
        onMount={handleMount}
        theme="omni-light"
        options={{
          readOnly,
          minimap: { enabled: false },
          fontSize: 13,
          fontFamily: "'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'SF Mono', Menlo, monospace",
          lineNumbers: "on",
          lineNumbersMinChars: 3,
          lineDecorationsWidth: 8,
          scrollBeyondLastLine: false,
          wordWrap: "on",
          wrappingIndent: "indent",
          folding: true,
          foldingStrategy: "indentation",
          renderLineHighlight: "line",
          cursorBlinking: "smooth",
          cursorSmoothCaretAnimation: "on",
          smoothScrolling: true,
          bracketPairColorization: { enabled: true },
          autoClosingBrackets: "always",
          autoClosingQuotes: "always",
          formatOnPaste: true,
          tabSize: 4,
          insertSpaces: true,
          suggest: {
            showKeywords: true,
            showSnippets: true,
            snippetsPreventQuickSuggestions: false,
          },
          quickSuggestions: true,
          parameterHints: { enabled: true },
        }}
        loading={
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#9ca3af", fontSize: 12 }}>
            加载编辑器中...
          </div>
        }
      />
    </ErrorCatcher>
  );
});

// ── Error boundary (class component needed for getDerivedStateFromError) ──

interface ErrorCatcherProps { children: React.ReactNode; onError: () => void; }
interface ErrorCatcherState { hasError: boolean; }

class ErrorCatcher extends React.Component<ErrorCatcherProps, ErrorCatcherState> {
  constructor(props: ErrorCatcherProps) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(): ErrorCatcherState {
    return { hasError: true };
  }

  componentDidCatch(error: Error) {
    console.warn("Monaco editor failed to load, falling back to textarea:", error.message);
    this.props.onError();
  }

  render() {
    if (this.state.hasError) return null;
    return this.props.children;
  }
}
