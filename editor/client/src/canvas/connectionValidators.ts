/** Shared connection validation used by both FlowCanvas (drag-drop) and
 *  OmniDriver (programmatic API).  Single source of truth for:
 *    - port discovery (getAllPorts — returns element defined ports)
 *    - connection validation (type compatibility, duplicates, cycles)
 *
 *  Note: Event ports (onPress, onChange etc.) are no longer injected here.
 *  Events are bound in Design mode PropertyPanel, not via code graph wires. */

import type { ElementData, PortData, WireData } from "../store/editor";
import { isTypeCompatible, typeMismatchMessage } from "./socketTypes";

// ── Port discovery ────────────────────────────────────────────────

/**
 * Get ports for an element. Returns element-defined ports directly.
 * Event injection is no longer needed — events are bound in design mode.
 */
export function getAllPorts(el: ElementData): PortData[] {
  return el.ports ? el.ports.map((p) => ({ ...p })) : [];
}

// ── Validation issue ──────────────────────────────────────────────

export interface ConnectionIssue {
  code:
    | "PORT_NOT_FOUND"
    | "DIRECTION_MISMATCH"
    | "TYPE_INCOMPATIBLE"
    | "DUPLICATE_WIRE"
    | "CYCLE_DETECTED"
    | "FAN_OUT_WARNING";
  message: string;
  /** "error" = wire must NOT be created.  "warning" = wire can be created but is suspicious. */
  severity: "error" | "warning";
}

// ── Unified validation ────────────────────────────────────────────

/**
 * Validate a proposed connection.  Returns all issues found.
 *
 * Callers should:
 * - Block the connection when ANY issue has `severity: "error"`
 * - Log a console warning for `severity: "warning"` issues
 */
export function validateConnection(
  fromEl: ElementData,
  fromPort: string,
  toEl: ElementData,
  toPort: string,
  existingWires: WireData[],
  fromPorts: PortData[],
  toPorts: PortData[],
): ConnectionIssue[] {
  const issues: ConnectionIssue[] = [];

  const sourcePort = fromPorts.find((p) => p.name === fromPort && p.direction === "out");
  const targetPort = toPorts.find((p) => p.name === toPort && p.direction === "in");

  // 1. Port existence
  if (fromPorts.length > 0 && !sourcePort) {
    issues.push({
      code: "PORT_NOT_FOUND",
      message: `Source port "${fromPort}" not found on <${fromEl.tag}>`,
      severity: "warning",
    });
  }
  if (toPorts.length > 0 && !targetPort) {
    issues.push({
      code: "PORT_NOT_FOUND",
      message: `Target port "${toPort}" not found on <${toEl.tag}>`,
      severity: "warning",
    });
  }

  if (sourcePort && targetPort) {
    // 2. Data type compatibility
    if (!isTypeCompatible(sourcePort.type, targetPort.type)) {
      issues.push({
        code: "TYPE_INCOMPATIBLE",
        message: typeMismatchMessage(sourcePort.type, targetPort.type),
        severity: "error",
      });
    }
  }

  // 4. Duplicate wire detection
  const hasDuplicate = existingWires.some(
    (w) =>
      w.fromId === fromEl.id &&
      w.fromPort === fromPort &&
      w.toId === toEl.id &&
      w.toPort === toPort,
  );
  if (hasDuplicate) {
    issues.push({
      code: "DUPLICATE_WIRE",
      message: `Wire already exists: ${fromEl.id}.${fromPort} → ${toEl.id}.${toPort}`,
      severity: "error",
    });
  }

  // 5. Cycle detection (warning only — data cycles may be intentional)
  if (wouldCreateCycle(fromEl.id, toEl.id, existingWires)) {
    issues.push({
      code: "CYCLE_DETECTED",
      message: `Connecting ${fromEl.id}.${fromPort} → ${toEl.id}.${toPort} would create a data cycle`,
      severity: "warning",
    });
  }

  // 6. Fan-out warning (one output port driving multiple inputs)
  const fanOut = existingWires.filter((w) => w.fromId === fromEl.id && w.fromPort === fromPort);
  if (fanOut.length > 0) {
    issues.push({
      code: "FAN_OUT_WARNING",
      message:
        `Port "${fromPort}" on <${fromEl.tag}> already connected to ${fanOut.length} target(s): ` +
        fanOut.map((w) => `${w.toId}.${w.toPort}`).join(", ") +
        `. Fan-out from a single output port is often unintended.`,
      severity: "warning",
    });
  }

  return issues;
}

// ── Cycle detection ───────────────────────────────────────────────

/**
 * Detect whether adding an edge `fromId → toId` would create a cycle.
 * Uses BFS backward from `toId` following existing outgoing edges.
 */
export function wouldCreateCycle(
  fromId: string,
  toId: string,
  existingWires: WireData[],
): boolean {
  // Build reverse adjacency: for each wire, toId can reach fromId
  const predMap = new Map<string, string[]>();
  for (const w of existingWires) {
    const list = predMap.get(w.toId);
    if (list) {
      list.push(w.fromId);
    } else {
      predMap.set(w.toId, [w.fromId]);
    }
  }

  // BFS backward from fromId through predecessors — if we can reach toId,
  // then toId already reaches fromId forward, so adding fromId→toId creates a cycle.
  const visited = new Set<string>();
  const queue = [fromId];
  visited.add(fromId);

  while (queue.length > 0) {
    const current = queue.shift()!;
    const predecessors = predMap.get(current) || [];
    for (const pred of predecessors) {
      if (pred === toId) return true;
      if (!visited.has(pred)) {
        visited.add(pred);
        queue.push(pred);
      }
    }
  }
  return false;
}
