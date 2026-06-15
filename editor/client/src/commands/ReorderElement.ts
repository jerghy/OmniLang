import type { Command, CommandContext } from "./types";

export class ReorderElement implements Command {
  readonly type = "ReorderElement";
  readonly description = "重新排序元素";
  private elementId: string;
  private targetSiblingId: string;
  private position: "before" | "after";
  /** The sibling that came BEFORE this element in the original order (null if it was first). */
  private _prevSiblingId: string | null = null;
  private _parentId: string | null = null;

  constructor(
    elementId: string,
    targetSiblingId: string,
    position: "before" | "after",
  ) {
    this.elementId = elementId;
    this.targetSiblingId = targetSiblingId;
    this.position = position;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el?.parentId) return;
    this._parentId = el.parentId;
    const parent = ctx.getElement(el.parentId);
    if (!parent) return;
    const idx = parent.children.indexOf(this.elementId);
    if (idx > 0) {
      this._prevSiblingId = parent.children[idx - 1];
    }
    ctx.reorderElement(this.elementId, this.targetSiblingId, this.position);
  }

  undo(ctx: CommandContext): void {
    if (!this._parentId) return;
    // Move element back to its original position
    if (this._prevSiblingId === null) {
      // Element was first — move it before whatever is currently first
      const parent = ctx.getElement(this._parentId);
      if (!parent || parent.children.length === 0) return;
      const currentFirst = parent.children[0];
      if (currentFirst === this.elementId) return; // already first
      ctx.reorderElement(this.elementId, currentFirst, "before");
    } else {
      // Move back after the sibling it was originally after
      ctx.reorderElement(this.elementId, this._prevSiblingId, "after");
    }
  }
}
