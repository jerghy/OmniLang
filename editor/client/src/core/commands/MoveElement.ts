import type { Command, CommandContext } from "./types";

export class MoveElement implements Command {
  readonly type = "MoveElement";
  readonly description = "移动元素到新容器";
  private elementId: string;
  private newParentId: string;
  private targetSiblingId: string | null;
  private position: "before" | "after";
  /** Previous parent and sibling for undo. */
  private _prevParentId: string | null = null;
  private _prevSiblingId: string | null = null;

  constructor(
    elementId: string,
    newParentId: string,
    targetSiblingId: string | null,
    position: "before" | "after",
  ) {
    this.elementId = elementId;
    this.newParentId = newParentId;
    this.targetSiblingId = targetSiblingId;
    this.position = position;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId);
    if (!el) return;
    this._prevParentId = el.parentId;
    if (el.parentId) {
      const oldParent = ctx.getElement(el.parentId);
      if (oldParent) {
        const idx = oldParent.children.indexOf(this.elementId);
        if (idx > 0) {
          this._prevSiblingId = oldParent.children[idx - 1];
        }
      }
    }
    ctx.moveElement(this.elementId, this.newParentId, this.targetSiblingId, this.position);
  }

  undo(ctx: CommandContext): void {
    if (this._prevParentId === null) {
      // Was a root element — move back to root
      ctx.moveElement(this.elementId, null, null, "after");
      return;
    }
    const parent = ctx.getElement(this._prevParentId);
    if (!parent || parent.children.length === 0) {
      // Parent is empty — just move back (append)
      ctx.moveElement(this.elementId, this._prevParentId, null, "after");
      return;
    }
    if (this._prevSiblingId === null) {
      // Was first child — move before current first
      const currentFirst = parent.children[0];
      if (currentFirst === this.elementId) return;
      ctx.moveElement(this.elementId, this._prevParentId, currentFirst, "before");
    } else {
      // Move back after the previous sibling
      ctx.moveElement(this.elementId, this._prevParentId, this._prevSiblingId, "after");
    }
  }
}
