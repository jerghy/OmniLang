import React from "react";
import { COMPONENT_ICONS } from "../../core/constants";
import type { TreeNode } from "./tree-utils";

interface TreeNodeViewProps {
  node: TreeNode;
  depth: number;
  selectedIds: string[];
  collapsed: Set<string>;
  onToggle: (id: string) => void;
  onSelect: (id: string, multi?: boolean) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
  onDragStart?: (id: string) => void;
  onDragOver: (targetId: string | null, position: "before" | "after" | "inside") => void;
  onDrop: (targetId: string) => void;
  dropTarget: { targetId: string; position: "before" | "after" | "inside" } | null;
}

export function TreeNodeView({
  node,
  depth,
  selectedIds,
  collapsed,
  onToggle,
  onSelect,
  onContextMenu,
  onDragStart,
  onDragOver,
  onDrop,
  dropTarget,
}: TreeNodeViewProps) {
  const isSelected = selectedIds.includes(node.id);
  const isCollapsed = collapsed.has(node.id);
  const hasChildren = node.children.length > 0;

  const getDropStyle = (): React.CSSProperties => {
    if (!dropTarget || dropTarget.targetId !== node.id) return {};
    const borderColor = "#4F46E5";
    switch (dropTarget.position) {
      case "before":
        return { borderTop: `2px solid ${borderColor}` };
      case "after":
        return { borderBottom: `2px solid ${borderColor}` };
      case "inside":
        return { outline: `2px solid ${borderColor}`, outlineOffset: -2 };
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height;
    let position: "before" | "after" | "inside";
    if (y < h * 0.25) position = "before";
    else if (y > h * 0.75) position = "after";
    else position = hasChildren ? "inside" : "after";
    onDragOver(node.id, position);
  };

  return (
    <div>
      {/* Node row */}
      <div
        draggable
        onClick={(e) => onSelect(node.id, e.ctrlKey || e.metaKey)}
        onContextMenu={(e) => onContextMenu(e, node.id)}
        onDragStart={(e) => {
          e.dataTransfer.setData("text/omni-tree-id", node.id);
          onDragStart?.(node.id);
        }}
        onDragOver={handleDragOver}
        onDragLeave={() => onDragOver(null, "after")}
        onDrop={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onDrop(node.id);
        }}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 3,
          padding: "3px 6px",
          paddingLeft: 8 + depth * 16,
          borderRadius: 4,
          cursor: "pointer",
          fontSize: 12,
          background: isSelected ? "#EEF2FF" : "transparent",
          color: isSelected ? "#4F46E5" : "#374151",
          fontWeight: isSelected ? 600 : 400,
          border: isSelected ? "1px solid #C7D2FE" : "1px solid transparent",
          marginBottom: 1,
          ...getDropStyle(),
        }}
        onMouseEnter={(e) => {
          if (!isSelected) {
            e.currentTarget.style.background = "#F9FAFB";
            e.currentTarget.style.borderColor = "#F3F4F6";
          }
        }}
        onMouseLeave={(e) => {
          if (!isSelected) {
            e.currentTarget.style.background = "transparent";
            e.currentTarget.style.borderColor = "transparent";
          }
        }}
      >
        {/* Expand/collapse toggle */}
        {hasChildren ? (
          <span
            onClick={(e) => {
              e.stopPropagation();
              onToggle(node.id);
            }}
            style={{
              fontSize: 8,
              width: 12,
              textAlign: "center",
              cursor: "pointer",
              flexShrink: 0,
              transition: "transform 0.15s",
              transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)",
            }}
          >
            ▼
          </span>
        ) : (
          <span style={{ width: 12, flexShrink: 0 }} />
        )}

        {/* Icon */}
        <span
          style={{
            fontSize: 11,
            width: 16,
            textAlign: "center",
            flexShrink: 0,
          }}
        >
          {COMPONENT_ICONS[node.tag] || "▪"}
        </span>

        {/* Label — 显示 HTML 真名 + 标识文字 */}
        <span
          style={{
            flex: 1,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          <span style={{ color: "#8B5CF6", fontWeight: 500 }}>&lt;{node.htmlTag}&gt;</span>
          {node.displayLabel && (
            <span style={{ color: "#6B7280", marginLeft: 4 }}>
              "{node.displayLabel}"
            </span>
          )}
        </span>

        {/* Custom component badge */}
        {node.componentRef && (
          <span
            style={{
              fontSize: 9,
              color: "#8B5CF6",
              background: "#F5F3FF",
              padding: "0 4px",
              borderRadius: 3,
            }}
          >
            ★
          </span>
        )}
      </div>

      {/* Children (recursive) */}
      {hasChildren && !isCollapsed && (
        <div>
          {node.children.map((child) => (
            <TreeNodeView
              key={child.id}
              node={child}
              depth={depth + 1}
              selectedIds={selectedIds}
              collapsed={collapsed}
              onToggle={onToggle}
              onSelect={onSelect}
              onContextMenu={onContextMenu}
              onDragStart={onDragStart}
              onDragOver={onDragOver}
              onDrop={onDrop}
              dropTarget={dropTarget}
            />
          ))}
        </div>
      )}
    </div>
  );
}
