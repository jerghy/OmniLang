import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../types";

export class UpdateStyleEntry implements Command {
  readonly type = "UpdateStyleEntry";
  readonly description: string;

  constructor(
    private elementId: string,
    private entryId: string,
    private oldEntry: StyleEntry,
    private newEntry: StyleEntry,
  ) {
    this.description = `修改样式 ${newEntry.name || newEntry.type}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) => {
      if (s.id !== this.entryId) return s;
      return { ...s, ...this.newEntry };
    });
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) => {
      if (s.id !== this.entryId) return s;
      return { ...s, ...this.oldEntry };
    });
  }
}
