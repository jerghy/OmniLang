// components/panels/inspector/ConditionalBlockEditor.tsx
// Full editor for a conditional block — click-to-edit condition expressions,
// child element lists, add/delete branch controls.

import { useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";
import { InlineConditionEditor } from "./InlineConditionEditor";
import styles from "./ConditionalBlockEditor.module.css";

interface ConditionalBlockEditorProps {
  blockId: string;
}

export function ConditionalBlockEditor({ blockId }: ConditionalBlockEditorProps) {
  const condBlock = usePoiesisStore((s) =>
    s.elements.find((e) => e.id === blockId && e.nodeType === "conditional")
  ) as ConditionalBlockData | undefined;

  const handleAddBranch = useCallback(() => {
    usePoiesisStore.getState().addConditionalBranch(blockId, "elif");
  }, [blockId]);

  const handleUpdateCondition = useCallback(
    (branchIndex: number, condition: string) => {
      usePoiesisStore.getState().updateBranchCondition(blockId, branchIndex, condition);
    },
    [blockId],
  );

  const handleDeleteBranch = useCallback(
    (branchIndex: number) => {
      usePoiesisStore.getState().removeConditionalBranch(blockId, branchIndex);
    },
    [blockId],
  );

  if (!condBlock) {
    return <div style={{ padding: 12, color: "var(--ol-text-muted)", fontSize: 12 }}>未选择条件块</div>;
  }

  return (
    <div style={{ padding: "0", fontSize: 12 }}>
      {condBlock.branches.map((branch, i) => {
        const isIf = branch.type === "if";
        const isElse = branch.type === "else";
        const isElif = branch.type === "elif";
        const branchLabel = isIf ? "@if" : isElse ? "@else" : "@else if";

        return (
          <div
            key={branch.type + "-" + branch.condition}
            style={{
              marginBottom: 8,
              padding: 10,
              border: "1px solid var(--ol-border)",
              borderRadius: 6,
              background: "var(--ol-gray-50)",
            }}
          >
            {/* Branch header */}
            <div style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              marginBottom: 8,
            }}>
              <span style={{
                fontSize: 12, fontWeight: 600,
                color: isElse ? "var(--ol-text-muted)" : "var(--ol-color-primary)",
              }}>
                {branchLabel}
              </span>
              {isElif && (
                <button type="button"
                  onClick={() => handleDeleteBranch(i)}
                  title="删除分支"
                  style={{
                    border: "none", background: "transparent", cursor: "pointer",
                    color: "var(--ol-text-muted)", fontSize: 14, padding: "0 4px", lineHeight: 1,
                  }}
                >
                  ×
                </button>
              )}
            </div>

            {/* Condition — click to edit */}
            <InlineConditionEditor
              value={branch.condition}
              placeholder={isIf ? "e.g. count > 5" : "e.g. status == \"error\""}
              disabled={isElse}
              onSave={(v) => handleUpdateCondition(i, v)}
            />

            {/* Child elements */}
            <div style={{ marginTop: 8, fontSize: 12, color: "var(--ol-text-secondary)" }}>
              {branch.children.length === 0 ? (
                <span style={{ fontStyle: "italic", color: "var(--ol-text-muted)", fontSize: 12 }}>
                  空分支 — 右键此分支添加子组件
                </span>
              ) : (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center" }}>
                  <span style={{ fontSize: 12 }}>子元素:</span>
                  {branch.children.map((cid) => (
                    <span key={cid} style={{
                      padding: "1px 6px", background: "var(--ol-color-primary-subtle)", borderRadius: 3,
                      fontSize: 12, color: "var(--ol-color-primary-hover)",
                    }}>
                      {cid.split(".").pop()}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </div>
        );
      })}

      {/* Add branch button */}
      <button type="button"
        onClick={handleAddBranch}
        className={styles.addBranchBtn}
      >
        ＋ 添加 @else if 分支
      </button>
    </div>
  );
}
