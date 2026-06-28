// core/commands/RemoveCssVariable.ts
import type { Command, CommandContext } from "./types";
import type { CssVariable } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class RemoveCssVariable implements Command {
  readonly type = "RemoveCssVariable";
  readonly description: string;

  constructor(
    private componentName: string,
    private variable: CssVariable,
  ) {
    this.description = `删除 CSS 变量 ${this.variable.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeVariable(this.componentName, this.variable.id);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addVariable(this.componentName, this.variable);
  }
}
