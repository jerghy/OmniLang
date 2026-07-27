// services/pid/pid-index.ts
// PID generation and management utilities.
// PID (Persistent IDentifier) is a stable identifier for each semantic block in .omni files.

import type { PidBlockType } from "../../core/types/pid";

/**
 * Generate a PID for a top-level or sub-block.
 *
 * Rules:
 *   component → c<N> or c<N>_n<M>
 *   store     → s<N> or s<N>_a<M>
 *   lib       → l<N> or l<N>_n<M>
 *   other types fall back to generic x<N> / x<N>_n<M>
 *
 * @param fileIndex  The appearance order of the parent block in the file (1-based).
 * @param blockType  The type of block.
 * @param subIndex   Optional sub-block index (e.g. 1=signal, 2=computed, etc. for component).
 * @returns A stable PID string.
 */
export function generatePid(
  fileIndex: number,
  blockType: PidBlockType,
  subIndex?: number,
): string {
  const prefix = getBlockPrefix(blockType);
  if (subIndex !== undefined) {
    const subPrefix = blockType === "store" ? "a" : blockType === "view" ? "v" : "n";
    return `${prefix}${fileIndex}_${subPrefix}${subIndex}`;
  }
  return `${prefix}${fileIndex}`;
}

/**
 * Map a PidBlockType to its PID prefix character.
 * Each block type gets a unique prefix for stable identification.
 */
function getBlockPrefix(blockType: PidBlockType): string {
  switch (blockType) {
    case "component":  return "c";
    case "store":      return "s";
    case "lib":        return "l";
    case "view":       return "v";
    case "signal":     return "sg";
    case "computed":   return "cp";
    case "prop":       return "p";
    case "fn":         return "f";
    case "event":      return "e";
    case "style":      return "st";
    case "getter":     return "g";
    case "action":     return "a";
    case "struct":     return "t";
    case "type_alias": return "ta";
    case "pub_fn":     return "pf";
    default:           return "x";
  }
}

