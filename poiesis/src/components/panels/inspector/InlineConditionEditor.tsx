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
    fontSize: 12,
    fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, Monaco, 'Courier New', monospace",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    border: "1px solid transparent",
    transition: "border-color 0.15s",
  },
  disabled: {
    fontSize: 12,
    color: "var(--ol-text-muted)",
    fontStyle: "italic",
  },
  editing: {
    width: "100%",
    padding: "4px 8px",
    border: "1px solid var(--ol-color-primary)",
    borderRadius: 4,
    fontSize: 12,
    fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, Monaco, 'Courier New', monospace",
    resize: "vertical",
    boxSizing: "border-box" as const,
    outline: "2px solid transparent",
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
    <button
      type="button"
      tabIndex={0}
      onClick={() => { setDraft(value); setEditing(true); }}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setDraft(value); setEditing(true); } }}
      title="Click to edit"
      style={{
        ...S.idle,
        background: hasContent ? "var(--ol-color-primary-subtle)" : "var(--ol-gray-50)",
        color: hasContent ? "var(--ol-color-primary)" : "var(--ol-text-muted)",
        border: "none",
        fontFamily: "inherit",
        fontSize: "inherit",
        cursor: "pointer",
      }}
      onMouseEnter={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "var(--ol-color-primary-muted)";
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLElement).style.borderColor = "transparent";
      }}
    >
      {hasContent ? value : "click to edit"}
    </button>
  );
}
