import { usePoiesisStore } from "../../store";
import type { CommandContext } from "./types";

export function createCommandContext(): CommandContext {
  return {
    getElement: (id) => usePoiesisStore.getState().elements.find((e) => e.id === id),
    get elements() {
      return usePoiesisStore.getState().elements;
    },
    get wires() {
      return usePoiesisStore.getState().wires;
    },
    addElement: (el) => usePoiesisStore.getState().addElement(el),
    removeElement: (id) => usePoiesisStore.getState().removeElement(id),
    setProp: (id, p, v) => usePoiesisStore.getState().setProp(id, p, v),
    getProp: (id, p) => usePoiesisStore.getState().getProp(id, p) ?? null,
    addWire: (w) => usePoiesisStore.getState().addWire(w),
    removeWire: (id) => usePoiesisStore.getState().removeWire(id),
    setElementsAndWires: (elements, wires) =>
      usePoiesisStore.getState().setElementsAndWires(elements, wires),
    reorderElement: (elementId, targetSiblingId, position) =>
      usePoiesisStore.getState().reorderElement(elementId, targetSiblingId, position),
    moveElement: (elementId, newParentId, targetSiblingId, position) =>
      usePoiesisStore.getState().moveElement(elementId, newParentId, targetSiblingId, position),
    getRootOrder: () => usePoiesisStore.getState().rootOrder,
  };
}
