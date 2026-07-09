// components/design/CollapsibleGroup.tsx
// Card-like collapsible property group with Poiesis design panel styling.

import React, { useState, useCallback, useRef } from "react";

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
      style={{
        background: "var(--dp-surface, #fff)",
        borderRadius: "var(--dp-radius, 8px)",
        margin: "0 8px 6px",
        boxShadow: open
          ? "var(--dp-shadow, 0 1px 3px rgba(99,102,241,0.06))"
          : hover
            ? "var(--dp-shadow, 0 1px 3px rgba(99,102,241,0.06))"
            : "var(--dp-shadow-sm, 0 1px 2px rgba(99,102,241,0.04))",
        transition: "box-shadow 150ms ease",
        overflow: "hidden",
      }}
    >
      <div
        onClick={handleHeaderClick}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "6px 10px",
          cursor: "pointer",
          userSelect: "none",
          fontSize: 12,
          lineHeight: "22px",
          color: open ? "var(--dp-text, #1e1b4b)" : "var(--dp-text-secondary, #6b6893)",
          transition: "color 150ms ease, background 150ms ease",
          background: hover ? "var(--dp-surface-hover, #f5f3ff)" : "transparent",
        }}
      >
        <span
          style={{
            fontSize: 8,
            color: open ? "var(--dp-accent, #6366f1)" : "var(--dp-text-muted, #a09db8)",
            transform: open ? "rotate(90deg)" : "rotate(0deg)",
            transition: "transform 150ms ease, color 150ms ease",
            flexShrink: 0,
            width: 12,
            textAlign: "center",
          }}
        >
          ▶
        </span>
        <span
          style={{
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.3px",
            fontSize: 11,
            color: "inherit",
            whiteSpace: "nowrap",
          }}
        >
          {title}
        </span>
        {!open && summary && (
          <span
            style={{
              color: "var(--dp-text-muted, #a09db8)",
              fontWeight: 400,
              fontSize: 12,
              fontFamily: "var(--dp-font-mono, monospace)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              flex: 1,
              textAlign: "right",
            }}
          >
            {summary}
          </span>
        )}
      </div>
      {open && (
        <div style={{ padding: "4px 10px 10px" }}>
          {children}
        </div>
      )}
    </div>
  );
});
