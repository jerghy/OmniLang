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

/**
 * Build a PidMap from an array of PidEntry values.
 * Populates both the entries lookup and the line-to-pid reverse lookup.
 */
function buildPidMap(entries: PidEntry[]): PidMap {
  const entriesMap: Record<string, PidEntry> = {};
  const lineToPid: Record<number, string> = {};
  for (const entry of entries) {
    entriesMap[entry.pid] = entry;
    for (let line = entry.startLine; line <= entry.endLine; line++) {
      lineToPid[line] = entry.pid;
    }
  }
  return { status: "ok", entries: entriesMap, lineToPid };
}

/**
 * Resolve a PID to its entry in the map.
 * Returns undefined for error pidMaps.
 */
function resolvePid(pidMap: PidMap, pid: string): PidEntry | undefined {
  if (pidMap.status !== "ok") return undefined;
  return pidMap.entries[pid];
}

/**
 * Resolve a line number to its PidEntry.
 * Returns undefined for error pidMaps.
 */
function resolvePidByLine(
  pidMap: PidMap,
  line: number,
): PidEntry | undefined {
  if (pidMap.status !== "ok") return undefined;
  const pid = pidMap.lineToPid[line];
  return pid ? pidMap.entries[pid] : undefined;
}

/**
 * Merge multiple PidMaps into one.
 * Error pidMaps are skipped. Later maps overwrite earlier ones for the same key.
 */
function mergePidMaps(maps: PidMap[]): PidMap {
  const entries: Record<string, PidEntry> = {};
  const lineToPid: Record<number, string> = {};
  for (const map of maps) {
    if (map.status !== "ok") continue;
    Object.assign(entries, map.entries);
    Object.assign(lineToPid, map.lineToPid);
  }
  return { status: "ok", entries, lineToPid };
}

/**
 * Detect duplicate PIDs across multiple PidMaps.
 * Returns an array of PID strings that appear more than once.
 * Error pidMaps are skipped.
 */
function detectPidConflicts(
  sourcePidMaps: Record<string, PidMap>,
): Array<{ pid: string; sources: string[] }> {
  const pidToSources: Record<string, string[]> = {};

  for (const [sourceName, pidMap] of Object.entries(sourcePidMaps)) {
    if (pidMap.status !== "ok") continue;
    for (const pid of Object.keys(pidMap.entries)) {
      if (!pidToSources[pid]) {
        pidToSources[pid] = [];
      }
      pidToSources[pid].push(sourceName);
    }
  }

  const result: Array<{ pid: string; sources: string[] }> = [];
  for (const [pid, sources] of Object.entries(pidToSources)) {
    if (sources.length > 1) result.push({ pid, sources });
  }
  return result;
}
