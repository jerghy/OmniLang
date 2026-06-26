import type { Command, CommandContext } from "./types";
import type { WireData } from "../core/types";

export class RemoveWire implements Command {
  readonly type = "RemoveWire";
  readonly description = "删除连线";
  private _snapshot: WireData | undefined;
  private id: string;

  constructor(id: string) {
    this.id = id;
  }

  execute(ctx: CommandContext): void {
    this._snapshot = ctx.wires.find((w) => w.id === this.id);
    if (this._snapshot) {
      ctx.removeWire(this.id);
    }
  }

  undo(ctx: CommandContext): void {
    if (this._snapshot) {
      ctx.addWire(this._snapshot);
    }
  }
}
