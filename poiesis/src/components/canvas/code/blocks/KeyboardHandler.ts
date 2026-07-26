// poiesis/src/components/canvas/code/blocks/KeyboardHandler.ts
// Task 5: Pure DOM keyboard handler — no Blockly dependency.
// Manages keyboard navigation and actions on a BlockModel, dispatching
// to callbacks for fuzzy search, raw input, and ops-changed notifications.

import type { BlockModel, SelectionTarget } from "./BlockModel";

export interface KBCallbacks {
  onFuzzySearch: (query: string, context: "slot" | "gap") => void;
  onRawInput: () => void;
  onOpsChanged: () => void;
}

export class KeyboardHandler {
  private container: HTMLElement;
  private model: BlockModel;
  private getElement: (target: SelectionTarget) => HTMLElement | SVGElement | null;
  private callbacks: KBCallbacks;
  private handleKeyDown: (e: KeyboardEvent) => void;

  constructor(
    container: HTMLElement,
    model: BlockModel,
    getElement: (target: SelectionTarget) => HTMLElement | SVGElement | null,
    callbacks: KBCallbacks,
  ) {
    this.container = container;
    this.model = model;
    this.getElement = getElement;
    this.callbacks = callbacks;
    this.handleKeyDown = this.onKeyDown.bind(this);
    container.setAttribute("tabindex", "0");
    container.addEventListener("keydown", this.handleKeyDown);
    container.focus();
  }

  private onKeyDown(e: KeyboardEvent): void {
    const target = this.model.getCurrentTarget();
    if (!target) return;

    // ── Navigation ──
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      this.model.moveLeft();
      return;
    }
    if (e.key === "ArrowRight") {
      e.preventDefault();
      this.model.moveRight();
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      this.moveSpatial("up");
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      this.moveSpatial("down");
      return;
    }

    // ── Context-sensitive ──
    switch (e.key) {
      case "Enter":
        e.preventDefault();
        this.handleEnter(target);
        break;
      case "Delete":
        e.preventDefault();
        this.handleDelete(target);
        break;
      case "Backspace":
        e.preventDefault();
        this.handleBackspace(target);
        break;
      case "\\":
        e.preventDefault();
        this.callbacks.onRawInput();
        break;
      default:
        if (e.key.length === 1) {
          this.handleCharacter(e.key, target);
        }
    }
  }

  private handleEnter(target: SelectionTarget): void {
    if (target.type === "gap") {
      this.model.insertGap(target.position);
      this.callbacks.onFuzzySearch("", "gap");
    } else if (target.type === "slot") {
      this.callbacks.onFuzzySearch("", "slot");
    }
    // block: no-op (RawCodeOp edit handled by RawBlock component)
  }

  private handleDelete(target: SelectionTarget): void {
    if (target.type === "block") {
      this.model.deleteBlock(target.blockIndex);
      this.callbacks.onOpsChanged();
    } else if (target.type === "slot") {
      this.model.clearSlot(target.blockIndex, target.slotName);
      this.callbacks.onOpsChanged();
    } else if (target.type === "gap") {
      this.model.deleteGap(target.position);
    }
  }

  private handleBackspace(target: SelectionTarget): void {
    if (target.type === "block") {
      this.model.deleteBlock(target.blockIndex);
      this.callbacks.onOpsChanged();
    } else if (target.type === "slot") {
      this.model.clearSlot(target.blockIndex, target.slotName);
      this.callbacks.onOpsChanged();
    } else if (target.type === "gap") {
      this.model.deleteGap(target.position);
      this.model.moveLeft();
    }
  }

  private handleCharacter(char: string, target: SelectionTarget): void {
    if (target.type === "slot") {
      this.callbacks.onFuzzySearch(char, "slot");
    } else if (target.type === "gap") {
      this.callbacks.onFuzzySearch(char, "gap");
    }
  }

  private moveSpatial(direction: "up" | "down"): void {
    const entries = this.model.getNavEntries();
    const target = this.model.getCurrentTarget();
    if (!target) return;

    const currentIdx = entries.findIndex((e) => {
      if (e.kind !== target.type) return false;
      if (e.kind === "block" && target.type === "block") return e.blockIndex === target.blockIndex;
      if (e.kind === "slot" && target.type === "slot") return e.blockIndex === target.blockIndex && e.slotName === target.slotName;
      if (e.kind === "gap" && target.type === "gap") return e.position === target.position;
      return false;
    });
    if (currentIdx < 0) return;

    const currentEl = this.getElement(target);
    if (!currentEl) return;
    const currentRect = currentEl.getBoundingClientRect();
    const cx = currentRect.x + currentRect.width / 2;
    const cy = currentRect.y + currentRect.height / 2;

    let bestIdx = currentIdx;
    let bestDist = Infinity;

    for (let i = 0; i < entries.length; i++) {
      if (i === currentIdx) continue;
      const entry = entries[i];
      const entryTarget = this.entryToTarget(entry);
      if (!entryTarget) continue;
      const el = this.getElement(entryTarget);
      if (!el) continue;
      const rect = el.getBoundingClientRect();
      const ex = rect.x + rect.width / 2;
      const ey = rect.y + rect.height / 2;

      const dy = ey - cy;
      const dx = ex - cx;

      if (direction === "up" && dy >= 0) continue;
      if (direction === "down" && dy <= 0) continue;

      const dist = Math.abs(dy) + Math.abs(dx) * 0.3; // weight vertical more
      if (dist < bestDist) {
        bestDist = dist;
        bestIdx = i;
      }
    }

    if (bestIdx !== currentIdx) {
      this.model.moveToIndex(bestIdx);
    }
  }

  private entryToTarget(entry: { kind: string; blockIndex: number; slotName?: string; position: number }): SelectionTarget | null {
    const ops = this.model.getOps();
    switch (entry.kind) {
      case "block":
        return { type: "block", blockIndex: entry.blockIndex, blockId: ops[entry.blockIndex]?.pid || "" };
      case "slot":
        return { type: "slot", blockIndex: entry.blockIndex, blockId: ops[entry.blockIndex]?.pid || "", slotName: entry.slotName || "" };
      case "gap":
        return { type: "gap", position: entry.position };
      default:
        return null;
    }
  }

  dispose(): void {
    this.container.removeEventListener("keydown", this.handleKeyDown);
  }
}
