import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class SetStyleEntryType implements Command {
  readonly type = "SetStyleEntryType";
  readonly description: string;

  constructor(
    private elementId: string,
    private entryId: string,
    private oldType: StyleEntry["type"],
    private newType: StyleEntry["type"],
  ) {
    this.description = `切换样式类型 ${oldType} → ${newType}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) =>
      s.id === this.entryId ? { ...s, type: this.newType } : s,
    );
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) =>
      s.id === this.entryId ? { ...s, type: this.oldType } : s,
    );
  }
}
