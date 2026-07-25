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
        this.selectionMgr.moveLeft();
        return true;
      case "ArrowRight":
        e.preventDefault();
        this.selectionMgr.moveRight();
        return true;
      case "ArrowUp":
        e.preventDefault();
        this.selectionMgr.moveUp();
        return true;
      case "ArrowDown":
        e.preventDefault();
        this.selectionMgr.moveDown();
        return true;
    }
    return false;
  }

  // ── Modifier key dispatch (Ctrl/Cmd + C/V/X) ──

  private handleModifier(e: KeyboardEvent): void {
    if (e.key === "c" || e.key === "C") {
      e.preventDefault();
      Blockly.clipboard.copy(this.workspace);
    } else if (e.key === "v" || e.key === "V") {
      e.preventDefault();
      Blockly.clipboard.paste(this.workspace);
      this.callbacks.onOpsChanged();
    } else if (e.key === "x" || e.key === "X") {
      e.preventDefault();
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

  // ── Context-sensitive handlers ──

  private handleEnter(target: SelectionTarget, e: KeyboardEvent): void {
    e.preventDefault();
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
    if (target.type === "block") {
      const block = this.workspace.getBlockById(target.blockId);
      if (block?.isDeletable()) {
        block.dispose(true);
        this.callbacks.onOpsChanged();
      }
    } else if (target.type === "gap") {
      this.gapMgr.deleteGap(target.position);
    }
    // On slot: no-op (clearing is handled via Backspace in text slot, or via fuzzy search)
  }

  private handleBackspace(target: SelectionTarget, e: KeyboardEvent): void {
    e.preventDefault();
    if (target.type === "block") {
      const block = this.workspace.getBlockById(target.blockId);
      if (block?.isDeletable()) {
        block.dispose(true);
        this.callbacks.onOpsChanged();
      }
    }
    // On slot/gap: no special behavior — leave to parent
  }

  private handleCharacter(e: KeyboardEvent, target: SelectionTarget): void {
    // Only handle single printable characters
    if (e.key.length !== 1) return;

    if (target.type === "slot") {
      e.preventDefault();
      this.callbacks.onFuzzySearch(e.key, "slot");
    } else if (target.type === "gap") {
      e.preventDefault();
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
