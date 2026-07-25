// poiesis/src/components/canvas/code/blocks/SelectionManager.ts
// Task 5: Game-controller-style cursor navigation system for the Block Editor.
//
// SelectionManager builds a spatial grid model on top of Blockly,
// tracks cursor position, and renders a visual selection box overlay.

import * as Blockly from "blockly";

// ── Public types ──

export type SelectionTarget =
  | { type: "block"; blockId: string }
  | { type: "slot"; blockId: string; inputName: string }
  | { type: "gap"; parentBlockId: string | null; position: number };

// ── Internal grid entry ──

interface GridEntry {
  blockId: string | null;
  isGap: boolean;
  position: number;
}

/** Tolerance for gap position comparison. */
const GAP_EPSILON = 0.0001;

/**
 * SelectionManager — single source of truth for where the "game controller cursor" is.
 *
 * Responsibilities:
 * 1. Compute a flat spatial grid of all selectable positions (blocks, gaps between blocks).
 * 2. Expose move(direction) methods that update position.
 * 3. Render a <div> overlay positioned at the current target's bounding rect.
 * 4. Fire an onChange callback when position changes.
 */
export class SelectionManager {
  private workspace: Blockly.WorkspaceSvg;
  private grid: GridEntry[] = [];
  private currentIndex = 0;
  private cursorEl: HTMLDivElement | null = null;
  private styleEl: HTMLStyleElement | null = null;
  private changeListenerId: string | undefined;

  /** Callback fired when cursor position changes. */
  onChange: ((target: SelectionTarget | null) => void) | null = null;

  constructor(workspace: Blockly.WorkspaceSvg) {
    this.workspace = workspace;
    this.refresh();

    // Auto-refresh grid when workspace changes
    this.changeListenerId = workspace.addChangeListener(() => {
      this.refresh();
    });
  }

  // ── Public API ──

  /** Return the current cursor target, or null when nothing is selectable. */
  getCurrentTarget(): SelectionTarget | null {
    if (this.grid.length === 0) return null;
    const entry = this.grid[this.currentIndex];
    return this.entryToTarget(entry);
  }

  /** Move cursor left (previous grid entry). */
  moveLeft(): void {
    if (this.grid.length === 0) return;
    this.currentIndex = Math.max(0, this.currentIndex - 1);
    this.afterMove();
  }

  /** Move cursor right (next grid entry). */
  moveRight(): void {
    if (this.grid.length === 0) return;
    this.currentIndex = Math.min(this.grid.length - 1, this.currentIndex + 1);
    this.afterMove();
  }

  /** Move cursor up (previous block, skipping gap entries). */
  moveUp(): void {
    if (this.grid.length === 0) return;
    let idx = this.currentIndex - 1;
    while (idx >= 0 && this.grid[idx].isGap) {
      idx--;
    }
    if (idx >= 0) {
      this.currentIndex = idx;
    }
    this.afterMove();
  }

  /** Move cursor down (next block, skipping gap entries). */
  moveDown(): void {
    if (this.grid.length === 0) return;
    let idx = this.currentIndex + 1;
    while (idx < this.grid.length && this.grid[idx].isGap) {
      idx++;
    }
    if (idx < this.grid.length) {
      this.currentIndex = idx;
    }
    this.afterMove();
  }

  /** Jump directly to a specific target. */
  moveTo(target: SelectionTarget): void {
    for (let i = 0; i < this.grid.length; i++) {
      if (this.targetsEqual(this.entryToTarget(this.grid[i]), target)) {
        this.currentIndex = i;
        this.afterMove();
        return;
      }
    }
  }

  /**
   * Enter slot navigation mode (API for KeyboardHandler, Task 7).
   * Moves cursor into a slot of the current block.
   */
  enterSlot(_inputName: string): void {
    // Stub — implemented in Task 7 KeyboardHandler
  }

  /** Exit slot navigation mode back to block level (API for KeyboardHandler, Task 7). */
  exitSlot(): void {
    // Stub — implemented in Task 7 KeyboardHandler
  }

  /** Rebuild the spatial grid from current workspace blocks. */
  refresh(): void {
    const topBlocks = this.workspace.getTopBlocks(true);
    const newGrid: GridEntry[] = [];

    for (let i = 0; i < topBlocks.length; i++) {
      // Block entry
      newGrid.push({
        blockId: topBlocks[i].id,
        isGap: false,
        position: i,
      });

      // Gap entry between blocks (not after the last one)
      if (i < topBlocks.length - 1) {
        newGrid.push({
          blockId: null,
          isGap: true,
          position: i + 0.5,
        });
      }
    }

    this.grid = newGrid;

    // Clamp index when grid shrinks
    if (this.currentIndex >= this.grid.length) {
      this.currentIndex = Math.max(0, this.grid.length - 1);
    }

    this.renderCursor();
  }

  /** Cleanup: remove DOM elements, unsubscribe listener, reset callback. */
  dispose(): void {
    if (this.changeListenerId !== undefined) {
      this.workspace.removeChangeListener(this.changeListenerId);
      this.changeListenerId = undefined;
    }
    if (this.cursorEl?.parentElement) {
      this.cursorEl.parentElement.removeChild(this.cursorEl);
    }
    if (this.styleEl?.parentElement) {
      this.styleEl.parentElement.removeChild(this.styleEl);
    }
    this.cursorEl = null;
    this.styleEl = null;
    this.onChange = null;
  }

  // ── Bounding rect (used by renderCursor; exposed for testing) ──

  getTargetBoundingRect(target: SelectionTarget): DOMRect | null {
    try {
      switch (target.type) {
        case "block": {
          const block = this.workspace.getBlockById(target.blockId);
          if (!block) return null;
          const svgRoot = block.getSvgRoot();
          if (!svgRoot) return null;
          return svgRoot.getBoundingClientRect();
        }
        case "slot": {
          const block = this.workspace.getBlockById(target.blockId);
          if (!block) return null;
          const blockRect = this.getTargetBoundingRect({
            type: "block",
            blockId: target.blockId,
          });
          if (!blockRect) return null;
          // Estimate slot rect from block's rect
          return new DOMRect(
            blockRect.x + 10,
            blockRect.y + 10,
            blockRect.width - 20,
            28,
          );
        }
        case "gap": {
          const topBlocks = this.workspace.getTopBlocks(true);
          const afterIndex = Math.floor(target.position);
          const afterBlock = topBlocks[afterIndex];
          const beforeIndex = Math.ceil(target.position);
          const beforeBlock = topBlocks[beforeIndex];

          if (afterBlock && beforeBlock) {
            const afterRect = this.getTargetBoundingRect({
              type: "block",
              blockId: afterBlock.id,
            });
            const beforeRect = this.getTargetBoundingRect({
              type: "block",
              blockId: beforeBlock.id,
            });
            if (afterRect && beforeRect) {
              const gapY =
                afterRect.bottom +
                (beforeRect.top - afterRect.bottom) / 2 -
                1;
              const maxW = Math.max(afterRect.width, beforeRect.width);
              return new DOMRect(afterRect.x, gapY, maxW, 4);
            }
          }
          return null;
        }
      }
    } catch (error) {
      console.warn("SelectionManager.getTargetBoundingRect:", error);
      return null;
    }
  }

  // ── Private helpers ──

  private entryToTarget(entry: GridEntry): SelectionTarget {
    if (entry.isGap) {
      return { type: "gap", parentBlockId: null, position: entry.position };
    }
    if (entry.blockId === null) {
      // Should not happen for non-gap entries, but guard defensively
      return { type: "gap", parentBlockId: null, position: entry.position };
    }
    return { type: "block", blockId: entry.blockId };
  }

  private targetsEqual(a: SelectionTarget, b: SelectionTarget): boolean {
    if (a.type === "block" && b.type === "block") {
      return a.blockId === b.blockId;
    }
    if (a.type === "slot" && b.type === "slot") {
      return a.blockId === b.blockId && a.inputName === b.inputName;
    }
    if (a.type === "gap" && b.type === "gap") {
      return Math.abs(a.position - b.position) < GAP_EPSILON;
    }
    return false;
  }

  private afterMove(): void {
    this.renderCursor();
    this.fireOnChange();
  }

  private renderCursor(): void {
    const target = this.getCurrentTarget();
    if (!target) {
      if (this.cursorEl) this.cursorEl.style.display = "none";
      return;
    }

    // Lazily create the cursor overlay element
    if (!this.cursorEl) {
      this.cursorEl = document.createElement("div");
      this.cursorEl.style.position = "absolute";
      this.cursorEl.style.pointerEvents = "none";
      this.cursorEl.style.zIndex = "1000";
      this.cursorEl.style.transition = "all 150ms ease";

      const parent = this.getSvgParent();
      if (parent) {
        parent.appendChild(this.cursorEl);
      }
    }

    // Try to position the cursor — gracefully fall back if bounds unavailable
    const rect = this.getTargetBoundingRect(target);
    if (rect && rect.width > 0 && rect.height > 0) {
      this.cursorEl.style.display = "";
      this.cursorEl.style.left = `${rect.x}px`;
      this.cursorEl.style.top = `${rect.y}px`;
      this.cursorEl.style.width = `${rect.width}px`;
      this.cursorEl.style.height = `${rect.height}px`;
    } else {
      // Show a small fallback indicator when coordinates aren't available
      this.cursorEl.style.display = "";
      this.cursorEl.style.left = "0px";
      this.cursorEl.style.top = "0px";
      this.cursorEl.style.width = "10px";
      this.cursorEl.style.height = "10px";
    }

    // ═══ Visual style per target type ═══
    const terracotta = "#cc6b4a";

    switch (target.type) {
      case "block":
        // Solid outline offset from block
        this.cursorEl.style.outline = `2px solid ${terracotta}`;
        this.cursorEl.style.outlineOffset = "2px";
        this.cursorEl.style.borderRadius = "4px";
        this.cursorEl.style.border = "none";
        this.cursorEl.style.animation = "none";
        break;
      case "slot":
        // Dashed border
        this.cursorEl.style.outline = "none";
        this.cursorEl.style.border = `2px dashed ${terracotta}`;
        this.cursorEl.style.borderRadius = "4px";
        this.cursorEl.style.animation = "none";
        break;
      case "gap":
        // Horizontal blinking line
        this.cursorEl.style.outline = "none";
        this.cursorEl.style.border = "none";
        this.cursorEl.style.borderTop = `2px solid ${terracotta}`;
        this.addBlinkKeyframes();
        this.cursorEl.style.animation =
          "selectionManagerBlink 1s step-end infinite";
        break;
    }
  }

  private addBlinkKeyframes(): void {
    if (this.styleEl) return;
    this.styleEl = document.createElement("style");
    this.styleEl.textContent = `@keyframes selectionManagerBlink { 50% { opacity: 0; } }`;
    document.head.appendChild(this.styleEl);
  }

  private getSvgParent(): HTMLElement | null {
    try {
      const svg = this.workspace.getParentSvg();
      return svg?.parentElement || null;
    } catch (error) {
      console.warn("SelectionManager.getSvgParent:", error);
      return null;
    }
  }

  private fireOnChange(): void {
    this.onChange?.(this.getCurrentTarget());
  }
}
