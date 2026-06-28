import type { Command, CommandContext } from "./types";
import type { ElementData } from "../types";

export class RemoveElement implements Command {
  readonly type = "RemoveElement";
  readonly description: string;
  private _snapshot: ElementData | undefined;
  private id: string;

  constructor(id: string) {
    this.id = id;
    this.description = `删除元素`;
  }

  execute(ctx: CommandContext): void {
    this._snapshot = ctx.getElement(this.id);
    if (this._snapshot) {
      ctx.removeElement(this.id);
    }
  }

  undo(ctx: CommandContext): void {
    if (this._snapshot) {
      ctx.addElement(this._snapshot);
    }
  }
}
