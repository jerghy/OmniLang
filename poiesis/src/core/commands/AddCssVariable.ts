// core/commands/AddCssVariable.ts
import type { Command, CommandContext } from "./types";
import type { CssVariable } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class AddCssVariable implements Command {
  readonly type = "AddCssVariable";
  readonly description: string;

  constructor(
    private componentName: string,
    private variable: CssVariable,
  ) {
    this.description = `添加 CSS 变量 ${this.variable.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addVariable(this.componentName, this.variable);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeVariable(this.componentName, this.variable.id);
  }
}
