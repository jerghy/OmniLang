/** Migration utility: convert old .omni.json code graphs to the new pure data-flow format.
 *
 *  Old format (v1): code.elements[] with event/action/flow nodes + code.wires[] with flow edges
 *  New format (v2): codeTabs[] with only data nodes (no event/action) + data-only wires
 *
 *  Migration rules:
 *  1. Drop deprecated node types: action, onPress, onChange, fn-call, runCodeBlock
 *  2. Drop wires that reference dropped elements
 *  3. Keep all other nodes (signal, computed, store, fn, literal, etc.)
 */

import type { ElementData, WireData } from "../store/editor";
import { Logger } from "../utils/logger";

const log = new Logger({ prefix: "Migration" });

/** Node tags that existed in v1 but are removed in v2 (pure data-flow). */
const DEPRECATED_TAGS = new Set([
  "action",
  "onPress",
  "onChange",
  "fn-call",
  "runCodeBlock",
]);

export interface MigrationResult {
  elements: ElementData[];
  wires: WireData[];
  droppedElements: number;
  droppedWires: number;
}

/** Migrate old code graph elements + wires to the new pure data-flow format.
 *  Returns only valid nodes and wires; drops deprecated event/action/flow nodes. */
export function migrateOldCodeGraph(
  elements: ElementData[],
  wires: WireData[],
): MigrationResult {
  // 1. Filter out deprecated node types
  const validElements = elements.filter((el) => {
    if (el.nodeType !== "codeblock") return true; // keep non-codeblock elements
    return !DEPRECATED_TAGS.has(el.tag);
  });

  const validIds = new Set(validElements.map((e) => e.id));

  // 2. Only keep wires where both endpoints still exist
  const validWires = wires.filter(
    (w) => validIds.has(w.fromId) && validIds.has(w.toId),
  );

  const droppedElements = elements.length - validElements.length;
  const droppedWires = wires.length - validWires.length;

  if (droppedElements > 0 || droppedWires > 0) {
    log.info(
      `Cleaned up old code graph: ` +
      `dropped ${droppedElements} deprecated nodes (action/onPress/onChange/fn-call/runCodeBlock), ` +
      `dropped ${droppedWires} orphaned wires.`,
    );
  }

  return { elements: validElements, wires: validWires, droppedElements, droppedWires };
}
