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
  PointerSensor,
  TouchSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
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

function DragHandle({ listeners, visible }: { listeners: DragListeners; visible: boolean }) {
  if (!visible) return null;
  return React.createElement("span", {
    ...listeners,
    style: {
      position: "absolute", left: -22, top: "50%", transform: "translateY(-50%)",
      cursor: "grab", fontSize: 14, color: "#9CA3AF", padding: "2px 4px",
      borderRadius: 3, background: "#F3F4F6", border: "1px solid #E5E7EB",
      lineHeight: 1, userSelect: "none", zIndex: 10,
      pointerEvents: "auto" as const,
    },
    title: "拖拽排序",
  }, "⋮⋮");
}

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
  const viewRoots = elements.filter((e) => !e.parentId && e.nodeType !== "codeblock");
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
  const handleDragStart = useCallback((event: DragStartEvent) => {
    setActiveId(event.active.id as string);
  }, []);

  const handleDragEnd = useCallback((event: DragEndEvent) => {
    setActiveId(null);
    const { active, over } = event;
    if (!over) return;

    const activeIdStr = active.id as string;
    const overIdStr = over.id as string;
    if (activeIdStr === overIdStr) return;

    const store = useEditorStore.getState();
    const activeEl = store.elements.find((e) => e.id === activeIdStr);
    const overEl = store.elements.find((e) => e.id === overIdStr);
    if (!activeEl || !overEl) return;

    const overAcceptsChildren = COMPONENT_PROTOCOL[overEl.tag]?.children === true;

    if (overAcceptsChildren && activeEl.parentId !== overIdStr) {
      // Dropped into an empty or different container — become a child of it
      undoExec(new MoveElement(activeIdStr, overIdStr, null, "after"), ctx);
      return;
    }

    // Determine before/after by comparing indices in the over element's parent.
    // If the active item was originally above the over item, it should go after it;
    // if below, it should go before it (matching dnd-kit's SortableContext preview).
    let position: "before" | "after" = "after";
    if (overEl.parentId) {
      const parent = store.elements.find((e) => e.id === overEl.parentId);
      if (parent) {
        const activeIdx = parent.children.indexOf(activeIdStr);
        const overIdx = parent.children.indexOf(overIdStr);
        // For cross-container: active item isn't in this parent's children yet.
        // Use rect comparison: if active center is above over center → "after".
        if (activeIdx === -1) {
          const activeRect = active.rect.current.translated;
          const overRect = over.rect;
          if (activeRect && overRect) {
            position = activeRect.top + activeRect.height / 2 < overRect.top + overRect.height / 2
              ? "after" : "before";
          }
        } else if (overIdx !== -1) {
          position = activeIdx < overIdx ? "after" : "before";
        }
      }
    }

    if (activeEl.parentId === overEl.parentId) {
      // Same-parent reorder
      undoExec(new ReorderElement(activeIdStr, overIdStr, position), ctx);
    } else if (overEl.parentId && activeEl.parentId !== overEl.parentId) {
      // Cross-container move next to a sibling
      const targetParent = store.elements.find((e) => e.id === overEl.parentId);
      if (targetParent && COMPONENT_PROTOCOL[targetParent.tag]?.children) {
        undoExec(new MoveElement(activeIdStr, overEl.parentId, overIdStr, position), ctx);
      }
    }
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
      collisionDetection={closestCenter}
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
            {viewRoots.map((el) => React.createElement(RenderElement, {
              key: el.id, el, allElements: elements, selectedIds, onSelect: selectElement,
              onDropOnElement: handleDropOnElement, onDragOverElement: handleDragOver,
              onContextMenu: handleContextMenu,
            }))}

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

  const layoutStyle = omniPropsToStyle(el);

  const style: React.CSSProperties = {
    ...layoutStyle,
    position: "relative",
    boxSizing: "border-box",
    ...(selected ? { outline: "2px solid #4F46E5", outlineOffset: "2px", zIndex: 1 } : {}),
    ...(hovered && !isDragging ? { outline: "1px dashed #93C5FD", outlineOffset: "1px" } : {}),
    ...(dragOver && canAcceptDrop ? { outline: "2px dashed #4F46E5", outlineOffset: "2px", background: "rgba(79, 70, 229, 0.06)" } : {}),
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
