// poiesis/src/components/canvas/code/cards/AutocompleteMenu.tsx
import React, { useEffect, useRef } from "react";

export interface AutocompleteItem {
  label: string;
  description: string;
  kind: string;
}

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
        position: "absolute",
        top: position.top, left: position.left,
        background: "#fff", borderRadius: 6,
        boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
        minWidth: 200, maxHeight: 240, overflowY: "auto",
        zIndex: 1000, fontFamily: "monospace", fontSize: 12,
      }}>
      {items.map((item, i) => (
        <div key={item.kind}
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

// Context-aware autocomplete items
export function getAutocompleteItems(context: string): AutocompleteItem[] {
  // Blank line — all possible entries
  return [
    { label: "let", description: "声明不可变变量", kind: "declare_var" },
    { label: "let mut", description: "声明可变变量", kind: "declare_var_mut" },
    { label: "set", description: "赋值变量", kind: "set_var" },
    { label: "if", description: "条件分支", kind: "if_block" },
    { label: "while", description: "while 循环", kind: "while" },
    { label: "for", description: "for..in 循环", kind: "for_in" },
    { label: "return", description: "返回", kind: "return" },
    { label: "await", description: "异步等待", kind: "await" },
    { label: "break", description: "跳出循环", kind: "break" },
    { label: "continue", description: "跳过本次", kind: "continue" },
    { label: "//", description: "注释", kind: "comment" },
  ];
}
