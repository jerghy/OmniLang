// components/design/DesignToolbar.tsx
// Toolbar row with undo/redo, save, open project, and code view buttons.

import type { CSSProperties, ReactNode } from "react";
import styles from "./DesignToolbar.module.css";

interface DesignToolbarProps {
  isDirty: boolean;
  saving: boolean;
  onUndo: () => void;
  onRedo?: () => void;
  onSave: () => void;
  onOpenProject: () => void;
  showCode: boolean;
  onToggleCode: () => void;
  saveError: string | null;
  style?: CSSProperties;
  children?: ReactNode;
}

export function DesignToolbar({
  isDirty,
  saving,
  onUndo,
  onRedo,
  onSave,
  onOpenProject,
  showCode,
  onToggleCode,
  saveError,
  style,
  children,
}: DesignToolbarProps) {
  return (
    <div className={styles.toolbar} style={style}>
      <button type="button"
        onClick={onUndo}
        title="撤销 (Ctrl+Z)"
        aria-label="撤销"
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
        onClick={onRedo}
        title="重做 (Ctrl+Shift+Z)"
        aria-label="重做"
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
        ↪
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
          color: isDirty ? "var(--ol-color-danger)" : "var(--ol-text-secondary)",
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
        style={{ color: "var(--ol-text-secondary)" }}
      >
        📂
      </button>
      <button type="button"
        onClick={onToggleCode}
        title={showCode ? "Hide generated code" : "Show generated code"}
        style={{
          padding: "2px 10px",
          fontSize: 12,
          border: showCode ? "1px solid var(--ol-color-primary)" : "1px solid var(--ol-border)",
          borderRadius: 4,
          background: showCode ? "var(--ol-color-primary)" : "transparent",
          cursor: "pointer",
          lineHeight: 1,
          color: showCode ? "var(--ol-surface)" : "var(--ol-text-secondary)",
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
            color: "var(--ol-color-danger)",
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
