// ── Undo/redo slice for Poiesis store ──

import type { ElementData } from "../core/types/element";
import type { WireData } from "../core/types/wire";
import { computeInverse } from "./undo-apply";

const MAX_UNDO_SIZE = 100;

// ── Types ──

export type UndoAction =
  | { type: "addElement"; element: ElementData }
  | { type: "removeElement"; id: string }
  | { type: "setProp"; id: string; prop: string; oldValue: unknown }
  | { type: "addWire"; wire: WireData }
  | { type: "removeWire"; id: string }
  | { type: "moveElement"; elementId: string; oldParentId: string | null }
  | { type: "reorderElement"; elementId: string; parentId: string | null; prevSiblingId: string | null; targetSiblingId: string; position: "before" | "after" }
  | { type: "setElementsAndWires"; prevElements: ElementData[]; prevWires: WireData[]; newElements: ElementData[]; newWires: WireData[] }
  | { type: "setElementBinding"; id: string; propName: string; oldBinding: string | null }
  | { type: "restoreConditionalBlock"; blockId: string; prevBranches: Array<{type: string; condition: string; children: string[]}>; prevChildren: string[] };

export interface UndoStack {
  undo: UndoAction[];
  redo: UndoAction[];
}

/**
 * Build undo stack update for CRUD operations.
 * No longer checks skipUndoGuard — undo/redo now applies state directly
 * via computeInverse without calling through CRUD operations.
 */
export function buildUndoUpdate(
  undoStacks: Record<string, UndoStack>,
  tabId: string | null,
  action: UndoAction,
): { undoStacks?: Record<string, UndoStack> } {
  if (!tabId) return {};
  const existingStack = undoStacks[tabId];
  const nextUndo = [...(existingStack?.undo ?? []), action];
  // Keep undo stack bounded
  if (nextUndo.length > MAX_UNDO_SIZE) {
    nextUndo.splice(0, nextUndo.length - MAX_UNDO_SIZE);
  }
  return {
    undoStacks: {
      ...undoStacks,
      [tabId]: {
        undo: nextUndo,
        redo: [] as UndoAction[],
      },
    },
  };
}

// ── Action slice factory ──

import type { PoiesisState } from "./types";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createUndoSlice(set: SetFn, get: GetFn) {
  return {
    undo: () => {
      const s = get();
      const tabId = s.designPage.activeTabId;
      if (!tabId) return;
      const stack = s.undoStacks[tabId];
      if (!stack || stack.undo.length === 0) return;
      const action = stack.undo[stack.undo.length - 1];

      // Compute inverse state change and apply it atomically with stack update
      const inverse = computeInverse(s, action, false);
      if (!inverse) return;

      set({
        ...inverse,
        undoStacks: {
          ...s.undoStacks,
          [tabId]: {
            undo: stack.undo.slice(0, -1),
            redo: [...stack.redo, action],
          },
        },
      });
    },

    redo: () => {
      const s = get();
      const tabId = s.designPage.activeTabId;
      if (!tabId) return;
      const stack = s.undoStacks[tabId];
      if (!stack || stack.redo.length === 0) return;
      const action = stack.redo[stack.redo.length - 1];

      // Compute redo state change and apply atomically with stack update
      const inverse = computeInverse(s, action, true);
      if (!inverse) return;

      set({
        ...inverse,
        undoStacks: {
          ...s.undoStacks,
          [tabId]: {
            undo: [...stack.undo, action],
            redo: stack.redo.slice(0, -1),
          },
        },
      });
    },
  };
}
