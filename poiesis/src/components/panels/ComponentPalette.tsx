/** 组件面板：展示内置组件 + 自定义组件，支持搜索、拖拽、编辑自定义组件。
 *
 *  META (COMPONENT_PROTOCOL) 是内置组件的单一真相源。
 *  自定义组件通过 useComponentRegistry 动态注入。 */

import { useState, useMemo } from "react";
import { useDraggable } from "@dnd-kit/core";
import { getUnifiedComponentMeta, useComponentRegistry } from "../../store/componentRegistry";
import { usePoiesisStore } from "../../store";
import { COMPONENT_PROTOCOL } from "../../data/componentProtocol";
import { CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS } from "../../core/constants";
import { createLogger } from "../../lib/utils/logger";
import styles from "./ComponentPalette.module.css";

const log = createLogger("ComponentPalette");

// Re-export for backward compatibility
export { COMPONENT_PROTOCOL };
export type { EventMeta } from "../../data/componentProtocol";

// ═══════════════════════════════════════════════
// PaletteDraggableItem — wraps each palette entry with @dnd-kit useDraggable
// ═══════════════════════════════════════════════

interface PaletteItemData {
  component: string;
  description: string;
  category: string;
  isCustom: boolean;
  icon: string;
  onEdit?: () => void;
  onDelete?: () => void;
}

function PaletteDraggableItem({ data }: { data: PaletteItemData }) {
  const { component, description, isCustom, icon, onEdit, onDelete } = data;

  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `palette-${component}`,
    data: { tag: component },
  });

  return (
    <div
      style={{ display: "flex", alignItems: "center", padding: isCustom ? "2px 4px" : "3px 4px", borderRadius: 4, marginBottom: 1 }}
    >
      {/* Drag handle — spread @dnd-kit listeners & attributes here */}
      <div
        ref={setNodeRef}
        {...listeners}
        {...attributes}
        title={description || component}
        className={isDragging ? undefined : styles.dragHandle}
        style={{
          flex: 1,
          padding: "3px 6px",
          borderRadius: 3,
          cursor: isDragging ? "grabbing" : "grab",
          fontSize: 11,
          border: "1px solid transparent",
          display: "flex",
          alignItems: "center",
          gap: 4,
          opacity: isDragging ? 0.4 : 1,
          touchAction: "none",
        }}
      >
        {/* Icon */}
        <span style={{ fontSize: 13, width: 18, textAlign: "center", flexShrink: 0 }}>
          {icon}
        </span>
        <span
          style={{
            color: isCustom ? "#4F46E5" : "#374151",
            fontWeight: isCustom ? 600 : 400,
          }}
        >
          {component}
        </span>
      </div>

      {/* Edit button (custom components only) */}
      {isCustom && onEdit && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onEdit();
          }}
          className={styles.editBtn}
          style={{
            padding: "1px 6px", fontSize: 10, border: "none",
            cursor: "pointer",
            borderRadius: 3,
          }}
          title={`编辑 ${component}`}
        >
          ✎
        </button>
      )}
      {isCustom && onDelete && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className={styles.deleteBtn}
          style={{
            padding: "1px 6px", fontSize: 10, border: "none",
            cursor: "pointer", color: "#dc2626",
            borderRadius: 3,
          }}
          title={`删除 ${component}`}
        >
          🗑
        </button>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════
// Component
// ═══════════════════════════════════════════════

export function ComponentPalette() {
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  const definitions = useComponentRegistry((s) => s.componentDefs) ?? {};
  const enterCanvas = usePoiesisStore((s) => s.enterComponentCanvas);
  const defineComponent = usePoiesisStore((s) => s.defineComponent);
  const removeComponent = usePoiesisStore((s) => s.removeComponent);

  // Build unified metadata list — re-compute when custom component definitions change
  const allMeta = useMemo(() => {
    const unified = getUnifiedComponentMeta();
    return Object.values(unified);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [definitions]);

  // Filter by search
  const filtered = useMemo(() => {
    if (!search.trim()) return allMeta;
    const q = search.toLowerCase();
    return allMeta.filter((m) =>
      m.component.toLowerCase().includes(q) ||
      m.description.toLowerCase().includes(q) ||
      (CATEGORY_LABELS[m.category] || m.category).includes(q)
    );
  }, [allMeta, search]);

  // Group by category, respecting CATEGORY_ORDER
  const grouped = useMemo(() => {
    const map = new Map<string, typeof filtered>();
    for (const m of filtered) {
      const list = map.get(m.category) || [];
      list.push(m);
      map.set(m.category, list);
    }
    // Sort by category order
    const sorted = new Map<string, typeof filtered>();
    for (const cat of CATEGORY_ORDER) {
      if (map.has(cat)) sorted.set(cat, map.get(cat)!);
    }
    // Append any categories not in the order
    for (const [cat, items] of map) {
      if (!sorted.has(cat)) sorted.set(cat, items);
    }
    return sorted;
  }, [filtered]);

  const toggleCollapse = (cat: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(cat)) next.delete(cat);
      else next.add(cat);
      return next;
    });
  };

  const handleCreate = (name: string) => {
    if (!name.trim()) return;
    defineComponent(name.trim(), {
      category: "Custom",
      description: "",
      props: {},
      events: {},
      styleProperties: ["padding", "margin", "width", "height"],
      children: true,
      storage: "config",
    });
    setShowCreateDialog(false);
    // Auto-enter the component canvas
    setTimeout(() => enterCanvas(name.trim()), 50);
  };

  return (
    <div style={{ padding: 8, fontSize: 12, userSelect: "none", display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Header */}
      <div style={{ fontWeight: 600, marginBottom: 6, fontSize: 11, color: "#6b7280", textTransform: "uppercase", display: "flex", alignItems: "center", gap: 4 }}>
        🧩 组件面板
      </div>

      {/* Search */}
      <input
        type="text"
        placeholder="搜索组件..."
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        style={{
          padding: "4px 8px", fontSize: 11, border: "1px solid #e5e7eb", borderRadius: 4,
          marginBottom: 6, outline: "none", width: "100%", boxSizing: "border-box",
        }}
        onFocus={(e) => { e.target.style.borderColor = "#4F46E5"; }}
        onBlur={(e) => { e.target.style.borderColor = "#e5e7eb"; }}
      />

      {/* Create Custom Component Button */}
      <button
        onClick={() => setShowCreateDialog(true)}
        style={{
          width: "100%", padding: "5px 8px", marginBottom: 8, fontSize: 11, fontWeight: 600,
          border: "1px dashed #4F46E5", borderRadius: 4, background: "#EEF2FF",
          color: "#4F46E5", cursor: "pointer",
        }}
      >
        + 自定义组件
      </button>

      {/* Category Groups */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {Array.from(grouped.entries()).map(([cat, items]) => {
          const isCustom = cat === "Custom";
          const isCollapsed = collapsed.has(cat);
          return (
            <div key={cat} style={{ marginBottom: 6 }}>
              {/* Category Header */}
              <div
                onClick={() => toggleCollapse(cat)}
                style={{
                  fontSize: 10, color: isCustom ? "#4F46E5" : "#9ca3af",
                  marginBottom: 2, textTransform: "uppercase",
                  cursor: "pointer", display: "flex", alignItems: "center", gap: 4,
                  padding: "1px 0", fontWeight: isCustom ? 700 : 500,
                  userSelect: "none",
                }}
              >
                <span style={{ fontSize: 8, transition: "transform 0.15s", transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)" }}>
                  ▼
                </span>
                {CATEGORY_LABELS[cat] || cat}
                <span style={{ fontWeight: 400, color: "#d1d5db" }}>({items.length})</span>
              </div>

              {!isCollapsed && items.map((m) => {
                const isCustomComp = definitions[m.component] !== undefined;
                return (
                  <PaletteDraggableItem
                    key={m.component}
                    data={{
                      component: m.component,
                      description: m.description,
                      category: m.category,
                      isCustom: isCustomComp,
                      icon: isCustomComp ? "★" : COMPONENT_ICONS[m.component] || "▪",
                      onEdit: isCustomComp ? () => enterCanvas(m.component) : undefined,
                      onDelete: isCustomComp
                        ? () => {
                            if (window.confirm(`确定删除自定义组件"${m.component}"？`)) {
                              removeComponent(m.component);
                              import("../../commands").then(({ api }) => {
                                api.annotation.deleteComponentFile(m.component).catch((e) => { log.error("Failed to delete component file:", e); });
                                api.annotation.deleteComponentDefinitions([m.component]).catch((e) => { log.error("Failed to delete component defs:", e); });
                              });
                            }
                          }
                        : undefined,
                    }}
                  />
                );
              })}
            </div>
          );
        })}
        {filtered.length === 0 && (
          <div style={{ textAlign: "center", color: "#9CA3AF", fontSize: 11, padding: "16px 0" }}>
            未找到匹配的组件
          </div>
        )}
      </div>

      {/* Create Dialog (simple inline modal) */}
      {showCreateDialog && (
        <>
          <div
            onClick={() => setShowCreateDialog(false)}
            style={{ position: "fixed", inset: 0, zIndex: 9998, background: "transparent" }}
          />
          <div style={{
            position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)",
            zIndex: 9999, background: "#fff", borderRadius: 12, padding: "20px 24px",
            boxShadow: "0 8px 32px rgba(0,0,0,0.2)", minWidth: 300,
          }}>
            <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
              新建自定义组件
            </div>
            <input
              id="new-comp-name"
              placeholder="组件名称 (如 MyCard)"
              autoFocus
              style={{
                width: "100%", padding: "6px 10px", fontSize: 13, border: "1px solid #d1d5db",
                borderRadius: 6, boxSizing: "border-box", marginBottom: 12,
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleCreate((e.target as HTMLInputElement).value);
                }
                if (e.key === "Escape") setShowCreateDialog(false);
              }}
            />
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button
                onClick={() => setShowCreateDialog(false)}
                style={{ padding: "5px 14px", fontSize: 12, border: "1px solid #d1d5db", borderRadius: 6, background: "#fff", cursor: "pointer" }}
              >
                取消
              </button>
              <button
                onClick={() => {
                  const input = document.getElementById("new-comp-name") as HTMLInputElement;
                  handleCreate(input?.value || "");
                }}
                style={{ padding: "5px 14px", fontSize: 12, border: "none", borderRadius: 6, background: "#4F46E5", color: "#fff", cursor: "pointer", fontWeight: 600 }}
              >
                创建并编辑
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

