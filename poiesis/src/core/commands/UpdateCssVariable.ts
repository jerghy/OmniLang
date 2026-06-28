// core/commands/UpdateCssVariable.ts
import type { Command, CommandContext } from "./types";
import type { CssVariable } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class UpdateCssVariable implements Command {
  readonly type = "UpdateCssVariable";
  readonly description: string;

  constructor(
    private componentName: string,
    private variableId: string,
    private oldPatch: Partial<CssVariable>,
    private newPatch: Partial<CssVariable>,
  ) {
    this.description = `更新 CSS 变量`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateVariable(this.componentName, this.variableId, this.newPatch);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateVariable(this.componentName, this.variableId, this.oldPatch);
  }
}
