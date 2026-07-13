// components/panels/inspector/ConditionalBlockEditor.tsx
// Full editor for a conditional block — shows all branches with condition editing,
// child element lists, add/delete branch controls.

import { useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";

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
    <div style={{ padding: 12, fontSize: 12 }}>
      <div style={{ fontWeight: 600, marginBottom: 12, color: "#374151", fontSize: 13 }}>
        🔀 条件渲染
      </div>

      {condBlock.branches.map((branch, i) => {
        const isIf = branch.type === "if";
        const isElse = branch.type === "else";
        const isElif = branch.type === "elif";

        const branchLabel = isIf ? "@if" : isElse ? "@else" : "@else if";

        return (
          <div
            key={i}
            style={{
              marginBottom: 10,
              padding: "8px 10px",
              border: "1px solid #E5E7EB",
              borderRadius: 6,
              background: "#FAFAFA",
            }}
          >
            {/* Branch header */}
            <div style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              marginBottom: 6,
            }}>
              <span style={{
                fontSize: 11, fontWeight: 600, color: isElse ? "#6b7280" : "#7C3AED",
              }}>
                {branchLabel}
              </span>
              {isElif && (
                <button
                  onClick={() => handleDeleteBranch(i)}
                  title="删除此分支"
                  style={{
                    border: "none", background: "transparent", cursor: "pointer",
                    color: "#9CA3AF", fontSize: 14, padding: "0 4px", lineHeight: 1,
                  }}
                >
                  ×
                </button>
              )}
            </div>

            {/* Condition textarea */}
            {isElse ? (
              <div style={{
                padding: "4px 8px", background: "#F3F4F6", borderRadius: 4,
                color: "#9CA3AF", fontSize: 11, marginBottom: 6,
              }}>
                else（无条件）
              </div>
            ) : (
              <textarea
                value={branch.condition}
                onChange={(e) => handleUpdateCondition(i, e.target.value)}
                placeholder={isIf ? "输入条件，如: count > 5" : "输入条件，如: status == \"error\""}
                rows={2}
                style={{
                  width: "100%", padding: "4px 8px", border: "1px solid #D1D5DB",
                  borderRadius: 4, fontSize: 11, fontFamily: "monospace",
                  resize: "vertical", boxSizing: "border-box", marginBottom: 6,
                }}
              />
            )}

            {/* Child elements */}
            <div style={{ fontSize: 11, color: "#6b7280" }}>
              {branch.children.length === 0 ? (
                <span style={{ fontStyle: "italic", color: "#9CA3AF" }}>
                  空分支 — 右键此分支添加子组件
                </span>
              ) : (
                <span>
                  子元素: {branch.children.map((cid, j) => (
                    <span key={cid} style={{
                      display: "inline-block", marginLeft: j > 0 ? 4 : 0,
                      padding: "1px 6px", background: "#EEF2FF", borderRadius: 3,
                      fontSize: 10,
                    }}>
                      {cid.split(".").pop()}
                    </span>
                  ))}
                </span>
              )}
            </div>
          </div>
        );
      })}

      {/* Add branch button */}
      <button
        onClick={handleAddBranch}
        style={{
          width: "100%", padding: "6px 0", border: "1px dashed #C4B5FD",
          borderRadius: 6, background: "transparent", cursor: "pointer",
          color: "#7C3AED", fontSize: 11, fontWeight: 500,
        }}
      >
        ＋ 添加 @else if 分支
      </button>
    </div>
  );
}
