/** Shared utilities for OmniDriver sub-modules.
 *  Foundation module with no dependencies on other driver modules. */

import { Logger } from "../lib/utils/logger";
import { usePoiesisStore } from "../store";

import { getAllPorts } from "../components/canvas/shared/connectionValidators";
import type { ElementData } from "../core/types/element";
import type { WireData } from "../core/types/wire";

const log = new Logger({ prefix: "OmniDriver" });

// ── Store Accessors ─────────────────────────────────

/** Access the design-mode Zustand store (for view elements, file ops, etc.). */
export function getDesignStore() {
  return usePoiesisStore.getState();
}

/** Check if we are in code mode. */
export function isCodeMode(): boolean {
  return usePoiesisStore.getState().mode === "code";
}

/** Access the code graph store for tab/graph operations. */
export function getCodeGraphStore() {
  return usePoiesisStore.getState();
}

/** Access the undo store (via unified store). */
export function getUndoStore() {
  return usePoiesisStore.getState();
}

/** Minimal interface for the store returned by getActiveStore(). */
interface ActiveStore {
  elements: ElementData[];
  wires: WireData[];
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setElementSource: (id: string, source: string) => void;
  selectElement: (id: string | null, multi?: boolean) => void;
  selectedElementIds: string[];
  previewUrl: string | null;
  activeFileContent: string | null;
  activeFilePath: string | null;
  projectDir: string | null;
}

/** Access the active store (design or code graph) for element/wire CRUD.
 *  Returns an object with .elements, .wires, .addElement(), .addWire(), .removeWire(). */
export function getActiveStore(): ActiveStore {
  if (isCodeMode()) {
    const state = usePoiesisStore.getState();
    return {
      get elements() { return state.elements; },
      get wires() { return state.wires; },
      addElement: (el: ElementData) => state.addElement(el),
      removeElement: (id: string) => state.removeElement(id),
      setProp: (id: string, prop: string, value: unknown) => state.setProp(id, prop, value),
      getProp: (id: string, prop: string) => state.getProp(id, prop),
      addWire: (w: WireData) => state.addWire(w),
      removeWire: (id: string) => state.removeWire(id),
      setElementSource: (id: string, source: string) => state.setElementSource(id, source),
      selectElement: (_id: string | null, _multi?: boolean) => {},
      get selectedElementIds() { return []; },
      get previewUrl() { return null; },
      get activeFileContent() { return ""; },
      get activeFilePath() { return ""; },
      get projectDir() { return ""; },
    };
  }
  const s = usePoiesisStore.getState();
  return {
    get elements() { return s.elements; },
    get wires() { return s.wires; },
    addElement: (el: ElementData) => s.addElement(el),
    removeElement: (id: string) => s.removeElement(id),
    setProp: (id: string, prop: string, value: unknown) => s.setProp(id, prop, value),
    getProp: (id: string, prop: string) => s.getProp(id, prop),
    addWire: (w: WireData) => s.addWire(w),
    removeWire: (id: string) => s.removeWire(id),
    setElementSource: (id: string, source: string) => s.setElementSource(id, source),
    selectElement: (id: string | null, multi?: boolean) => s.selectElement(id, multi),
    get selectedElementIds() { return s.selectedElementIds; },
    get previewUrl() { return s.previewUrl; },
    get activeFileContent() { return s.activeFileContent; },
    get activeFilePath() { return s.activeFilePath; },
    get projectDir() { return s.projectDir; },
  };
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
