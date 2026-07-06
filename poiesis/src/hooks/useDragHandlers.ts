// hooks/useDragHandlers.ts
// Hook that provides drag-and-drop handlers for the DesignPage: palette-to-canvas and tree-node reordering.

import { useState, useRef, useCallback } from "react";
import type {
  DragStartEvent,
  DragMoveEvent,
  DragEndEvent,
} from "@dnd-kit/core";
import { usePoiesisStore } from "../store";
import { createCommandContext } from "../core/commands/createCommandContext";
import { uid } from "../core/utils/id";
import { getDefaultProps } from "../core/constants/defaults";
import type { ElementData } from "../core/types/element";
import { MoveElement } from "../core/commands/MoveElement";
import { ReorderElement } from "../core/commands/ReorderElement";
import type { TreeDropState } from "../components/panels/TreeNodeRenderer";

export function useDragHandlers(ctx: ReturnType<typeof createCommandContext>) {
  const addElement = usePoiesisStore((s) => s.addElement);
  const [activeComponent, setActiveComponent] = useState<string | null>(null);
  const [treeDropState, setTreeDropState] = useState<TreeDropState | null>(null);
  const treeDropRef = useRef<TreeDropState | null>(null);
  const [activeTreeId, setActiveTreeId] = useState<string | null>(null);

  const exec = useCallback(
    (cmd: import("../core/commands/types").Command) => cmd.execute(ctx),
    [ctx],
  );

  // ── Unified drag start (palette + tree) ──
  const handleDragStart = (event: DragStartEvent) => {
    const data = event.active.data.current;
    if (data?.type === "tree-node") {
      setActiveTreeId(data.elementId as string);
      return;
    }
    const tag = data?.tag as string | undefined;
    setActiveComponent(tag ?? null);
  };

  // ── Unified drag move (tree reorder position calculation) ──
  const handleDragMove = (event: DragMoveEvent) => {
    const data = event.active.data.current;
    if (data?.type !== "tree-node") return;

    const over = event.over;
    if (!over || typeof over.id !== "string" || !over.id.startsWith("tree-")) {
      setTreeDropState(null);
      treeDropRef.current = null;
      return;
    }

    const targetId = over.id.replace("tree-", "");
    const activeRect = event.active.rect.current.translated;
    const overRect = over.rect;

    if (!activeRect || !overRect) {
      setTreeDropState(null);
      treeDropRef.current = null;
      return;
    }

    // top 25% → before, bottom 25% → after, middle 50% → inside
    const dragCenterY = activeRect.top + activeRect.height / 2;
    const overTop = overRect.top;
    const overHeight = overRect.height;

    let position: "before" | "after" | "inside";
    if (dragCenterY < overTop + overHeight * 0.25) {
      position = "before";
    } else if (dragCenterY > overTop + overHeight * 0.75) {
      position = "after";
    } else {
      position = "inside";
    }

    if (targetId === data.elementId) {
      setTreeDropState(null);
      treeDropRef.current = null;
      return;
    }

    setTreeDropState({ targetId, position });
    treeDropRef.current = { targetId, position };
  };

  // ── Unified drag end (palette drop + tree reorder/nest) ──
  const handleDragEnd = (event: DragEndEvent) => {
    const data = event.active.data.current;

    // ── Tree-node drop ──
    if (data?.type === "tree-node") {
      setActiveTreeId(null);
      const draggedId = data.elementId as string | undefined;
      const dropInfo = treeDropRef.current;
      setTreeDropState(null);
      treeDropRef.current = null;

      // Resolve target from dropInfo (pointer drag) or event.over (keyboard / no-move drags)
      let targetId: string | undefined;
      let position: "before" | "after" | "inside" = "after";
      if (dropInfo && dropInfo.targetId !== draggedId) {
        targetId = dropInfo.targetId;
        position = dropInfo.position;
      } else if (event.over && typeof event.over.id === "string" && event.over.id.startsWith("tree-")) {
        targetId = event.over.id.replace("tree-", "");
      }

      if (!draggedId || !targetId || draggedId === targetId) return;

      if (position === "inside") {
        exec(new MoveElement(draggedId, targetId, null, "after"));
      } else {
        const draggedEl = ctx.getElement(draggedId);
        const targetEl = ctx.getElement(targetId);

        if (draggedEl && targetEl && draggedEl.parentId === targetEl.parentId) {
          exec(new ReorderElement(draggedId, targetId, position));
        } else if (targetEl) {
          exec(new MoveElement(draggedId, targetEl.parentId ?? (null as unknown as string), targetId, position));
        }
      }
      return;
    }

    // ── Palette drop on canvas ──
    setActiveComponent(null);

    if (event.over?.id !== "design-canvas") return;

    const tag = data?.tag as string | undefined;
    if (!tag) return;

    const id = uid(tag.toLowerCase());
    const props = getDefaultProps(tag);

    // Only PascalCase tags are custom component references; built-in HTML
    // tags (div, p, button, etc.) should not carry a componentRef.
    const isPascal = /^[A-Z]/.test(tag);

    const newEl: ElementData = {
      id,
      nodeType: "view",
      tag,
      props,
      children: [],
      parentId: null,
      ...(isPascal ? { componentRef: tag } : {}),
    };

    addElement(newEl);
  };

  return {
    handleDragStart,
    handleDragMove,
    handleDragEnd,
    activeComponent,
    activeTreeId,
    treeDropState,
  };
}
