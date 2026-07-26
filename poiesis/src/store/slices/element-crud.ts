// ── Element/Wire CRUD slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { ElementData, ConditionalBlockData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import type { StyleProperty } from "../../core/types/style";
import { buildUndoUpdate } from "../undo-slice";
import { createLogger } from "../../lib/utils/logger";

const log = createLogger("store");

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createElementCrudSlice(set: SetFn, get: GetFn) {
  return {
    addElement: (el: ElementData) => set((s) => {
      const normalized = "props" in el ? { ...el, props: el.props || {} } : el;
      const elements = [...s.elements, normalized];
      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "removeElement", id: normalized.id },
      );
      if (normalized.parentId) {
        return {
          ...undoUpdate,
          elements: elements.map(e =>
            e.id === normalized.parentId
              ? { ...e, children: "children" in e ? [...e.children, normalized.id] : [normalized.id] }
              : e
          ),
          isDirty: true,
        };
      }
      return { ...undoUpdate, elements, rootOrder: [...s.rootOrder, normalized.id], isDirty: true };
    }),

    removeElement: (id: string) => set((s) => {
      // Recursively collect all descendant IDs for proper wire cleanup
      const removedIds = new Set<string>();
      function collectDescendants(eid: string): void {
        removedIds.add(eid);
        s.elements.filter(e => e.parentId === eid).forEach(c => collectDescendants(c.id));
      }
      const el = s.elements.find(e => e.id === id);
      if (el) collectDescendants(el.id);

      // Save element snapshot for undo
      const undoPayload = el ? { element: el, rootOrder: s.rootOrder } : null;
      let elements = s.elements.filter(e => !removedIds.has(e.id));
      if (el?.parentId) {
        elements = elements.map(e =>
          e.id === el.parentId
            ? { ...e, children: "children" in e ? e.children.filter(cid => !removedIds.has(cid)) : [] }
            : e
        );
      }
      const undoAction = undoPayload
        ? { type: "addElement" as const, element: undoPayload.element }
        : null;
      const undoUpdate = undoAction
        ? buildUndoUpdate(s.undoStacks, s.designPage.activeTabId, undoAction)
        : {};
      return {
        ...undoUpdate,
        elements,
        rootOrder: s.rootOrder.filter(rid => !removedIds.has(rid)),
        wires: s.wires.filter(w => !removedIds.has(w.fromId) && !removedIds.has(w.toId)),
        isDirty: true,
      };
    }),

    setProp: (id: string, prop: string, value: unknown) => set((s) => {
      const elements = s.elements.map(e => {
        if (e.id !== id) return e;
        if (!("props" in e)) return e;
        return { ...e, props: { ...e.props, [prop]: value } };
      });
      const oldEl = s.elements.find(e => e.id === id);
      const oldValue = oldEl && "props" in oldEl ? oldEl.props[prop] : undefined;
      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "setProp", id, prop, oldValue },
      );
      return { ...undoUpdate, elements, isDirty: true };
    }),

    getProp: (id: string, prop: string) => {
      const el = get().elements.find(e => e.id === id);
      return el && "props" in el ? el.props[prop] : undefined;
    },

    // ── Two-way data bindings ──

    /** Set or remove a two-way data binding on a view element.
     *  Stored as element.bindings[propName] = signalName.
     *  Pass signalName = null to remove. */
    setElementBinding: (id: string, propName: string, signalName: string | null) => set((s) => {
      const oldEl = s.elements.find(e => e.id === id);
      const oldBinding = (oldEl && oldEl.nodeType === "view" ? oldEl.bindings?.[propName] : undefined) ?? null;
      const elements = s.elements.map(e => {
        if (e.id !== id || e.nodeType !== "view") return e;
        const current = (e as import("../../core/types/element").ViewElementData).bindings ?? {};
        if (signalName === null) {
          const { [propName]: _, ...rest } = current;
          return { ...e, bindings: Object.keys(rest).length > 0 ? rest : undefined };
        }
        return { ...e, bindings: { ...current, [propName]: signalName } };
      });
      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "setElementBinding", id, propName, oldBinding },
      );
      return { ...undoUpdate, elements, isDirty: true };
    }),

    selectElement: (id: string | null, multi = false) => {
      set((s) => {
        if (id === null) {
          return { selectedElementIds: [] };
        } else if (multi) {
          const exists = s.selectedElementIds.includes(id);
          return {
            selectedElementIds: exists
              ? s.selectedElementIds.filter(x => x !== id)
              : [...s.selectedElementIds, id],
          };
        } else {
          return { selectedElementIds: [id] };
        }
      });

      // Side effect OUTSIDE set() -- notify debug window of selection change
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent("poiesis:editor-select", {
          detail: { elementId: id },
        }));
      }
    },

    clearSelection: () => set({ selectedElementIds: [] }),

    addWire: (w: WireData) => set((s) => {
      const wires = [...s.wires, w];
      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "removeWire", id: w.id },
      );
      return { ...undoUpdate, wires, isDirty: true };
    }),

    removeWire: (id: string) => set((s) => {
      const wire = s.wires.find(w => w.id === id);
      const wires = s.wires.filter(w => w.id !== id);
      if (!wire) return { wires, isDirty: true };
      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "addWire", wire },
      );
      return { ...undoUpdate, wires, isDirty: true };
    }),

    setElementSource: (id: string, source: string) => set((s) => ({
      elements: s.elements.map(e => e.id === id ? { ...e, source } as ElementData : e),
      isDirty: true,
    })),

    setElementPosition: (id: string, pos: { x: number; y: number }) => set((s) => ({
      elements: s.elements.map(e => e.id === id ? { ...e, position: { ...pos } } as ElementData : e),
      isDirty: true,
    })),

    setElementsAndWires: (elements: ElementData[], wires: WireData[]) => set((s) => {
      const prevElements = [...s.elements];
      const prevWires = [...s.wires];
      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "setElementsAndWires", prevElements, prevWires, newElements: elements, newWires: wires },
      );
      return {
        ...undoUpdate,
        elements,
        wires,
        rootOrder: elements.filter(e => !e.parentId).map(e => e.id),
        selectedElementIds: [],
      };
    }),

    moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => set((s) => {
      const el = s.elements.find(e => e.id === elementId);
      if (!el) return s;
      const oldParentId = el.parentId;
      let elements = s.elements.map(e => e.id === elementId ? { ...e, parentId: newParentId } : e);
      let rootOrder = s.rootOrder;

      // Remove from old parent
      if (oldParentId) {
        elements = elements.map(e =>
          e.id === oldParentId
            ? { ...e, children: "children" in e ? e.children.filter(cid => cid !== elementId) : [] }
            : e
        );
      } else {
        rootOrder = rootOrder.filter(id => id !== elementId);
      }

      // Add to new parent
      if (newParentId) {
        const newParent = elements.find(e => e.id === newParentId);
        if (newParent && "children" in newParent) {
          const newChildren = [...newParent.children];
          if (targetSiblingId && newChildren.includes(targetSiblingId)) {
            const idx = newChildren.indexOf(targetSiblingId);
            newChildren.splice(position === "before" ? idx : idx + 1, 0, elementId);
          } else {
            newChildren.push(elementId);
          }
          elements = elements.map(e =>
            e.id === newParentId
              ? { ...e, children: newChildren } as ElementData
              : e
          );
        } else if (newParent) {
          // Parent exists but has no children property (e.g., codeblock)
          throw new Error(`Element "${newParentId}" cannot have children`);
        }
      } else {
        // Moving to root level — insert into rootOrder
        if (targetSiblingId && rootOrder.includes(targetSiblingId)) {
          const targetIdx = rootOrder.indexOf(targetSiblingId);
          const insertAt = position === "before" ? targetIdx : targetIdx + 1;
          rootOrder = [...rootOrder];
          rootOrder.splice(insertAt, 0, elementId);
        } else {
          rootOrder = [...rootOrder, elementId];
        }
      }

      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "moveElement", elementId, oldParentId },
      );
      return { ...undoUpdate, elements, rootOrder, isDirty: true };
    }),

    setElementClassRefs: (elementId: string, classRefs: string[]) => set((s) => {
      const newElements = s.elements.map(e => {
        if (e.id !== elementId) return e;
        if (e.nodeType !== "view") return e;
        return { ...e, style: { ...(e.style ?? { classRefs: [], inlineProperties: [] }), classRefs } } as ElementData;
      });

      // Sync updated element back to componentDef for cross-tab canvas preview
      const updatedEl = newElements.find(e => e.id === elementId);
      const activeTab = s.designPage.activeTabId
        ? s.designPage.openTabs.find(t => t.id === s.designPage.activeTabId)
        : undefined;
      const compName = activeTab && activeTab.tabType === "component" ? activeTab.componentName : undefined;
      let componentDefs = s.componentDefs;
      if (compName && updatedEl && componentDefs[compName]) {
        const def = componentDefs[compName];
        const defEls = def.elements ?? [];
        const idx = defEls.findIndex(e => e.id === elementId);
        componentDefs = {
          ...componentDefs,
          [compName]: { ...def, elements: idx >= 0 ? defEls.map(e => e.id === elementId ? updatedEl : e) : [...defEls, updatedEl] },
        };
      }

      return { elements: newElements, componentDefs, isDirty: true };
    }),

    setElementInlineStyles: (elementId: string, properties: StyleProperty[]) => set((s) => {
      const newElements = s.elements.map(e => {
        if (e.id !== elementId) return e;
        if (e.nodeType !== "view") return e;
        return { ...e, style: { ...(e.style ?? { classRefs: [], inlineProperties: [] }), inlineProperties: properties } } as ElementData;
      });

      // Sync updated element back to componentDef for cross-tab canvas preview
      const updatedEl = newElements.find(e => e.id === elementId);
      const activeTab = s.designPage.activeTabId
        ? s.designPage.openTabs.find(t => t.id === s.designPage.activeTabId)
        : undefined;
      const compName = activeTab && activeTab.tabType === "component" ? activeTab.componentName : undefined;
      let componentDefs = s.componentDefs;
      if (compName && updatedEl && componentDefs[compName]) {
        const def = componentDefs[compName];
        const defEls = def.elements ?? [];
        const idx = defEls.findIndex(e => e.id === elementId);
        componentDefs = {
          ...componentDefs,
          [compName]: { ...def, elements: idx >= 0 ? defEls.map(e => e.id === elementId ? updatedEl : e) : [...defEls, updatedEl] },
        };
      }

      return { elements: newElements, componentDefs, isDirty: true };
    }),

    reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => set((s) => {
      const el = s.elements.find(e => e.id === elementId);
      if (!el) return s;
      const parentId = el.parentId;
      const parent = parentId ? s.elements.find(e => e.id === parentId) : undefined;
      if (parentId && !parent) {
        log.warn('reorderElement: parent not found for', elementId);
        return s;
      }
      const siblings = parent && "children" in parent ? parent.children : s.rootOrder;
      const idx = siblings.indexOf(elementId);
      if (idx === -1) return s;
      const targetIdx = siblings.indexOf(targetSiblingId);
      if (targetIdx === -1) return s;
      const prevSiblingId = idx > 0 ? siblings[idx - 1] : null;
      const undoUpdate = buildUndoUpdate(
        s.undoStacks,
        s.designPage.activeTabId,
        { type: "reorderElement", elementId, parentId, prevSiblingId, targetSiblingId, position },
      );
      const newSiblings = [...siblings];
      newSiblings.splice(idx, 1);
      const insertAt = position === "before" ? targetIdx : targetIdx + 1;
      newSiblings.splice(insertAt, 0, elementId);
      if (parentId) {
        return {
          ...undoUpdate,
          elements: s.elements.map(e =>
            e.id === parentId ? { ...e, children: newSiblings } as ElementData : e
          ),
          isDirty: true,
        };
      }
      return { ...undoUpdate, rootOrder: newSiblings, isDirty: true };
    }),

    // ── Conditional block CRUD ──

    addConditionalBlock: (parentId: string) => {
      set((s) => {
        const condId = `conditional.${Date.now()}`;
        const condBlock: ConditionalBlockData = {
          id: condId,
          nodeType: "conditional",
          parentId,
          branches: [
            { type: "if", condition: "", children: [] },
            { type: "else", condition: "", children: [] },
          ],
          children: [],
          isFolded: false,
        };
        const elements = [
          ...s.elements,
          condBlock,
        ].map((e) =>
          e.id === parentId && "children" in e
            ? { ...e, children: [...e.children, condId] }
            : e
        );
        const undoUpdate = buildUndoUpdate(
          s.undoStacks,
          s.designPage.activeTabId,
          { type: "removeElement", id: condId },
        );
        return { ...undoUpdate, elements, isDirty: true };
      });
    },

    addConditionalBranch: (blockId: string, type: "elif" | "else") => {
      set((s) => {
        const block = s.elements.find(e => e.id === blockId && e.nodeType === "conditional") as ConditionalBlockData | undefined;
        const undoUpdate = block
          ? buildUndoUpdate(s.undoStacks, s.designPage.activeTabId, {
              type: "restoreConditionalBlock",
              blockId,
              prevBranches: block.branches.map(b => ({ ...b })),
              prevChildren: [...block.children],
            })
          : {};
        const elements = s.elements.map((e) => {
          if (e.id !== blockId || e.nodeType !== "conditional") return e;
          const cond = e as ConditionalBlockData;
          const newBranch = {
            type: type === "elif" ? "elif" as const : "else" as const,
            condition: "",
            children: [] as string[],
          };
          // Insert elif before last branch (else), else replaces the else
          if (type === "elif") {
            const newBranches: ConditionalBlockData["branches"] = [
              ...cond.branches.slice(0, -1),
              newBranch,
              cond.branches[cond.branches.length - 1],
            ];
            return {
              ...cond,
              branches: newBranches,
              children: newBranches.flatMap((b) => b.children),
            };
          } else {
            // Already has else — no-op
            return e;
          }
        });
        return { ...s, ...undoUpdate, elements, isDirty: true };
      });
    },

    updateBranchCondition: (blockId: string, branchIndex: number, condition: string) => {
      set((s) => {
        const block = s.elements.find(e => e.id === blockId && e.nodeType === "conditional") as ConditionalBlockData | undefined;
        const undoUpdate = block
          ? buildUndoUpdate(s.undoStacks, s.designPage.activeTabId, {
              type: "restoreConditionalBlock",
              blockId,
              prevBranches: block.branches.map(b => ({ ...b })),
              prevChildren: [...block.children],
            })
          : {};
        const elements = s.elements.map((e) => {
          if (e.id !== blockId || e.nodeType !== "conditional") return e;
          const cond = e as ConditionalBlockData;
          if (branchIndex < 0 || branchIndex >= cond.branches.length) return e;
          const newBranches = cond.branches.map((b, i) =>
            i === branchIndex ? { ...b, condition } : b
          );
          return { ...cond, branches: newBranches };
        });
        return { ...s, ...undoUpdate, elements, isDirty: true };
      });
    },

    addChildToBranch: (blockId: string, branchIndex: number, childId: string) => {
      set((s) => {
        const child = s.elements.find((e) => e.id === childId);
        if (!child) return s;

        const block = s.elements.find(e => e.id === blockId && e.nodeType === "conditional") as ConditionalBlockData | undefined;
        const undoUpdate = block
          ? buildUndoUpdate(s.undoStacks, s.designPage.activeTabId, {
              type: "restoreConditionalBlock",
              blockId,
              prevBranches: block.branches.map(b => ({ ...b })),
              prevChildren: [...block.children],
            })
          : {};

        const elements = s.elements
          .map((e) => {
            // Update child's parentId
            if (e.id === childId) {
              return { ...e, parentId: blockId } as typeof e;
            }
            // Remove from old parent's children
            if (child.parentId && e.id === child.parentId && "children" in e) {
              return { ...e, children: e.children.filter((c) => c !== childId) };
            }
            return e;
          })
          .map((e) => {
            if (e.id !== blockId || e.nodeType !== "conditional") return e;
            const cond = e as ConditionalBlockData;
            const newBranches = cond.branches.map((b, i) => {
              if (i !== branchIndex) return b;
              if (b.children.includes(childId)) return b; // already there
              return { ...b, children: [...b.children, childId] };
            });
            return {
              ...cond,
              branches: newBranches,
              children: newBranches.flatMap((b) => b.children),
            };
          });

        return { ...s, ...undoUpdate, elements, isDirty: true };
      });
    },

    removeChildFromBranch: (blockId: string, branchIndex: number, childId: string) => {
      set((s) => {
        const block = s.elements.find(e => e.id === blockId && e.nodeType === "conditional") as ConditionalBlockData | undefined;
        const undoUpdate = block
          ? buildUndoUpdate(s.undoStacks, s.designPage.activeTabId, {
              type: "restoreConditionalBlock",
              blockId,
              prevBranches: block.branches.map(b => ({ ...b })),
              prevChildren: [...block.children],
            })
          : {};
        const elements = s.elements.map((e) => {
          if (e.id !== blockId || e.nodeType !== "conditional") return e;
          const cond = e as ConditionalBlockData;
          const newBranches = cond.branches.map((b, i) => {
            if (i !== branchIndex) return b;
            return { ...b, children: b.children.filter((c) => c !== childId) };
          });
          return {
            ...cond,
            branches: newBranches,
            children: newBranches.flatMap((b) => b.children),
          };
        });
        return { ...s, ...undoUpdate, elements, isDirty: true };
      });
    },

    removeConditionalBranch: (blockId: string, branchIndex: number) => {
      set((s) => {
        const block = s.elements.find(e => e.id === blockId && e.nodeType === "conditional") as ConditionalBlockData | undefined;
        const undoUpdate = block
          ? buildUndoUpdate(s.undoStacks, s.designPage.activeTabId, {
              type: "restoreConditionalBlock",
              blockId,
              prevBranches: block.branches.map(b => ({ ...b })),
              prevChildren: [...block.children],
            })
          : {};
        const elements = s.elements.map((e) => {
          if (e.id !== blockId || e.nodeType !== "conditional") return e;
          const cond = e as ConditionalBlockData;
          // Cannot remove @if (index 0) or @else (last branch)
          if (branchIndex <= 0 || branchIndex >= cond.branches.length - 1) return e;
          const newBranches = cond.branches.filter((_, i) => i !== branchIndex);
          return {
            ...cond,
            branches: newBranches,
            children: newBranches.flatMap((b) => b.children),
          };
        });
        return { ...s, ...undoUpdate, elements, isDirty: true };
      });
    },
  };
}
