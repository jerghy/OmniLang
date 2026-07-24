// poiesis/src/components/canvas/code/lexical/autocomplete/AutocompleteMenu.tsx
import React, { useEffect, useRef } from "react";
import type { AutocompleteItem } from "./autocomplete-rules";

interface Props {
  items: AutocompleteItem[];
  selectedIndex: number;
  onSelect: (item: AutocompleteItem) => void;
  onClose: () => void;
  position: { top: number; left: number };
}

export const AutocompleteMenu: React.FC<Props> = ({
  items, selectedIndex, onSelect, onClose, position,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  if (items.length === 0) return null;

  return (
    <div ref={menuRef}
      style={{
        position: "fixed",
        top: position.top, left: position.left,
        background: "#fff", borderRadius: 6,
        boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
        minWidth: 200, maxHeight: 240, overflowY: "auto",
        zIndex: 10001, fontFamily: "monospace", fontSize: 12,
      }}>
      {items.map((item, i) => (
        <div key={`${item.cardKind}-${item.label}`}
          onClick={() => onSelect(item)}
          style={{
            padding: "6px 12px", cursor: "pointer",
            background: i === selectedIndex ? "#eef2ff" : "transparent",
            borderBottom: "1px solid #f1f5f9",
            display: "flex", justifyContent: "space-between",
          }}>
          <span style={{ fontWeight: 600, color: "#1e293b" }}>{item.label}</span>
          <span style={{ color: "#94a3b8" }}>{item.description}</span>
        </div>
      ))}
    </div>
  );
};
