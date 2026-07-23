// components/design/ClassEditorSidebar.tsx
// Sidebar listing CSS classes with selection, rename, add, and remove.

import { useMemo } from "react";
import type { DesignClass } from "../../core/types/design";
import styles from "./ClassEditorSidebar.module.css";

// ── Styles ──

const SIDEBAR_STYLE: React.CSSProperties = {
  width: 180,
  borderRight: "1px solid var(--dp-border, var(--ol-border))",
  display: "flex",
  flexDirection: "column",
  flexShrink: 0,
  background: "var(--dp-surface, var(--ol-surface))",
};

const SIDEBAR_HEADER: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, var(--ol-text-secondary))",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  padding: "10px 10px 6px",
};

const CLASS_ITEM_BASE: React.CSSProperties = {
  padding: "6px 10px",
  fontSize: 12,
  cursor: "pointer",
  color: "var(--dp-text-secondary, var(--ol-text-secondary))",
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  borderRadius: "var(--dp-radius-sm, 6px)",
  margin: "1px 6px",
  transition: "background 100ms ease, color 100ms ease",
};

const CLASS_ITEM_ACTIVE: React.CSSProperties = {
  ...CLASS_ITEM_BASE,
  background: "var(--dp-accent-subtle, var(--ol-color-primary-subtle))",
  color: "var(--dp-accent, var(--ol-color-primary))",
  fontWeight: 600,
};

const REMOVE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "var(--dp-text-muted, var(--ol-text-muted))",
  fontSize: 14,
  padding: "2px 4px",
  lineHeight: 1,
  flexShrink: 0,
  borderRadius: 3,
};

// ── Props ──

interface ClassEditorSidebarProps {
  classes: DesignClass[];
  selectedClassId: string | null;
  editingClassName: string | null;
  onSelectClass: (id: string) => void;
  onAddClass: () => void;
  onRemoveClass: (e: React.MouseEvent, id: string) => void;
  onClassNameDone: (id: string, name: string) => void;
  onEditingClassNameChange: (id: string | null) => void;
}

export function ClassEditorSidebar({
  classes,
  selectedClassId,
  editingClassName,
  onSelectClass,
  onAddClass,
  onRemoveClass,
  onClassNameDone,
  onEditingClassNameChange,
}: ClassEditorSidebarProps) {
  return (
    <div style={SIDEBAR_STYLE}>
      <div style={SIDEBAR_HEADER}>Classes ({classes.length})</div>
      <div style={{ flex: 1, overflow: "auto" }}>
        {classes.map((cls) => (
          <button
            key={cls.id}
            type="button"
            onClick={(e) => { if ((e.target as HTMLElement).getAttribute('data-action') === 'remove-class') { onRemoveClass(e as unknown as React.MouseEvent, cls.id); } else { onSelectClass(cls.id); } }}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelectClass(cls.id); } }}
            style={cls.id === selectedClassId ? CLASS_ITEM_ACTIVE : CLASS_ITEM_BASE}
          >
            <span
              className={styles.classNameSpan}
              onDoubleClick={() => onEditingClassNameChange(cls.id)}
            >
              {cls.name}
            </span>
            <span data-action="remove-class" style={REMOVE_BTN} title="Remove class">
              &times;
            </span>
          </button>
        ))}
      </div>
      <button type="button"
        onClick={onAddClass}
        className={styles.addClassBtn}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLElement).style.color = "var(--dp-accent, var(--ol-color-primary))";
          (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-accent, var(--ol-color-primary))";
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLElement).style.color = "var(--dp-text-muted, var(--ol-text-muted))";
          (e.currentTarget as HTMLElement).style.borderColor = "var(--dp-border, var(--ol-border))";
        }}
      >
        + Add Class
      </button>
    </div>
  );
}
