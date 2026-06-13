/** Command 模式撤销/重做栈 + IndexedDB 持久化。*/

import { create } from "zustand";
import type { Command, CommandContext } from "../commands/types";

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
  },

  clear: () => set({ undoStack: [], redoStack: [] }),
}));

// ── Tab 冻结恢复 ──────────────────────────────

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    // 恢复：从 IndexedDB 重建（M1b 完成时实现）
  }
});
