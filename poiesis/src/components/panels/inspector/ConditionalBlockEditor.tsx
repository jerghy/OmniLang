// components/panels/inspector/ConditionalBlockEditor.tsx
// Full editor for a conditional block — click-to-edit condition expressions,
// child element lists, add/delete branch controls.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";

interface ConditionalBlockEditorProps {
  blockId: string;
}

/** Inline click-to-edit condition expression. */
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
    if (e.key === "Escape") {
      setEditing(false);
    }
  };

  if (disabled) {
    return (
      <span style={{ color: "#9CA3AF", fontSize: 11, fontStyle: "italic" }}>
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
          width: "100%", padding: "4px 8px", border: "1px solid #818CF8",
          borderRadius: 4, fontSize: 11, fontFamily: "monospace",
          resize: "vertical", boxSizing: "border-box",
          outline: "none",
        }}
      />
    );
  }

  return (
    <span
      onClick={handleStart}
      title="点击编辑条件"
      style={{
        display: "block", padding: "4px 8px", borderRadius: 4,
        border: "1px solid transparent", cursor: "pointer",
        fontSize: 11, fontFamily: "monospace",
        color: value ? "#1e293b" : "#9CA3AF",
        background: "#F9FAFB",
        transition: "border-color 0.15s",
      }}
      onMouseEnter={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "#C4B5FD";
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "transparent";
      }}
    >
      {value || "click to edit"}
    </span>
  );
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
            key={i}
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
                fontSize: 11, fontWeight: 600,
                color: isElse ? "#9CA3AF" : "#7C3AED",
              }}>
                {branchLabel}
              </span>
              {isElif && (
                <button
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
            <div style={{ marginTop: 8, fontSize: 11, color: "#6b7280" }}>
              {branch.children.length === 0 ? (
                <span style={{ fontStyle: "italic", color: "#9CA3AF", fontSize: 10 }}>
                  空分支 — 右键此分支添加子组件
                </span>
              ) : (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center" }}>
                  <span style={{ fontSize: 10 }}>子元素:</span>
                  {branch.children.map((cid) => (
                    <span key={cid} style={{
                      padding: "1px 6px", background: "#EEF2FF", borderRadius: 3,
                      fontSize: 10, color: "#4F46E5",
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
      <button
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
