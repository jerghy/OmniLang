// components/design/TokenSlashPicker.tsx
// Slash-command token picker: typing "/" in a property input opens this popover.

import React, { useRef, useEffect } from "react";
import type { DesignToken } from "../../core/types/design";

interface TokenSlashPickerProps {
  tokens: DesignToken[];
  filter: string;
  onSelect: (tokenName: string) => void;
  onClose: () => void;
  position: { top: number; left: number };
}

const CATEGORY_LABELS: Record<string, string> = {
  colors: "Colors",
  spacing: "Spacing",
  typography: "Typography",
  radii: "Radii",
  shadows: "Shadows",
  durations: "Durations",
};

export function TokenSlashPicker({
  tokens,
  filter,
  onSelect,
  onClose,
  position,
}: TokenSlashPickerProps) {
  const ref = useRef<HTMLDivElement>(null);

  const grouped = React.useMemo(() => {
    const query = filter.toLowerCase();
    const result: Record<string, DesignToken[]> = {};
    for (const t of tokens) {
      if (query && !t.name.toLowerCase().includes(query)) continue;
      const cat = t.category ?? "other";
      if (!result[cat]) result[cat] = [];
      result[cat].push(t);
    }
    return result;
  }, [tokens, filter]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const timer = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handler);
    };
  }, [onClose]);

  const categories = Object.keys(grouped);
  if (categories.length === 0) return null;

  return (
    <div
      ref={ref}
      style={{
        position: "fixed",
        top: position.top,
        left: position.left,
        background: "#fff",
        border: "1px solid #e2e8f0",
        borderRadius: 8,
        boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
        zIndex: 1000,
        minWidth: 200,
        maxHeight: 260,
        overflow: "auto",
        padding: 4,
      }}
    >
      {categories.map((cat) => (
        <div key={cat}>
          <div
            style={{
              fontSize: 9,
              fontWeight: 600,
              color: "#94a3b8",
              padding: "4px 8px 2px",
              textTransform: "uppercase",
            }}
          >
            {CATEGORY_LABELS[cat] || cat}
          </div>
          {grouped[cat].map((t) => (
            <button
              key={t.id}
              onClick={() => onSelect(t.name)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                width: "100%",
                padding: "4px 8px",
                fontSize: 11,
                border: "none",
                background: "transparent",
                color: "var(--dp-text-secondary, #6b6893)",
                cursor: "pointer",
                textAlign: "left",
                fontFamily: "var(--dp-font-mono, monospace)",
                borderRadius: 4,
              }}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLElement).style.background = "var(--dp-accent-subtle, #eef0ff)";
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLElement).style.background = "transparent";
              }}
            >
              {t.tokenType === "color" && (
                <span
                  style={{
                    width: 12, height: 12, borderRadius: 2,
                    background: t.value, border: "1px solid #cbd5e1", flexShrink: 0,
                  }}
                />
              )}
              <span>{t.name}</span>
              <span style={{ marginLeft: "auto", fontSize: 9, color: "#94a3b8" }}>
                {t.value}
              </span>
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
