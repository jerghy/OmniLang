import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../types";

export class ReorderStyleEntry implements Command {
  readonly type = "ReorderStyleEntry";
  readonly description = "重新排序样式";

  constructor(
    private elementId: string,
    private oldOrder: string[],
    private newOrder: string[],
  ) {}

  private applyOrder(ctx: CommandContext, order: string[]): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    const byId = new Map(el.styles.map((s: StyleEntry) => [s.id, s]));
    el.styles = order.map((id) => byId.get(id)!).filter(Boolean);
  }

  execute(ctx: CommandContext): void { this.applyOrder(ctx, this.newOrder); }
  undo(ctx: CommandContext): void { this.applyOrder(ctx, this.oldOrder); }
}
