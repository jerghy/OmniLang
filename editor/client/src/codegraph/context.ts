/** Creates a CommandContext that operates on the active code graph tab.
 *  Commands (AddElement, AddWire, SetProp, etc.) use this to read/write
 *  code graph elements/wires instead of design elements/wires.
 *
 *  Usage:
 *    const codeCtx = createCodeGraphContext();
 *    useCodeGraphStore.getState().pushUndo(activeTabId, cmd);
 *    cmd.execute(codeCtx);
 */

import { useCodeGraphStore, type CodeGraphCommandContext } from "./store";
import type { ElementData } from "../store/editor";

export function createCodeGraphContext(): CodeGraphCommandContext {
  const store = useCodeGraphStore;

  return {
    get elements(): ElementData[] {
      return store.getState().activeGraph.elements;
    },
    get wires() {
      return store.getState().activeGraph.wires;
    },

    getElement: (id) => store.getState().activeGraph.elements.find((e) => e.id === id),

    addElement: (el) => store.getState().addElement(el),

    removeElement: (id) => store.getState().removeElement(id),

    setProp: (id, prop, value) => store.getState().setProp(id, prop, value),

    getProp: (id, prop) => store.getState().getProp(id, prop),

    addWire: (w) => store.getState().addWire(w),

    removeWire: (id) => store.getState().removeWire(id),

    setElementsAndWires: (elements, wires) =>
      store.getState().setElementsAndWires(elements, wires),

    setElementSource: (id, source) => store.getState().setElementSource(id, source),

    setElementPosition: (id, pos) => store.getState().setElementPosition(id, pos),

    // No-op for code graph (no tree nesting in flow canvas)
    reorderElement: () => {},
    moveElement: () => {},
  };
}
