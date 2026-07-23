// components/design/DesignToolbar.tsx
// Toolbar row with undo, save, open project, and code view buttons.

import type { ReactNode } from "react";
import styles from "./DesignToolbar.module.css";

interface DesignToolbarProps {
  isDirty: boolean;
  saving: boolean;
  onUndo: () => void;
  onSave: () => void;
  onOpenProject: () => void;
  showCode: boolean;
  onToggleCode: () => void;
  saveError: string | null;
  children?: ReactNode;
}

export function DesignToolbar({
  isDirty,
  saving,
  onUndo,
  onSave,
  onOpenProject,
  showCode,
  onToggleCode,
  saveError,
  children,
}: DesignToolbarProps) {
  return (
    <div className={styles.toolbar}>
      <button type="button"
        onClick={onUndo}
        title="撤销 (Ctrl+Z)"
        style={{
          padding: "2px 8px",
          fontSize: 13,
          border: "1px solid transparent",
          borderRadius: 4,
          background: "transparent",
          cursor: "pointer",
          lineHeight: 1,
        }}
      >
        ↩
      </button>
      <button type="button"
        onClick={onSave}
        disabled={saving}
        title="保存项目 (Ctrl+S)"
        style={{
          padding: "2px 8px",
          fontSize: 13,
          border: "1px solid transparent",
          borderRadius: 4,
          background: "transparent",
          cursor: saving ? "wait" : "pointer",
          lineHeight: 1,
          color: isDirty ? "#DC2626" : "#6B7280",
          fontWeight: isDirty ? 700 : 400,
          opacity: saving ? 0.6 : 1,
        }}
      >
        {saving ? "⏳" : "💾"}
      </button>
      <button type="button"
        onClick={onOpenProject}
        title="打开项目"
        className={styles.toolBtn}
        style={{ color: "#6B7280" }}
      >
        📂
      </button>
      <button type="button"
        onClick={onToggleCode}
        title={showCode ? "Hide generated code" : "Show generated code"}
        style={{
          padding: "2px 10px",
          fontSize: 12,
          border: showCode ? "1px solid #6366f1" : "1px solid #e2e8f0",
          borderRadius: 4,
          background: showCode ? "#6366f1" : "transparent",
          cursor: "pointer",
          lineHeight: 1,
          color: showCode ? "#fff" : "#6B7280",
          fontWeight: showCode ? 600 : 400,
          marginLeft: 2,
        }}
      >
        {showCode ? "Code ▾" : "Code ▸"}
      </button>
      {children}
      {saveError && (
        <span
          title={saveError}
          style={{
            marginLeft: 8,
            fontSize: 12,
            color: "#dc2626",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
            maxWidth: 300,
          }}
        >
          ⚠ {saveError}
        </span>
      )}
    </div>
  );
}
