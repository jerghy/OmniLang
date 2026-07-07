// PID annotation system types

export type PidBlockType =
  | "component" | "store" | "lib" | "signal" | "computed" | "prop"
  | "fn" | "event" | "view" | "style" | "getter" | "action"
  | "struct" | "type_alias" | "pub_fn"
  | "if" | "for" | "slot";

export interface PidEntry {
  pid: string;           // e.g. "c1", "c1_n1", "s1", "s1_a1"
  blockType: PidBlockType;
  blockName: string;
  sourceFile: string;    // relative path from project root
  startLine: number;
  endLine: number;
}

export type PidMap =
  | { status: "ok"; entries: Record<string, PidEntry>; lineToPid: Record<number, string> }
  | { status: "error"; error: string };
