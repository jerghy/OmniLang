// components/panels/inspector/BranchConditionEditor.tsx
// Editor for conditional branch properties: condition expression, child elements, delete.

import React, { useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";

interface BranchConditionEditorProps {
  blockId: string;
  branchIndex: number;
}

export function BranchConditionEditor({ blockId, branchIndex }: BranchConditionEditorProps) {
  const condBlock = usePoiesisStore((s) =>
    s.elements.find((e) => e.id === blockId && e.nodeType === "conditional")
  ) as ConditionalBlockData | undefined;

  const branch = condBlock?.branches[branchIndex];
  const isElse = branch?.type === "else";
  const isIf = branch?.type === "if";

  const handleConditionChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      usePoiesisStore.getState().updateBranchCondition(blockId, branchIndex, e.target.value);
    },
    [blockId, branchIndex],
  );

  const handleDeleteBranch = useCallback(() => {
    usePoiesisStore.getState().removeConditionalBranch(blockId, branchIndex);
  }, [blockId, branchIndex]);

  if (!condBlock || !branch) {
    return <div style={{ padding: 12, color: "#9CA3AF", fontSize: 12 }}>未选择分支</div>;
  }

  return (
    <div style={{ padding: 12, fontSize: 12 }}>
      <div style={{ fontWeight: 600, marginBottom: 8, color: "#374151" }}>
        {isIf ? "🔀 @if 分支" : isElse ? "🔀 @else 分支" : "🔀 @else if 分支"}
      </div>

      {/* Condition expression */}
      <div style={{ marginBottom: 12 }}>
        <label style={{ display: "block", marginBottom: 4, color: "#6b7280", fontSize: 11 }}>
          条件表达式
        </label>
        {isElse ? (
          <div style={{
            padding: "6px 8px", background: "#F9FAFB", border: "1px solid #E5E7EB",
            borderRadius: 4, color: "#9CA3AF", fontSize: 12,
          }}>
            else（无条件）
          </div>
        ) : (
          <textarea
            value={branch.condition}
            onChange={handleConditionChange}
            placeholder="例如: count > 5"
            rows={2}
            style={{
              width: "100%", padding: "6px 8px", border: "1px solid #D1D5DB",
              borderRadius: 4, fontSize: 12, fontFamily: "monospace",
              resize: "vertical", boxSizing: "border-box",
            }}
          />
        )}
      </div>

      {/* Child elements list */}
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
          <span style={{ color: "#6b7280", fontSize: 11 }}>子元素 ({branch.children.length})</span>
        </div>
        {branch.children.length === 0 ? (
          <div style={{ color: "#9CA3AF", fontSize: 11, fontStyle: "italic" }}>
            右键分支 → 添加子组件
          </div>
        ) : (
          <div style={{ fontSize: 11, color: "#374151" }}>
            {branch.children.map((cid) => (
              <div key={cid} style={{ padding: "2px 0" }}>• {cid}</div>
            ))}
          </div>
        )}
      </div>

      {/* Delete branch button — only for @else if (not @if or @else) */}
      {!isIf && !isElse && (
        <button
          onClick={handleDeleteBranch}
          style={{
            padding: "4px 12px", border: "1px solid #FCA5A5", borderRadius: 4,
            background: "#FEF2F2", color: "#DC2626", cursor: "pointer", fontSize: 11,
          }}
        >
          删除此分支
        </button>
      )}
    </div>
  );
}
