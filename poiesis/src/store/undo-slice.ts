// ── Undo/redo slice for Poiesis store ──

import type { ElementData } from "../core/types/element";
import type { WireData } from "../core/types/wire";

// ── Types ──

export type UndoAction =
  | { type: "addElement"; element: ElementData }
  | { type: "removeElement"; id: string }
  | { type: "setProp"; id: string; prop: string; oldValue: unknown }
  | { type: "addWire"; wire: WireData }
  | { type: "removeWire"; id: string }
  | { type: "moveElement"; elementId: string; oldParentId: string | null };

export interface UndoStack {
  undo: UndoAction[];
  redo: UndoAction[];
}

/**
 * Module-level flag that skips undo recording during undo/redo operations.
 * Wrapped in a closure to prevent external mutation.
 * The try/finally pattern ensures the flag is always reset even on uncaught exceptions.
 */
function createSkipUndoGuard() {
  let __skipUndo = false;
  return {
    isSkipped: (): boolean => __skipUndo,
    /** Execute `fn` with undo recording suppressed. Resets on exception too. */
    runWithoutUndo: <T>(fn: () => T): T => {
      __skipUndo = true;
      try {
        return fn();
      } finally {
        __skipUndo = false;
      }
    },
  };
}

export const skipUndoGuard = createSkipUndoGuard();

/**
 * Build undo stack update for CRUD operations.
 * Returns {} when undo recording is skipped (during undo/redo execution).
 */
export function buildUndoUpdate(
  undoStacks: Record<string, UndoStack>,
  tabId: string | null,
  action: UndoAction,
): { undoStacks?: Record<string, UndoStack> } {
  if (skipUndoGuard.isSkipped() || !tabId) return {};
  const existingStack = undoStacks[tabId];
  return {
    undoStacks: {
      ...undoStacks,
      [tabId]: {
        undo: [...(existingStack?.undo ?? []), action],
        redo: [] as UndoAction[],
      },
    },
  };
}

// ── Action slice factory ──

import type { PoiesisState } from "./index";

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
      // Execute inverse (skip undo recording to avoid contamination)
      skipUndoGuard.runWithoutUndo(() => {
        switch (action.type) {
          case "removeElement":
            s.removeElement(action.id);
            break;
          case "addElement":
            s.addElement(action.element);
            break;
          case "setProp":
            s.setProp(action.id, action.prop, action.oldValue);
            break;
          case "addWire":
            s.addWire(action.wire);
            break;
          case "removeWire":
            s.removeWire(action.id);
            break;
          case "moveElement":
            set((st) => {
              const element = st.elements.find(e => e.id === action.elementId);
              if (!element) return st;
              const currentParent = element.parentId;
              let elements = st.elements.map(e =>
                e.id === action.elementId ? { ...e, parentId: action.oldParentId } : e
              );
              if (currentParent) {
                elements = elements.map(e =>
                  e.id === currentParent ? { ...e, children: e.children.filter(c => c !== action.elementId) } : e
                );
              }
              let rootOrder = st.rootOrder.filter(rid => rid !== action.elementId);
              if (action.oldParentId) {
                elements = elements.map(e =>
                  e.id === action.oldParentId ? { ...e, children: [...e.children, action.elementId] } : e
                );
              } else {
                rootOrder = [...rootOrder, action.elementId];
              }
              return { elements, rootOrder };
            });
            break;
        }
      });
      // Update stacks (after the inverse execution modified the store)
      const s2 = get();
      set({
        undoStacks: {
          ...s2.undoStacks,
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
      // Re-execute the action (skip undo recording)
      skipUndoGuard.runWithoutUndo(() => {
        switch (action.type) {
          case "removeElement":
            s.removeElement(action.id);
            break;
          case "addElement":
            s.addElement(action.element);
            break;
          case "setProp":
            s.setProp(action.id, action.prop, action.oldValue);
            break;
          case "addWire":
            s.addWire(action.wire);
            break;
          case "removeWire":
            s.removeWire(action.id);
            break;
          case "moveElement":
            set((st) => {
              const element = st.elements.find(e => e.id === action.elementId);
              if (!element) return st;
              const currentParent = element.parentId;
              let elements = st.elements.map(e =>
                e.id === action.elementId ? { ...e, parentId: action.oldParentId } : e
              );
              if (currentParent) {
                elements = elements.map(e =>
                  e.id === currentParent ? { ...e, children: e.children.filter(c => c !== action.elementId) } : e
                );
              }
              let rootOrder = st.rootOrder.filter(rid => rid !== action.elementId);
              if (action.oldParentId) {
                elements = elements.map(e =>
                  e.id === action.oldParentId ? { ...e, children: [...e.children, action.elementId] } : e
                );
              } else {
                rootOrder = [...rootOrder, action.elementId];
              }
              return { elements, rootOrder };
            });
            break;
        }
      });
      // Update stacks
      const s2 = get();
      set({
        undoStacks: {
          ...s2.undoStacks,
          [tabId]: {
            undo: [...stack.undo, action],
            redo: stack.redo.slice(0, -1),
          },
        },
      });
    },
  };
}
