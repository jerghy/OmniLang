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
  private _isRoot: boolean = false;

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
    if (!el) return;
    this._parentId = el.parentId;
    this._isRoot = !el.parentId;

    if (this._isRoot) {
      const rootOrder = ctx.getRootOrder?.() ?? [];
      const idx = rootOrder.indexOf(this.elementId);
      if (idx > 0) {
        this._prevSiblingId = rootOrder[idx - 1];
      }
    } else {
      const parent = ctx.getElement(el.parentId!);
      if (!parent) return;
      const idx = parent.children.indexOf(this.elementId);
      if (idx > 0) {
        this._prevSiblingId = parent.children[idx - 1];
      }
    }
    ctx.reorderElement(this.elementId, this.targetSiblingId, this.position);
  }

  undo(ctx: CommandContext): void {
    if (this._isRoot) {
      const rootOrder = ctx.getRootOrder?.() ?? [];
      if (this._prevSiblingId === null) {
        if (rootOrder.length === 0 || rootOrder[0] === this.elementId) return;
        ctx.reorderElement(this.elementId, rootOrder[0], "before");
      } else {
        ctx.reorderElement(this.elementId, this._prevSiblingId, "after");
      }
      return;
    }
    if (!this._parentId) return;
    if (this._prevSiblingId === null) {
      const parent = ctx.getElement(this._parentId);
      if (!parent || parent.children.length === 0) return;
      const currentFirst = parent.children[0];
      if (currentFirst === this.elementId) return;
      ctx.reorderElement(this.elementId, currentFirst, "before");
    } else {
      ctx.reorderElement(this.elementId, this._prevSiblingId, "after");
    }
  }
}
