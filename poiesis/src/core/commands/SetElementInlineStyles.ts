// core/commands/SetElementInlineStyles.ts
import type { Command, CommandContext } from "./types";
import type { StyleProperty } from "../types/style";
import { useEditorStore } from "../../store/editor";

export class SetElementInlineStyles implements Command {
  readonly type = "SetElementInlineStyles";
  readonly description = "设置元素内联样式";

  constructor(
    private elementId: string,
    private oldProperties: StyleProperty[],
    private newProperties: StyleProperty[],
  ) {
    this.description = `设置元素内联样式`;
  }

  execute(_ctx: CommandContext): void {
    useEditorStore.getState().setElementInlineStyles(this.elementId, this.newProperties);
  }

  undo(_ctx: CommandContext): void {
    useEditorStore.getState().setElementInlineStyles(this.elementId, this.oldProperties);
  }
}
