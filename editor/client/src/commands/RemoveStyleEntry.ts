import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class RemoveStyleEntry implements Command {
  readonly type = "RemoveStyleEntry";
  readonly description: string;

  constructor(
    private elementId: string,
    private entry: StyleEntry,
    private index: number,
  ) {
    this.description = `删除样式 ${this.entry.name || this.entry.type}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.filter((s) => s.id !== this.entry.id);
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el) return;
    const styles = el.styles ?? [];
    const newStyles = [...styles];
    newStyles.splice(this.index, 0, this.entry);
    el.styles = newStyles;
  }
}
