/** ComponentTree -- 元素树状面板，支持右键菜单、选择、折叠/展开。 */

import React, { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { useEditorStore } from "../../store/editor";
import type { ElementData } from "../../core/types";
import { useUndoStore } from "../../store/undo";
import { getUnifiedComponentMeta, useComponentRegistry } from "../../store/componentRegistry";
import type { CommandContext } from "../../core/commands/types";
import { getDefaultProps, CATEGORY_LABELS, CATEGORY_ORDER, COMPONENT_ICONS } from "../../core/constants";
import { AddElement } from "../../core/commands/AddElement";
import { RemoveElement } from "../../core/commands/RemoveElement";
import { ReorderElement } from "../../core/commands/ReorderElement";
import { MoveElement } from "../../core/commands/MoveElement";
import { copySelection, pasteFromClipboard } from "../../core/commands/CopyPaste";
import { uid } from "../../lib/utils/id";
import { resolveHtmlTag, getDisplayLabel, type TreeNode } from "./tree-utils";
import { TreeNodeView } from "./TreeNodeRenderer";

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

  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; elementId: string } | null>(null);
  const [dropTarget, setDropTarget] = useState<{ targetId: string; position: "before" | "after" | "inside" } | null>(null);
  const dragIdRef = useRef<string | null>(null);
  const dropPosRef = useRef<{ targetId: string; position: "before" | "after" | "inside" } | null>(null);

  // ── Build tree from flat elements ──
  const treeData = useMemo((): TreeNode[] => {
    const elMap = new Map(elements.map((e) => [e.id, e]));
    const rootIds = rootOrder.filter((id) => elMap.has(id));

    const childrenOf = (parentId: string): ElementData[] => {
      const parent = elMap.get(parentId);
      if (!parent) return [];
      const p = parent as any;
      return (p.children || []).map((cid: string) => elMap.get(cid)).filter((e: any): e is ElementData => e !== undefined);
    };

    const buildNode = (el: ElementData): TreeNode => ({
      id: el.id,
      tag: el.tag,
      htmlTag: resolveHtmlTag(el.tag, el.props as Record<string, unknown>),
      displayLabel: getDisplayLabel(el.props as Record<string, unknown>),
      componentRef: el.componentRef,
      children: childrenOf(el.id).map(buildNode),
    });

    return rootIds.map((id: string) => { const el: any = elMap.get(id); return el ? buildNode(el as ElementData) : null; }).filter((n): n is TreeNode => n !== null);
  }, [elements, rootOrder]);

  // ── Handlers ──
  const toggleCollapse = useCallback((id: string) => {
    setCollapsed((prev) => { const n = new Set(prev); if (n.has(id)) n.delete(id); else n.add(id); return n; });
  }, []);

  const handleContextMenu = useCallback((e: React.MouseEvent, elementId: string) => {
    e.preventDefault(); e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, elementId });
  }, []);

  const closeMenu = useCallback(() => setContextMenu(null), []);

  // ── Drag & Drop ──
  const handleDragOver = useCallback((targetId: string | null, position: "before" | "after" | "inside") => {
    if (targetId) { setDropTarget({ targetId, position }); dropPosRef.current = { targetId, position }; }
    else { setDropTarget(null); dropPosRef.current = null; }
  }, []);

  const handleDrop = useCallback((targetId: string) => {
    setDropTarget(null);
    const draggedId = dragIdRef.current;
    if (!draggedId || draggedId === targetId) return;

    const dropInfo = dropPosRef.current;
    const dropPos = dropInfo?.targetId === targetId ? dropInfo.position : "after";

    if (dropPos === "inside") {
      undoExec(new MoveElement(draggedId, targetId, null, "after"), ctx);
    } else {
      const draggedEl = ctx.getElement(draggedId);
      const targetEl = ctx.getElement(targetId);
      if (draggedEl && targetEl && draggedEl.parentId === targetEl.parentId) {
        undoExec(new ReorderElement(draggedId, targetId, dropPos), ctx);
      } else if (targetEl) {
        const newParentId = targetEl.parentId;
        if (newParentId !== null) undoExec(new MoveElement(draggedId, newParentId, targetId, dropPos), ctx);
      }
    }
    dragIdRef.current = null;
    dropPosRef.current = null;
  }, [undoExec, ctx]);

  // ── Render ──
  return (
    <div style={{ padding: 8, fontSize: 12, userSelect: "none", display: "flex", flexDirection: "column", height: "100%" }}>
      <div style={{ fontWeight: 600, marginBottom: 6, fontSize: 11, color: "#6b7280", textTransform: "uppercase", display: "flex", alignItems: "center", gap: 4 }}>
        🌳 元素树
      </div>
      <div style={{ flex: 1, overflow: "auto" }}>
        {treeData.length === 0 ? (
          <div style={{ textAlign: "center", color: "#9CA3AF", fontSize: 11, padding: "16px 0" }}>
            空画布 -- 从组件面板拖入元素
          </div>
        ) : (
          treeData.map((node) => (
            <TreeNodeView
              key={node.id}
              node={node} depth={0}
              selectedIds={selectedIds} collapsed={collapsed}
              onToggle={toggleCollapse}
              onSelect={(id, multi) => selectElement(id, multi)}
              onContextMenu={handleContextMenu}
              onDragStart={(id) => { dragIdRef.current = id; }}
              onDragOver={(e: any) => { e.preventDefault(); handleDragOver(node.id, "inside"); }}
              onDrop={(targetId: string) => handleDrop(targetId)}
              dropTarget={dropTarget}
            />
          ))
        )}
      </div>

      {contextMenu && (
        <>
          <div style={{ position: "fixed", inset: 0, zIndex: 9998 }} onClick={closeMenu} onContextMenu={(e) => { e.preventDefault(); closeMenu(); }} />
          <ContextMenu x={contextMenu.x} y={contextMenu.y} elementId={contextMenu.elementId}
            elements={elements} rootOrder={rootOrder} undoExec={undoExec} ctx={ctx}
            enterCanvas={enterCanvas} onClose={closeMenu} />
        </>
      )}
    </div>
  );
}

// ═════════════════════════════════════════════════════
// ContextMenu — right-click context menu
// ═════════════════════════════════════════════════════

interface ContextMenuProps {
  x: number; y: number; elementId: string;
  elements: ElementData[]; rootOrder: string[];
  undoExec: (cmd: import("../../core/commands/types").Command, ctx: CommandContext) => void;
  ctx: CommandContext; enterCanvas: (name: string) => void; onClose: () => void;
}

function ContextMenu({ x, y, elementId, elements, rootOrder, undoExec, ctx, enterCanvas, onClose }: ContextMenuProps) {
  const [showAddChild, setShowAddChild] = useState(false);
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const addChildTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const contextElement = elements.find((e) => e.id === elementId);
  const registryDefs = useComponentRegistry((s) => s.definitions);

  useEffect(() => { const h = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); }; document.addEventListener("keydown", h); return () => document.removeEventListener("keydown", h); }, [onClose]);

  const groupedComponents = useMemo(() => {
    const allMeta = getUnifiedComponentMeta();
    const map = new Map<string, Array<{ component: string; isCustom: boolean }>>();
    for (const [name, meta] of Object.entries(allMeta)) {
      const list = map.get(meta.category) || [];
      list.push({ component: name, isCustom: (meta as any).isCustom ?? false });
      map.set(meta.category, list);
    }
    const sorted = new Map<string, Array<{ component: string; isCustom: boolean }>>();
    for (const cat of CATEGORY_ORDER) { if (map.has(cat)) sorted.set(cat, map.get(cat)!); }
    for (const [cat, items] of map) { if (!sorted.has(cat)) sorted.set(cat, items); }
    return sorted;
  }, []); // getUnifiedComponentMeta() reads fresh data from store internally

  const handleAddChild = useCallback((parentId: string, tag: string) => {
    const isCustom = registryDefs[tag] !== undefined;
    undoExec(new AddElement({ id: uid(tag), nodeType: "view", tag, props: getDefaultProps(tag), children: [], parentId, ...(isCustom ? { componentRef: tag } : {}) }), ctx);
    onClose();
  }, [undoExec, ctx, onClose, registryDefs]);

  const handleCopy = useCallback(() => { copySelection(ctx, [elementId]); onClose(); }, [ctx, elementId, onClose]);
  const handlePaste = useCallback(() => { const ids = pasteFromClipboard(ctx); if (ids?.length) useEditorStore.setState({ selectedElementIds: ids }); onClose(); }, [ctx, onClose]);
  const handleMoveUp = useCallback(() => {
    const el = elements.find((e) => e.id === elementId); if (!el) return;
    const siblings = el.parentId ? (elements.find((e) => e.id === el.parentId)?.children || []) : rootOrder;
    const idx = siblings.indexOf(elementId); if (idx <= 0) return;
    undoExec(new ReorderElement(elementId, siblings[idx - 1], "before"), ctx); onClose();
  }, [elements, elementId, rootOrder, undoExec, ctx, onClose]);
  const handleMoveDown = useCallback(() => {
    const el = elements.find((e) => e.id === elementId); if (!el) return;
    const siblings = el.parentId ? (elements.find((e) => e.id === el.parentId)?.children || []) : rootOrder;
    const idx = siblings.indexOf(elementId); if (idx < 0 || idx >= siblings.length - 1) return;
    undoExec(new ReorderElement(elementId, siblings[idx + 1], "after"), ctx); onClose();
  }, [elements, elementId, rootOrder, undoExec, ctx, onClose]);
  const handleDelete = useCallback(() => { undoExec(new RemoveElement(elementId), ctx); onClose(); }, [elementId, undoExec, ctx, onClose]);
  const handleEditComponent = useCallback(() => { if (contextElement?.componentRef) enterCanvas(contextElement.componentRef); onClose(); }, [contextElement, enterCanvas, onClose]);

  const [adjustedX, adjustedY] = useMemo(() => [Math.min(x, window.innerWidth - 210), Math.min(y, window.innerHeight - 300)], [x, y]);

  return (
    <div style={{ position: "fixed", left: adjustedX, top: adjustedY, zIndex: 9999, background: "#fff", border: "1px solid #E5E7EB", borderRadius: 8, boxShadow: "0 4px 16px rgba(0,0,0,0.15)", minWidth: 200, padding: "4px 0", fontSize: 12 }}
      onClick={(e) => e.stopPropagation()} onContextMenu={(e) => { e.preventDefault(); onClose(); }}>
      {/* Add Child */}
      <div style={ST.itemBase} onMouseEnter={() => { setShowAddChild(true); if (addChildTimer.current) clearTimeout(addChildTimer.current); }}
        onMouseLeave={() => { addChildTimer.current = setTimeout(() => { setShowAddChild(false); setActiveCategory(null); }, 150); }}>
        <span>添加子组件 ▸</span>
        {showAddChild && (
          <div style={ST.submenu} onMouseEnter={() => { if (addChildTimer.current) clearTimeout(addChildTimer.current); }}
            onMouseLeave={() => { setShowAddChild(false); setActiveCategory(null); }}>
            {Array.from(groupedComponents.entries()).map(([cat, items]) => (
              <div key={cat} style={{ ...ST.itemBase, paddingRight: 24 }}
                onMouseEnter={() => setActiveCategory(cat)} onMouseLeave={() => { if (activeCategory === cat) setActiveCategory(null); }}>
                <span>{CATEGORY_LABELS[cat] || cat}</span>
                <span style={{ color: "#9CA3AF", fontSize: 10 }}>▸</span>
                {activeCategory === cat && (
                  <div style={ST.subSubmenu} onMouseEnter={() => { if (addChildTimer.current) clearTimeout(addChildTimer.current); }}>
                    {items.map((comp) => (
                      <div key={comp.component} style={{ ...ST.itemBase, whiteSpace: "nowrap" }}
                        onClick={() => handleAddChild(elementId, comp.component)}
                        onMouseEnter={(e) => { e.currentTarget.style.background = "#F5F3FF"; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}>
                        <span><span style={{ marginRight: 4, fontSize: 11 }}>{COMPONENT_ICONS[comp.component] || (comp.isCustom ? "★" : "▪")}</span>{comp.component}</span>
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
      {contextElement?.componentRef && <><Separator /><MenuRow label={`编辑 ${contextElement.componentRef}`} onClick={handleEditComponent} /></>}
    </div>
  );
}

function MenuRow({ label, shortcut, onClick }: { label: string; shortcut?: string; onClick: () => void }) {
  return (
    <div onClick={onClick} style={{ padding: "6px 16px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 12, color: "#374151" }}
      onMouseEnter={(e) => { e.currentTarget.style.background = "#F5F3FF"; }}
      onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}>
      <span>{label}</span>
      {shortcut && <span style={{ color: "#9CA3AF", fontSize: 10, marginLeft: 24, whiteSpace: "nowrap" }}>{shortcut}</span>}
    </div>
  );
}

function Separator() {
  return <div style={{ height: 1, background: "#F3F4F6", margin: "4px 8px" }} />;
}

const ST = {
  itemBase: { padding: "6px 16px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 12, color: "#374151", position: "relative" as const },
  submenu: { position: "absolute" as const, left: "100%", top: 0, background: "#fff", border: "1px solid #E5E7EB", borderRadius: 6, boxShadow: "0 4px 12px rgba(0,0,0,0.12)", minWidth: 160, padding: "3px 0", zIndex: 10000 },
  subSubmenu: { position: "absolute" as const, left: "100%", top: 0, background: "#fff", border: "1px solid #E5E7EB", borderRadius: 6, boxShadow: "0 4px 12px rgba(0,0,0,0.12)", minWidth: 140, padding: "3px 0", zIndex: 10001, maxHeight: 280, overflowY: "auto" as const },
};
