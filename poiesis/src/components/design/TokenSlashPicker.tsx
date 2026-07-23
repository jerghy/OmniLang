// components/design/TokenSlashPicker.tsx
// Slash-command token picker: typing "/" in a property input opens this popover.

import React, { useRef, useEffect } from "react";
import type { DesignToken } from "../../core/types/design";
import styles from "./TokenSlashPicker.module.css";

interface TokenSlashPickerProps {
  tokens: DesignToken[];
  filter?: string;
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
  onSelect,
  onClose,
  position,
}: TokenSlashPickerProps) {
  const ref = useRef<HTMLDivElement>(null);

  const grouped = React.useMemo(() => {
    const result: Record<string, DesignToken[]> = {};
    for (const t of tokens) {
      const cat = t.category ?? "other";
      if (!result[cat]) result[cat] = [];
      result[cat].push(t);
    }
    return result;
  }, [tokens]);

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
      className={styles.picker}
      style={{ top: position.top, left: position.left }}
    >
      {categories.map((cat) => (
        <div key={cat}>
          <div className={styles.categoryLabel}>
            {CATEGORY_LABELS[cat] || cat}
          </div>
          {grouped[cat].map((t) => (
            <button type="button"
              key={t.id}
              onClick={() => onSelect(t.name)}
              className={styles.tokenBtn}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLElement).style.background = "var(--dp-accent-subtle, var(--ol-color-primary-subtle))";
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLElement).style.background = "transparent";
              }}
            >
              {t.tokenType === "color" && (
                <span
                  style={{
                    width: 12, height: 12, borderRadius: 2,
                    background: t.value, border: "1px solid var(--ol-border-hover)", flexShrink: 0,
                  }}
                />
              )}
              <span>{t.name}</span>
              <span style={{ marginLeft: "auto", fontSize: 12, color: "var(--ol-text-muted)" }}>
                {t.value}
              </span>
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
