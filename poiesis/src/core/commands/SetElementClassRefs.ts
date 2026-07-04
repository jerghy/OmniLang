// core/commands/SetElementClassRefs.ts
import type { Command, CommandContext } from "./types";
import { usePoiesisStore } from "../../store";

export class SetElementClassRefs implements Command {
  readonly type = "SetElementClassRefs";
  readonly description = "设置元素类引用";

  constructor(
    private elementId: string,
    private oldClassRefs: string[],
    private newClassRefs: string[],
  ) {
    this.description = `设置元素类引用`;
  }

  execute(_ctx: CommandContext): void {
    usePoiesisStore.getState().setElementClassRefs(this.elementId, this.newClassRefs);
  }

  undo(_ctx: CommandContext): void {
    usePoiesisStore.getState().setElementClassRefs(this.elementId, this.oldClassRefs);
  }
}
