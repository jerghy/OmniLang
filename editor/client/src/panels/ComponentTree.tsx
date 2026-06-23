/** 组件树面板 — 画布元素的层级树视图。
 *  点击 → 选中画布元素。拖拽 → 跨容器移动 / 同级重排。 */

import React, { useState, useCallback, useEffect } from "react";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { MoveElement } from "../commands/MoveElement";
import { ReorderElement } from "../commands/ReorderElement";
import { RemoveElement } from "../commands/RemoveElement";
import { copySelection, pasteFromClipboard } from "../commands/CopyPaste";
import type { CommandContext } from "../commands/types";
import { COMPONENT_PROTOCOL } from "../data/componentProtocol";

// ═══════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════

interface TreeNode {
  el: ElementData;
  children: TreeNode[];
  depth: number;
}

type DropPos = "before" | "inside" | "after" | null;

// ═══════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════

function buildTree(elements: ElementData[]): TreeNode[] {
  const map = new Map<string, TreeNode>();
  for (const el of elements) {
    if (el.nodeType !== "view") continue;
    map.set(el.id, { el, children: [], depth: 0 });
  }
  const roots: TreeNode[] = [];
  for (const node of map.values()) {
    if (!node.el.parentId || !map.has(node.el.parentId)) {
      roots.push(node);
    } else {
      map.get(node.el.parentId)!.children.push(node);
    }
  }
  // Assign depths
  function assignDepth(nodes: TreeNode[], d: number) {
    for (const n of nodes) {
      n.depth = d;
      assignDepth(n.children, d + 1);
    }
  }
  assignDepth(roots, 0);
  return roots;
}

const TAG_ICONS: Record<string, string> = {
  Container: "📦", Flex: "↔", Grid: "⊞", Card: "🃏", Divider: "—",
  Text: "📝", Paragraph: "📄", Heading: "📰",
  Button: "🔘", Input: "📋", TextArea: "📄", Select: "📌", Checkbox: "☑",
  List: "📃", ListItem: "📎", Image: "🖼", Link: "🔗",
  Spinner: "⏳", Progress: "📊", Alert: "⚠", Form: "📨", Stack: "📚",
  Div: "🔲", Span: "〰", Spacer: "▫",
};

function getIcon(tag: string): string {
  return TAG_ICONS[tag] || "▪";
}

function getLabel(el: ElementData): string {
  const p = el.props || {};
  return String(p.label || p.name || p.placeholder || `<${el.tag}>`);
}

function acceptsChildren(tag: string): boolean {
  return COMPONENT_PROTOCOL[tag]?.children === true;
}

// ═══════════════════════════════════════════════
// Component
// ═══════════════════════════════════════════════

interface ComponentTreeProps {
  ctx: CommandContext;
}

export function ComponentTree({ ctx }: ComponentTreeProps) {
  const elements = useEditorStore((s) => s.elements);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const selectElement = useEditorStore((s) => s.selectElement);
  const undoExec = useUndoStore((s) => s.execute);

  // All container-like nodes expanded by default
  const [expanded, setExpanded] = useState<Set<string>>(() => {
    const s = new Set<string>();
    for (const el of elements) {
      if (el.nodeType !== "view") continue;
      if (el.children.length > 0 || acceptsChildren(el.tag)) s.add(el.id);
    }
    return s;
  });

  // Drop hover state — per-element, reset on dragLeave / drop
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const [dropPos, setDropPos] = useState<DropPos>(null);

  const viewElements = elements.filter((e) => e.nodeType === "view");
  const tree = buildTree(viewElements);

  const toggleExpand = useCallback((id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const handleSelect = useCallback(
    (id: string) => {
      selectElement(id);
    },
    [selectElement],
  );

  // ── Tree drag-and-drop ──

  const handleDragStart = useCallback((e: React.DragEvent, id: string) => {
    e.dataTransfer.setData("omni-tree-node", id);
    e.dataTransfer.effectAllowed = "move";
  }, []);

  const handleDragOver = useCallback(
    (e: React.DragEvent, targetId: string) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";

      const rect = e.currentTarget.getBoundingClientRect();
      const y = (e.clientY - rect.top) / rect.height;
      const target = elements.find((el) => el.id === targetId);
      const canChild = target ? acceptsChildren(target.tag) : false;

      let pos: DropPos;
      if (y < 0.25) {
        pos = "before";
      } else if (y > 0.75) {
        pos = "after";
      } else if (canChild) {
        pos = "inside";
      } else {
        pos = y < 0.5 ? "before" : "after";
      }

      setDropTarget(targetId);
      setDropPos(pos);
    },
    [elements],
  );

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    // Only clear if truly leaving the row, not entering a child
    if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) {
      setDropTarget(null);
      setDropPos(null);
    }
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent, targetId: string) => {
      e.preventDefault();
      setDropTarget(null);
      setDropPos(null);

      const draggedId = e.dataTransfer.getData("omni-tree-node");
      if (!draggedId || draggedId === targetId) return;
      if (!dropPos) return;

      if (dropPos === "inside") {
        const target = elements.find((el) => el.id === targetId);
        if (target && acceptsChildren(target.tag)) {
          undoExec(new MoveElement(draggedId, targetId, null, "after"), ctx);
        }
      } else {
        undoExec(new ReorderElement(draggedId, targetId, dropPos), ctx);
      }
    },
    [dropPos, elements, undoExec, ctx],
  );

  // ── Keyboard shortcuts ──

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const store = useEditorStore.getState();
      const selectedIds = store.selectedElementIds;
      if (selectedIds.length === 0) return;
      const elementId = selectedIds[0];

      // Don't intercept when typing in input fields
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      )
        return;

      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        const cmd = new RemoveElement(elementId);
        useUndoStore.getState().execute(cmd, ctx);
        return;
      }

      if (e.ctrlKey || e.metaKey) {
        switch (e.key) {
          case "c":
            e.preventDefault();
            copySelection(ctx, selectedIds);
            break;
          case "v":
            e.preventDefault();
            const pasted = pasteFromClipboard(ctx);
            if (pasted && pasted.length > 0) {
              store.selectElement(pasted[0]);
            }
            break;
          case "ArrowUp":
            e.preventDefault();
            moveElementUp(elementId);
            break;
          case "ArrowDown":
            e.preventDefault();
            moveElementDown(elementId);
            break;
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [ctx]);

  function moveElementUp(elementId: string) {
    const store = useEditorStore.getState();
    const elements = store.elements;
    const el = elements.find((e) => e.id === elementId);
    if (!el || !el.parentId) return;

    const parent = elements.find((e) => e.id === el.parentId);
    if (!parent) return;

    const idx = parent.children.indexOf(elementId);
    if (idx <= 0) return;

    const cmd = new ReorderElement(
      elementId,
      parent.children[idx - 1],
      "before",
    );
    useUndoStore.getState().execute(cmd, ctx);
  }

  function moveElementDown(elementId: string) {
    const store = useEditorStore.getState();
    const elements = store.elements;
    const el = elements.find((e) => e.id === elementId);
    if (!el || !el.parentId) return;

    const parent = elements.find((e) => e.id === el.parentId);
    if (!parent) return;

    const idx = parent.children.indexOf(elementId);
    if (idx < 0 || idx >= parent.children.length - 1) return;

    const cmd = new ReorderElement(
      elementId,
      parent.children[idx + 1],
      "after",
    );
    useUndoStore.getState().execute(cmd, ctx);
  }

  // ── Render ──

  function renderNode(node: TreeNode): React.ReactElement {
    const isSel = selectedIds.includes(node.el.id);
    const isExp = expanded.has(node.el.id);
    const hasKids = node.children.length > 0;
    const isOver = dropTarget === node.el.id;
    const canChild = acceptsChildren(node.el.tag);

    // Drop indicator — absolutely-positioned overlay (before / after / inside)
    const indicator = isOver
      ? React.createElement("div", {
          style: {
            position: "absolute",
            left: 0,
            right: 0,
            pointerEvents: "none",
            zIndex: 10,
            ...(dropPos === "before"
              ? { top: 0, height: 0, borderTop: "2px solid #4F46E5" }
              : dropPos === "after"
                ? { bottom: 0, height: 0, borderBottom: "2px solid #4F46E5" }
                : {
                    top: 0,
                    bottom: 0,
                    outline: "2px solid #4F46E5",
                    outlineOffset: -2,
                    borderRadius: 4,
                    background: "rgba(79,70,229,0.06)",
                  }),
          },
        })
      : null;

    return React.createElement(
      "div",
      { key: node.el.id, style: { position: "relative" } },
      // ── Row ──
      React.createElement(
        "div",
        {
          draggable: true,
          "data-omni-id": node.el.id,
          onDragStart: (e: React.DragEvent) => handleDragStart(e, node.el.id),
          onDragOver: (e: React.DragEvent) => handleDragOver(e, node.el.id),
          onDragLeave: handleDragLeave,
          onDrop: (e: React.DragEvent) => handleDrop(e, node.el.id),
          onClick: () => handleSelect(node.el.id),
          style: {
            display: "flex",
            alignItems: "center",
            gap: 3,
            padding: "3px 6px 3px 0",
            paddingLeft: 8 + node.depth * 16,
            cursor: "pointer",
            borderRadius: 3,
            fontSize: 12,
            color: isSel ? "#4F46E5" : "#374151",
            background: isSel ? "#EEF2FF" : "transparent",
            userSelect: "none",
            transition: "background 0.1s",
          },
          onMouseEnter: (e: React.MouseEvent) => {
            if (!isSel)
              (e.currentTarget as HTMLElement).style.background = "#F3F4F6";
          },
          onMouseLeave: (e: React.MouseEvent) => {
            if (!isSel && !isOver)
              (e.currentTarget as HTMLElement).style.background = "transparent";
          },
        },
        indicator,
        // Toggle arrow
        React.createElement(
          "span",
          {
            onClick: (e: React.MouseEvent) => {
              e.stopPropagation();
              toggleExpand(node.el.id);
            },
            style: {
              display: "inline-block",
              width: 14,
              textAlign: "center",
              fontSize: 9,
              color: "#9CA3AF",
              flexShrink: 0,
              visibility: hasKids || canChild ? "visible" : "hidden",
            },
          },
          isExp ? "▼" : "▶",
        ),
        // Icon
        React.createElement(
          "span",
          { style: { flexShrink: 0, fontSize: 12 } },
          getIcon(node.el.tag),
        ),
        // Tag name
        React.createElement(
          "span",
          {
            style: {
              fontWeight: 500,
              whiteSpace: "nowrap",
              flexShrink: 0,
            },
          },
          node.el.tag,
        ),
        // Label
        React.createElement(
          "span",
          {
            style: {
              color: "#9CA3AF",
              fontSize: 11,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              flex: 1,
              minWidth: 0,
            },
          },
          getLabel(node.el),
        ),
      ),
      // ── Children ──
      isExp && hasKids
        ? React.createElement(
            "div",
            null,
            ...node.children.map((c) => renderNode(c)),
          )
        : null,
    );
  }

  if (viewElements.length === 0) {
    return React.createElement(
      "div",
      {
        style: {
          padding: 16,
          fontSize: 11,
          color: "#9CA3AF",
          textAlign: "center",
        },
      },
      "画布为空 — 从组件面板拖入组件",
    );
  }

  return React.createElement(
    "div",
    { style: { padding: "2px 0", fontSize: 12, overflow: "auto" } },
    ...tree.map((n) => renderNode(n)),
  );
}
