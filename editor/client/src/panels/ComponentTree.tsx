/** ComponentTree -- 元素树状面板，支持右键菜单、选择、折叠/展开。
 *
 *  Phase 3: 作为结构编辑的主要界面。右键菜单提供添加子组件（按分类级联）、
 *  复制/粘贴、上移/下移、删除、编辑自定义组件等操作。 */

import React, { useState, useCallback, useEffect, useMemo } from "react";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { getUnifiedComponentMeta, useComponentRegistry } from "../store/componentRegistry";
import type { CommandContext } from "../commands/types";
import { AddElement } from "../commands/AddElement";
import { RemoveElement } from "../commands/RemoveElement";
import { ReorderElement } from "../commands/ReorderElement";
import { copySelection, pasteFromClipboard } from "../commands/CopyPaste";
import { uid } from "../utils/id";

// ── Constants ───────────────────────────────────────

const CATEGORY_LABELS: Record<string, string> = {
  Layout: "布局", Typography: "排版", Actions: "操作", Forms: "表单",
  Data: "数据", Media: "媒体", Feedback: "反馈", Navigation: "导航",
  Custom: "★ 自定义",
};

const CATEGORY_ORDER = ["Custom", "Layout", "Typography", "Actions", "Forms", "Data", "Media", "Feedback", "Navigation"];

const COMPONENT_ICONS: Record<string, string> = {
  Container: "📦", Flex: "↔", Grid: "⊞", Card: "🃏",
  Text: "📝", Heading: "📰",
  Button: "🔘", Input: "📋", TextArea: "📄", Select: "📌", Checkbox: "☑",
  List: "📃", ListItem: "📎",
  Image: "🖼",
  Spinner: "⏳", Progress: "📊", Alert: "⚠",
  Link: "🔗", Form: "📨",
};

// ── Types ───────────────────────────────────────────

interface TreeNode {
  id: string;
  tag: string;
  componentRef?: string;
  children: TreeNode[];
}

// ═════════════════════════════════════════════════════
// ComponentTree
// ═════════════════════════════════════════════════════

interface ComponentTreeProps {
  ctx: CommandContext;
}

export function ComponentTree({ ctx }: ComponentTreeProps) {
  const elements = useEditorStore((s) => s.elements);
  const rootOrder = useEditorStore((s) => s.rootOrder);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const selectElement = useEditorStore((s) => s.selectElement);
  const undoExec = useUndoStore((s) => s.execute);
  const enterCanvas = useEditorStore((s) => s.enterComponentCanvas);

  // Collapse/expand state (by element ID)
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    elementId: string;
  } | null>(null);

  // ── Build tree from flat elements ──────────────────

  const treeData = useMemo((): TreeNode[] => {
    const elMap = new Map(elements.map((e) => [e.id, e]));
    const rootIds = rootOrder.filter((id) => elMap.has(id));

    const childrenOf = (parentId: string): ElementData[] => {
      const parent = elMap.get(parentId);
      if (!parent) return [];
      return parent.children
        .map((cid) => elMap.get(cid))
        .filter((e): e is ElementData => e !== undefined);
    };

    const buildNode = (el: ElementData): TreeNode => ({
      id: el.id,
      tag: el.tag,
      componentRef: el.componentRef,
      children: childrenOf(el.id).map(buildNode),
    });

    return rootIds
      .map((id) => {
        const el = elMap.get(id);
        return el ? buildNode(el) : null;
      })
      .filter((n): n is TreeNode => n !== null);
  }, [elements, rootOrder]);

  // ── Handlers ───────────────────────────────────────

  const toggleCollapse = useCallback((id: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, elementId: string) => {
      e.preventDefault();
      e.stopPropagation();
      setContextMenu({ x: e.clientX, y: e.clientY, elementId });
    },
    [],
  );

  const closeMenu = useCallback(() => {
    setContextMenu(null);
  }, []);

  // ── Render ─────────────────────────────────────────

  return (
    <div
      style={{
        padding: 8,
        fontSize: 12,
        userSelect: "none",
        display: "flex",
        flexDirection: "column",
        height: "100%",
      }}
    >
      {/* Header */}
      <div
        style={{
          fontWeight: 600,
          marginBottom: 6,
          fontSize: 11,
          color: "#6b7280",
          textTransform: "uppercase",
          display: "flex",
          alignItems: "center",
          gap: 4,
        }}
      >
        🌳 元素树
      </div>

      {/* Tree */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {treeData.length === 0 ? (
          <div
            style={{
              textAlign: "center",
              color: "#9CA3AF",
              fontSize: 11,
              padding: "16px 0",
            }}
          >
            空画布 -- 从组件面板拖入元素
          </div>
        ) : (
          treeData.map((node) => (
            <TreeNodeView
              key={node.id}
              node={node}
              depth={0}
              selectedIds={selectedIds}
              collapsed={collapsed}
              onToggle={toggleCollapse}
              onSelect={(id, multi) => selectElement(id, multi)}
              onContextMenu={handleContextMenu}
            />
          ))
        )}
      </div>

      {/* ── Context Menu ── */}
      {contextMenu && (
        <>
          {/* Backdrop — catches clicks outside the menu */}
          <div
            style={{ position: "fixed", inset: 0, zIndex: 9998 }}
            onClick={closeMenu}
            onContextMenu={(e) => {
              e.preventDefault();
              closeMenu();
            }}
          />
          <ContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            elementId={contextMenu.elementId}
            elements={elements}
            rootOrder={rootOrder}
            undoExec={undoExec}
            ctx={ctx}
            enterCanvas={enterCanvas}
            onClose={closeMenu}
          />
        </>
      )}
    </div>
  );
}

// ═════════════════════════════════════════════════════
// TreeNodeView — recursive tree node
// ═════════════════════════════════════════════════════

interface TreeNodeViewProps {
  node: TreeNode;
  depth: number;
  selectedIds: string[];
  collapsed: Set<string>;
  onToggle: (id: string) => void;
  onSelect: (id: string, multi?: boolean) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
}

function TreeNodeView({
  node,
  depth,
  selectedIds,
  collapsed,
  onToggle,
  onSelect,
  onContextMenu,
}: TreeNodeViewProps) {
  const isSelected = selectedIds.includes(node.id);
  const isCollapsed = collapsed.has(node.id);
  const hasChildren = node.children.length > 0;

  return (
    <div>
      {/* Node row */}
      <div
        onClick={(e) => onSelect(node.id, e.ctrlKey || e.metaKey)}
        onContextMenu={(e) => onContextMenu(e, node.id)}
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

        {/* Label */}
        <span
          style={{
            flex: 1,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {node.tag}
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
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ═════════════════════════════════════════════════════
// ContextMenu — right-click context menu
// ═════════════════════════════════════════════════════

interface ContextMenuProps {
  x: number;
  y: number;
  elementId: string;
  elements: ElementData[];
  rootOrder: string[];
  undoExec: (cmd: unknown, ctx: CommandContext) => void;
  ctx: CommandContext;
  enterCanvas: (name: string) => void;
  onClose: () => void;
}

function ContextMenu({
  x,
  y,
  elementId,
  elements,
  rootOrder,
  undoExec,
  ctx,
  enterCanvas,
  onClose,
}: ContextMenuProps) {
  const [showAddChild, setShowAddChild] = useState(false);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);

  const contextElement = elements.find((e) => e.id === elementId);

  // Escape key to close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  // Track registry version so "Add Child" submenu stays fresh
  const registryVersion = useComponentRegistry((s) => Object.keys(s.definitions).length);

  // Build grouped component metadata for "Add Child"
  const groupedComponents = useMemo(() => {
    const allMeta = getUnifiedComponentMeta();
    const map = new Map<
      string,
      Array<{ component: string; isCustom: boolean }>
    >();
    for (const [name, meta] of Object.entries(allMeta)) {
      const list = map.get(meta.category) || [];
      list.push({ component: name, isCustom: meta.isCustom });
      map.set(meta.category, list);
    }
    // Sort by CATEGORY_ORDER
    const sorted = new Map<
      string,
      Array<{ component: string; isCustom: boolean }>
    >();
    for (const cat of CATEGORY_ORDER) {
      if (map.has(cat)) sorted.set(cat, map.get(cat)!);
    }
    for (const [cat, items] of map) {
      if (!sorted.has(cat)) sorted.set(cat, items);
    }
    return sorted;
  }, [registryVersion]);

  // ── Action handlers ───────────────────────────────

  const handleAddChild = useCallback(
    (parentId: string, tag: string) => {
      const el: ElementData = {
        id: uid(),
        nodeType: "view",
        tag,
        props: {},
        children: [],
        parentId,
      };
      const cmd = new AddElement(el);
      undoExec(cmd, ctx);
      onClose();
    },
    [undoExec, ctx, onClose],
  );

  const handleCopy = useCallback(() => {
    copySelection(ctx, [elementId]);
    onClose();
  }, [ctx, elementId, onClose]);

  const handlePaste = useCallback(() => {
    const pastedIds = pasteFromClipboard(ctx);
    if (pastedIds && pastedIds.length > 0) {
      useEditorStore.setState({ selectedElementIds: pastedIds });
    }
    onClose();
  }, [ctx, onClose]);

  const handleMoveUp = useCallback(() => {
    const el = elements.find((e) => e.id === elementId);
    if (!el) return;
    const siblings = el.parentId
      ? (elements.find((e) => e.id === el.parentId)?.children || [])
      : rootOrder;
    const idx = siblings.indexOf(elementId);
    if (idx <= 0) return;
    const cmd = new ReorderElement(elementId, siblings[idx - 1], "before");
    undoExec(cmd, ctx);
    onClose();
  }, [elements, elementId, rootOrder, undoExec, ctx, onClose]);

  const handleMoveDown = useCallback(() => {
    const el = elements.find((e) => e.id === elementId);
    if (!el) return;
    const siblings = el.parentId
      ? (elements.find((e) => e.id === el.parentId)?.children || [])
      : rootOrder;
    const idx = siblings.indexOf(elementId);
    if (idx < 0 || idx >= siblings.length - 1) return;
    const cmd = new ReorderElement(elementId, siblings[idx + 1], "after");
    undoExec(cmd, ctx);
    onClose();
  }, [elements, elementId, rootOrder, undoExec, ctx, onClose]);

  const handleDelete = useCallback(() => {
    const cmd = new RemoveElement(elementId);
    undoExec(cmd, ctx);
    onClose();
  }, [elementId, undoExec, ctx, onClose]);

  const handleEditComponent = useCallback(() => {
    if (contextElement?.componentRef) {
      enterCanvas(contextElement.componentRef);
    }
    onClose();
  }, [contextElement, enterCanvas, onClose]);

  // ── Adjust position to stay within viewport ───────

  const [adjustedX, adjustedY] = useMemo(() => {
    const menuWidth = 210;
    const menuHeight = 300;
    return [
      Math.min(x, window.innerWidth - menuWidth),
      Math.min(y, window.innerHeight - menuHeight),
    ];
  }, [x, y]);

  // ── Styles ─────────────────────────────────────────

  const itemBase: React.CSSProperties = {
    padding: "6px 16px",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    fontSize: 12,
    color: "#374151",
    position: "relative",
  };

  const submenu: React.CSSProperties = {
    position: "absolute",
    left: "100%",
    top: 0,
    background: "#fff",
    border: "1px solid #E5E7EB",
    borderRadius: 6,
    boxShadow: "0 4px 12px rgba(0,0,0,0.12)",
    minWidth: 160,
    padding: "3px 0",
    zIndex: 10000,
  };

  const subSubmenu: React.CSSProperties = {
    position: "absolute",
    left: "100%",
    top: 0,
    background: "#fff",
    border: "1px solid #E5E7EB",
    borderRadius: 6,
    boxShadow: "0 4px 12px rgba(0,0,0,0.12)",
    minWidth: 140,
    padding: "3px 0",
    zIndex: 10001,
    maxHeight: 280,
    overflowY: "auto",
  };

  // ── Render ─────────────────────────────────────────

  return (
    <div
      style={{
        position: "fixed",
        left: adjustedX,
        top: adjustedY,
        zIndex: 9999,
        background: "#fff",
        border: "1px solid #E5E7EB",
        borderRadius: 8,
        boxShadow: "0 4px 16px rgba(0,0,0,0.15)",
        minWidth: 200,
        padding: "4px 0",
        fontSize: 12,
      }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      {/* ── Add Child (cascading submenu) ── */}
      <div
        style={itemBase}
        onMouseEnter={() => setShowAddChild(true)}
        onMouseLeave={() => {
          setShowAddChild(false);
          setActiveCategory(null);
        }}
      >
        <span>添加子组件 ▸</span>

        {showAddChild && (
          <div style={submenu}>
            {Array.from(groupedComponents.entries()).map(([cat, items]) => (
              <div
                key={cat}
                style={{ ...itemBase, paddingRight: 24 }}
                onMouseEnter={() => setActiveCategory(cat)}
                onMouseLeave={() => {
                  if (activeCategory === cat) setActiveCategory(null);
                }}
              >
                <span>{CATEGORY_LABELS[cat] || cat}</span>
                <span style={{ color: "#9CA3AF", fontSize: 10 }}>▸</span>

                {activeCategory === cat && (
                  <div style={subSubmenu}>
                    {items.map((comp) => (
                      <div
                        key={comp.component}
                        style={{
                          ...itemBase,
                          whiteSpace: "nowrap",
                        }}
                        onClick={() => handleAddChild(elementId, comp.component)}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.background = "#F5F3FF";
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.background = "transparent";
                        }}
                      >
                        <span>
                          <span style={{ marginRight: 4, fontSize: 11 }}>
                            {COMPONENT_ICONS[comp.component] ||
                              (comp.isCustom ? "★" : "▪")}
                          </span>
                          {comp.component}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* ── Separator ── */}
      <Separator />

      {/* ── Copy ── */}
      <MenuRow label="复制" shortcut="Ctrl+C" onClick={handleCopy} />

      {/* ── Paste ── */}
      <MenuRow label="粘贴" shortcut="Ctrl+V" onClick={handlePaste} />

      {/* ── Separator ── */}
      <Separator />

      {/* ── Move Up ── */}
      <MenuRow label="上移" shortcut="Ctrl+↑" onClick={handleMoveUp} />

      {/* ── Move Down ── */}
      <MenuRow label="下移" shortcut="Ctrl+↓" onClick={handleMoveDown} />

      {/* ── Delete ── */}
      <MenuRow label="删除" shortcut="Delete" onClick={handleDelete} />

      {/* ── Edit Component (only for custom components) ── */}
      {contextElement?.componentRef && (
        <>
          <Separator />
          <MenuRow
            label={`编辑 ${contextElement.componentRef}`}
            onClick={handleEditComponent}
          />
        </>
      )}
    </div>
  );
}

// ═════════════════════════════════════════════════════
// MenuRow — single context menu row
// ═════════════════════════════════════════════════════

function MenuRow({
  label,
  shortcut,
  onClick,
}: {
  label: string;
  shortcut?: string;
  onClick: () => void;
}) {
  return (
    <div
      onClick={onClick}
      style={{
        padding: "6px 16px",
        cursor: "pointer",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        fontSize: 12,
        color: "#374151",
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.background = "#F5F3FF";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = "transparent";
      }}
    >
      <span>{label}</span>
      {shortcut && (
        <span
          style={{
            color: "#9CA3AF",
            fontSize: 10,
            marginLeft: 24,
            whiteSpace: "nowrap",
          }}
        >
          {shortcut}
        </span>
      )}
    </div>
  );
}

// ═════════════════════════════════════════════════════
// Separator — horizontal divider in context menu
// ═════════════════════════════════════════════════════

function Separator() {
  return (
    <div
      style={{ height: 1, background: "#F3F4F6", margin: "4px 8px" }}
    />
  );
}
