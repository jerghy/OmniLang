// ── Pure function that computes the inverse state partial for an undo/redo action ──
// Avoids calling set() or CRUD operations directly, enabling atomic single-set() undo/redo.

import type { PoiesisState } from "./types";
import type { UndoAction } from "./undo-slice";
import type { ElementData } from "../core/types/element";

/**
 * Compute the state changes needed to apply (or reverse) an undo/redo action.
 *
 * 动作语义：undo 栈记录**正向动作**（原操作），本函数按方向应用：
 * - undo（isRedo=false）：撤销正向动作（addElement → 移除；removeElement → 恢复…）
 * - redo（isRedo=true）：重放正向动作（addElement → 添加；removeElement → 移除…）
 *
 * 此前 removeElement/addElement、addWire/removeWire 等只存逆动作/单方向数据，
 * redo 方向无数据可用且方向与 undo 相同 → redo 按钮对添加/删除/连线/setProp
 * 全部失效（E2E 缺陷 1）。本实现为方向性动作补齐双方向数据（undo-slice.ts 类型）。
 *
 * @param state  Current store state (snapshot before applying).
 * @param action The undo/redo action to apply (forward action).
 * @param isRedo Whether this is a redo (vs undo).
 * @returns A Partial<PoiesisState> to merge via set(), or null if the action cannot be applied.
 */
export function computeInverse(
  state: PoiesisState,
  action: UndoAction,
  isRedo: boolean = false,
): Partial<PoiesisState> | null {
  switch (action.type) {
    case "addElement": {
      if (isRedo) {
        // Redo：重放添加（元素已在 state 中时幂等跳过）
        if (state.elements.some((e) => e.id === action.element.id)) return null;
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
      // Undo：移除元素 + 后代 + 相连 wire
      const addRemovedIds = new Set<string>();
      function addCollectDescendants(eid: string): void {
        addRemovedIds.add(eid);
        for (const c of state.elements) {
          if (c.parentId === eid) addCollectDescendants(c.id);
        }
      }
      const addEl = state.elements.find((e) => e.id === action.element.id);
      if (addEl) addCollectDescendants(addEl.id);
      let addElements = state.elements.filter((e) => !addRemovedIds.has(e.id));
      if (addEl?.parentId) {
        addElements = addElements.map((e) =>
          e.id === addEl.parentId
            ? "children" in e
              ? { ...e, children: e.children.filter((cid) => !addRemovedIds.has(cid)) }
              : e
            : e,
        );
      }
      return {
        elements: addElements,
        rootOrder: state.rootOrder.filter((rid) => !addRemovedIds.has(rid)),
        wires: state.wires.filter((w) => !addRemovedIds.has(w.fromId) && !addRemovedIds.has(w.toId)),
        isDirty: true,
      };
    }

    case "removeElement": {
      if (isRedo) {
        // Redo：重放删除（移除元素 + 后代 + 相连 wire）
        const removedIds = new Set<string>();
        function collectDescendants(eid: string): void {
          removedIds.add(eid);
          for (const c of state.elements) {
            if (c.parentId === eid) collectDescendants(c.id);
          }
        }
        const el = state.elements.find((e) => e.id === action.id);
        if (el) collectDescendants(el.id);
        if (removedIds.size === 0) return null;

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
      // Undo：恢复被删元素 + 被连带删除的 wire（removeElement 会清理相连 wire）
      if (state.elements.some((e) => e.id === action.element.id)) return null;
      const normalized = { ...action.element, props: "props" in action.element ? action.element.props || {} : {} };
      const elements = [...state.elements, normalized];
      const restoredWires = action.wires.filter((w) => !state.wires.some((sw) => sw.id === w.id));
      if (normalized.parentId) {
        return {
          elements: elements.map((e) =>
            e.id === normalized.parentId
              ? "children" in e
                ? { ...e, children: [...e.children, normalized.id] }
                : e
              : e,
          ),
          wires: [...state.wires, ...restoredWires],
          rootOrder: state.rootOrder,
          isDirty: true,
        };
      }
      return {
        elements,
        wires: [...state.wires, ...restoredWires],
        rootOrder: [...state.rootOrder, normalized.id],
        isDirty: true,
      };
    }

    case "setProp": {
      return {
        elements: state.elements.map((e) =>
          e.id === action.id
            ? "props" in e
              ? { ...e, props: { ...e.props, [action.prop]: isRedo ? action.newValue : action.oldValue } }
              : e
            : e,
        ),
        isDirty: true,
      };
    }

    case "addWire": {
      if (isRedo) {
        // Redo：重放添加
        if (state.wires.some((w) => w.id === action.wire.id)) return null;
        return { wires: [...state.wires, action.wire], isDirty: true };
      }
      // Undo：移除
      return { wires: state.wires.filter((w) => w.id !== action.wire.id), isDirty: true };
    }

    case "removeWire": {
      if (isRedo) {
        // Redo：重放删除
        if (!state.wires.some((w) => w.id === action.id)) return null;
        return { wires: state.wires.filter((w) => w.id !== action.id), isDirty: true };
      }
      // Undo：恢复被删 wire
      if (state.wires.some((w) => w.id === action.id)) return null;
      return { wires: [...state.wires, action.wire], isDirty: true };
    }

    case "moveElement": {
      // Undo：移回 oldParentId；Redo：重放移动到 newParentId
      const element = state.elements.find((e) => e.id === action.elementId);
      if (!element) return null;
      const targetParentId = isRedo ? action.newParentId : action.oldParentId;
      const currentParent = element.parentId;
      let elements = state.elements.map((e) =>
        e.id === action.elementId ? { ...e, parentId: targetParentId } : e,
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
      if (targetParentId) {
        elements = elements.map((e) =>
          e.id === targetParentId
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

    case "setElementBinding": {
      // 既有实现从未处理此动作（undo/redo 均静默无操作）——补齐双向；
      // null 语义镜像 setElementBinding 原实现（删除键，空则 bindings 置 undefined）
      const targetBinding = isRedo ? action.newBinding : action.oldBinding;
      return {
        elements: state.elements.map((e) => {
          if (e.id !== action.id || e.nodeType !== "view") return e;
          if (targetBinding === null) {
            const { [action.propName]: _, ...rest } = (e.bindings ?? {});
            return Object.keys(rest).length > 0
              ? { ...e, bindings: rest }
              : { ...e, bindings: undefined };
          }
          return { ...e, bindings: { ...(e.bindings ?? {}), [action.propName]: targetBinding } };
        }),
        isDirty: true,
      };
    }

    case "restoreConditionalBlock": {
      return {
        elements: state.elements.map((e): ElementData =>
          e.id === action.blockId && e.nodeType === "conditional"
            ? {
                ...e,
                branches: isRedo ? action.newBranches : action.prevBranches,
                children: isRedo ? action.newChildren : action.prevChildren,
              }
            : e
        ),
        isDirty: true,
      };
    }

    case "updateForBlock": {
      return {
        elements: state.elements.map((e): ElementData =>
          e.id === action.blockId && e.nodeType === "for"
            ? {
                ...e,
                itemName: isRedo ? action.next.itemName ?? e.itemName : action.prev.itemName ?? e.itemName,
                collection: isRedo ? action.next.collection ?? e.collection : action.prev.collection ?? e.collection,
                keyExpr: isRedo ? action.next.keyExpr ?? e.keyExpr : action.prev.keyExpr ?? e.keyExpr,
              }
            : e
        ),
        isDirty: true,
      };
    }

    default:
      return null;
  }
}
