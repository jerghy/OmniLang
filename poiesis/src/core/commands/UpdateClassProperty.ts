// core/commands/UpdateClassProperty.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useStyleRegistryStore } from "../state/styleRegistry";

export class UpdateClassProperty implements Command {
  readonly type = "UpdateClassProperty";
  readonly description: string;

  constructor(
    private componentName: string,
    private classId: string,
    private propertyId: string,
    private oldPatch: Partial<StyleProperty>,
    private newPatch: Partial<StyleProperty>,
  ) {
    this.description = `更新属性`;
  }

  execute(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateProperty(this.componentName, this.classId, this.propertyId, this.newPatch);
  }

  undo(_ctx: CommandContext): void {
    useStyleRegistryStore.getState().updateProperty(this.componentName, this.classId, this.propertyId, this.oldPatch);
  }
}
