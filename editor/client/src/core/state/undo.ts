/** Command 模式撤销/重做栈 + IndexedDB 持久化。（migrated to core/state/）*/

import { create } from "zustand";
import type { Command, CommandContext } from "../../core/commands/types";
import { saveSnapshot, loadSnapshot } from "../../store/undoPersistence";
import { useElementStore } from "../../core/state/elements";
import { useProjectStore } from "../../core/state/project";
import { useWireStore } from "../../core/state/wires";

interface UndoState {
  undoStack: Command[];
  redoStack: Command[];
  push: (cmd: Command) => void;
  /** Execute a command and push it to the undo stack (for user-invoked actions). */
  execute: (cmd: Command, ctx: CommandContext) => void;
  undo: (ctx: CommandContext) => void;
  redo: (ctx: CommandContext) => void;
  clear: () => void;
}

export const useUndoStore = create<UndoState>((set, get) => ({
  undoStack: [],
  redoStack: [],

  push: (cmd) =>
    set((s) => ({
      undoStack: [...s.undoStack.slice(-199), cmd], // 最多保留 200 条
    })),

  execute: (cmd, ctx) => {
    cmd.execute(ctx);
    // Trigger re-render and mark dirty
    useElementStore.setState((s) => ({
      elements: [...s.elements],
    }));
    useProjectStore.getState().markDirty();
    set((s) => ({
      undoStack: [...s.undoStack.slice(-199), cmd],
      redoStack: [], // 新操作清空 redo 栈
    }));
    // IndexedDB snapshot
    saveSnapshot(ctx.elements, ctx.wires).catch(() => {});
  },

  undo: (ctx) => {
    const { undoStack } = get();
    if (undoStack.length === 0) return;
    const cmd = undoStack[undoStack.length - 1];
    cmd.undo(ctx);
    useElementStore.setState((s) => ({
      elements: [...s.elements],
    }));
    useProjectStore.getState().markDirty();
    set({
      undoStack: undoStack.slice(0, -1),
      redoStack: [...get().redoStack.slice(-199), cmd],
    });
    saveSnapshot(ctx.elements, ctx.wires).catch(() => {});
  },

  redo: (ctx) => {
    const { redoStack } = get();
    if (redoStack.length === 0) return;
    const cmd = redoStack[redoStack.length - 1];
    cmd.execute(ctx);
    useElementStore.setState((s) => ({
      elements: [...s.elements],
    }));
    useProjectStore.getState().markDirty();
    set({
      redoStack: redoStack.slice(0, -1),
      undoStack: [...get().undoStack.slice(-199), cmd],
    });
    saveSnapshot(ctx.elements, ctx.wires).catch(() => {});
  },

  clear: () => set({ undoStack: [], redoStack: [] }),
}));

// ── Tab 冻结恢复 ──────────────────────────────

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    loadSnapshot().then((snapshot) => {
      if (snapshot && snapshot.elements.length > 0) {
        const elStore = useElementStore.getState();
        const wireStore = useWireStore.getState();
        // Only restore if the current canvas is empty (possible freeze-discard scenario)
        if (elStore.elements.length === 0 && wireStore.wires.length === 0) {
          elStore.setElementsAndWires(snapshot.elements, snapshot.wires);
          wireStore.setWires(snapshot.wires);
        }
      }
    }).catch(() => {});
  }
});
