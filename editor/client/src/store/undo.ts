/** Command 模式撤销/重做栈 + IndexedDB 持久化。*/

import { create } from "zustand";
import type { Command, CommandContext } from "../commands/types";
import { saveSnapshot, loadSnapshot } from "./undoPersistence";
import { useEditorStore } from "./editor";

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
        const store = useEditorStore.getState();
        // Only restore if the current canvas is empty (possible freeze-discard scenario)
        if (store.elements.length === 0 && store.wires.length === 0) {
          store.setElementsAndWires(snapshot.elements, snapshot.wires);
        }
      }
    }).catch(() => {});
  }
});
