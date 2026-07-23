// components/panels/inspector/BranchConditionEditor.tsx
// Editor for a single conditional branch — click-to-edit condition, child elements, delete.

import { useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";
import { InlineConditionEditor } from "./InlineConditionEditor";

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

  const handleUpdateCondition = useCallback(
    (condition: string) => {
      usePoiesisStore.getState().updateBranchCondition(blockId, branchIndex, condition);
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
      <div style={{ fontWeight: 600, marginBottom: 12, color: "#374151" }}>
        {isIf ? "🔀 @if 分支" : isElse ? "🔀 @else 分支" : "🔀 @else if 分支"}
      </div>

      {/* Condition — click to edit */}
      <div style={{ marginBottom: 12 }}>
        <label style={{ display: "block", marginBottom: 4, color: "#6b7280", fontSize: 12 }}>
          条件表达式
        </label>
        <InlineConditionEditor
          value={branch.condition}
          placeholder={isIf ? "e.g. count > 5" : "e.g. status == \"error\""}
          disabled={isElse}
          onSave={handleUpdateCondition}
        />
      </div>

      {/* Child elements */}
      <div style={{ marginBottom: 12 }}>
        <span style={{ color: "#6b7280", fontSize: 12 }}>子元素 ({branch.children.length})</span>
        {branch.children.length === 0 ? (
          <div style={{ color: "#9CA3AF", fontSize: 12, fontStyle: "italic", marginTop: 4 }}>
            右键此分支 → 添加子组件
          </div>
        ) : (
          <div style={{ fontSize: 12, color: "#374151", marginTop: 4, display: "flex", flexWrap: "wrap", gap: 4 }}>
            {branch.children.map((cid) => (
              <span key={cid} style={{
                padding: "2px 8px", background: "#EEF2FF", borderRadius: 3, color: "#4F46E5",
              }}>
                {cid.split(".").pop()}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Delete — only @else if */}
      {!isIf && !isElse && (
        <button type="button"
          onClick={handleDeleteBranch}
          style={{
            padding: "4px 12px", border: "1px solid #FCA5A5", borderRadius: 4,
            background: "#FEF2F2", color: "#b91c1c", cursor: "pointer", fontSize: 12,
          }}
        >
          删除此分支
        </button>
      )}
    </div>
  );
}
