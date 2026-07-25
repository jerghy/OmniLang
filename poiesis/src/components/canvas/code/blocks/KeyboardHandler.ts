// poiesis/src/components/canvas/code/blocks/KeyboardHandler.ts
// Task 7: Global keyboard input interception and dispatch for the Block Editor.
//
// KeyboardHandler attaches to the Blockly workspace SVG and intercepts ALL
// keyboard events. Based on SelectionManager's current cursor position, it
// dispatches to navigation (arrow keys), fuzzy search (character keys on
// slot/gap), deletion (Delete/Backspace), raw input (\), and clipboard (Ctrl+C/V/X).

import * as Blockly from "blockly";
import type { SelectionManager, SelectionTarget } from "./SelectionManager";
import type { GapManager } from "./GapManager";

// ── Public types ──

export interface KeyboardHandlerCallbacks {
  /** Called when user types a character while cursor is on a slot or gap. */
  onFuzzySearch: (query: string, context: "slot" | "gap") => void;
  /** Called when user presses \ */
  onRawInput: () => void;
  /** Called after any operation that changes blocks */
  onOpsChanged: () => void;
}

// ── KeyboardHandler ──

export class KeyboardHandler {
  private workspace: Blockly.WorkspaceSvg;
  private selectionMgr: SelectionManager;
  private gapMgr: GapManager;
  private callbacks: KeyboardHandlerCallbacks;
  private handleKeyDown: (e: KeyboardEvent) => void;

  constructor(
    ws: Blockly.WorkspaceSvg,
    selectionMgr: SelectionManager,
    gapMgr: GapManager,
    callbacks: KeyboardHandlerCallbacks,
  ) {
    this.workspace = ws;
    this.selectionMgr = selectionMgr;
    this.gapMgr = gapMgr;
    this.callbacks = callbacks;
    this.handleKeyDown = this.onKeyDown.bind(this);
    this.attach();
  }

  // ── Event binding ──

  private attach(): void {
    const svg = this.workspace.getParentSvg();
    if (svg) {
      svg.addEventListener("keydown", this.handleKeyDown);
      // Make SVG focusable
      svg.setAttribute("tabindex", "0");
      svg.focus();
    }

    // Suppress Blockly's default keyboard handling to prevent interference.
    // Blockly v13 disables keyboard nav via keyboardAccessibilityOptions;
    // silently ignore if this API doesn't exist.
    try {
      const ws = this.workspace as unknown as Record<string, unknown>;
      if (ws.keyboardAccessibilityOptions !== undefined) {
        (ws.keyboardAccessibilityOptions as Record<string, boolean>).enabled =
          false;
      }
    } catch {
      // Fallback: stopPropagation in each handler prevents event from reaching
      // document-level Blockly listeners.
    }
  }

  // ── Main keydown handler ──

  private onKeyDown(e: KeyboardEvent): void {
    const target = this.selectionMgr.getCurrentTarget();
    if (!target) return;

    // ── Modifier keys (Ctrl/Cmd) ──
    if (e.ctrlKey || e.metaKey) {
      this.handleModifier(e);
      return;
    }

    // ── Navigation keys ──
    if (this.handleNavigation(e)) return;

    // ── Context-sensitive keys ──
    switch (e.key) {
      case "Enter":
        this.handleEnter(target, e);
        break;
      case "Delete":
        this.handleDelete(target, e);
        break;
      case "Backspace":
        this.handleBackspace(target, e);
        break;
      case "\\":
        e.preventDefault();
        e.stopPropagation();
        this.callbacks.onRawInput();
        break;
      case "Escape":
        // Close any open popup — handled by parent component
        break;
      default:
        // Character key
        this.handleCharacter(e, target);
        break;
    }
  }

  // ── Navigation dispatch ──

  private handleNavigation(e: KeyboardEvent): boolean {
    switch (e.key) {
      case "ArrowLeft":
        e.preventDefault();
        e.stopPropagation();
        this.selectionMgr.moveLeft();
        return true;
      case "ArrowRight":
        e.preventDefault();
        e.stopPropagation();
        this.selectionMgr.moveRight();
        return true;
      case "ArrowUp":
        e.preventDefault();
        e.stopPropagation();
        this.selectionMgr.moveUp();
        return true;
      case "ArrowDown":
        e.preventDefault();
        e.stopPropagation();
        this.selectionMgr.moveDown();
        return true;
    }
    return false;
  }

  // ── Modifier key dispatch (Ctrl/Cmd + C/V/X) ──

  private handleModifier(e: KeyboardEvent): void {
    if (e.key === "c" || e.key === "C") {
      e.preventDefault();
      e.stopPropagation();
      Blockly.clipboard.copy(this.workspace);
    } else if (e.key === "v" || e.key === "V") {
      e.preventDefault();
      e.stopPropagation();
      Blockly.clipboard.paste(this.workspace);
      this.callbacks.onOpsChanged();
    } else if (e.key === "x" || e.key === "X") {
      e.preventDefault();
      e.stopPropagation();
      Blockly.clipboard.copy(this.workspace);
      const target = this.selectionMgr.getCurrentTarget();
      if (target?.type === "block") {
        const block = this.workspace.getBlockById(target.blockId);
        if (block?.isDeletable()) {
          block.dispose(true);
          this.callbacks.onOpsChanged();
        }
      }
    }
  }

  /**
   * Clear a slot input by disposing its connected child block (if any).
   * Called when Delete/Backspace is pressed on a slot target.
   */
  private clearSlotInput(blockId: string, inputName: string): void {
    const block = this.workspace.getBlockById(blockId);
    if (!block) return;
    const input = block.getInput(inputName);
    if (!input?.connection) return;
    const childBlock = input.connection.targetBlock();
    if (childBlock && childBlock.isDeletable()) {
      childBlock.dispose(true);
    }
  }

  // ── Context-sensitive handlers ──

  private handleEnter(target: SelectionTarget, e: KeyboardEvent): void {
    e.preventDefault();
    e.stopPropagation();
    if (target.type === "slot") {
      // Open fuzzy search with no query (show all options)
      this.callbacks.onFuzzySearch("", "slot");
    } else if (target.type === "gap") {
      // Insert a gap row before opening search
      this.gapMgr.insertGap(target.position);
      this.callbacks.onFuzzySearch("", "gap");
    }
    // On block: no-op
  }

  private handleDelete(target: SelectionTarget, e: KeyboardEvent): void {
    e.preventDefault();
    e.stopPropagation();
    if (target.type === "block") {
      const block = this.workspace.getBlockById(target.blockId);
      if (block?.isDeletable()) {
        block.dispose(true);
        this.callbacks.onOpsChanged();
      }
    } else if (target.type === "slot") {
      this.clearSlotInput(target.blockId, target.inputName);
      this.callbacks.onOpsChanged();
    } else if (target.type === "gap") {
      this.gapMgr.deleteGap(target.position);
    }
  }

  private handleBackspace(target: SelectionTarget, e: KeyboardEvent): void {
    e.preventDefault();
    e.stopPropagation();
    if (target.type === "block") {
      const block = this.workspace.getBlockById(target.blockId);
      if (block?.isDeletable()) {
        block.dispose(true);
        this.callbacks.onOpsChanged();
      }
    } else if (target.type === "slot") {
      this.clearSlotInput(target.blockId, target.inputName);
      this.callbacks.onOpsChanged();
    } else if (target.type === "gap") {
      // Merge gap up: delete the gap and move cursor to the previous position
      this.gapMgr.deleteGap(target.position);
      this.selectionMgr.moveLeft();
    }
  }

  private handleCharacter(e: KeyboardEvent, target: SelectionTarget): void {
    // Only handle single printable characters
    if (e.key.length !== 1) return;

    if (target.type === "slot") {
      e.preventDefault();
      e.stopPropagation();
      this.callbacks.onFuzzySearch(e.key, "slot");
    } else if (target.type === "gap") {
      e.preventDefault();
      e.stopPropagation();
      this.callbacks.onFuzzySearch(e.key, "gap");
    }
    // On block: let it pass (no-op in Blockly since blocks aren't editable)
  }

  // ── Cleanup ──

  dispose(): void {
    const svg = this.workspace.getParentSvg();
    if (svg) {
      svg.removeEventListener("keydown", this.handleKeyDown);
    }
  }
}
