// core/commands/RemoveClassProperty.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class RemoveClassProperty implements Command {
  readonly type = "RemoveClassProperty";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private property: StyleProperty,
  ) {
    this.description = `删除属性 ${this.property.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeProperty(this.componentName, this.classId, this.property.id);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addProperty(this.componentName, this.classId, this.property);
  }
}
