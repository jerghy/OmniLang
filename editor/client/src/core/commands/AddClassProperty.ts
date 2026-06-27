// core/commands/AddClassProperty.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class AddClassProperty implements Command {
  readonly type = "AddClassProperty";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private property: StyleProperty,
  ) {
    this.description = `添加属性 ${this.property.name}`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().addProperty(this.componentName, this.classId, this.property);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().removeProperty(this.componentName, this.classId, this.property.id);
  }
}
