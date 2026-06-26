import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../core/types";

export class RenameStyleEntry implements Command {
  readonly type = "RenameStyleEntry";
  readonly description: string;

  constructor(
    private elementId: string,
    private entryId: string,
    private oldName: string,
    private newName: string,
  ) {
    this.description = `重命名样式 ${oldName} → ${newName}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) =>
      s.id === this.entryId ? { ...s, name: this.newName } : s,
    );
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) =>
      s.id === this.entryId ? { ...s, name: this.oldName } : s,
    );
  }
}
