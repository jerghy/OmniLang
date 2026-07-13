// components/panels/inspector/BranchConditionEditor.tsx
// Editor for a single conditional branch — click-to-edit condition, child elements, delete.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";

interface BranchConditionEditorProps {
  blockId: string;
  branchIndex: number;
}

/** Inline click-to-edit condition expression (shared with ConditionalBlockEditor). */
function InlineConditionEditor({
  value,
  placeholder,
  disabled,
  onSave,
}: {
  value: string;
  placeholder: string;
  disabled?: boolean;
  onSave: (v: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);

  const handleStart = () => {
    if (disabled) return;
    setDraft(value);
    setEditing(true);
  };

  const handleCommit = () => {
    onSave(draft);
    setEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleCommit();
    }
    if (e.key === "Escape") setEditing(false);
  };

  if (disabled) {
    return (
      <span style={{ color: "#9CA3AF", fontSize: 12, fontStyle: "italic" }}>
        else（无条件）
      </span>
    );
  }

  if (editing) {
    return (
      <textarea
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={handleCommit}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        rows={2}
        style={{
          width: "100%", padding: "6px 8px", border: "1px solid #818CF8",
          borderRadius: 4, fontSize: 12, fontFamily: "monospace",
          resize: "vertical", boxSizing: "border-box", outline: "none",
        }}
      />
    );
  }

  return (
    <span
      onClick={handleStart}
      title="点击编辑条件"
      style={{
        display: "block", padding: "6px 8px", borderRadius: 4,
        border: "1px solid #E5E7EB", cursor: "pointer",
        fontSize: 12, fontFamily: "monospace",
        color: value ? "#1e293b" : "#9CA3AF",
        background: "#F9FAFB",
      }}
      onMouseEnter={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "#C4B5FD";
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "#E5E7EB";
      }}
    >
      {value || "click to edit"}
    </span>
  );
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
        <label style={{ display: "block", marginBottom: 4, color: "#6b7280", fontSize: 11 }}>
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
        <span style={{ color: "#6b7280", fontSize: 11 }}>子元素 ({branch.children.length})</span>
        {branch.children.length === 0 ? (
          <div style={{ color: "#9CA3AF", fontSize: 11, fontStyle: "italic", marginTop: 4 }}>
            右键此分支 → 添加子组件
          </div>
        ) : (
          <div style={{ fontSize: 11, color: "#374151", marginTop: 4, display: "flex", flexWrap: "wrap", gap: 4 }}>
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
