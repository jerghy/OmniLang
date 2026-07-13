// InlineConditionEditor — click-to-edit condition expression, styled like Computed editor.
// Shared by ConditionalBlockEditor and BranchConditionEditor.

import { useState } from "react";

interface InlineConditionEditorProps {
  value: string;
  placeholder: string;
  disabled?: boolean;
  onSave: (v: string) => void;
}

const S: Record<string, React.CSSProperties> = {
  idle: {
    display: "block",
    padding: "2px 6px",
    borderRadius: 4,
    cursor: "pointer",
    fontSize: 11,
    fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, Monaco, 'Courier New', monospace",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    border: "1px solid transparent",
    transition: "border-color 0.15s",
  },
  disabled: {
    fontSize: 11,
    color: "#9CA3AF",
    fontStyle: "italic",
  },
  editing: {
    width: "100%",
    padding: "4px 8px",
    border: "1px solid #818CF8",
    borderRadius: 4,
    fontSize: 11,
    fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, Monaco, 'Courier New', monospace",
    resize: "vertical",
    boxSizing: "border-box" as const,
    outline: "none",
  },
};

export function InlineConditionEditor({
  value,
  placeholder,
  disabled,
  onSave,
}: InlineConditionEditorProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);

  if (disabled) {
    return <span style={S.disabled}>else（无条件）</span>;
  }

  if (editing) {
    const commit = () => {
      onSave(draft);
      setEditing(false);
    };
    return (
      <textarea
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); commit(); }
          if (e.key === "Escape") setEditing(false);
        }}
        placeholder={placeholder}
        rows={2}
        style={S.editing}
      />
    );
  }

  const hasContent = value.length > 0;
  return (
    <span
      onClick={() => { setDraft(value); setEditing(true); }}
      title="Click to edit"
      style={{
        ...S.idle,
        background: hasContent ? "#EEF0FF" : "#F9FAFB",
        color: hasContent ? "#6366F1" : "#9CA3AF",
      }}
      onMouseEnter={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "#C4B5FD";
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "transparent";
      }}
    >
      {hasContent ? value : "click to edit"}
    </span>
  );
}
