// poiesis/src/components/canvas/code/card-editor/AutocompleteMenu.tsx
import React, { useEffect, useRef } from "react";
import type { AutocompleteItem } from "./autocomplete-rules";
import styles from "./CodeCardEditor.module.css";

const ROW_HEIGHT = 32;

interface Props {
  items: AutocompleteItem[];
  selectedIndex: number;
  onSelect: (item: AutocompleteItem) => void;
  onClose: () => void;
  rowIndex: number;
}

export const AutocompleteMenu: React.FC<Props> = ({
  items,
  selectedIndex,
  onSelect,
  onClose,
  rowIndex,
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

  // Position below the current row
  const top = (rowIndex + 1) * ROW_HEIGHT + 60; // 60 = toolbar height + padding

  return (
    <div
      ref={menuRef}
      className={styles.autocomplete}
      style={{ top, left: 48 }}
    >
      {items.map((item, i) => (
        <div
          key={item.kind}
          className={`${styles.acItem} ${i === selectedIndex ? styles.acSelected : ""}`}
          onClick={() => onSelect(item)}
        >
          <span className={styles.acLabel}>{item.label}</span>
          <span className={styles.acDesc}>{item.description}</span>
        </div>
      ))}
    </div>
  );
};
