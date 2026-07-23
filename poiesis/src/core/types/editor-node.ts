// poiesis/src/core/types/editor-node.ts
// Fine-grained EditorNode types for the CodeCard Editor v2.
// These are the internal editor format; convert to/from CodeOp[] for persistence.

export type EditorNode =
  | ContainerNode
  | BlockNode
  | InlineNode
  | InputSlot;

// ── Container cards (let, set, return, await, call, array, map) ──
export interface ContainerNode {
  kind: "let" | "set" | "return" | "await" | "call" | "array" | "map";
  pid: string;
  children: EditorNode[];
}

// ── Block cards (if, while, for) ──
export interface BlockNode {
  kind: "if" | "while" | "for";
  pid: string;
  head: EditorNode[];       // condition/loop header cards + slots
  body: BodyLine[];         // expandable code lines
  tail: TailLine;           // closing bar
  branches?: Branch[];      // elif/else (if only)
}

export interface Branch {
  kind: "elif" | "else";
  pid: string;
  head: EditorNode[];
  body: BodyLine[];
}

export interface BodyLine {
  pid: string;
  children: EditorNode[];
}

export interface TailLine {
  pid: string;
}

// ── Inline cards (every token that's not a container) ──
export interface InlineNode {
  kind: "mut" | "break" | "continue" | "operator" | "type" | "var_ref" | "literal" | "dot" | "separator" | "comment";
  cardLabel: string;        // displayed text: ">", "+=", "i32", "sum", etc.
  pid: string;
}

// ── Input slot (backslash-triggered inline input) ──
export interface InputSlot {
  kind: "slot";
  slotKind: "name" | "type_value" | "expression" | "condition" | "fn_name";
  value: string;
  pid: string;
}

// ── Helpers ──
export type ContainerKind = ContainerNode["kind"];
export type BlockKind = BlockNode["kind"];
export type InlineKind = InlineNode["kind"];
export type SlotKind = InputSlot["slotKind"];
