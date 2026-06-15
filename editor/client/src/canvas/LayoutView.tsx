/** DOM 布局画布渲染器。OmniView 属性 → CSS 样式映射。Drag-to-reorder 由 @dnd-kit 驱动。*/

import React, { useCallback, useState, useEffect } from "react";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { AddElement } from "../commands/AddElement";
import { RemoveElement } from "../commands/RemoveElement";
import { ReorderElement } from "../commands/ReorderElement";
import { MoveElement } from "../commands/MoveElement";
import type { CommandContext } from "../commands/types";
import { uid } from "../utils/id";
import { COMPONENT_PROTOCOL } from "../data/componentProtocol";
import { useComponentRegistry } from "../store/componentRegistry";
import { CustomComponentRenderer } from "./CustomComponentRenderer";

import {
  DndContext,
  DragOverlay,
  closestCenter,
  pointerWithin,
  PointerSensor,
  TouchSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
  useDroppable,
  type DragStartEvent,
  type DragEndEvent,
  type DragOverEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

const TAG_MAP: Record<string, keyof HTMLElementTagNameMap> = {
  Container: "div", Flex: "div", Grid: "div", Stack: "div", Spacer: "div",
  Text: "span", Heading: "h2", Image: "img", Link: "a", Divider: "hr",
  Button: "button", Input: "input", TextArea: "textarea",
  Checkbox: "input", Select: "select", List: "ul", ListItem: "li",
  Spinner: "div", Progress: "progress", Alert: "div", Card: "div", Form: "form",
};

const VOID_TAGS = new Set(["input", "img", "hr", "br", "progress", "textarea", "select"]);

/** Check if ancestorId is an ancestor of descendantId in the element tree. */
function isDescendantOf(ancestorId: string, descendantId: string, elements: ElementData[]): boolean {
  let currentId: string | null | undefined = descendantId;
  while (currentId) {
    const el = elements.find((e) => e.id === currentId);
    if (!el) return false;
    if (el.parentId === ancestorId) return true;
    currentId = el.parentId;
  }
  return false;
}

// ═══════════════════════════════════════════════════
// OmniView prop → CSS mapping
// ═══════════════════════════════════════════════════

function omniPropsToStyle(el: ElementData): React.CSSProperties {
  const p = el.props || {};
  const style: React.CSSProperties = {};

  if (p.direction === "row") { style.display = "flex"; style.flexDirection = "row"; }
  else if (p.direction === "column") { style.display = "flex"; style.flexDirection = "column"; }
  else if (el.tag === "Flex" || el.tag === "Container" || el.tag === "Grid") {
    style.display = "flex";
    style.flexDirection = el.tag === "Flex" ? "row" : "column";
  }

  if (p.gap) style.gap = String(p.gap);
  if (p.padding) style.padding = String(p.padding);
  if (p.margin) style.margin = String(p.margin);
  if (p.align) style.alignItems = String(p.align);
  if (p.justify) style.justifyContent = String(p.justify);
  if (p.wrap === "wrap") style.flexWrap = "wrap";
  if (p.width) style.width = String(p.width);
  if (p.height) style.height = String(p.height);

  const bg = p.background || p.background_color;
  if (bg) style.background = String(bg);
  if (p.color) style.color = String(p.color);

  if (p.fontSize || p.font_size) style.fontSize = String(p.fontSize || p.font_size);
  if (p.fontWeight || p.font_weight) style.fontWeight = String(p.fontWeight || p.font_weight);
  if (p.borderRadius || p.border_radius) style.borderRadius = String(p.borderRadius || p.border_radius);
  if (p.border) style.border = String(p.border);

  const boxShadowVal = p.boxShadow || p.shadow;
  if (boxShadowVal === "md") style.boxShadow = "0 4px 6px rgba(0,0,0,0.1)";
  else if (boxShadowVal) style.boxShadow = String(boxShadowVal);

  if (el.tag === "Container" || el.tag === "Card") {
    if (!p.padding) style.padding = "16px";
    if (el.tag === "Card") {
      style.background = style.background || "#fff";
      style.borderRadius = style.borderRadius || "8px";
      style.boxShadow = style.boxShadow || "0 2px 8px rgba(0,0,0,0.08)";
    }
  }

  if (el.tag === "Text") style.fontSize = style.fontSize || "14px";

  if (el.tag === "Button") {
    style.padding = style.padding || "8px 16px";
    style.borderRadius = style.borderRadius || "6px";
    style.border = "1px solid #d1d5db";
    style.background = style.background || "#fff";
    style.cursor = "pointer";
    style.fontSize = style.fontSize || "13px";
    if (p.variant === "primary") { style.background = "#4F46E5"; style.color = "#fff"; style.borderColor = "#4F46E5"; }
    else if (p.variant === "outline") { style.background = "transparent"; style.color = "#1F2937"; style.border = "1px solid #E5E7EB"; }
    else if (p.variant === "danger") { style.background = "#FEE2E2"; style.color = "#DC2626"; style.borderColor = "#FECACA"; }
    else if (p.variant === "secondary") { style.background = "#F3F4F6"; style.color = "#374151"; style.borderColor = "#D1D5DB"; }
    else if (p.variant === "ghost") { style.background = "transparent"; style.borderColor = "transparent"; }
  }

  if (el.tag === "Input") {
    style.padding = style.padding || "6px 10px";
    style.border = "1px solid #d1d5db";
    style.borderRadius = style.borderRadius || "4px";
    style.fontSize = style.fontSize || "13px";
    style.minWidth = "120px";
  }

  if (el.tag === "List") {
    style.listStyle = "none";
    style.padding = style.padding || "0";
    style.display = "flex";
    style.flexDirection = "column";
    style.gap = style.gap || "8px";
  }
  if (el.tag === "ListItem") style.padding = style.padding || "4px 0";

  if (el.tag === "Heading") {
    const lvl = Number(p.level) || 1;
    const sizes = { 1: "24px", 2: "20px", 3: "16px", 4: "14px" };
    style.fontSize = style.fontSize || ((sizes as Record<number, string>)[lvl] || "16px");
    style.fontWeight = "600";
    style.margin = style.margin || "0 0 8px 0";
  }

  return style;
}

// ═══════════════════════════════════════════════════
// Drag Handle
// ═══════════════════════════════════════════════════

type DragListeners = ReturnType<typeof useSortable>["listeners"];

function DragHandle({ listeners, visible }: {
  listeners: DragListeners;
  visible: boolean;
}) {
  if (!visible) return null;
  return React.createElement("span", {
    ...listeners,
    "data-drag-type": "sort",
    style: {
      position: "absolute", left: -22, top: "25%", transform: "translateY(-50%)",
      cursor: "grab", fontSize: 13, color: "#9CA3AF", padding: "1px 3px",
      borderRadius: 3, background: "#F3F4F6", border: "1px solid #E5E7EB",
      lineHeight: 1, userSelect: "none", zIndex: 10,
      pointerEvents: "auto" as const,
    },
    title: "同级排序",
  }, "⋮⋮");
}

// ═══════════════════════════════════════════════════
// Move Handle — uses sortable listeners (NOT useDraggable) to avoid
// dnd-kit PointerSensor conflicts between two draggable sources.
// Operation type ("sort"|"move") is tracked via dragOpRef.
// ═══════════════════════════════════════════════════

function MoveHandleWrapper({ listeners, visible }: {
  listeners: DragListeners;
  visible: boolean;
}) {
  if (!visible) return null;
  return React.createElement("span", {
    ...listeners,
    "data-drag-type": "move",
    style: {
      position: "absolute", left: -22, top: "75%", transform: "translateY(-50%)",
      cursor: "grab", fontSize: 13, color: "#9CA3AF",
      padding: "1px 3px", borderRadius: 3, background: "#F3F4F6",
      border: "1px solid #E5E7EB", lineHeight: 1, userSelect: "none", zIndex: 10,
      pointerEvents: "auto" as const,
    },
    title: "跨容器移动",
  }, "⇄");
}

// ═══════════════════════════════════════════════════
// Drag operation tracker — module-level to be shared across components.
// Stores which handle ("sort"|"move") initiated the current drag, per element id.
// ═══════════════════════════════════════════════════
// Sortable Wrapper
// ═══════════════════════════════════════════════════

interface SortableItemProps {
  el: ElementData;
  allElements: ElementData[];
  selectedIds: string[];
  onSelect: (id: string | null) => void;
  onDropOnElement: (e: React.DragEvent, parentId: string) => void;
  onDragOverElement: (e: React.DragEvent) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
}

function SortableItem({ el, ...rest }: SortableItemProps) {
  const {
    attributes, listeners, setNodeRef, transform, transition, isDragging,
  } = useSortable({ id: el.id });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.3 : undefined,
    position: "relative",
  };

  const isSelected = rest.selectedIds.includes(el.id);

  return React.createElement("div", {
    ref: setNodeRef,
    style,
    ...attributes,
    onClick: (e: React.MouseEvent) => {
      // Select the item on click (don't propagate to parent)
      e.stopPropagation();
      rest.onSelect(el.id);
    },
    onContextMenu: (e: React.MouseEvent) => rest.onContextMenu(e, el.id),
  },
    React.createElement(DragHandle, { listeners, visible: isSelected }),
    React.createElement(MoveHandleWrapper, { listeners, visible: isSelected }),
    React.createElement(RenderElement, {
      el, ...rest, isDragging, isSelected,
    }),
  );
}

// ═══════════════════════════════════════════════════
// Drag operation tracker — module-level.
// Set by handleDragStart by reading data-drag-type from the activator event target.
// ═══════════════════════════════════════════════════

const dragOpMap = new Map<string, "sort" | "move">();

// ═══════════════════════════════════════════════════
// Root Sortable Wrapper — lighter version without role="button" on wrapper
// ═══════════════════════════════════════════════════

function RootSortableItem({ el, ...rest }: SortableItemProps) {
  const {
    listeners, setNodeRef, transform, transition, isDragging,
  } = useSortable({ id: el.id });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.3 : undefined,
    position: "relative",
  };

  const isSelected = rest.selectedIds.includes(el.id);

  return React.createElement("div", {
    ref: setNodeRef,
    style,
    onClick: (e: React.MouseEvent) => {
      e.stopPropagation();
      rest.onSelect(el.id);
    },
    onContextMenu: (e: React.MouseEvent) => rest.onContextMenu(e, el.id),
  },
    React.createElement(DragHandle, { listeners, visible: isSelected }),
    React.createElement(MoveHandleWrapper, { listeners, visible: isSelected }),
    React.createElement(RenderElement, {
      el, ...rest, isDragging, isSelected,
    }),
  );
}

// ═══════════════════════════════════════════════════
// Main Component
// ═══════════════════════════════════════════════════

interface LayoutViewProps {
  ctx: CommandContext;
  hideCodeBlocks?: boolean;
}

export function LayoutView({ ctx, hideCodeBlocks = false }: LayoutViewProps) {
  const elements = useEditorStore((s) => s.elements);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const selectElement = useEditorStore((s) => s.selectElement);
  const undoExec = useUndoStore((s) => s.execute);
  const rootOrder = useEditorStore((s) => s.rootOrder);
  const viewRoots = elements
    .filter((e) => !e.parentId && e.nodeType !== "codeblock")
    .sort((a, b) => rootOrder.indexOf(a.id) - rootOrder.indexOf(b.id));
  const codeRoots = hideCodeBlocks ? [] : elements.filter((e) => !e.parentId && e.nodeType === "codeblock");
  const hasContent = viewRoots.length > 0 || codeRoots.length > 0;

  const [activeId, setActiveId] = useState<string | null>(null);

  const activeElement = activeId ? elements.find((e) => e.id === activeId) ?? null : null;

  // ── DnD Sensors ──────────────────────────────────
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 200, tolerance: 5 } }),
    useSensor(KeyboardSensor),
  );

  // ── DnD Handlers ─────────────────────────────────

  // Custom collision detection:
  // - Move ops (⇄) → pointerWithin to detect containers under cursor
  // - Sort ops (⋮⋮) → closestCenter excluding container-* droppables
  const collisionDetection = useCallback((args: any) => {
    const activeIdStr = String(args.active.id);
    const op = dragOpMap.get(activeIdStr);
    if (op === "move") {
      return pointerWithin(args);
    }
    // For sort: exclude container-* droppables so sort targets are clean
    const filtered = args.droppableContainers.filter(
      (c: any) => !String(c.id).startsWith("container-")
    );
    return closestCenter({ ...args, droppableContainers: filtered });
  }, []);

  const handleDragStart = useCallback((event: DragStartEvent) => {
    const activeIdStr = event.active.id as string;
    setActiveId(activeIdStr);

    // Detect which handle was used by reading data-drag-type from the activator event target
    const activatorEvent = event.activatorEvent as PointerEvent | undefined;
    const target = activatorEvent?.target as HTMLElement | undefined;
    const dragType = target?.closest?.("[data-drag-type]")?.getAttribute("data-drag-type");
    if (dragType === "move" || dragType === "sort") {
      dragOpMap.set(activeIdStr, dragType);
    } else {
      dragOpMap.set(activeIdStr, "sort"); // default
    }
  }, []);

  const handleDragEnd = useCallback((event: DragEndEvent) => {
    setActiveId(null);
    const { active, over } = event;
    if (!over) {
      dragOpMap.delete(active?.id as string);
      return;
    }

    const activeIdStr = active.id as string;
    const overIdRaw = over.id as string;
    if (activeIdStr === overIdRaw) {
      dragOpMap.delete(activeIdStr);
      return;
    }

    const store = useEditorStore.getState();
    const op = dragOpMap.get(activeIdStr) || "sort";

    // Sanitize over id — strip container- prefix if present
    const overIdStr = overIdRaw.startsWith("container-") ? overIdRaw.slice(10) : overIdRaw;

    // ── Move operation (⇄ handle) ──
    if (op === "move") {
      const activeEl = store.elements.find((e) => e.id === activeIdStr);
      if (!activeEl) { dragOpMap.delete(activeIdStr); return; }

      const overEl = store.elements.find((e) => e.id === overIdStr);
      if (!overEl || overEl.id === activeIdStr) { dragOpMap.delete(activeIdStr); return; }

      const currentParentId = activeEl.parentId;

      // Rule 1: If over target is a LAYOUT container (not small widgets like Button), move INTO it
      const layoutContainers = new Set(["Container", "Flex", "Grid", "Card", "Form", "List", "Alert", "Stack"]);
      if (layoutContainers.has(overEl.tag) && COMPONENT_PROTOCOL[overEl.tag]?.children === true) {
        // Prevent dropping into own descendant (would create cycle)
        if (!isDescendantOf(overEl.id, activeIdStr, store.elements)) {
          undoExec(new MoveElement(activeIdStr, overEl.id, null, "after"), ctx);
          dragOpMap.delete(activeIdStr);
          return;
        }
      }

      // Rule 2: Escape — move to over's level, positioned after over
      if (currentParentId === null) { dragOpMap.delete(activeIdStr); return; }
      const targetParentId = overEl.parentId;
      undoExec(new MoveElement(activeIdStr, targetParentId, overIdStr, "after"), ctx);
      dragOpMap.delete(activeIdStr);
      return;
    }

    // ── Sort operation (⋮⋮ handle) — same-parent reorder only ──
    const activeEl = store.elements.find((e) => e.id === activeIdStr);
    const overEl = store.elements.find((e) => e.id === overIdStr);
    if (!activeEl || !overEl) { dragOpMap.delete(activeIdStr); return; }

    // Only reorder within same parent
    if (activeEl.parentId !== overEl.parentId) { dragOpMap.delete(activeIdStr); return; }

    let position: "before" | "after" = "after";
    if (activeEl.parentId) {
      const parent = store.elements.find((e) => e.id === activeEl.parentId);
      if (parent) {
        const activeIdx = parent.children.indexOf(activeIdStr);
        const overIdx = parent.children.indexOf(overIdStr);
        if (activeIdx !== -1 && overIdx !== -1) {
          position = activeIdx < overIdx ? "after" : "before";
        }
      }
    } else {
      // Root-level: use rootOrder
      const rootOrder = store.rootOrder;
      const activeIdx = rootOrder.indexOf(activeIdStr);
      const overIdx = rootOrder.indexOf(overIdStr);
      if (activeIdx !== -1 && overIdx !== -1) {
        position = activeIdx < overIdx ? "after" : "before";
      }
    }
    undoExec(new ReorderElement(activeIdStr, overIdStr, position), ctx);
    dragOpMap.delete(activeIdStr);
  }, [undoExec, ctx]);

  // ── Keyboard: Delete / Backspace ──
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Delete" || e.key === "Backspace") {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
        const ids = useEditorStore.getState().selectedElementIds;
        if (ids.length > 0) {
          e.preventDefault();
          for (const id of ids) {
            undoExec(new RemoveElement(id), ctx);
          }
          useEditorStore.getState().selectElement(null);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undoExec, ctx]);

  // ── Context Menu ──
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; elementId: string } | null>(null);
  const handleContextMenu = useCallback((e: React.MouseEvent, elementId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setCtxMenu({ x: e.clientX, y: e.clientY, elementId });
  }, []);
  const closeCtxMenu = useCallback(() => setCtxMenu(null), []);

  // ── Palette → canvas (HTML5 DnD, preserved) ──
  const handleDropOnCanvas = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const tag = e.dataTransfer.getData("application/omni-component");
    if (!tag) return;
    const props: Record<string, unknown> = {};
    if (tag === "Text") props.label = "Text";
    else if (tag === "Heading") props.label = "标题";
    else if (tag === "Button") props.label = "按钮";
    else if (tag === "Input") props.placeholder = "输入框";
    // Check if this is a custom component — must set componentRef for CustomComponentRenderer
    const isCustom = useComponentRegistry.getState().definitions[tag] !== undefined;
    const el: ElementData = {
      id: uid(), nodeType: "view", tag,
      props, children: [], parentId: null,
      ...(isCustom ? { componentRef: tag } : {}),
    };
    undoExec(new AddElement(el), ctx);
  }, [undoExec, ctx]);

  const handleDropOnElement = useCallback((e: React.DragEvent, parentId: string) => {
    e.preventDefault();
    e.stopPropagation();
    const tag = e.dataTransfer.getData("application/omni-component");
    if (!tag) return;
    const props: Record<string, unknown> = {};
    if (tag === "Text") props.label = "Text";
    else if (tag === "Heading") props.label = "标题";
    else if (tag === "Button") props.label = "按钮";
    else if (tag === "Input") props.placeholder = "输入框";
    // Check if this is a custom component — must set componentRef for CustomComponentRenderer
    const isCustom = useComponentRegistry.getState().definitions[tag] !== undefined;
    const el: ElementData = {
      id: uid(), nodeType: "view", tag,
      props, children: [], parentId,
      ...(isCustom ? { componentRef: tag } : {}),
    };
    undoExec(new AddElement(el), ctx);
  }, [undoExec, ctx]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  }, []);

  // ── Render ───────────────────────────────────────
  return (
    <DndContext
      sensors={sensors}
      collisionDetection={collisionDetection}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <div onDrop={handleDropOnCanvas} onDragOver={handleDragOver} style={{ padding: 16, minHeight: "100%" }}>
        {!hasContent ? (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#9ca3af", fontSize: 13 }}>
            <div style={{ textAlign: "center" }}>
              <div style={{ fontSize: 32, marginBottom: 8 }}>🎨</div>
              <div>编译 .omni 或在左侧组件面板拖入组件</div>
              <div style={{ fontSize: 11, marginTop: 4, color: "#d1d5db" }}>Ctrl+Enter 编译</div>
            </div>
          </div>
        ) : (
          <>
            <SortableContext items={viewRoots.map(e => e.id)} strategy={verticalListSortingStrategy}>
              {viewRoots.map((el) => React.createElement(RootSortableItem, {
                key: el.id, el, allElements: elements, selectedIds, onSelect: selectElement,
                onDropOnElement: handleDropOnElement, onDragOverElement: handleDragOver,
                onContextMenu: handleContextMenu,
              }))}
            </SortableContext>

            {codeRoots.length > 0 && (
              <div style={{ marginTop: 24, borderTop: "1px dashed #e5e7eb", paddingTop: 16 }}>
                <div style={{ fontSize: 11, fontWeight: 600, color: "#6B7280", marginBottom: 12, textTransform: "uppercase" }}>
                  🧩 Code Blocks
                </div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 12 }}>
                  {codeRoots.map((el) => React.createElement(CodeBlockCard, { key: el.id, el, allElements: elements, selectedIds, onSelect: selectElement }))}
                </div>
              </div>
            )}
          </>
        )}

        {/* Context Menu */}
        {ctxMenu && (
          <>
            <div onClick={closeCtxMenu} style={{ position: "fixed", inset: 0, zIndex: 998, background: "transparent" }} />
            <div style={{
              position: "fixed", left: ctxMenu.x, top: ctxMenu.y, zIndex: 999,
              background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8,
              boxShadow: "0 4px 12px rgba(0,0,0,0.15)", padding: 4, minWidth: 140,
            }}>
              <div
                onClick={() => {
                  undoExec(new RemoveElement(ctxMenu.elementId), ctx);
                  useEditorStore.getState().selectElement(null);
                  closeCtxMenu();
                }}
                style={{ padding: "6px 12px", cursor: "pointer", borderRadius: 4, fontSize: 13, color: "#DC2626" }}
                onMouseEnter={(e) => { e.currentTarget.style.background = "#FEF2F2"; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
              >
                🗑 删除
              </div>
            </div>
          </>
        )}
      </div>

      {/* Drag Overlay — semi-transparent clone */}
      <DragOverlay dropAnimation={null}>
        {activeElement ? React.createElement(DragPreview, { el: activeElement }) : null}
      </DragOverlay>
    </DndContext>
  );
}

// ═══════════════════════════════════════════════════
// Drag Preview (semi-transparent clone in DragOverlay)
// ═══════════════════════════════════════════════════

function DragPreview({ el }: { el: ElementData }) {
  const style = omniPropsToStyle(el);
  const previewStyle: React.CSSProperties = {
    ...style,
    opacity: 0.85,
    boxShadow: "0 4px 20px rgba(0,0,0,0.15)",
    cursor: "grabbing",
    pointerEvents: "none",
  };

  const htmlTag = TAG_MAP[el.tag] || "div";
  const isVoid = VOID_TAGS.has(htmlTag);
  const p = el.props || {};
  let label = "";
  if (p.label && typeof p.label === "string" && el.children.length === 0) {
    label = p.label;
  } else if (el.tag === "Text" && !p.label) {
    label = "Text";
  }

  const childContent = label || React.createElement("span", { style: { color: "#d1d5db", fontSize: 10, fontStyle: "italic" } }, `<${el.tag}>`);

  // Void elements can't have children — wrap in a div
  if (isVoid) {
    return React.createElement("div", { style: { ...previewStyle, display: "inline-block" } },
      React.createElement(htmlTag, { style: { pointerEvents: "none" } }),
      React.createElement("span", { style: { marginLeft: 4, fontSize: 11, color: "#6B7280" } }, childContent),
    );
  }

  return React.createElement(htmlTag, { style: previewStyle }, childContent);
}

// ═══════════════════════════════════════════════════
// Recursive Render Component
// ═══════════════════════════════════════════════════

interface ElementProps {
  el: ElementData;
  allElements: ElementData[];
  selectedIds: string[];
  onSelect: (id: string | null) => void;
  onDropOnElement: (e: React.DragEvent, parentId: string) => void;
  onDragOverElement: (e: React.DragEvent) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
  isDragging?: boolean;
  isSelected?: boolean;
}

function RenderElement({ el, allElements, selectedIds, onSelect, onDropOnElement, onDragOverElement, onContextMenu, isDragging, isSelected }: ElementProps): React.ReactElement {
  // ── Custom component instance branch ──────────
  if (el.componentRef) {
    const def = useComponentRegistry.getState().definitions[el.componentRef];
    if (def) {
      return React.createElement(CustomComponentRenderer, {
        instanceEl: el,
        definition: def,
        allElements,
        selectedIds,
        onSelect,
        onDropOnElement,
        onDragOverElement,
        onContextMenu,
        isDragging,
        isSelected,
        RenderElement: RenderElement as any,
      });
    }
    // Broken component reference — render placeholder
    return React.createElement("div", {
      key: el.id,
      "data-omni-id": el.id,
      style: {
        padding: 16,
        border: "2px dashed #DC2626",
        borderRadius: 8,
        color: "#DC2626",
        fontSize: 12,
        textAlign: "center",
        background: "#FEF2F2",
      },
      onClick: (e: React.MouseEvent) => {
        e.stopPropagation();
        onSelect(el.id);
      },
    }, `⚠ 组件 "${el.tag}" 未找到 — 请检查组件定义是否存在`);
  }

  const htmlTag = TAG_MAP[el.tag] || "div";
  const selected = isSelected ?? selectedIds.includes(el.id);
  const isVoid = VOID_TAGS.has(htmlTag);
  const [hovered, setHovered] = React.useState(false);
  const [dragOver, setDragOver] = React.useState(false);
  const p = el.props || {};

  const canAcceptDrop = !isVoid && (COMPONENT_PROTOCOL[el.tag]?.children === true);
  const acceptsChildren = COMPONENT_PROTOCOL[el.tag]?.children === true;

  // Register as droppable for move-handle targeting (pointerWithin detection)
  const { setNodeRef: droppableRef, isOver: isDndOver } = useDroppable({
    id: `container-${el.id}`,
    disabled: !acceptsChildren,
  });

  const layoutStyle = omniPropsToStyle(el);

  const style: React.CSSProperties = {
    ...layoutStyle,
    position: "relative",
    boxSizing: "border-box",
    ...(selected ? { outline: "2px solid #4F46E5", outlineOffset: "2px", zIndex: 1 } : {}),
    ...(hovered && !isDragging ? { outline: "1px dashed #93C5FD", outlineOffset: "1px" } : {}),
    ...(dragOver && canAcceptDrop ? { outline: "2px dashed #4F46E5", outlineOffset: "2px", background: "rgba(79, 70, 229, 0.06)" } : {}),
    ...(isDndOver ? { outline: "2px solid #4F46E5", outlineOffset: "2px", background: "rgba(79, 70, 229, 0.08)" } : {}),
  };

  let textLabel = "";
  if (p.label && typeof p.label === "string" && el.children.length === 0) {
    textLabel = p.label;
  }

  // ── Build children ──
  const childElements: React.ReactNode[] = [];
  if (textLabel) childElements.push(textLabel);

  if (el.children.length > 0) {
    // Build sortable child items
    const sortableChildren = el.children
      .map((childId) => {
        const child = allElements.find((e) => e.id === childId);
        if (!child) return null;
        return React.createElement(SortableItem, {
          key: child.id, el: child, allElements, selectedIds, onSelect,
          onDropOnElement, onDragOverElement, onContextMenu,
        });
      })
      .filter(Boolean);

    // Wrap in SortableContext
    childElements.push(
      React.createElement(SortableContext, {
        key: `${el.id}__sortable`,
        items: el.children,
        strategy: verticalListSortingStrategy,
      }, ...sortableChildren),
    );
  } else if (acceptsChildren) {
    // Empty container — show placeholder
    childElements.push(
      React.createElement("div", {
        key: "__empty-container",
        style: { minHeight: 20, display: "flex", alignItems: "center", justifyContent: "center" },
      }, React.createElement("span", {
        style: { color: "#D1D5DB", fontSize: 10, fontStyle: "italic" },
      }, "拖入组件")),
    );
  } else if (!isVoid && el.children.length === 0 && el.tag !== "Text") {
    childElements.push(React.createElement("span", { key: "__empty", style: { color: "#d1d5db", fontSize: 10, fontStyle: "italic" } }, `<${el.tag}>`));
  }
  if (el.tag === "Text" && childElements.length === 0 && !isVoid) {
    childElements.push(React.createElement("span", { key: "__empty", style: { color: "#D1D5DB", fontSize: 13 } }, "Text"));
  }

  const commonProps: Record<string, unknown> = {
    key: el.id,
    "data-omni-id": el.id,
    "data-omni-tag": el.tag,
    ...(acceptsChildren ? { ref: droppableRef } : {}),
    style,
    onClick: (e: React.MouseEvent) => {
      if (isDragging) return;
      e.stopPropagation();
      onSelect(el.id);
    },
    onContextMenu: (e: React.MouseEvent) => onContextMenu(e, el.id),
    onMouseEnter: () => setHovered(true),
    onMouseLeave: () => { setHovered(false); setDragOver(false); },
    ...(canAcceptDrop ? {
      onDragOver: (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        e.dataTransfer.dropEffect = "copy";
        setDragOver(true);
      },
      onDragEnter: (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(true);
      },
      onDragLeave: (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(false);
      },
      onDrop: (e: React.DragEvent) => {
        setDragOver(false);
        onDropOnElement(e, el.id);
      },
    } : {}),
  };

  if (p.value && el.tag === "Input") commonProps.placeholder = String(p.placeholder || "");

  if (isVoid) {
    if (el.tag === "Input" && p.value !== undefined) commonProps.defaultValue = String(p.value);
    if (el.tag === "Input") commonProps.type = String(p.type || "text");
    if (el.tag === "Checkbox") {
      commonProps.type = "checkbox";
      if (p.checked !== undefined) commonProps.defaultChecked = Boolean(p.checked);
    }
    if (el.tag === "Select") {
      const rawOptions = p.options;
      const options: string[] = Array.isArray(rawOptions) ? rawOptions
        : (typeof rawOptions === "string" && rawOptions ? rawOptions.split(",").map((s: string) => s.trim()) : []);
      return React.createElement(htmlTag, commonProps as React.HTMLAttributes<HTMLElement>,
        ...options.map((opt, i) => React.createElement("option", { key: i, value: opt }, opt))
      );
    }
    if (el.tag === "TextArea") {
      commonProps.defaultValue = String(p.placeholder || "");
      return React.createElement(htmlTag, commonProps as React.HTMLAttributes<HTMLElement>);
    }
    return React.createElement(htmlTag, commonProps as React.HTMLAttributes<HTMLElement>);
  }

  return React.createElement(htmlTag, commonProps as React.HTMLAttributes<HTMLElement>, ...childElements);
}

// ═══════════════════════════════════════════════════
// CodeBlock Card Renderer
// ═══════════════════════════════════════════════════

const BLOCK_COLORS: Record<string, { bg: string; border: string; dot: string }> = {
  store:    { bg: "#F5F3FF", border: "#7C3AED", dot: "#7C3AED" },
  signal:   { bg: "#FEF3C7", border: "#D97706", dot: "#D97706" },
  computed: { bg: "#DBEAFE", border: "#2563EB", dot: "#2563EB" },
  action:   { bg: "#FCE7F3", border: "#DB2777", dot: "#DB2777" },
};

const BLOCK_ICONS: Record<string, string> = {
  store: "🗄️", signal: "⚡", computed: "🧮", action: "🔄",
};

function CodeBlockCard({ el, allElements, selectedIds, onSelect }: ElementProps): React.ReactElement {
  const blockType = (el.tag || "signal") as string;
  const colors = BLOCK_COLORS[blockType] || BLOCK_COLORS.signal;
  const icon = BLOCK_ICONS[blockType] || "📦";
  const isSelected = selectedIds.includes(el.id);
  const [hovered, setHovered] = React.useState(false);
  const cp = el.props || {};

  const name = (cp.name as string) || el.id;
  const typeAnn = (cp.typeAnnotation as string) || "";
  const parentStore = (cp.parentStore as string) || "";

  const childBlocks = allElements.filter((e) => e.parentId === el.id && e.nodeType === "codeblock");

  return React.createElement("div", {
    key: el.id,
    "data-omni-id": el.id,
    "data-omni-tag": el.tag,
    style: {
      background: colors.bg,
      border: `1.5px solid ${isSelected ? colors.border : `${colors.border}60`}`,
      borderRadius: 10, padding: "10px 14px", minWidth: 160, maxWidth: 260,
      cursor: "pointer", position: "relative",
      boxShadow: hovered || isSelected ? "0 2px 8px rgba(0,0,0,0.08)" : "none",
      transition: "box-shadow 0.15s, border-color 0.15s",
    },
    onClick: (e: React.MouseEvent) => { e.stopPropagation(); onSelect(el.id); },
    onMouseEnter: () => setHovered(true),
    onMouseLeave: () => setHovered(false),
  },
    React.createElement("div", {
      style: { position: "absolute", right: -4, top: "50%", transform: "translateY(-50%)", width: 8, height: 8, borderRadius: "50%", background: colors.dot, zIndex: 2 },
    }),
    blockType !== "store" && React.createElement("div", {
      style: { position: "absolute", left: -4, top: "50%", transform: "translateY(-50%)", width: 8, height: 8, borderRadius: "50%", background: "#10B981", zIndex: 2 },
    }),
    React.createElement("div", { style: { display: "flex", alignItems: "center", gap: 6, marginBottom: 4 } },
      React.createElement("span", { style: { fontSize: 14 } }, icon),
      React.createElement("span", { style: { fontSize: 11, fontWeight: 700, color: "#1F2937" } }, name),
    ),
    typeAnn && React.createElement("div", { style: { fontSize: 10, color: "#6B7280", fontFamily: "monospace", marginBottom: 2 } }, `: ${typeAnn}`),
    parentStore && React.createElement("div", { style: { fontSize: 9, color: "#9CA3AF" } }, `@ ${parentStore}`),
    childBlocks.length > 0 && React.createElement("div", { style: { marginTop: 8, display: "flex", flexWrap: "wrap", gap: 6 } },
      ...childBlocks.map(cb => React.createElement(CodeBlockCard, { key: cb.id, el: cb, allElements, selectedIds, onSelect })),
    ),
  );
}
