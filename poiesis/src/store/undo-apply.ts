// ── Pure function that computes the inverse state partial for an undo/redo action ──
// Avoids calling set() or CRUD operations directly, enabling atomic single-set() undo/redo.

import type { PoiesisState } from "./types";
import type { UndoAction } from "./undo-slice";
import type { ElementData } from "../core/types/element";

/**
 * Compute the state changes needed to apply (or reverse) an undo/redo action.
 *
 * @param state  Current store state (snapshot before applying).
 * @param action The undo/redo action to apply.
 * @param isRedo Whether this is a redo (vs undo). Only matters for actions where
 *               the undo and redo directions differ (reorderElement, setElementsAndWires).
 * @returns A Partial<PoiesisState> to merge via set(), or null if the action cannot be applied.
 */
export function computeInverse(
  state: PoiesisState,
  action: UndoAction,
  isRedo: boolean = false,
): Partial<PoiesisState> | null {
  switch (action.type) {
    case "removeElement": {
      // Remove the element (undoing an addElement)
      const removedIds = new Set<string>();
      function collectDescendants(eid: string): void {
        removedIds.add(eid);
        for (const c of state.elements) {
          if (c.parentId === eid) collectDescendants(c.id);
        }
      }
      const el = state.elements.find((e) => e.id === action.id);
      if (el) collectDescendants(el.id);

      let elements = state.elements.filter((e) => !removedIds.has(e.id));
      if (el?.parentId) {
        elements = elements.map((e) =>
          e.id === el.parentId
            ? "children" in e
              ? { ...e, children: e.children.filter((cid) => !removedIds.has(cid)) }
              : e
            : e,
        );
      }
      return {
        elements,
        rootOrder: state.rootOrder.filter((rid) => !removedIds.has(rid)),
        wires: state.wires.filter((w) => !removedIds.has(w.fromId) && !removedIds.has(w.toId)),
        isDirty: true,
      };
    }

    case "addElement": {
      // Add the element back (undoing a removeElement)
      const normalized = { ...action.element, props: "props" in action.element ? action.element.props || {} : {} };
      const elements = [...state.elements, normalized];
      if (normalized.parentId) {
        return {
          elements: elements.map((e) =>
            e.id === normalized.parentId
              ? "children" in e
                ? { ...e, children: [...e.children, normalized.id] }
                : e
              : e,
          ),
          rootOrder: state.rootOrder,
          isDirty: true,
        };
      }
      return {
        elements,
        rootOrder: [...state.rootOrder, normalized.id],
        isDirty: true,
      };
    }

    case "setProp": {
      return {
        elements: state.elements.map((e) =>
          e.id === action.id
            ? "props" in e
              ? { ...e, props: { ...e.props, [action.prop]: action.oldValue } }
              : e
            : e,
        ),
        isDirty: true,
      };
    }

    case "addWire": {
      return { wires: [...state.wires, action.wire], isDirty: true };
    }

    case "removeWire": {
      return { wires: state.wires.filter((w) => w.id !== action.id), isDirty: true };
    }

    case "moveElement": {
      // Move element back to oldParentId (same logic for undo and redo)
      const element = state.elements.find((e) => e.id === action.elementId);
      if (!element) return null;
      const currentParent = element.parentId;
      let elements = state.elements.map((e) =>
        e.id === action.elementId ? { ...e, parentId: action.oldParentId } : e,
      );
      if (currentParent) {
        elements = elements.map((e) =>
          e.id === currentParent
            ? "children" in e
              ? { ...e, children: e.children.filter((c) => c !== action.elementId) }
              : e
            : e,
        );
      }
      let rootOrder = state.rootOrder.filter((rid) => rid !== action.elementId);
      if (action.oldParentId) {
        elements = elements.map((e) =>
          e.id === action.oldParentId
            ? "children" in e
              ? { ...e, children: [...e.children, action.elementId] }
              : e
            : e,
        );
      } else {
        rootOrder = [...rootOrder, action.elementId];
      }
      return { elements, rootOrder, isDirty: true };
    }

    case "reorderElement": {
      if (isRedo) {
        // Redo: move to targetSibling (re-apply original reorder)
        const parentEl = action.parentId ? state.elements.find((e) => e.id === action.parentId) : undefined;
        const newSiblings = parentEl && "children" in parentEl
          ? [...parentEl.children]
          : action.parentId ? []
            : [...state.rootOrder];
        const currentIdx = newSiblings.indexOf(action.elementId);
        if (currentIdx !== -1) newSiblings.splice(currentIdx, 1);
        const targetIdx = newSiblings.indexOf(action.targetSiblingId);
        if (targetIdx !== -1) {
          const insertAt = action.position === "before" ? targetIdx : targetIdx + 1;
          newSiblings.splice(insertAt, 0, action.elementId);
        } else {
          newSiblings.push(action.elementId);
        }
        if (action.parentId) {
          return {
            elements: state.elements.map((e) =>
              e.id === action.parentId ? { ...e, children: newSiblings } as ElementData : e,
            ),
            isDirty: true,
          };
        }
        return { rootOrder: newSiblings, isDirty: true };
      } else {
        // Undo: move back to prevSibling
        const parentEl = action.parentId ? state.elements.find((e) => e.id === action.parentId) : undefined;
        const newSiblings = parentEl && "children" in parentEl
          ? [...parentEl.children]
          : action.parentId ? []
            : [...state.rootOrder];
        const currentIdx = newSiblings.indexOf(action.elementId);
        if (currentIdx !== -1) newSiblings.splice(currentIdx, 1);
        if (action.prevSiblingId !== null) {
          const prevIdx = newSiblings.indexOf(action.prevSiblingId);
          newSiblings.splice(prevIdx + 1, 0, action.elementId);
        } else {
          newSiblings.unshift(action.elementId);
        }
        if (action.parentId) {
          return {
            elements: state.elements.map((e) =>
              e.id === action.parentId ? { ...e, children: newSiblings } as ElementData : e,
            ),
            isDirty: true,
          };
        }
        return { rootOrder: newSiblings, isDirty: true };
      }
    }

    case "setElementsAndWires": {
      if (isRedo) {
        return {
          elements: action.newElements,
          wires: action.newWires,
          rootOrder: action.newElements.reduce<string[]>((acc, e) => {
            if (!e.parentId) acc.push(e.id);
            return acc;
          }, []),
          selectedElementIds: [],
          isDirty: true,
        };
      }
      return {
        elements: action.prevElements,
        wires: action.prevWires,
        rootOrder: action.prevElements.reduce<string[]>((acc, e) => {
          if (!e.parentId) acc.push(e.id);
          return acc;
        }, []),
        selectedElementIds: [],
        isDirty: true,
      };
    }

    case "restoreConditionalBlock": {
      return {
        elements: state.elements.map((e) =>
          e.id === action.blockId && e.nodeType === "conditional"
            ? { ...e, branches: action.prevBranches, children: action.prevChildren }
            : e
        ),
        isDirty: true,
      };
    }

    default:
      return null;
  }
}
