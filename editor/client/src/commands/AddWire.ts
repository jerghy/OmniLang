import type { Command, CommandContext } from "./types";
import type { WireData } from "../store/editor";

export class AddWire implements Command {
  readonly type = "AddWire";
  readonly description: string;
  private wire: WireData;

  constructor(wire: WireData) {
    this.wire = wire;
    this.description = `连线 ${wire.fromPort} → ${wire.toPort}`;
  }

  execute(ctx: CommandContext): void {
    ctx.addWire(this.wire);
  }

  undo(ctx: CommandContext): void {
    ctx.removeWire(this.wire.id);
  }
}
