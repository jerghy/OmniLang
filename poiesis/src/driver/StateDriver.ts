/** StateDriver — state queries, snapshots, diff, undo/redo.
 *  Extracted from OmniDriver god class. */

import { Logger } from "../lib/utils/logger";
import { getDesignStore, getActiveStore, getUndoStore } from "./store";
import { useEditorStore } from "../store/editor";
import { ElementHandle } from "./ElementHandle";
import type { EditorSnapshot, StateDiff, WireData } from "./types";
import type { ElementData } from "../store/editor";

const log = new Logger({ prefix: "OmniDriver" });

// ═══════════════════════════════════════════════════════
// StateDriver
// ═══════════════════════════════════════════════════════

export class StateDriver {
  // ── State Queries ─────────────────────────────────

  /** Get a full snapshot of the editor state. */
  getState(): EditorSnapshot {
    const s = getDesignStore();
    return {
      mode: window.__OMNI_MODE__ || "design",
      elements: s.elements,
      wires: s.wires,
      selectedElementIds: s.selectedElementIds,
      projectDir: s.projectDir,
      activeFilePath: s.activeFilePath,
      compileErrors: s.compileErrors,
      previewUrl: s.previewUrl,
    };
  }

  /** Get all elements currently on the canvas (design store). */
  getElements(): ElementData[] {
    return getDesignStore().elements;
  }

  /** Get all wires currently on the canvas (uses active store for both modes). */
  getWires(): WireData[] {
    return getActiveStore().wires;
  }

  /** Get the currently selected element, or null. */
  getSelectedElement(): ElementHandle | null {
    const id = getDesignStore().selectedElementIds[0] ?? null;
    return id ? new ElementHandle(id) : null;
  }

  /** Get the current editor mode. */
  getMode(): "design" | "code" {
    return window.__OMNI_MODE__ || "design";
  }

  /** Get the preview URL (after deploy). */
  getPreviewUrl(): string | null {
    return getDesignStore().previewUrl;
  }

  // ── State Snapshots ───────────────────────────────

  /** Export the full canvas state as a JSON string.
   *  Use with importState() to save/restore canvas snapshots. */
  exportState(): string {
    const state = this.getState();
    return JSON.stringify({
      version: "1.0",
      exportedAt: new Date().toISOString(),
      elements: state.elements,
      wires: state.wires,
    });
  }

  /** Import a canvas state previously exported with exportState().
   *  Replaces all current elements and wires. */
  importState(json: string): void {
    const data = JSON.parse(json);
    if (!data.elements || !data.wires) {
      log.warn("importState: invalid state JSON — expected { elements, wires }");
      return;
    }
    useEditorStore.setState({
      elements: data.elements,
      wires: data.wires,
    });
  }

  // ── State Diff ────────────────────────────────────

  /** Compare the current canvas against a previous snapshot.
   *  Returns added/removed/modified elements and wires. */
  diff(previous: EditorSnapshot): StateDiff {
    const current = this.getState();

    const prevElIds = new Set(previous.elements.map((e) => e.id));
    const currElIds = new Set(current.elements.map((e) => e.id));

    // Added elements
    const added = current.elements.filter((e) => !prevElIds.has(e.id));

    // Removed element IDs
    const removed = previous.elements
      .filter((e) => !currElIds.has(e.id))
      .map((e) => e.id);

    // Modified elements
    const prevElMap = new Map(previous.elements.map((e) => [e.id, e]));
    const modified: StateDiff["modified"] = [];
    for (const currEl of current.elements) {
      const prevEl = prevElMap.get(currEl.id);
      if (!prevEl) continue;
      const changes: Record<string, { from: unknown; to: unknown }> = {};
      // Check props
      if (currEl.props) {
        for (const [k, v] of Object.entries(currEl.props)) {
          const prevV = (prevEl.props as Record<string, unknown>)?.[k];
          if (JSON.stringify(v) !== JSON.stringify(prevV)) {
            changes[k] = { from: prevV, to: v };
          }
        }
      }
      // Check source
      if (currEl.source !== prevEl.source) {
        changes["source"] = { from: prevEl.source, to: currEl.source };
      }
      // Check ports
      if (JSON.stringify(currEl.ports) !== JSON.stringify(prevEl.ports)) {
        changes["ports"] = { from: prevEl.ports, to: currEl.ports };
      }
      if (Object.keys(changes).length > 0) {
        modified.push({ id: currEl.id, changes });
      }
    }

    // Wire changes
    const prevWireIds = new Set(previous.wires.map((w) => w.id));
    const currWireIds = new Set(current.wires.map((w) => w.id));
    const addedWires = current.wires.filter((w) => !prevWireIds.has(w.id));
    const removedWires = previous.wires
      .filter((w) => !currWireIds.has(w.id))
      .map((w) => w.id);

    return { added, removed, modified, addedWires, removedWires };
  }

  // ── Undo / Redo ──────────────────────────────────

  /** Undo the last operation. */
  undo(): void {
    const store = getDesignStore();
    getUndoStore().undo({
      getElement: (id) => store.elements.find((e) => e.id === id),
      get elements() { return useEditorStore.getState().elements; },
      get wires() { return useEditorStore.getState().wires; },
      addElement: (el) => store.addElement(el),
      removeElement: (id) => store.removeElement(id),
      setProp: (id, p, v) => store.setProp(id, p, v),
      getProp: (id, p) => store.getProp(id, p) ?? null,
      addWire: (w) => store.addWire(w),
      removeWire: (id) => store.removeWire(id),
      setElementsAndWires: () => {},
      reorderElement: () => {},
      moveElement: () => {},
    });
  }

  /** Redo the last undone operation. */
  redo(): void {
    const store = getDesignStore();
    getUndoStore().redo({
      getElement: (id) => store.elements.find((e) => e.id === id),
      get elements() { return useEditorStore.getState().elements; },
      get wires() { return useEditorStore.getState().wires; },
      addElement: (el) => store.addElement(el),
      removeElement: (id) => store.removeElement(id),
      setProp: (id, p, v) => store.setProp(id, p, v),
      getProp: (id, p) => store.getProp(id, p) ?? null,
      addWire: (w) => store.addWire(w),
      removeWire: (id) => store.removeWire(id),
      setElementsAndWires: () => {},
      reorderElement: () => {},
      moveElement: () => {},
    });
  }

  /** Save a snapshot token that can be restored later with restoreSnapshot().
   *  Use this before risky canvas operations to enable safe rollback. */
  saveSnapshot(): string {
    return this.exportState();
  }

  /** Restore the canvas to a previously saved snapshot.
   *  @returns true if restore succeeded, false if the snapshot was invalid. */
  restoreSnapshot(snapshotToken: string): boolean {
    try {
      this.importState(snapshotToken);
      return true;
    } catch (e) {
      log.warn("restoreSnapshot: failed to restore — invalid snapshot token", e);
      return false;
    }
  }
}
