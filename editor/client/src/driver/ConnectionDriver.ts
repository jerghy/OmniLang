/** ConnectionDriver — wire connection/disconnection and canvas validation.
 *  Extracted from OmniDriver god class.
 *  ALSO exports a standalone connect() function for ElementHandle to import. */

import { Logger } from "../utils/logger";
import { getActiveStore, getAllPorts, ensureHelp, getDesignStore } from "./store";
import { validateConnection } from "../ui/canvas/shared/connectionValidators";
import { COMPONENT_PROTOCOL } from "../ui/panels/ComponentPalette";
import { WireHandle } from "./WireHandle";
import type { ElementData } from "../store/editor";
import type { PortDescriptor, ValidationReport } from "./types";

const log = new Logger({ prefix: "OmniDriver" });

// ═══════════════════════════════════════════════════════
// Standalone connect() — used by ElementHandle.connectTo
// ═══════════════════════════════════════════════════════

/** Create a wire between two elements.
 *  Runs full shared validation: portType mismatch, type compatibility,
 *  duplicate wires, cycle detection, fan-out warnings. */
export function connect(
  fromId: string,
  fromPort: string,
  toId: string,
  toPort: string,
  wireId?: string,
): WireHandle {
  ensureHelp();
  const store = getActiveStore();
  const id = wireId || `wire-${fromId}-${fromPort}-${toId}-${toPort}`;
  const fromEl = store.elements.find((e: ElementData) => e.id === fromId);
  const toEl = store.elements.find((e: ElementData) => e.id === toId);

  if (!fromEl) throw new Error(`Source element ${fromId} not found`);
  if (!toEl) throw new Error(`Target element ${toId} not found`);

  // Use shared getAllPorts for consistent port discovery
  const fromPorts = getAllPorts(fromEl) as PortDescriptor[];
  const toPorts = getAllPorts(toEl) as PortDescriptor[];

  // Run shared validation (same as FlowCanvas.handleConnect)
  const issues = validateConnection(
    fromEl, fromPort, toEl, toPort, store.wires, fromPorts, toPorts,
  );

  // Log all issues
  for (const issue of issues) {
    const level = issue.severity === "error" ? "error" : "warn";
    if (level === "error") {
      log.error(`connect: ${issue.code} — ${issue.message}`);
    } else {
      log.warn(`connect: ${issue.code} — ${issue.message}`);
    }
  }

  // Block connection on errors (portType mismatch, type incompat, duplicate)
  const errors = issues.filter((i) => i.severity === "error");
  if (errors.length > 0) {
    log.error(
      `connect: Connection blocked (${errors.length} error(s)). ` +
      `Wire NOT created: ${fromId}.${fromPort} → ${toId}.${toPort}`,
    );
    return new WireHandle(id);
  }

  store.addWire({ id, fromId, fromPort, toId, toPort });
  return new WireHandle(id);
}

// ═══════════════════════════════════════════════════════
// ConnectionDriver class
// ═══════════════════════════════════════════════════════

export class ConnectionDriver {
  /** Create a wire between two elements.
   *  Runs full shared validation. Delegates to standalone connect(). */
  connect(
    fromId: string,
    fromPort: string,
    toId: string,
    toPort: string,
    wireId?: string,
  ): WireHandle {
    return connect(fromId, fromPort, toId, toPort, wireId);
  }

  /** Disconnect a wire by ID. */
  disconnect(wireId: string): void {
    getActiveStore().removeWire(wireId);
  }

  // ── Validation (Canvas Health Check) ──────────────

  /** Run a canvas health check — find orphan elements, unwired events,
   *  type mismatches, and unreachable code blocks.
   *  Use this to diagnose issues in a programmatically-built canvas. */
  validate(): ValidationReport {
    const state = getDesignStore();
    const elements = state.elements;
    const wires = state.wires;

    // 1. Orphan elements: view elements without parent (when multiple roots exist)
    const rootElements = elements.filter((e) => !e.parentId);
    const orphanElements = rootElements
      .filter((e) => e.nodeType === "view" && rootElements.length > 1)
      .map((e) => e.id);

    // 2. Unwired events: view elements with defined events but no connecting wire
    const unwiredEvents: ValidationReport["unwiredEvents"] = [];
    for (const el of elements) {
      if (el.nodeType !== "view") continue;
      const meta = COMPONENT_PROTOCOL[el.tag];
      if (!meta?.events) continue;
      for (const [evtName, evtMeta] of Object.entries(meta.events)) {
        const hasWire = wires.some(
          (w) => w.fromId === el.id && w.fromPort === evtMeta.id,
        );
        if (!hasWire) {
          unwiredEvents.push({
            elementId: el.id,
            tag: el.tag,
            eventName: evtName,
            eventPortId: evtMeta.id,
          });
        }
      }
    }

    // 3. Type mismatches: wires connecting incompatible port types
    const typeMismatches: ValidationReport["typeMismatches"] = [];
    for (const w of wires) {
      const fromEl = elements.find((e) => e.id === w.fromId);
      const toEl = elements.find((e) => e.id === w.toId);
      if (!fromEl || !toEl) continue;
      const fromPorts = getAllPorts(fromEl) as PortDescriptor[];
      const toPorts = getAllPorts(toEl) as PortDescriptor[];
      const fromPort = fromPorts.find((p) => p.name === w.fromPort);
      const toPort = toPorts.find((p) => p.name === w.toPort);
      if (
        fromPort && toPort &&
        fromPort.type !== toPort.type &&
        fromPort.type !== "any" && toPort.type !== "any" &&
        fromPort.type !== "event" && toPort.type !== "event"
      ) {
        typeMismatches.push({
          wireId: w.id,
          fromPort: w.fromPort,
          fromType: fromPort.type,
          toPort: w.toPort,
          toType: toPort.type,
        });
      }
    }

    // 4. Unreachable blocks: code blocks with no inbound data wire
    const reachableIds = new Set(wires.map((w) => w.toId));
    const unreachableBlocks = elements
      .filter((e) => e.nodeType === "codeblock" && !reachableIds.has(e.id))
      .map((e) => e.id);

    return {
      orphanElements,
      unwiredEvents,
      typeMismatches,
      unreachableBlocks,
      isValid: orphanElements.length === 0 && typeMismatches.length === 0,
    };
  }
}
