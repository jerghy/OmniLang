// components/design/CollapsibleGroup.tsx
// Collapsible property group — matches Poiesis design panel aesthetic.

import { useState, useCallback } from "react";

interface CollapsibleGroupProps {
  title: string;
  summary: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

export function CollapsibleGroup({
  title,
  summary,
  defaultOpen = false,
  children,
}: CollapsibleGroupProps) {
  const [open, setOpen] = useState(defaultOpen);

  const toggle = useCallback(() => setOpen((v) => !v), []);

  return (
    <div>
      <div
        onClick={toggle}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "5px 0",
          cursor: "pointer",
          userSelect: "none",
          fontSize: 11,
          lineHeight: "20px",
          color: "#374151",
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLElement).style.color = "#6366f1";
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLElement).style.color = "#374151";
        }}
      >
        <span
          style={{
            fontSize: 8,
            color: "#94a3b8",
            transform: open ? "rotate(90deg)" : "rotate(0deg)",
            transition: "transform 150ms ease",
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
            letterSpacing: "0.5px",
            fontSize: 10,
            color: "inherit",
            whiteSpace: "nowrap",
          }}
        >
          {title}
        </span>
        {!open && summary && (
          <span
            style={{
              color: "#94a3b8",
              fontWeight: 400,
              fontSize: 11,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              flex: 1,
            }}
          >
            {summary}
          </span>
        )}
      </div>
      {open && (
        <div style={{ padding: "0 0 8px 16px" }}>
          {children}
        </div>
      )}
    </div>
  );
}
