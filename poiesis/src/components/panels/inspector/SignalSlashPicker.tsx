// components/panels/inspector/SignalSlashPicker.tsx
// "/"-triggered picker for component signals, computeds, store signals, and getters.
// Modeled on TokenSlashPicker (components/design/TokenSlashPicker.tsx).

import { useRef, useEffect, useState, useCallback } from "react";

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

  // Reset active index when filter changes
  useEffect(() => {
    setActiveIndex(0);
  }, [filter]);

  if (flatItems.length === 0) {
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
          padding: "8px 12px",
          fontSize: 11,
          color: "#94a3b8",
        }}
      >
        No bindings found
      </div>
    );
  }

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
        minWidth: 220,
        maxHeight: 300,
        overflow: "auto",
        padding: 4,
      }}
    >
      {CATEGORY_ORDER.map((cat) => {
        const catItems = grouped[cat];
        if (!catItems || catItems.length === 0) return null;

        return (
          <div key={cat}>
            <div
              style={{
                fontSize: 9,
                fontWeight: 600,
                color: "#94a3b8",
                padding: "4px 8px 2px",
                textTransform: "uppercase",
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              <span>{CATEGORY_ICONS[cat]}</span>
              <span>{CATEGORY_LABELS[cat]}</span>
            </div>
            {catItems.map((item) => {
              const flatIdx = flatItems.indexOf(item);
              const isActive = flatIdx === safeIndex;

              return (
                <button
                  key={item.pid}
                  onClick={() => onSelect(item)}
                  onMouseEnter={() => setActiveIndex(flatIdx)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    width: "100%",
                    padding: "4px 8px",
                    fontSize: 11,
                    border: "none",
                    background: isActive
                      ? "var(--dp-accent-subtle, #eef0ff)"
                      : "transparent",
                    color: "var(--dp-text-secondary, #6b6893)",
                    cursor: "pointer",
                    textAlign: "left",
                    fontFamily: "var(--dp-font-mono, monospace)",
                    borderRadius: 4,
                  }}
                >
                  <span style={{ fontWeight: 500 }}>{item.name}</span>
                  <span
                    style={{
                      marginLeft: "auto",
                      fontSize: 9,
                      color: "#94a3b8",
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
