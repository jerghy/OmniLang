/** ComponentTree — element tree panel with @dnd-kit drag-and-drop for reordering/nesting.
 *  Drag events are handled by the parent DesignPage's unified DndContext.
 *  Also supports right-click context menu with "Add child component" picker. */

import React, { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { usePoiesisStore, getUnifiedComponentMeta } from "../../store";
import { useElements, useRootOrder, useSelectedElementIds, useComponentDefs } from "../../store/hooks";
import type { ElementData } from "../../core/types";
import styles from "./ComponentTree.module.css";
import { getDefaultProps } from "../../core/constants/defaults";
import {
  CATEGORY_LABELS,
  CATEGORY_ORDER,
  COMPONENT_ICONS,
} from "../../core/constants/components";
import { copySelection, pasteFromClipboard } from "../../core/clipboard";
import { uid } from "../../lib/utils/id";
import { buildTreeNode, type TreeNode } from "./tree-utils";
import { TreeNodeView, type TreeDropState } from "./TreeNodeRenderer";

interface ComponentTreeProps {
  treeDropState: TreeDropState | null;
}

export function ComponentTree({ treeDropState }: ComponentTreeProps) {
  const elements = useElements();
  const rootOrder = useRootOrder();
  const selectedIds = useSelectedElementIds();
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const enterCanvas = usePoiesisStore((s) => s.enterComponentCanvas);

  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    elementId: string;
  } | null>(null);

  // ── Build tree from flat elements ──
  const treeData = useMemo((): TreeNode[] => {
    const elMap = new Map(elements.map((e) => [e.id, e]));
    const rootIds = rootOrder.filter((id) => elMap.has(id));

    return rootIds
      .map((id) => {
        const el = elMap.get(id);
        return el ? buildTreeNode(el, elMap) : null;
      })
      .filter((n): n is TreeNode => n !== null);
  }, [elements, rootOrder]);

  // ── Tree toggle / context menu ──
  const toggleCollapse = useCallback((id: string) => {
    setCollapsed((prev) => {
      const n = new Set(prev);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });
  }, []);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, elementId: string) => {
      e.preventDefault();
      e.stopPropagation();
      // Select the right-clicked element first
      selectElement(elementId);
      setContextMenu({ x: e.clientX, y: e.clientY, elementId });
    },
    [selectElement],
  );

  const closeMenu = useCallback(() => setContextMenu(null), []);

  // ── Render ──
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
          fontSize: 12,
          color: "#6b7280",
          textTransform: "uppercase",
          display: "flex",
          alignItems: "center",
          gap: 4,
        }}
      >
        🌳 元素树
      </div>

      {/* Tree content */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {treeData.length === 0 ? (
          <div
            style={{
              textAlign: "center",
              color: "#9CA3AF",
              fontSize: 12,
              padding: "16px 0",
            }}
          >
            空画布 — 从组件面板拖入元素
          </div>
        ) : (
          treeData.map((node) => (
            <TreeNodeView
              key={node.id}
              node={node}
              depth={0}
              selectedIds={new Set(selectedIds)}
              collapsed={collapsed}
              onToggle={toggleCollapse}
              onSelect={(id, multi) => selectElement(id, multi)}
              onContextMenu={handleContextMenu}
              treeDropState={treeDropState}
            />
          ))
        )}
      </div>

      {/* Right-click context menu */}
      {contextMenu && (
        <>
          <button
            type="button"
            tabIndex={0}
            aria-label="Close context menu"
            style={{ position: "fixed", inset: 0, zIndex: 9998, background: "transparent", border: "none", cursor: "pointer" }}
            onClick={closeMenu}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); closeMenu(); } }}
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
            enterCanvas={enterCanvas}
            onClose={closeMenu}
          />
        </>
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
  enterCanvas: (name: string) => void;
  onClose: () => void;
}

function ContextMenu({
  x,
  y,
  elementId,
  elements,
  rootOrder,
  enterCanvas,
  onClose,
}: ContextMenuProps) {
  const [showAddChild, setShowAddChild] = useState(false);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const addChildTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const contextElement = elements.find((e) => e.id === elementId);
  const registryDefs = useComponentDefs() ?? {};

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", h);
    return () => document.removeEventListener("keydown", h);
  }, [onClose]);

  const groupedComponents = useMemo(() => {
    const allMeta = getUnifiedComponentMeta();
    const map = new Map<string, Array<{ component: string; isCustom: boolean }>>();
    for (const [name, meta] of Object.entries(allMeta)) {
      const list = map.get(meta.category) || [];
      list.push({
        component: name,
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        isCustom: meta.isCustom ?? false,
      });
      map.set(meta.category, list);
    }
    const sorted = new Map<string, Array<{ component: string; isCustom: boolean }>>();
    for (const cat of CATEGORY_ORDER) {
      if (map.has(cat)) sorted.set(cat, map.get(cat)!);
    }
    for (const [cat, items] of map) {
      if (!sorted.has(cat)) sorted.set(cat, items);
    }
    return sorted;
  }, []);

  const handleAddChild = useCallback(
    (parentId: string, tag: string) => {
      const isCustom = registryDefs[tag] !== undefined;
      const childId = uid(tag);

      // Branch node: synthetic ID like "condId.br0" — add to block then move to branch
      const branchMatch = parentId.match(/^(.+)\.br(\d+)$/);
      if (branchMatch) {
        const blockId = branchMatch[1];
        const branchIdx = parseInt(branchMatch[2], 10);
        usePoiesisStore.getState().addElement({
          id: childId, nodeType: "view", tag,
          props: getDefaultProps(tag), children: [],
          parentId: blockId,
          ...(isCustom ? { componentRef: tag } : {}),
        });
        usePoiesisStore.getState().addChildToBranch(blockId, branchIdx, childId);
      } else {
        usePoiesisStore.getState().addElement({
          id: childId, nodeType: "view", tag,
          props: getDefaultProps(tag), children: [],
          parentId,
          ...(isCustom ? { componentRef: tag } : {}),
        });
      }
      onClose();
    },
    [onClose, registryDefs],
  );

  const handleCopy = useCallback(() => {
    copySelection([elementId]);
    onClose();
  }, [elementId, onClose]);

  const handlePaste = useCallback(() => {
    const ids = pasteFromClipboard();
    if (ids?.length) usePoiesisStore.setState({ selectedElementIds: ids });
    onClose();
  }, [onClose]);

  const handleMoveUp = useCallback(() => {
    const el = elements.find((e) => e.id === elementId);
    if (!el) return;
    const parent = el.parentId ? elements.find((e) => e.id === el.parentId) : undefined;
    const siblings = parent && "children" in parent ? parent.children : rootOrder;
    const idx = siblings.indexOf(elementId);
    if (idx <= 0) return;
    usePoiesisStore.getState().reorderElement(elementId, siblings[idx - 1], "before");
    onClose();
  }, [elements, elementId, rootOrder, onClose]);

  const handleMoveDown = useCallback(() => {
    const el = elements.find((e) => e.id === elementId);
    if (!el) return;
    const parent = el.parentId ? elements.find((e) => e.id === el.parentId) : undefined;
    const siblings = parent && "children" in parent ? parent.children : rootOrder;
    const idx = siblings.indexOf(elementId);
    if (idx < 0 || idx >= siblings.length - 1) return;
    usePoiesisStore.getState().reorderElement(elementId, siblings[idx + 1], "after");
    onClose();
  }, [elements, elementId, rootOrder, onClose]);

  const handleDelete = useCallback(() => {
    usePoiesisStore.getState().removeElement(elementId);
    onClose();
  }, [elementId, onClose]);

  const handleEditComponent = useCallback(() => {
    const compRef = contextElement && "componentRef" in contextElement ? contextElement.componentRef : undefined;
    if (compRef) enterCanvas(compRef);
    onClose();
  }, [contextElement, enterCanvas, onClose]);

  const [adjustedX, adjustedY] = useMemo(
    () => [
      Math.min(x, window.innerWidth - 210),
      Math.min(y, window.innerHeight - 300),
    ],
    [x, y],
  );

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
      {/* Add Child */}
      <div
        style={ST.itemBase}
        onMouseEnter={() => {
          setShowAddChild(true);
          if (addChildTimer.current) clearTimeout(addChildTimer.current);
        }}
        onMouseLeave={() => {
          addChildTimer.current = setTimeout(() => {
            setShowAddChild(false);
            setActiveCategory(null);
          }, 150);
        }}
      >
        <span>添加子组件 ▸</span>
        {showAddChild && (
          <div
            style={ST.submenu}
            onMouseEnter={() => {
              if (addChildTimer.current) clearTimeout(addChildTimer.current);
            }}
            onMouseLeave={() => {
              setShowAddChild(false);
              setActiveCategory(null);
            }}
          >
            {/* 条件块 — 特殊子组件类型 */}
            <button
              type="button"
              tabIndex={0}
              className={styles.subMenuItem}
              style={{ ...ST.itemBase, whiteSpace: "nowrap", background: "none", border: "none", width: "100%", textAlign: "left", fontFamily: "inherit", fontSize: "inherit" }}
              onClick={() => {
                usePoiesisStore.getState().addConditionalBlock(elementId);
                onClose();
              }}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); usePoiesisStore.getState().addConditionalBlock(elementId); onClose(); } }}
            >
              <span>
                <span style={{ marginRight: 4, fontSize: 12 }}>🔀</span>
                条件块
              </span>
            </div>
            <div style={{ height: 1, background: "#E5E7EB", margin: "3px 8px" }} />

            {Array.from(groupedComponents.entries()).map(([cat, items]) => (
              <div
                key={cat}
                style={{ ...ST.itemBase, paddingRight: 24 }}
                onMouseEnter={() => setActiveCategory(cat)}
                onMouseLeave={() => {
                  if (activeCategory === cat) setActiveCategory(null);
                }}
              >
                <span>{CATEGORY_LABELS[cat] || cat}</span>
                <span style={{ color: "#9CA3AF", fontSize: 12 }}>▸</span>
                {activeCategory === cat && (
                  <div
                    style={ST.subSubmenu}
                    onMouseEnter={() => {
                      if (addChildTimer.current) clearTimeout(addChildTimer.current);
                    }}
                  >
                    {items.map((comp) => (
                      <button
                        key={comp.component}
                        type="button"
                        tabIndex={0}
                        className={styles.subMenuItem}
                        style={{ ...ST.itemBase, whiteSpace: "nowrap", background: "none", border: "none", width: "100%", textAlign: "left", fontFamily: "inherit", fontSize: "inherit" }}
                        onClick={() => handleAddChild(elementId, comp.component)}
                        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleAddChild(elementId, comp.component); } }}
                      >
                        <span>
                          <span style={{ marginRight: 4, fontSize: 12 }}>
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

      <Separator />
      <MenuRow label="复制" shortcut="Ctrl+C" onClick={handleCopy} />
      <MenuRow label="粘贴" shortcut="Ctrl+V" onClick={handlePaste} />
      <Separator />
      <MenuRow label="上移" shortcut="Ctrl+↑" onClick={handleMoveUp} />
      <MenuRow label="下移" shortcut="Ctrl+↓" onClick={handleMoveDown} />
      <MenuRow label="删除" shortcut="Delete" onClick={handleDelete} />

      {/* ── 条件块菜单项 ── */}
      {/* Branch node: synthetic IDs like "comp.@if.0.br1" */}
      {(() => {
        const branchMatch = elementId.match(/^(.+)\.br(\d+)$/);
        if (branchMatch) {
          const blockId = branchMatch[1];
          const branchIdx = parseInt(branchMatch[2], 10);
          const block = elements.find((e) => e.id === blockId && e.nodeType === "conditional") as import("../../core/types/element").ConditionalBlockData | undefined;
          const branch = block?.branches[branchIdx];
          if (branch && branch.type !== "if" && branch.type !== "else") {
            return (
              <MenuRow
                label="删除此分支"
                onClick={() => {
                  usePoiesisStore.getState().removeConditionalBranch(blockId, branchIdx);
                  onClose();
                }}
              />
            );
          }
          return null;
        }
        return null;
      })()}

      {contextElement?.nodeType === "conditional" ? (
        <>
          <MenuRow
            label="添加 @else if 分支"
            onClick={() => {
              const store = usePoiesisStore.getState();
              store.addConditionalBranch(elementId, "elif");
              onClose();
            }}
          />
          <MenuRow
            label="添加 @else 分支"
            onClick={() => {
              const store = usePoiesisStore.getState();
              store.addConditionalBranch(elementId, "else");
              onClose();
            }}
          />
          <Separator />
        </>
      ) : null}

      {contextElement && "componentRef" in contextElement && contextElement.componentRef && (
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
    <button
      type="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick(); } }}
      className={styles.menuRow}
      style={{
        padding: "6px 16px",
        cursor: "pointer",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        fontSize: 12,
        color: "#374151",
        background: "none",
        border: "none",
        width: "100%",
        textAlign: "left",
        fontFamily: "inherit",
      }}
    >
      <span>{label}</span>
      {shortcut && (
        <span
          style={{
            color: "#9CA3AF",
            fontSize: 12,
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

function Separator() {
  return <div style={{ height: 1, background: "#F3F4F6", margin: "4px 8px" }} />;
}

const ST = {
  itemBase: {
    padding: "6px 16px",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    fontSize: 12,
    color: "#374151",
    position: "relative" as const,
  },
  submenu: {
    position: "absolute" as const,
    left: "100%",
    top: 0,
    background: "#fff",
    border: "1px solid #E5E7EB",
    borderRadius: 6,
    boxShadow: "0 4px 12px rgba(0,0,0,0.12)",
    minWidth: 160,
    padding: "3px 0",
    zIndex: 10000,
  },
  subSubmenu: {
    position: "absolute" as const,
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
    overflowY: "auto" as const,
  },
};
