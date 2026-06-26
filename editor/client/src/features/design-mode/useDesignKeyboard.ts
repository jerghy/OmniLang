import { useEffect } from "react";
import { useEditorStore } from "../../store/editor";
import { useUndoStore } from "../../store/undo";
import { showToast } from "../../notifications/toast";
import {
  copySelection,
  pasteFromClipboard,
  cutSelection,
  duplicateSelection,
} from "../../core/commands/CopyPaste";
import type { CommandContext } from "../../core/commands/types";

export function useDesignKeyboard(
  ctx: CommandContext,
  opts: {
    setShowSearch: (v: boolean) => void;
    setActivePanel: (fn: (prev: string | null) => string | null) => void;
    setShowHUD: (v: boolean) => void;
    setShowConflict: (v: boolean) => void;
    mode: "design" | "code";
  },
) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      const { selectedElementIds } = useEditorStore.getState();

      // Activity Bar panel shortcuts (mode-aware)
      if (ctrl && e.key === "1") {
        e.preventDefault();
        opts.setActivePanel((p) => {
          const target: string = opts.mode === "design" ? "components" : "files";
          return p === target ? null : target;
        });
      }
      if (ctrl && e.key === "2") {
        e.preventDefault();
        opts.setActivePanel((p) => {
          const target: string = opts.mode === "design" ? "canvas" : "state";
          return p === target ? null : target;
        });
      }
      if (ctrl && e.key === "3") {
        e.preventDefault();
        opts.setActivePanel((p) => (p === "functions" ? null : "functions"));
      }

      // Clipboard shortcuts
      if (ctrl && e.key === "c" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const data = copySelection(ctx, selectedElementIds);
        if (data) showToast(`已复制 ${data.elements.length} 个元素`, "info");
      }
      if (ctrl && e.key === "v" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        e.preventDefault();
        const pastedIds = pasteFromClipboard(ctx);
        if (pastedIds && pastedIds.length > 0) {
          useEditorStore.setState({ selectedElementIds: pastedIds });
          showToast(`已粘贴 ${pastedIds.length} 个元素`, "success");
        } else {
          showToast("剪贴板为空", "info");
        }
      }
      if (ctrl && e.key === "x" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const ids = cutSelection(ctx, selectedElementIds);
        if (ids) showToast(`已剪切 ${ids.length} 个元素`, "success");
      }
      if (ctrl && e.key === "d" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const dupIds = duplicateSelection(ctx, selectedElementIds);
        if (dupIds) {
          useEditorStore.setState({ selectedElementIds: dupIds });
          showToast(`已复制 ${dupIds.length} 个元素`, "success");
        }
      }
      // Select all
      if (ctrl && e.key === "a" && !e.shiftKey) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        e.preventDefault();
        useEditorStore.getState().selectAll();
        const allIds = useEditorStore.getState().selectedElementIds;
        showToast(`已选择 ${allIds.length} 个元素`, "info");
      }

      // Arrow key nudge
      if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.key)) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        if (selectedElementIds.length === 0) return;
        e.preventDefault();
        const delta = e.shiftKey ? 10 : 1;
        const dx = e.key === "ArrowLeft" ? -delta : e.key === "ArrowRight" ? delta : 0;
        const dy = e.key === "ArrowUp" ? -delta : e.key === "ArrowDown" ? delta : 0;
        const store = useEditorStore.getState();
        for (const id of selectedElementIds) {
          const el = store.elements.find((x) => x.id === id);
          if (el?.position) {
            store.setElementPosition(id, {
              x: el.position.x + dx,
              y: el.position.y + dy,
            });
          }
        }
      }

      // Escape: close popups + clear selection
      if (e.key === "Escape") {
        opts.setShowHUD(false);
        opts.setShowSearch(false);
        opts.setShowConflict(false);
        opts.setActivePanel(() => null);
        useEditorStore.getState().clearSelection();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [ctx, opts.mode]);
}
