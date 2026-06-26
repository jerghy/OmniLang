/** ComponentTree -- 元素树状面板，支持右键菜单、选择、折叠/展开。
 *
 *  Phase 3: 作为结构编辑的主要界面。右键菜单提供添加子组件（按分类级联）、
 *  复制/粘贴、上移/下移、删除、编辑自定义组件等操作。 */

import React, { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { useEditorStore } from "../store/editor";
import type { ElementData } from "../core/types";
import { useUndoStore } from "../store/undo";
import { getUnifiedComponentMeta, useComponentRegistry } from "../store/componentRegistry";
import type { CommandContext } from "../core/commands/types";
import { getDefaultProps } from "../core/constants";
import { AddElement } from "../core/commands/AddElement";
import { RemoveElement } from "../core/commands/RemoveElement";
import { ReorderElement } from "../core/commands/ReorderElement";
import { MoveElement } from "../core/commands/MoveElement";
import { copySelection, pasteFromClipboard } from "../core/commands/CopyPaste";
import { uid } from "../utils/id";
import { CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS } from "../core/constants";

// ── Constants ───────────────────────────────────────

/** OmniLang 组件名 → HTML 元素真名映射 */
const TAG_TO_HTML: Record<string, string> = {
  Container: "div", Flex: "div", Grid: "div", Card: "div",
  Text: "p", Heading: "h1",
  Button: "button", Input: "input", TextArea: "textarea",
  Select: "select", Checkbox: "input[checkbox]",
  List: "ul", ListItem: "li",
  Image: "img", Spinner: "div", Progress: "progress",
  Alert: "div", Link: "a", Form: "form",
};

/** 根据 tag + props 推导 HTML 元素真名（处理动态 tagByProp 如 Heading level → h2） */
function resolveHtmlTag(tag: string, props: Record<string, unknown>): string {
  const base = TAG_TO_HTML[tag];
  if (!base) return tag;
  // Heading: level="2" → h2
  if (tag === "Heading" && props.level) return `h${props.level}`;
  return base;
}

/** 从 props 中提取元素的可读标识文字 */
function getDisplayLabel(props: Record<string, unknown>): string {
  const p = props as Record<string, unknown>;
  return String(p.label ?? p.placeholder ?? p.alt ?? p.href ?? "");
}

// ── Types ───────────────────────────────────────────

interface TreeNode {
  id: string;
  tag: string;
  htmlTag: string;
  displayLabel: string;
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

  // Drag & drop state
  const [dropTarget, setDropTarget] = useState<{
    targetId: string;
    position: "before" | "after" | "inside";
  } | null>(null);
  const dragIdRef = useRef<string | null>(null);
  const dropPosRef = useRef<{ targetId: string; position: "before" | "after" | "inside" } | null>(null);

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
      htmlTag: resolveHtmlTag(el.tag, el.props as Record<string, unknown>),
      displayLabel: getDisplayLabel(el.props as Record<string, unknown>),
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

  // ── Drag & Drop ───────────────────────────────────

  const handleDragOver = useCallback(
    (targetId: string | null, position: "before" | "after" | "inside") => {
      if (targetId) {
        setDropTarget({ targetId, position });
        dropPosRef.current = { targetId, position };
      } else {
        setDropTarget(null);
        dropPosRef.current = null;
      }
    },
    [],
  );

  const handleDrop = useCallback(
    (targetId: string) => {
      setDropTarget(null);
      const draggedId = dragIdRef.current;
      if (!draggedId || draggedId === targetId) return;

      // Use ref for reliable drop position (state may be stale)
      const dropInfo = dropPosRef.current;
      const dropPos = dropInfo?.targetId === targetId ? dropInfo.position : "after";

      if (dropPos === "inside") {
        // Move dragged element INTO target as a child
        undoExec(new MoveElement(draggedId, targetId, null, "after"), ctx);
      } else {
        const draggedEl = ctx.getElement(draggedId);
        const targetEl = ctx.getElement(targetId);

        if (draggedEl && targetEl && draggedEl.parentId === targetEl.parentId) {
          // Same parent → reorder among siblings
          undoExec(new ReorderElement(draggedId, targetId, dropPos), ctx);
        } else if (targetEl) {
          // Different parent → move dragged element INTO target's parent
          // at the position relative to target (e.g. p after div-B)
          const newParentId = targetEl.parentId;
          if (newParentId !== null) {
            undoExec(new MoveElement(draggedId, newParentId, targetId, dropPos), ctx);
          }
          // If target is root-level (parentId === null), cross-parent drop to root
          // is not supported yet — currently drops silently.
        }
      }
      dragIdRef.current = null;
      dropPosRef.current = null;
    },
    [undoExec, ctx],
  );

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
              onDragStart={(id) => { dragIdRef.current = id; }}
              onDragOver={handleDragOver}
              onDrop={handleDrop}
              dropTarget={dropTarget}
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
  onDragStart?: (id: string) => void;
  onDragOver: (targetId: string | null, position: "before" | "after" | "inside") => void;
  onDrop: (targetId: string) => void;
  dropTarget: { targetId: string; position: "before" | "after" | "inside" } | null;
}

function TreeNodeView({
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
      case "before": return { borderTop: `2px solid ${borderColor}` };
      case "after": return { borderBottom: `2px solid ${borderColor}` };
      case "inside": return { outline: `2px solid ${borderColor}`, outlineOffset: -2 };
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
        onDragStart={(e) => { e.dataTransfer.setData("text/omni-tree-id", node.id); onDragStart?.(node.id); }}
        onDragOver={handleDragOver}
        onDragLeave={() => onDragOver(null, "after")}
        onDrop={(e) => { e.preventDefault(); e.stopPropagation(); onDrop(node.id); }}
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
  const addChildTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

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

  // For custom components, check registry and set componentRef
  const registryDefs = useComponentRegistry((s) => s.definitions);

  const handleAddChild = useCallback(
    (parentId: string, tag: string) => {
      const isCustom = registryDefs[tag] !== undefined;
      const el: ElementData = {
        id: uid(tag),
        nodeType: "view",
        tag,
        props: getDefaultProps(tag),
        children: [],
        parentId,
        ...(isCustom ? { componentRef: tag } : {}),
      };
      const cmd = new AddElement(el);
      undoExec(cmd, ctx);
      onClose();
    },
    [undoExec, ctx, onClose, registryDefs],
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
        onMouseEnter={() => {
          setShowAddChild(true);
          if (addChildTimer.current) clearTimeout(addChildTimer.current);
        }}
        onMouseLeave={() => {
          // Delay hiding to allow moving to submenu
          addChildTimer.current = setTimeout(() => {
            setShowAddChild(false);
            setActiveCategory(null);
          }, 150);
        }}
      >
        <span>添加子组件 ▸</span>

        {showAddChild && (
          <div style={submenu}
            onMouseEnter={() => {
              if (addChildTimer.current) clearTimeout(addChildTimer.current);
            }}
            onMouseLeave={() => {
              setShowAddChild(false);
              setActiveCategory(null);
            }}
          >
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
                  <div style={subSubmenu}
                    onMouseEnter={() => {
                      if (addChildTimer.current) clearTimeout(addChildTimer.current);
                    }}
                  >
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
