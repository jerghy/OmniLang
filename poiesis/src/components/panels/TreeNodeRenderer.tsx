/** TreeNodeRenderer — renders a single tree node with @dnd-kit drag-and-drop support.
 *  Each node is both draggable (can be reordered/nested) and droppable (accepts other nodes).
 *  Visual indicators show the drop position: line (before/after) or highlight (inside). */

import React from "react";
import { useDraggable, useDroppable } from "@dnd-kit/core";
import type { TreeNode } from "./tree-utils";
import styles from "./TreeNodeRenderer.module.css";

export interface TreeDropState {
  targetId: string;
  position: "before" | "after" | "inside";
}

interface TreeNodeViewProps {
  node: TreeNode;
  depth: number;
  selectedIds: Set<string>;
  collapsed: Set<string>;
  onToggle: (id: string) => void;
  onSelect: (id: string, multi?: boolean) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
  treeDropState: TreeDropState | null;
}

export const TreeNodeView = React.memo(function TreeNodeView({
  node,
  depth,
  selectedIds,
  collapsed,
  onToggle,
  onSelect,
  onContextMenu,
  treeDropState,
}: TreeNodeViewProps) {
  const hasChildren = node.children && node.children.length > 0;
  const isSelected = selectedIds.has(node.id);
  const isCollapsed = collapsed.has(node.id);

  // ── @dnd-kit: draggable ──
  const {
    attributes,
    listeners,
    setNodeRef: setDragRef,
    isDragging,
  } = useDraggable({
    id: `tree-${node.id}`,
    data: { type: "tree-node", elementId: node.id },
  });

  // ── @dnd-kit: droppable ──
  const { setNodeRef: setDropRef, isOver } = useDroppable({
    id: `tree-${node.id}`,
    data: { type: "tree-node", elementId: node.id },
  });

  // Combine refs (both draggable + droppable on the same element)
  const setRefs = (el: HTMLDivElement | null) => {
    setDragRef(el);
    setDropRef(el);
  };

  // ── Drop indicator computation ──
  const isDropTarget = treeDropState?.targetId === node.id;
  const dropBefore = isDropTarget && treeDropState?.position === "before";
  const dropAfter = isDropTarget && treeDropState?.position === "after";
  const dropInside = isDropTarget && treeDropState?.position === "inside";

  // ── Conditional branch node rendering ──
  if (node.nodeType === "conditional-branch") {
    const isElse = node.branchType === "else";
    return (
      <div>
        {dropBefore && <DropLine depth={depth} />}
        <div
          role="treeitem"
          tabIndex={0}
          ref={setRefs}
          {...listeners}
          {...attributes}
          onClick={(e) => onSelect(node.id, e.ctrlKey || e.metaKey)}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(node.id, e.ctrlKey || e.metaKey); } }}
          onContextMenu={(e) => onContextMenu(e, node.id)}
          className={styles.treeNode}
          style={{
            paddingLeft: depth * 16 + 4,
            cursor: isDragging ? "grabbing" : "pointer",
            background: isSelected ? "var(--ol-color-primary-subtle)" : "transparent",
            opacity: isDragging ? 0.35 : 1,
          }}
        >
          {hasChildren ? (
            <button
              type="button"
              tabIndex={0}
              onClick={(e) => { e.stopPropagation(); onToggle(node.id); }}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); e.preventDefault(); onToggle(node.id); } }}
              className={styles.toggleBtn}
              style={{
                transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)",
              }}
            >
              ▼
            </button>
          ) : (
            <span style={{ width: 12 }} />
          )}
          <span style={{ color: isElse ? "var(--ol-text-muted)" : "var(--ol-color-primary)", fontSize: 12, fontWeight: 500 }}>
            {node.branchType}
          </span>
          {!isElse && (
            <span style={{ color: "var(--ol-gray-700)", fontSize: 12, fontFamily: "monospace" }}>
              {node.condition || "(空条件)"}
            </span>
          )}
        </div>
        {dropAfter && <DropLine depth={depth} />}
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
                treeDropState={treeDropState}
              />
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <div>
      {/* Before indicator */}
      {dropBefore && <DropLine depth={depth} />}

      <div
        role="treeitem"
        tabIndex={0}
        ref={setRefs}
        {...listeners}
        {...attributes}
        onClick={(e) => onSelect(node.id, e.ctrlKey || e.metaKey)}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(node.id, e.ctrlKey || e.metaKey); } }}
        onContextMenu={(e) => onContextMenu(e, node.id)}
        className={styles.treeNode}
        style={{
          paddingLeft: depth * 16 + 4,
          cursor: isDragging ? "grabbing" : "grab",
          background: dropInside ? "var(--ol-color-primary-subtle)" : isSelected ? "var(--ol-color-primary-subtle)" : "transparent",
          opacity: isDragging ? 0.35 : 1,
          outline: dropInside ? "1px solid var(--ol-color-primary-hover)" : isOver ? "1px dashed #A5B4FC" : "none",
          outlineOffset: "-1px",
        }}
      >
        {/* Expand/collapse toggle */}
        {hasChildren ? (
          <button
            type="button"
            tabIndex={0}
            onClick={(e) => {
              e.stopPropagation();
              onToggle(node.id);
            }}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); e.preventDefault(); onToggle(node.id); } }}
            className={styles.toggleBtn}
            style={{
              transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)",
            }}
          >
            ▼
          </button>
        ) : (
          <span style={{ width: 12 }} />
        )}

        {/* ── Conditional block root node extra styling ── */}
        {node.nodeType === "conditional" && (
          <span style={{ color: "var(--ol-color-primary)", fontSize: 13 }}>🔀</span>
        )}

        {/* Element icon + tag */}
        <span style={{ color: "var(--ol-gray-700)", fontSize: 12, fontWeight: 500 }}>
          {node.tag}
        </span>

        {/* Display label */}
        {node.displayLabel && (
          <span
            style={{
              color: "var(--ol-text-muted)",
              fontSize: 12,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              maxWidth: 120,
            }}
          >
            {node.displayLabel}
          </span>
        )}
      </div>

      {/* After indicator */}
      {dropAfter && <DropLine depth={depth} />}

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
              treeDropState={treeDropState}
            />
          ))}
        </div>
      )}
    </div>
  );
});

/** Thin blue line indicator for before/after drop position. */
function DropLine({ depth }: { depth: number }) {
  return (
    <div
      className={styles.dropLine}
      style={{
        marginLeft: depth * 16 + 4,
      }}
    />
  );
}
