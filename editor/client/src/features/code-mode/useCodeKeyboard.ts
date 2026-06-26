import { useEffect } from "react";
import { useEditorStore } from "../../store/editor";
import { showToast } from "../../notifications/toast";

export interface CodeKeyboardOptions {
  handleCompile: () => void;
  saveProject: () => void;
  searchProject: () => void;
  undoUndo: (ctx: import("../../core/commands/types").CommandContext) => void;
  undoRedo: (ctx: import("../../core/commands/types").CommandContext) => void;
  ctx: import("../../core/commands/types").CommandContext;
}

export function useCodeKeyboard(opts: CodeKeyboardOptions) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;

      // Compile
      if (ctrl && e.key === "Enter") { e.preventDefault(); opts.handleCompile(); }
      // Undo / Redo
      if (ctrl && e.key === "z" && !e.shiftKey) { e.preventDefault(); opts.undoUndo(opts.ctx); }
      if (ctrl && (e.key === "y" || (e.key === "z" && e.shiftKey))) { e.preventDefault(); opts.undoRedo(opts.ctx); }
      // Save
      if (ctrl && e.key === "s") { e.preventDefault(); opts.saveProject(); }
      // Search
      if (ctrl && e.shiftKey && e.key === "F") { e.preventDefault(); opts.searchProject(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [opts.handleCompile, opts.saveProject, opts.undoUndo, opts.undoRedo, opts.ctx]);
}
