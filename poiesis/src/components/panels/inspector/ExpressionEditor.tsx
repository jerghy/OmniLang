// components/panels/inspector/ExpressionEditor.tsx
// Lightweight Monaco editor embed for editing inline event handler expressions.
// Shares Monaco initialization pattern with CodeEditorModal but is NOT a modal.

import { useEffect, useRef } from "react";
import * as monaco from "monaco-editor";

interface ExpressionEditorProps {
  initialValue: string;
  onChange: (value: string) => void;
  height?: number;
}

export function ExpressionEditor({
  initialValue,
  onChange,
  height = 260,
}: ExpressionEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const ignoreChangeRef = useRef(false);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  // Initialize Monaco editor (once)
  useEffect(() => {
    if (!containerRef.current) return;

    let editor: monaco.editor.IStandaloneCodeEditor | null = null;
    try {
      editor = monaco.editor.create(containerRef.current, {
        value: initialValue,
        language: "omnilang",
        theme: "vs-dark",
        fontSize: 13,
        minimap: { enabled: false },
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        automaticLayout: true,
      });
      editorRef.current = editor;

      editor.onDidChangeModelContent(() => {
        if (!ignoreChangeRef.current) {
          onChangeRef.current(editor!.getValue());
        }
      });
    } catch (err) {
      console.error("ExpressionEditor: failed to create Monaco editor", err);
      return;
    }

    return () => {
      editor.dispose();
      editorRef.current = null;
    };
    // Only initialize once — value synced via effects below
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Sync external value changes into the editor
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) return;
    const currentValue = editor.getValue();
    if (initialValue !== currentValue) {
      ignoreChangeRef.current = true;
      try {
        editor.setValue(initialValue);
      } finally {
        ignoreChangeRef.current = false;
      }
    }
  }, [initialValue]);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height,
        border: "1px solid #d1d5db",
        borderRadius: 6,
        overflow: "hidden",
      }}
    />
  );
}
