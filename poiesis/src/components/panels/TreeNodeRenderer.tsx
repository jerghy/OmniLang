import React from "react";
import type { TreeNode } from "./tree-utils";

interface TreeNodeViewProps {
  node: TreeNode;
  depth: number;
  selectedIds: Set<string>;
  collapsed: Set<string>;
  onToggle: (id: string) => void;
  onSelect: (id: string, multi?: boolean) => void;
  onDrop: (targetId: string, position: "before" | "after" | "inside") => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
  onDragStart: (id: string) => void;
  onDragOver: (e: React.DragEvent, id: string) => void;
  onDragEnd?: () => void;
  dropTarget: { targetId: string; position: string } | null;
}

export function TreeNodeView({
  node,
  depth,
  selectedIds,
  collapsed,
  onToggle,
  onSelect,
  onDrop,
  onContextMenu,
  onDragStart,
  onDragOver,
  onDragEnd,
  dropTarget,
}: TreeNodeViewProps) {
  const hasChildren = node.children && node.children.length > 0;
  const isSelected = selectedIds.has(node.id);
  const isCollapsed = collapsed.has(node.id);

  return (
    <div>
      <div
        draggable
        onDragStart={() => onDragStart(node.id)}
        onDragEnd={onDragEnd}
        onClick={(e) => onSelect(node.id, e.ctrlKey || e.metaKey)}
        onContextMenu={(e) => onContextMenu(e, node.id)}
        onDragOver={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onDragOver(e, node.id);
        }}
        onDrop={(e) => {
          e.preventDefault();
          onDrop(node.id, "inside");
        }}
        style={{
          paddingLeft: depth * 16 + 4,
          paddingRight: 4,
          paddingTop: 2,
          paddingBottom: 2,
          fontSize: 12,
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          gap: 4,
          background: isSelected ? "#EEF2FF" : "transparent",
          borderRadius: 3,
          userSelect: "none",
        }}
      >
        {hasChildren ? (
          <span
            onClick={(e) => { e.stopPropagation(); onToggle(node.id); }}
            style={{ fontSize: 8, width: 12, cursor: "pointer", transition: "transform 0.15s", transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)" }}
          >
            ▼
          </span>
        ) : (
          <span style={{ width: 12 }} />
        )}
        <span style={{ color: "#374151", fontSize: 11, fontWeight: 500 }}>{node.tag}</span>
        {node.displayLabel && (
          <span style={{ color: "#9ca3af", fontSize: 10, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 120 }}>
            {node.displayLabel}
          </span>
        )}
      </div>
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
              onDrop={onDrop}
              onContextMenu={onContextMenu}
              onDragStart={onDragStart}
              onDragOver={onDragOver}
              onDragEnd={onDragEnd}
              dropTarget={dropTarget}
            />
          ))}
        </div>
      )}
    </div>
  );
}
