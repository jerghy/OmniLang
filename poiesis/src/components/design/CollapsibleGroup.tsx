// components/design/CollapsibleGroup.tsx
// Card-like collapsible property group with Poiesis design panel styling.

import React, { useState, useCallback, useRef } from "react";
import styles from "./CollapsibleGroup.module.css";

interface CollapsibleGroupProps {
  title: string;
  summary: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
  onDoubleClick?: () => void;
}

export const CollapsibleGroup = React.memo(function CollapsibleGroup({
  title,
  summary,
  defaultOpen = false,
  children,
  onDoubleClick,
}: CollapsibleGroupProps) {
  const [open, setOpen] = useState(defaultOpen);
  const [hover, setHover] = useState(false);

  const clickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingToggleRef = useRef(false);

  const handleHeaderClick = useCallback(() => {
    if (clickTimerRef.current !== null) {
      // Second click within double-click window
      clearTimeout(clickTimerRef.current);
      clickTimerRef.current = null;
      // Undo the toggle from the first click
      if (pendingToggleRef.current) {
        setOpen((v) => !v);
        pendingToggleRef.current = false;
      }
      onDoubleClick?.();
    } else {
      // First click — toggle immediately
      setOpen((v) => !v);
      pendingToggleRef.current = true;
      clickTimerRef.current = setTimeout(() => {
        clickTimerRef.current = null;
        pendingToggleRef.current = false;
      }, 300);
    }
  }, [onDoubleClick]);

  return (
    <div
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      className={styles.container}
      style={{
        boxShadow: open
          ? "var(--dp-shadow, 0 1px 3px rgba(99,102,241,0.06))"
          : hover
            ? "var(--dp-shadow, 0 1px 3px rgba(99,102,241,0.06))"
            : "var(--dp-shadow-sm, 0 1px 2px rgba(99,102,241,0.04))",
      }}
    >
      <button
        type="button"
        tabIndex={0}
        onClick={handleHeaderClick}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleHeaderClick(); } }}
        className={styles.headerBtn}
        style={{
          color: open ? "var(--dp-text, var(--ol-gray-700))" : "var(--dp-text-secondary, var(--ol-text-secondary))",
          background: hover ? "var(--dp-surface-hover, var(--ol-color-primary-subtle))" : "transparent",
        }}
      >
        <span
          className={styles.arrow}
          style={{
            color: open ? "var(--dp-accent, var(--ol-color-primary))" : "var(--dp-text-muted, var(--ol-text-muted))",
            transform: open ? "rotate(90deg)" : "rotate(0deg)",
          }}
        >
          ▶
        </span>
        <span className={styles.title}>
          {title}
        </span>
        {!open && summary && (
          <span className={styles.summary}>
            {summary}
          </span>
        )}
      </button>
      {open && (
        <div className={styles.body}>
          {children}
        </div>
      )}
    </div>
  );
});
