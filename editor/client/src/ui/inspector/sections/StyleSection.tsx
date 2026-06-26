/** Style section — StyleEntry list with drag-and-drop reorder. */

import React, { useState } from "react";
import type { ElementData, StyleEntry } from "../../../core/types";
import { StyleEntryEditor } from "../../panels/StyleEntryEditor";
import { AddStyleEntry } from "../../../core/commands/AddStyleEntry";
import { RemoveStyleEntry } from "../../../core/commands/RemoveStyleEntry";
import { UpdateStyleEntry } from "../../../core/commands/UpdateStyleEntry";
import { ReorderStyleEntry } from "../../../core/commands/ReorderStyleEntry";
import { useUndoStore } from "../../../store/undo";
import { useEditorStore } from "../../../store/editor";
import { uid } from "../../../core/utils";
import type { CommandContext } from "../../../core/commands/types";

interface StyleSectionProps {
  element: ElementData;
  ctx: CommandContext;
  onRecompile: () => void;
}

export function StyleSection({ element: el, ctx, onRecompile }: StyleSectionProps) {
  const [expandedStyleId, setExpandedStyleId] = useState<string | null>(null);
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [dropIndex, setDropIndex] = useState<number | null>(null);
  const undoExec = useUndoStore((s) => s.execute);

  const handleDropAt = (toIndex: number) => {
    if (dragIndex !== null && dragIndex !== toIndex) {
      const entries = el.styles || [];
      const oldIds = entries.map((e: StyleEntry) => e.id);
      const newIds = reorderArray(oldIds, dragIndex, toIndex);
      undoExec(new ReorderStyleEntry(el.id, oldIds, newIds), ctx);
      onRecompile();
    }
    setDragIndex(null);
    setDropIndex(null);
  };

  return (
    <>
      <div style={{ ...sectionHeaderStyle, display: "flex", alignItems: "center" }}>
        样式
        <button
          onClick={() => {
            const entry: StyleEntry = {
              id: uid("s"),
              type: "inline",
              name: "",
              properties: [],
            };
            undoExec(new AddStyleEntry(el.id, entry), ctx);
            setExpandedStyleId(entry.id);
            onRecompile();
          }}
          style={{
            background: "none", border: "1px solid #d1d5db", borderRadius: 4,
            color: "#4F46E5", cursor: "pointer", fontSize: 11,
            padding: "2px 8px", marginLeft: "auto",
          }}
        >
          +添加
        </button>
      </div>

      {/* Render each style entry */}
      {(el.styles || []).map((entry: StyleEntry, index: number) => {
        const isExpanded = expandedStyleId === entry.id;
        const displayName = entry.type === "inline"
          ? `inline (${entry.properties.map(p => p.name).join(", ") || "empty"})`
          : entry.type === "variable"
            ? entry.name || "--unnamed"
            : `.${entry.name}`;

        return (
          <div key={entry.id}>
            {/* Collapsed row — draggable unless expanded */}
            <div
              draggable={!isExpanded}
              onDragStart={(e) => {
                setDragIndex(index);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragOver={(e) => {
                e.preventDefault();
                e.dataTransfer.dropEffect = "move";
                if (dragIndex !== index && dropIndex !== index) {
                  setDropIndex(index);
                }
              }}
              onDragLeave={() => {
                if (dropIndex === index) setDropIndex(null);
              }}
              onDrop={() => handleDropAt(index)}
              onDragEnd={() => {
                setDragIndex(null);
                setDropIndex(null);
              }}
              onClick={() => setExpandedStyleId(isExpanded ? null : entry.id)}
              style={{
                display: "flex", alignItems: "center",
                cursor: dragIndex === index ? "grabbing" : "grab",
                padding: "4px 0", color: "#374151", fontSize: 12,
                userSelect: "none",
                opacity: dragIndex === index ? 0.4 : 1,
                borderTop: dropIndex === index ? "2px solid #4F46E5" : "2px solid transparent",
                transition: "opacity 0.15s, border-color 0.15s",
              }}
            >
              <span style={{ marginRight: 6 }}>{isExpanded ? "▼" : "▶"}</span>
              <span style={{ flex: 1 }}>{displayName}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  undoExec(new RemoveStyleEntry(el.id, entry, index), ctx);
                  if (isExpanded) setExpandedStyleId(null);
                  onRecompile();
                }}
                style={{
                  background: "none", border: "none", color: "#DC2626",
                  cursor: "pointer", fontSize: 14, padding: "0 4px",
                }}
              >
                ×
              </button>
            </div>

            {/* Expanded editor */}
            {isExpanded && (
              <StyleEntryEditor
                entry={entry}
                elementId={el.id}
                onChange={(patch) => {
                  const oldEntry = useEditorStore
                    .getState()
                    .getStyles(el.id)
                    .find((s) => s.id === entry.id);
                  if (!oldEntry) return;
                  const newEntry = { ...oldEntry, ...patch };
                  undoExec(
                    new UpdateStyleEntry(el.id, entry.id, oldEntry, newEntry),
                    ctx,
                  );
                  onRecompile();
                }}
                onClose={() => setExpandedStyleId(null)}
              />
            )}
          </div>
        );
      })}

      {/* Bottom drop zone — dropping below the last entry */}
      {(el.styles?.length ?? 0) > 0 && (
        <div
          onDragOver={(e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
            const len = el.styles!.length;
            if (dropIndex !== len) setDropIndex(len);
          }}
          onDragLeave={() => {
            const len = el.styles!.length;
            if (dropIndex === len) setDropIndex(null);
          }}
          onDrop={() => { handleDropAt(el.styles!.length); }}
          style={{
            height: 4,
            background: dropIndex === el.styles!.length ? "#4F46E5" : "transparent",
            transition: "background 0.15s",
          }}
        />
      )}
    </>
  );
}

// ═══════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════

/** Immutable reorder of an array: move element at `from` index to `to` index. */
function reorderArray<T>(arr: readonly T[], from: number, to: number): T[] {
  const result = [...arr];
  const [removed] = result.splice(from, 1);
  result.splice(to, 0, removed);
  return result;
}

// ═══════════════════════════════════════════════
// Styles
// ═══════════════════════════════════════════════

const sectionHeaderStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" };
