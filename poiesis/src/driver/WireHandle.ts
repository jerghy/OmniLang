/** WireHandle — handle for wire control.
 *  Extracted from OmniDriver god class.
 *  Standalone: imports store directly, constructs ElementHandle for source/target. */

import { getDesignStore } from "./store";
import { ElementHandle } from "./ElementHandle";
import type { WireData } from "../store/editor";

// ═══════════════════════════════════════════════════════
// WireHandle
// ═══════════════════════════════════════════════════════

export class WireHandle {
  constructor(public readonly id: string) {}

  /** Get the live wire data from the store. */
  get data(): WireData | undefined {
    return getDesignStore().wires.find((w) => w.id === this.id);
  }

  /** Disconnect (remove) this wire. */
  disconnect(): void {
    getDesignStore().removeWire(this.id);
  }

  /** Get the source element handle. */
  source(): ElementHandle | null {
    const d = this.data;
    return d ? new ElementHandle(d.fromId) : null;
  }

  /** Get the target element handle. */
  target(): ElementHandle | null {
    const d = this.data;
    return d ? new ElementHandle(d.toId) : null;
  }
}
