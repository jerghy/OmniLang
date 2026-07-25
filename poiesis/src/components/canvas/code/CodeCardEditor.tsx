// poiesis/src/components/canvas/code/CodeCardEditor.tsx
import { useCallback, useRef, useState } from "react";
import { LexicalComposer } from "@lexical/react/LexicalComposer";
import type { InitialConfigType } from "@lexical/react/LexicalComposer";
import { RichTextPlugin } from "@lexical/react/LexicalRichTextPlugin";
import { LexicalErrorBoundary } from "@lexical/react/LexicalErrorBoundary";
import { ContentEditable } from "@lexical/react/LexicalContentEditable";
import { HistoryPlugin } from "@lexical/react/LexicalHistoryPlugin";
import type { LexicalEditor } from "lexical";
import type { CodeOp } from "../../../core/types/code-op";
import { AutocompletePlugin } from "./lexical/plugins/AutocompletePlugin";
import { KeyboardPlugin } from "./lexical/plugins/KeyboardPlugin";
import { CodeOpSyncPlugin } from "./lexical/plugins/CodeOpSyncPlugin";
import { CardNode } from "./lexical/nodes/CardNode";
import { ContainerElementNode } from "./lexical/nodes/ContainerNode";
import { BlockElementNode } from "./lexical/nodes/BlockNode";
import { BodyLineNode } from "./lexical/nodes/BodyLineNode";
import { SlotCardNode } from "./lexical/nodes/SlotCardNode";
import { parseOpsWasm } from "../../../services/codegen/code-op-parser";
import { CodeEditorModal } from "./CodeEditorModal";
import styles from "./CodeCardEditor.module.css";

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  initialSource: string;  // for Monaco fallback
  language?: "typescript" | "omnilang";
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

const lexicalConfig = {
  namespace: "code-card-editor-v2",
  nodes: [CardNode, ContainerElementNode, BlockElementNode, BodyLineNode, SlotCardNode],
  onError: (error: Error, _editor: LexicalEditor) => {
    console.error("Lexical error:", error);
  },
};

export const CodeCardEditor: React.FC<Props> = ({
  elementId,
  initialOps,
  initialSource,
  language = "omnilang",
  onSave,
  onClose,
}) => {
  const [showMonaco, setShowMonaco] = useState(false);
  const opsRef = useRef<CodeOp[]>(initialOps);
  const [opsCount, setOpsCount] = useState(initialOps.length);

  const handleSync = useCallback((ops: CodeOp[]) => {
    opsRef.current = ops;
    setOpsCount(ops.length);
  }, []);

  const handleSave = useCallback(() => {
    onSave(opsRef.current);
  }, [onSave]);

  const handleMonacoSave = useCallback(async (source: string, _desc: string) => {
    try {
      const parsedOps = await parseOpsWasm(source);
      onSave(parsedOps);
    } catch (err) {
      console.error("Failed to parse Monaco source:", err);
      // Silently fall back — the user can switch back to card mode
    }
  }, [onSave]);

  // ── Monaco fallback mode ──
  if (showMonaco) {
    return (
      <CodeEditorModal
        elementId={elementId}
        initialSource={initialSource}
        initialDescription=""
        language={language}
        onSave={handleMonacoSave}
        onClose={() => setShowMonaco(false)}
      />
    );
  }

  // ── Lexical card editor ──
  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        {/* Header */}
        <div className={styles.header}>
          <h3 className={styles.title}>
            Code Card Editor — {opsCount} ops
          </h3>
          <div className={styles.actions}>
            <button
              type="button"
              className={styles.textModeBtn}
              onClick={() => setShowMonaco(true)}
            >
              文本模式
            </button>
            <button type="button" className={styles.saveBtn} onClick={handleSave}>
              Save
            </button>
            <button type="button" className={styles.cancelBtn} onClick={onClose}>
              Cancel
            </button>
          </div>
        </div>

        {/* Editor body */}
        <div className={styles.body}>
          <LexicalComposer initialConfig={lexicalConfig as InitialConfigType}>
            <RichTextPlugin
              ErrorBoundary={LexicalErrorBoundary}
              contentEditable={
                <ContentEditable className={styles.contentEditable} />
              }
              placeholder={
                <div style={{ color: "#94a3b8", fontSize: 12, fontStyle: "italic", padding: "8px 0" }}>
                  Type to add operations...
                </div>
              }
            />
            <HistoryPlugin />
            <KeyboardPlugin />
            <AutocompletePlugin />
            <CodeOpSyncPlugin initialOps={initialOps} onSync={handleSync} />
          </LexicalComposer>
        </div>
      </div>
    </div>
  );
};
