import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class AddStyleEntry implements Command {
  readonly type = "AddStyleEntry";
  readonly description: string;

  constructor(
    private elementId: string,
    private entry: StyleEntry,
  ) {
    this.description = `添加样式 ${entry.name || entry.type}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el) return;
    el.styles = [...(el.styles ?? []), this.entry];
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.filter((s) => s.id !== this.entry.id);
  }
}
