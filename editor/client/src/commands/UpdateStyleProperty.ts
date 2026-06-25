import type { Command, CommandContext } from "./types";
import type { StyleEntry, StyleProperty } from "../constants/style-properties";

export class UpdateStyleProperty implements Command {
  readonly type = "UpdateStyleProperty";
  readonly description: string;

  constructor(
    private elementId: string,
    private entryId: string,
    private propertyIndex: number,
    private oldProperty: StyleProperty | null,
    private newProperty: StyleProperty | null,
  ) {
    this.description = newProperty
      ? `修改样式属性 ${newProperty.name}`
      : `删除样式属性 ${oldProperty?.name}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) => {
      if (s.id !== this.entryId) return s;
      const newProps = [...s.properties];
      if (this.propertyIndex === -1 && this.newProperty) {
        newProps.push(this.newProperty);
      } else if (this.newProperty === null) {
        newProps.splice(this.propertyIndex, 1);
      } else {
        newProps[this.propertyIndex] = this.newProperty;
      }
      return { ...s, properties: newProps };
    });
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.styles) return;
    el.styles = el.styles.map((s) => {
      if (s.id !== this.entryId) return s;
      const newProps = [...s.properties];
      if (this.propertyIndex === -1 && this.newProperty) {
        newProps.pop();
      } else if (this.newProperty === null && this.oldProperty) {
        newProps.splice(this.propertyIndex, 0, this.oldProperty);
      } else if (this.oldProperty) {
        newProps[this.propertyIndex] = this.oldProperty;
      }
      return { ...s, properties: newProps };
    });
  }
}
