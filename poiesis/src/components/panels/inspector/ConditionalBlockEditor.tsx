// components/panels/inspector/ConditionalBlockEditor.tsx
// Full editor for a conditional block — click-to-edit condition expressions,
// child element lists, add/delete branch controls.

import { useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";
import { InlineConditionEditor } from "./InlineConditionEditor";

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
    return <div style={{ padding: 12, color: "#9CA3AF", fontSize: 12 }}>未选择条件块</div>;
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
              border: "1px solid #E5E7EB",
              borderRadius: 6,
              background: "#FAFAFA",
            }}
          >
            {/* Branch header */}
            <div style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              marginBottom: 8,
            }}>
              <span style={{
                fontSize: 12, fontWeight: 600,
                color: isElse ? "#9CA3AF" : "#7C3AED",
              }}>
                {branchLabel}
              </span>
              {isElif && (
                <button type="button"
                  onClick={() => handleDeleteBranch(i)}
                  title="删除分支"
                  style={{
                    border: "none", background: "transparent", cursor: "pointer",
                    color: "#9CA3AF", fontSize: 14, padding: "0 4px", lineHeight: 1,
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
            <div style={{ marginTop: 8, fontSize: 12, color: "#6b7280" }}>
              {branch.children.length === 0 ? (
                <span style={{ fontStyle: "italic", color: "#9CA3AF", fontSize: 12 }}>
                  空分支 — 右键此分支添加子组件
                </span>
              ) : (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center" }}>
                  <span style={{ fontSize: 12 }}>子元素:</span>
                  {branch.children.map((cid) => (
                    <span key={cid} style={{
                      padding: "1px 6px", background: "#EEF2FF", borderRadius: 3,
                      fontSize: 12, color: "#4F46E5",
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
        style={{
          width: "100%", padding: "8px 0", border: "1px dashed #C4B5FD",
          borderRadius: 6, background: "transparent", cursor: "pointer",
          color: "#7C3AED", fontSize: 12, fontWeight: 500,
        }}
      >
        ＋ 添加 @else if 分支
      </button>
    </div>
  );
}
