// poiesis/src/components/panels/inspector/FunctionPicker.tsx
// Searchable function/action picker dropdown for event handler binding.

import { useState, useMemo, useCallback } from "react";
import type { HandlerItem } from "./useHandlerItems";

interface FunctionPickerProps {
  items: HandlerItem[];
  selectedName: string | null;
  onSelect: (item: HandlerItem) => void;
  onClose: () => void;
}

export function FunctionPicker({
  items,
  selectedName,
  onSelect,
  onClose,
}: FunctionPickerProps) {
  const [search, setSearch] = useState("");

  const filtered = useMemo(() => {
    if (!search.trim()) return items;
    const q = search.toLowerCase();
    return items.filter(
      (item) =>
        item.name.toLowerCase().includes(q) ||
        item.displayName.toLowerCase().includes(q)
    );
  }, [items, search]);

  const componentFns = filtered.filter((item) => item.source === "component");
  const storeActions = filtered.filter((item) => item.source === "store");

  const handleSelect = useCallback(
    (item: HandlerItem) => {
      onSelect(item);
    },
    [onSelect]
  );

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={onClose}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 10000,
        }}
      />
      {/* Dropdown panel */}
      <div
        style={{
          position: "absolute",
          top: "100%",
          left: 0,
          right: 0,
          zIndex: 10001,
          marginTop: 4,
          background: "#fff",
          border: "1px solid #e2e8f0",
          borderRadius: 8,
          boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
          maxHeight: 280,
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {/* Search */}
        <div style={{ padding: "8px 10px", borderBottom: "1px solid #f1f5f9" }}>
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索函数/Action..."
            style={{
              width: "100%",
              padding: "6px 10px",
              fontSize: 12,
              border: "1px solid #e2e8f0",
              borderRadius: 4,
              outline: "2px solid transparent",
              fontFamily: "inherit",
              boxSizing: "border-box",
            }}
            onFocus={(e) => (e.target.style.borderColor = "#6366F1")}
            onBlur={(e) => (e.target.style.borderColor = "#e2e8f0")}
          />
        </div>

        {/* List */}
        <div style={{ overflowY: "auto", flex: 1 }}>
          {componentFns.length > 0 && (
            <>
              <GroupLabel>组件函数</GroupLabel>
              {componentFns.map((item) => (
                <PickerRow
                  key={item.pid}
                  item={item}
                  isSelected={item.name === selectedName}
                  onSelect={handleSelect}
                />
              ))}
            </>
          )}
          {storeActions.length > 0 && (
            <>
              <GroupLabel>Store Actions</GroupLabel>
              {storeActions.map((item) => (
                <PickerRow
                  key={item.pid}
                  item={item}
                  isSelected={item.name === selectedName}
                  onSelect={handleSelect}
                />
              ))}
            </>
          )}
          {filtered.length === 0 && (
            <div
              style={{
                padding: 16,
                textAlign: "center",
                fontSize: 12,
                color: "#94a3b8",
              }}
            >
              无匹配结果
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function GroupLabel({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        padding: "4px 12px",
        fontSize: 12,
        fontWeight: 600,
        color: "#94a3b8",
        textTransform: "uppercase",
        background: "#f8fafc",
        borderTop: "1px solid #f1f5f9",
      }}
    >
      {children}
    </div>
  );
}

function PickerRow({
  item,
  isSelected,
  onSelect,
}: {
  item: HandlerItem;
  isSelected: boolean;
  onSelect: (item: HandlerItem) => void;
}) {
  return (
    <button type="button"
      onClick={() => onSelect(item)}
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        width: "100%",
        padding: "6px 12px",
        border: "none",
        background: isSelected ? "#eef2ff" : "transparent",
        cursor: "pointer",
        textAlign: "left",
        fontFamily: "var(--dp-font-mono, monospace)",
        fontSize: 12,
        color: isSelected ? "#4f46e5" : "#1e293b",
      }}
    >
      <span style={{ fontWeight: isSelected ? 600 : 400 }}>
        {item.displayName}
      </span>
      <span style={{ fontSize: 12, color: "#94a3b8" }}>
        {item.sourceName}
      </span>
    </button>
  );
}
