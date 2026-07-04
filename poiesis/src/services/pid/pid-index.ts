// services/pid/pid-index.ts
// PID generation and management utilities.
// PID (Persistent IDentifier) is a stable identifier for each semantic block in .omni files.

import type { PidEntry, PidMap, PidBlockType } from "../../core/types/pid";

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
    const subPrefix = blockType === "store" ? "a" : "n";
    return `${prefix}${fileIndex}_${subPrefix}${subIndex}`;
  }
  return `${prefix}${fileIndex}`;
}

/**
 * Map a PidBlockType to its PID prefix character.
 */
function getBlockPrefix(blockType: PidBlockType): string {
  switch (blockType) {
    case "component": return "c";
    case "store":     return "s";
    case "lib":       return "l";
    default:          return "x"; // fallback for sub-block types
  }
}

/**
 * Build a PidMap from an array of PidEntry values.
 * Populates both the entries lookup and the line-to-pid reverse lookup.
 */
export function buildPidMap(entries: PidEntry[]): PidMap {
  const pidMap: PidMap = { entries: {}, lineToPid: {} };
  for (const entry of entries) {
    pidMap.entries[entry.pid] = entry;
    for (let line = entry.startLine; line <= entry.endLine; line++) {
      pidMap.lineToPid[line] = entry.pid;
    }
  }
  return pidMap;
}

/**
 * Resolve a PID to its entry in the map.
 */
export function resolvePid(pidMap: PidMap, pid: string): PidEntry | undefined {
  return pidMap.entries[pid];
}

/**
 * Resolve a line number to its PidEntry.
 */
export function resolvePidByLine(
  pidMap: PidMap,
  line: number,
): PidEntry | undefined {
  const pid = pidMap.lineToPid[line];
  return pid ? pidMap.entries[pid] : undefined;
}

/**
 * Merge multiple PidMaps into one.
 * Later maps overwrite earlier ones for the same key.
 */
export function mergePidMaps(maps: PidMap[]): PidMap {
  const merged: PidMap = { entries: {}, lineToPid: {} };
  for (const map of maps) {
    Object.assign(merged.entries, map.entries);
    Object.assign(merged.lineToPid, map.lineToPid);
  }
  return merged;
}
