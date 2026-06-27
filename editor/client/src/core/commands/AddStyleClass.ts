// core/commands/AddStyleClass.ts
import type { Command, CommandContext } from "./types";
import type { StyleClass } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class AddStyleClass implements Command {
  readonly type = "AddStyleClass";
  readonly description: string;

  constructor(
    private componentName: string,
    private styleClass: StyleClass,
  ) {
    this.description = `添加类 ${this.styleClass.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addClass(this.componentName, this.styleClass);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeClass(this.componentName, this.styleClass.id);
  }
}
