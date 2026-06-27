// core/commands/RenameStyleClass.ts
import type { Command, CommandContext } from "./types";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class RenameStyleClass implements Command {
  readonly type = "RenameStyleClass";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private oldName: string,
    private newName: string,
  ) {
    this.description = `重命名类 ${oldName} → ${newName}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().renameClass(this.componentName, this.classId, this.newName);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().renameClass(this.componentName, this.classId, this.oldName);
  }
}
