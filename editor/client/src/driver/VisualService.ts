/** VisualService — highlight indicators and waitFor polling.
 *  Extracted from OmniDriver god class. */

import { Logger } from "../utils/logger";
import { getDesignStore } from "./store";
import { ElementHandle } from "./ElementHandle";
import { WireHandle } from "./WireHandle";

const log = new Logger({ prefix: "OmniDriver" });

// ═══════════════════════════════════════════════════════
// VisualService
// ═══════════════════════════════════════════════════════

export class VisualService {
  // ── Visual Indicators ─────────────────────────────

  /** Highlight an element on the canvas with a temporary visual indicator.
   *  Dispatches a custom DOM event that the FlowCanvas can listen to.
   *  Use clearHighlights() to remove all indicators.
   *
   *  @example
   *  O.highlight("el-123");          // highlight one element
   *  O.highlightAll("Button");       // highlight all Buttons
   *  O.clearHighlights();            // remove all highlights
   */
  highlight(elementId: string): void {
    const el = getDesignStore().elements.find((e) => e.id === elementId);
    if (!el) {
      log.warn(`highlight: element "${elementId}" not found`);
      return;
    }
    window.dispatchEvent(
      new CustomEvent("omni-highlight", {
        detail: { elementIds: [elementId], action: "add" },
      }),
    );
  }

  /** Highlight all elements with the given tag on the canvas. */
  highlightAll(tag: string): void {
    const ids = getDesignStore()
      .elements.filter((e) => e.tag === tag)
      .map((e) => e.id);
    if (ids.length === 0) {
      log.warn(`highlightAll: no elements with tag "${tag}" found`);
      return;
    }
    window.dispatchEvent(
      new CustomEvent("omni-highlight", {
        detail: { elementIds: ids, action: "add" },
      }),
    );
  }

  /** Remove all visual highlights from the canvas. */
  clearHighlights(): void {
    window.dispatchEvent(
      new CustomEvent("omni-highlight", {
        detail: { elementIds: [], action: "clear" },
      }),
    );
  }

  // ── Wait Mechanisms ───────────────────────────────

  /** Wait for an element with the given tag to appear on the canvas. */
  async waitForElement(
    tag: string,
    timeout = 5000,
  ): Promise<ElementHandle> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      const found = getDesignStore().elements.find((e) => e.tag === tag);
      if (found) return new ElementHandle(found.id);
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(
      `Element with tag "${tag}" did not appear within ${timeout}ms`,
    );
  }

  /** Wait for a wire matching the given port to appear. */
  async waitForWire(fromPort?: string, timeout = 5000): Promise<WireHandle> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      const found = fromPort
        ? getDesignStore().wires.find((w) => w.fromPort === fromPort)
        : getDesignStore().wires[0];
      if (found) return new WireHandle(found.id);
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(
      `Wire matching fromPort="${fromPort}" did not appear within ${timeout}ms`,
    );
  }

  /** Wait for a specific condition to be true. */
  async waitFor(
    predicate: () => boolean,
    timeout = 5000,
    message = "Condition not met",
  ): Promise<void> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      if (predicate()) return;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(`${message} within ${timeout}ms`);
  }
}
