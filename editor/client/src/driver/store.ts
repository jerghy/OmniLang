/** Shared utilities for OmniDriver sub-modules.
 *  Foundation module with no dependencies on other driver modules. */

import { Logger } from "../utils/logger";
import { useEditorStore } from "../store/editor";
import { useCodeGraphStore } from "../codegraph/store";
import { useUndoStore } from "../store/undo";
import { getAllPorts } from "../canvas/connectionValidators";
import type { ElementData } from "../store/editor";

const log = new Logger({ prefix: "OmniDriver" });

// ── Store Accessors ─────────────────────────────────

/** Access the design-mode Zustand store (for view elements, file ops, etc.). */
export function getDesignStore() {
  return useEditorStore.getState();
}

/** Check if we are in code mode. */
export function isCodeMode(): boolean {
  return ((window as any).__OMNI_MODE__ || "design") === "code";
}

/** Access the code graph store for tab/graph operations. */
export function getCodeGraphStore() {
  return useCodeGraphStore.getState();
}

/** Access the undo store. */
export function getUndoStore() {
  return useUndoStore.getState();
}

/** Access the active store (design or code graph) for element/wire CRUD.
 *  Returns an object with .elements, .wires, .addElement(), .addWire(), .removeWire(). */
export function getActiveStore(): any {
  if (isCodeMode()) {
    const cg = useCodeGraphStore.getState();
    const graph = cg.getActiveGraph();
    return {
      get elements() { return graph.elements; },
      get wires() { return graph.wires; },
      addElement: (el: ElementData) => cg.addElement(el),
      removeElement: (id: string) => cg.removeElement(id),
      setProp: (id: string, prop: string, value: unknown) => cg.setProp(id, prop, value),
      getProp: (id: string, prop: string) => cg.getProp(id, prop),
      addWire: (w: any) => cg.addWire(w),
      removeWire: (id: string) => cg.removeWire(id),
      setElementSource: (id: string, source: string) => cg.setElementSource(id, source),
      setElementPorts: (id: string, ports: any) => cg.setElementPorts(id, ports),
      selectElement: (_id: string | null, _multi?: boolean) => {},
      get selectedElementIds() { return []; },
      get previewUrl() { return null; },
      get activeFileContent() { return ""; },
      get activeFilePath() { return ""; },
      get projectDir() { return ""; },
    };
  }
  return useEditorStore.getState();
}

// ── ID Generation ───────────────────────────────────

/** Generate a unique element ID. */
export function nextId(): string {
  return `el-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

// ── Help Reminder ──────────────────────────────────

let _helpShown = false;

/** Show a one-time reminder for AI agents to call help(). */
export function ensureHelp(): void {
  if (_helpShown) return;
  log.info(
    "\u{1F4A1}\u{FE0F} \u{9996}\u{6B21}\u{4F7F}\u{7528}\u{FF1F}\u{8C03}\u{7528} O.help() \u{67E5}\u{770B}\u{5B8C}\u{6574} API \u{53C2}\u{8003}\u{3001}\u{7AEF}\u{53E3}\u{7ED3}\u{6784}\u{548C}\u{4E8B}\u{4EF6} UUID \u{8868}",
  );
  _helpShown = true;
}

// ── Re-export ──────────────────────────────────────

export { getAllPorts };
