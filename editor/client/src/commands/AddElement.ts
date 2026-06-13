import type { Command, CommandContext } from "./types";
import type { ElementData } from "../store/editor";

export class AddElement implements Command {
  readonly type = "AddElement";
  readonly description: string;
  private el: ElementData;

  constructor(el: ElementData) {
    this.el = el;
    this.description = `添加 <${el.tag}>`;
  }

  execute(ctx: CommandContext): void {
    ctx.addElement(this.el);
  }

  undo(ctx: CommandContext): void {
    ctx.removeElement(this.el.id);
  }
}
