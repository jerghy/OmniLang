// core/commands/UpdateStyleClass.ts
import type { Command, CommandContext } from "./types";
import type { StyleClass } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class UpdateStyleClass implements Command {
  readonly type = "UpdateStyleClass";
  readonly description: string;

  constructor(
    private componentName: string,
    private oldClass: StyleClass,
    private newClass: StyleClass,
  ) {
    this.description = `更新类 ${newClass.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().replaceClass(this.componentName, this.newClass);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().replaceClass(this.componentName, this.oldClass);
  }
}
