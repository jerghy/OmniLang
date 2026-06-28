import { useEditorStore } from "../../store/editor";
import type { CommandContext } from "./types";

export function createCommandContext(): CommandContext {
  return {
    getElement: (id) => useEditorStore.getState().elements.find((e) => e.id === id),
    get elements() {
      return useEditorStore.getState().elements;
    },
    get wires() {
      return useEditorStore.getState().wires;
    },
    addElement: (el) => useEditorStore.getState().addElement(el),
    removeElement: (id) => useEditorStore.getState().removeElement(id),
    setProp: (id, p, v) => useEditorStore.getState().setProp(id, p, v),
    getProp: (id, p) => useEditorStore.getState().getProp(id, p) ?? null,
    addWire: (w) => useEditorStore.getState().addWire(w),
    removeWire: (id) => useEditorStore.getState().removeWire(id),
    setElementsAndWires: (elements, wires) =>
      useEditorStore.getState().setElementsAndWires(elements, wires),
    reorderElement: (elementId, targetSiblingId, position) =>
      useEditorStore.getState().reorderElement(elementId, targetSiblingId, position),
    moveElement: (elementId, newParentId, targetSiblingId, position) =>
      useEditorStore.getState().moveElement(elementId, newParentId, targetSiblingId, position),
    getRootOrder: () => useEditorStore.getState().rootOrder,
  };
}
