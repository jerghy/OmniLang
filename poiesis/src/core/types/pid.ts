// PID annotation system types

export type PidBlockType =
  | "component" | "store" | "signal" | "computed" | "prop"
  | "fn" | "event" | "view" | "style" | "getter" | "action"
  | "struct" | "type_alias" | "pub_fn";

export interface PidEntry {
  pid: string;           // e.g. "c1", "c1_n1", "s1", "s1_a1"
  blockType: PidBlockType;
  blockName: string;
  sourceFile: string;    // relative path from project root
  startLine: number;
  endLine: number;
}

export interface PidMap {
  entries: Record<string, PidEntry>;
  /** line number → pid */
  lineToPid: Record<number, string>;
}
