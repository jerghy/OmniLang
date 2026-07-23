// components/panels/inspector/SignalSlashPicker.tsx
// "/"-triggered picker for component signals, computeds, store signals, and getters.
// Modeled on TokenSlashPicker (components/design/TokenSlashPicker.tsx).

import { useRef, useEffect, useState, useCallback } from "react";
import styles from "./SignalSlashPicker.module.css";

export interface SignalPickerItem {
  pid: string;
  name: string;          // display: "count" or "TodoStore.items"
  insertionText: string; // inserts "{count}" or "{TodoStore.items}"
  type: string;          // "i32", "String", "bool"
  category: "signals" | "computeds" | "store-signals" | "store-getters";
}

interface SignalSlashPickerProps {
  items: SignalPickerItem[];
  filter: string;
  onSelect: (item: SignalPickerItem) => void;
  onClose: () => void;
  position: { top: number; left: number };
}

const CATEGORY_LABELS: Record<SignalPickerItem["category"], string> = {
  signals: "Signals",
  computeds: "Computeds",
  "store-signals": "Store Signals",
  "store-getters": "Store Getters",
};

const CATEGORY_ICONS: Record<SignalPickerItem["category"], string> = {
  signals: "⚡",
  computeds: "🔄",
  "store-signals": "📦",
  "store-getters": "📋",
};

const CATEGORY_ORDER: SignalPickerItem["category"][] = [
  "signals",
  "computeds",
  "store-signals",
  "store-getters",
];

export function SignalSlashPicker({
  items,
  filter,
  onSelect,
  onClose,
  position,
}: SignalSlashPickerProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [activeIndex, setActiveIndex] = useState(0);

  // Filter items by the text typed after "/"
  const filtered = filter
    ? items.filter((item) =>
        item.name.toLowerCase().includes(filter.toLowerCase())
      )
    : items;

  // Group by category, maintaining category order
  const grouped = (() => {
    const result: Record<string, SignalPickerItem[]> = {};
    for (const item of filtered) {
      if (!result[item.category]) result[item.category] = [];
      result[item.category].push(item);
    }
    return result;
  })();

  // Flatten for keyboard navigation
  const flatItems = CATEGORY_ORDER.flatMap(
    (cat) => grouped[cat] ?? []
  );

  // Clamp active index when filter changes
  const maxIndex = Math.max(0, flatItems.length - 1);
  const safeIndex = Math.min(activeIndex, maxIndex);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setActiveIndex((prev) => Math.min(prev + 1, maxIndex));
          break;
        case "ArrowUp":
          e.preventDefault();
          setActiveIndex((prev) => Math.max(prev - 1, 0));
          break;
        case "Enter":
          e.preventDefault();
          if (flatItems[safeIndex]) {
            onSelect(flatItems[safeIndex]);
          }
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [maxIndex, safeIndex, flatItems, onSelect, onClose]
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // Click outside to close
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const timer = setTimeout(
      () => document.addEventListener("mousedown", handler),
      0
    );
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handler);
    };
  }, [onClose]);

  // Reset active index when filter changes — only resets one state value (not "all state")
  // eslint-disable-next-line react-doctor/no-reset-all-state-on-prop-change, react-hooks/exhaustive-deps
  useEffect(() => {
    // eslint-disable-next-line react-doctor/no-adjust-state-on-prop-change
    setActiveIndex(0);
  }, [filter]);

  if (flatItems.length === 0) {
    return (
      <div
        ref={ref}
        className={styles.emptyState}
        style={{
          top: position.top,
          left: position.left,
        }}
      >
        No bindings found
      </div>
    );
  }

  return (
    <div
      ref={ref}
      className={styles.picker}
      style={{
        top: position.top,
        left: position.left,
      }}
    >
      {CATEGORY_ORDER.map((cat) => {
        const catItems = grouped[cat];
        if (!catItems || catItems.length === 0) return null;

        return (
          <div key={cat}>
            <div className={styles.categoryHeader}>
              <span>{CATEGORY_ICONS[cat]}</span>
              <span>{CATEGORY_LABELS[cat]}</span>
            </div>
            {catItems.map((item) => {
              const flatIdx = flatItems.indexOf(item);
              const isActive = flatIdx === safeIndex;

              return (
                <button type="button"
                  key={item.pid}
                  onClick={() => onSelect(item)}
                  onMouseEnter={() => setActiveIndex(flatIdx)}
                  className={styles.itemBtn}
                  style={{
                    background: isActive
                      ? "var(--dp-accent-subtle, var(--ol-color-primary-subtle))"
                      : "transparent",
                    color: "var(--dp-text-secondary, var(--ol-text-secondary))",
                  }}
                >
                  <span style={{ fontWeight: 500 }}>{item.name}</span>
                  <span
                    style={{
                      marginLeft: "auto",
                      fontSize: 12,
                      color: "var(--ol-text-secondary)",
                    }}
                  >
                    {item.type}
                  </span>
                </button>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}
