// core/commands/RemoveStyleClass.ts
import type { Command, CommandContext } from "./types";
import type { StyleClass } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class RemoveStyleClass implements Command {
  readonly type = "RemoveStyleClass";
  readonly description: string;

  constructor(
    private componentName: string,
    private styleClass: StyleClass,
  ) {
    this.description = `删除类 ${this.styleClass.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeClass(this.componentName, this.styleClass.id);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addClass(this.componentName, this.styleClass);
  }
}
