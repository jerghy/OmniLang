// components/design/VariantSidebar.tsx
// Sidebar listing CSS variant groups with selection, add, and remove.

import { useState } from "react";
import type { VariantDefinition } from "../../core/types/design";

// ── Styles ──

const SIDEBAR_STYLE: React.CSSProperties = {
  width: 180,
  borderRight: "1px solid var(--dp-border, var(--ol-border))",
  display: "flex",
  flexDirection: "column",
  flexShrink: 0,
  background: "var(--dp-surface, var(--ol-surface))",
};

const SIDEBAR_HEADER: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "var(--dp-text-secondary, var(--ol-text-secondary))",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  padding: "10px 8px 6px",
};

const ITEM_BASE: React.CSSProperties = {
  padding: "6px 12px",
  fontSize: 13,
  cursor: "pointer",
  color: "var(--dp-text-secondary, var(--ol-text-secondary))",
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  borderRadius: "var(--dp-radius-sm, 6px)",
  margin: "1px 6px",
  transition: "background 100ms ease, color 100ms ease",
};

const ITEM_ACTIVE: React.CSSProperties = {
  ...ITEM_BASE,
  background: "var(--dp-accent-subtle, var(--ol-color-primary-subtle))",
  color: "var(--dp-accent, var(--ol-color-primary))",
  fontWeight: 600,
};

const INPUT_STYLE: React.CSSProperties = {
  padding: "4px 6px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  border: "1px solid var(--dp-border, var(--ol-border))",
  borderRadius: "var(--dp-radius-sm, 6px)",
  outline: "2px solid transparent",
  background: "var(--dp-surface, var(--ol-surface))",
  color: "var(--dp-text, var(--ol-gray-700))",
  boxSizing: "border-box",
  flex: 1,
  minWidth: 0,
};

const REMOVE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "var(--dp-text-muted, var(--ol-text-muted))",
  fontSize: 14,
  padding: "2px 4px",
  lineHeight: 1,
  flexShrink: 0,
  borderRadius: 3,
};

// ── Props ──

interface VariantSidebarProps {
  variants: VariantDefinition[];
  selectedVariantId: string | null;
  onSelectVariant: (id: string) => void;
  onAddVariant: (name: string) => void;
  onRemoveVariant: (e: React.MouseEvent, id: string) => void;
}

export function VariantSidebar({
  variants,
  selectedVariantId,
  onSelectVariant,
  onAddVariant,
  onRemoveVariant,
}: VariantSidebarProps) {
  const [newVariantName, setNewVariantName] = useState("");

  const handleAdd = () => {
    const name = newVariantName.trim();
    if (!name) return;
    onAddVariant(name);
    setNewVariantName("");
  };

  return (
    <div style={SIDEBAR_STYLE}>
      <div style={SIDEBAR_HEADER}>Variants ({variants.length})</div>
      <div style={{ flex: 1, overflow: "auto" }}>
        {variants.map((v) => (
          <button
            key={v.id}
            type="button"
            onClick={(e) => { if ((e.target as HTMLElement).getAttribute('data-action') === 'remove-variant') { onRemoveVariant(e as unknown as React.MouseEvent, v.id); } else { onSelectVariant(v.id); } }}
            style={v.id === selectedVariantId ? ITEM_ACTIVE : ITEM_BASE}
          >
            <span
              style={{
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                flex: 1,
              }}
            >
              {v.name}
            </span>
            <span data-action="remove-variant" style={REMOVE_BTN} title="Remove variant">
              &times;
            </span>
          </button>
        ))}
      </div>
      {/* Add variant */}
      <div style={{ padding: "6px 8px", display: "flex", gap: 4 }}>
        <input
          aria-label="Add variant"
          value={newVariantName}
          onChange={(e) => setNewVariantName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleAdd();
          }}
          placeholder="+ Add Variant"
          style={{
            ...INPUT_STYLE,
            flex: 1,
            minWidth: 0,
            fontSize: 12,
          }}
        />
        <button type="button"
          onClick={handleAdd}
          disabled={!newVariantName.trim()}
          style={{
            padding: "3px 8px",
            fontSize: 12,
            border: "none",
            borderRadius: 4,
            background: newVariantName.trim() ? "var(--dp-accent, var(--ol-color-primary))" : "var(--dp-border, var(--ol-border))",
            color: newVariantName.trim() ? "var(--ol-surface)" : "var(--dp-text-muted, var(--ol-text-muted))",
            cursor: newVariantName.trim() ? "pointer" : "default",
          }}
        >
          +
        </button>
      </div>
    </div>
  );
}
