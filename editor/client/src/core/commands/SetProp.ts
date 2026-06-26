import type { Command, CommandContext } from "./types";

export class SetProp implements Command {
  readonly type = "SetProp";
  readonly description: string;
  private _oldValue: unknown;
  private elementId: string;
  private prop: string;
  private newValue: unknown;

  constructor(
    elementId: string,
    prop: string,
    newValue: unknown,
  ) {
    this.elementId = elementId;
    this.prop = prop;
    this.newValue = newValue;
    this.description = `修改 ${prop}`;
  }

  execute(ctx: CommandContext): void {
    this._oldValue = ctx.getProp(this.elementId, this.prop);
    ctx.setProp(this.elementId, this.prop, this.newValue);
  }

  undo(ctx: CommandContext): void {
    ctx.setProp(this.elementId, this.prop, this._oldValue);
  }
}
