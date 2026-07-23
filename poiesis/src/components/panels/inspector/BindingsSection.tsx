// components/panels/inspector/BindingsSection.tsx
// Two-way data binding management section. Shows element bindings (bind:value, etc.)
// with prop selector, signal selector, and add/remove controls.

import { useState, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";
import type { SignalPickerItem } from "./SignalSlashPicker";

// ── Bindable properties per tag ───────────────────────────────────────

const BINDABLE_PROPS: Record<string, string[]> = {
  input: ["value"],
  textarea: ["value"],
  select: ["value"],
  // Future: input checkbox → "checked", etc.
};

function getBindableProps(tag: string): string[] {
  return BINDABLE_PROPS[tag] ?? [];
}

// ── Category labels ────────────────────────────────────────────────────

const CATEGORY_LABELS: Record<SignalPickerItem["category"], string> = {
  signals: "Signals",
  computeds: "Computeds",
  "store-signals": "Store Signals",
  "store-getters": "Store Getters",
};

const CATEGORY_ORDER: SignalPickerItem["category"][] = [
  "signals",
  "computeds",
  "store-signals",
  "store-getters",
];

// ── Styles ─────────────────────────────────────────────────────────────

const sectionHeader: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  marginBottom: 6,
};

const rowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 4,
  marginBottom: 4,
};

const badgeStyle: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  padding: "2px 6px",
  borderRadius: 4,
  background: "#e0e7ff",
  color: "#3730a3",
  fontFamily: "var(--dp-font-mono, monospace)",
  flexShrink: 0,
};

const signalBtnStyle: React.CSSProperties = {
  flex: 1,
  fontSize: 12,
  padding: "3px 6px",
  border: "1px solid #e2e8f0",
  borderRadius: 4,
  background: "#fff",
  cursor: "pointer",
  textAlign: "left" as const,
  color: "#1e293b",
  fontFamily: "var(--dp-font-mono, monospace)",
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap" as const,
};

const removeBtnStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  fontSize: 14,
  padding: "0 4px",
  color: "#94a3b8",
  lineHeight: 1,
};

const addBtnStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  fontSize: 14,
  padding: "0 2px",
  color: "#6366f1",
  lineHeight: 1,
  fontWeight: 700,
};

const popoverStyle: React.CSSProperties = {
  position: "fixed",
  zIndex: 1000,
  background: "#fff",
  border: "1px solid #e2e8f0",
  borderRadius: 6,
  boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
  maxHeight: 240,
  overflow: "auto",
  minWidth: 200,
};

const popoverItemStyle: React.CSSProperties = {
  padding: "4px 10px",
  fontSize: 12,
  cursor: "pointer",
  fontFamily: "var(--dp-font-mono, monospace)",
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
};

const categoryHeaderStyle: React.CSSProperties = {
  padding: "3px 10px",
  fontSize: 12,
  fontWeight: 600,
  color: "#94a3b8",
  textTransform: "uppercase",
  background: "#f8fafc",
  borderBottom: "1px solid #f1f5f9",
};

const emptyStyle: React.CSSProperties = {
  fontSize: 12,
  color: "#94a3b8",
  padding: "4px 0",
};

// ── Props ──────────────────────────────────────────────────────────────

interface BindingsSectionProps {
  element: ElementData;
  signalItems: SignalPickerItem[];
}

// ── Component ──────────────────────────────────────────────────────────

export function BindingsSection({ element, signalItems }: BindingsSectionProps) {
  // ── All hooks must be called unconditionally (Rules of Hooks) ──
  const setElementBinding = usePoiesisStore((s) => s.setElementBinding);

  const isViewEl = "tag" in element && element.nodeType === "view";
  const tag = isViewEl ? (element as import("../../../core/types/element").ViewElementData).tag : "";
  const bindableProps = isViewEl ? getBindableProps(tag) : [];
  const bindings = isViewEl
    ? (element as import("../../../core/types/element").ViewElementData).bindings ?? {}
    : {};
  const boundProps = Object.keys(bindings);

  const [openPopover, setOpenPopover] = useState<{
    propName: string;
    position: { top: number; left: number };
  } | null>(null);

  const handleAdd = useCallback(() => {
    const available = bindableProps.find((p) => !boundProps.includes(p));
    if (available) {
      setElementBinding(element.id, available, "");
    }
  }, [element.id, bindableProps, boundProps, setElementBinding]);

  const handleRemove = useCallback(
    (propName: string) => {
      setElementBinding(element.id, propName, null);
    },
    [element.id, setElementBinding],
  );

  const handleSignalSelect = useCallback(
    (propName: string, signalName: string) => {
      setElementBinding(element.id, propName, signalName);
      setOpenPopover(null);
    },
    [element.id, setElementBinding],
  );

  const openSignalPicker = useCallback(
    (propName: string, el: HTMLElement) => {
      const rect = el.getBoundingClientRect();
      setOpenPopover({
        propName,
        position: { top: rect.bottom + 2, left: rect.left },
      });
    },
    [],
  );

  const cleanupRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    if (!openPopover) return;
    const id = setTimeout(() => {
      const handler = () => setOpenPopover(null);
      cleanupRef.current = handler;
      document.addEventListener("click", handler);
    }, 0);
    return () => {
      clearTimeout(id);
      if (cleanupRef.current) {
        document.removeEventListener("click", cleanupRef.current);
        cleanupRef.current = null;
      }
    };
  }, [openPopover]);

  const hasAvailableBindableProp = bindableProps.some((p) => !boundProps.includes(p));

  // ── Early return only after all hooks ──
  if (!isViewEl || bindableProps.length === 0) return null;

  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
        <span style={sectionHeader}>🔗 绑定</span>
        {hasAvailableBindableProp && (
          <button type="button"
            onClick={handleAdd}
            style={addBtnStyle}
            title="添加绑定"
          >
            +
          </button>
        )}
      </div>

      {boundProps.length === 0 && (
        <div style={emptyStyle}>暂无绑定</div>
      )}

      {boundProps.map((propName) => {
        const signalName = bindings[propName];
        return (
          <div key={propName} style={rowStyle}>
            {/* Prop name badge */}
            <span style={badgeStyle}>{propName}</span>

            {/* Signal selector button */}
            <button type="button"
              style={{
                ...signalBtnStyle,
                color: signalName ? "#1e293b" : "#94a3b8",
                fontStyle: signalName ? "normal" : "italic",
              }}
              onClick={(e) => {
                e.stopPropagation();
                openSignalPicker(propName, e.currentTarget);
              }}
            >
              {signalName || "选择信号..."}
            </button>

            {/* Remove button */}
            <button type="button"
              onClick={() => handleRemove(propName)}
              style={removeBtnStyle}
              title="移除绑定"
            >
              ×
            </button>
          </div>
        );
      })}

      {/* Signal picker popover */}
      {openPopover && (
        <SignalPickerPopover
          items={signalItems}
          position={openPopover.position}
          onSelect={(item) => handleSignalSelect(openPopover.propName, item.insertionText)}
          onClose={() => setOpenPopover(null)}
        />
      )}
    </div>
  );
}

// ── Signal picker popover ──────────────────────────────────────────────

interface SignalPickerPopoverProps {
  items: SignalPickerItem[];
  position: { top: number; left: number };
  onSelect: (item: SignalPickerItem) => void;
  onClose: () => void;
}

function SignalPickerPopover({ items, position, onSelect, onClose }: SignalPickerPopoverProps) {
  const [filter, setFilter] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const filtered = filter
    ? items.filter((item) =>
        item.name.toLowerCase().includes(filter.toLowerCase())
      )
    : items;

  // Group by category
  const grouped = new Map<SignalPickerItem["category"], SignalPickerItem[]>();
  for (const cat of CATEGORY_ORDER) {
    grouped.set(cat, []);
  }
  for (const item of filtered) {
    const cat = grouped.get(item.category);
    if (cat) cat.push(item);
  }

  return (
    <div style={popoverStyle} onClick={(e) => e.stopPropagation()}>
      {/* Search input */}
      <div style={{ padding: 4 }}>
        <input
          ref={inputRef}
          type="text"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="搜索信号..."
          style={{
            width: "100%",
            padding: "3px 6px",
            fontSize: 12,
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            outline: "2px solid transparent",
            boxSizing: "border-box",
            fontFamily: "var(--dp-font-mono, monospace)",
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              onClose();
            }
          }}
        />
      </div>

      {/* Grouped items */}
      {CATEGORY_ORDER.map((cat) => {
        const catItems = grouped.get(cat) ?? [];
        if (catItems.length === 0) return null;
        return (
          <div key={cat}>
            <div style={categoryHeaderStyle}>{CATEGORY_LABELS[cat]}</div>
            {catItems.map((item) => (
              <div
                key={item.pid}
                style={popoverItemStyle}
                onClick={() => onSelect(item)}
                onMouseEnter={(e) => {
                  (e.target as HTMLElement).style.background = "#eef2ff";
                }}
                onMouseLeave={(e) => {
                  (e.target as HTMLElement).style.background = "transparent";
                }}
              >
                <span>{item.name}</span>
                <span style={{ color: "#94a3b8", fontSize: 12 }}>{item.type}</span>
              </div>
            ))}
          </div>
        );
      })}

      {filtered.length === 0 && (
        <div style={{ padding: "8px 10px", fontSize: 12, color: "#94a3b8", textAlign: "center" }}>
          无匹配信号
        </div>
      )}
    </div>
  );
}
