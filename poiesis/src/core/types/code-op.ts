// poiesis/src/core/types/code-op.ts
// Structured operation types for CodeCard Editor.
// Each operation maps to a single OmniLang statement/expression inside a code-block.

import type { SlotValue } from "./slot-value";

export type ScopeKind = "fn" | "container";

// ── Base ──

interface CodeOpBase {
  pid: string; // 编辑器内部 ID，不写入 .omni
}

// ── Concrete operations ──

export interface DeclareVarOp extends CodeOpBase {
  kind: "declare_var";
  mutable: boolean;
  name: SlotValue; // text slot
  varType?: SlotValue; // text slot
  initValue?: SlotValue; // expr slot
  scope: ScopeKind;
}

export interface SetVarOp extends CodeOpBase {
  kind: "set_var";
  name: SlotValue; // text slot
  operator: "=" | "+=" | "-=" | "*=" | "/=" | "%=";
  value: SlotValue; // expr slot
}

export interface IfBlockOp extends CodeOpBase {
  kind: "if_block";
  branches: {
    condition: SlotValue; // expr slot
    body: CodeOp[];
  }[];
  bodyId: string;
}

export interface WhileLoopOp extends CodeOpBase {
  kind: "while";
  condition: SlotValue; // expr slot
  body: CodeOp[];
  bodyId: string;
}

export interface ForLoopOp extends CodeOpBase {
  kind: "for_in";
  itemName: SlotValue; // text slot
  collection: SlotValue; // expr slot
  body: CodeOp[];
  bodyId: string;
}

export interface ReturnExprOp extends CodeOpBase {
  kind: "return";
  value?: SlotValue; // expr slot
}

export interface CallFnOp extends CodeOpBase {
  kind: "call_fn";
  fnName: SlotValue; // text slot
  args: SlotValue[]; // expr slots
}

export interface AwaitExprOp extends CodeOpBase {
  kind: "await";
  value: SlotValue; // expr slot
}

export interface BreakStmtOp extends CodeOpBase {
  kind: "break";
}

export interface ContinueStmtOp extends CodeOpBase {
  kind: "continue";
}

export interface CommentOp extends CodeOpBase {
  kind: "comment";
  text: string; // plain string (not a slot)
}

export interface RawCodeOp extends CodeOpBase {
  kind: "raw";
  source: string; // plain string (not a slot)
}

export type CodeOp =
  | DeclareVarOp
  | SetVarOp
  | IfBlockOp
  | WhileLoopOp
  | ForLoopOp
  | ReturnExprOp
  | CallFnOp
  | AwaitExprOp
  | BreakStmtOp
  | ContinueStmtOp
  | CommentOp
  | RawCodeOp;

export function createBodyId(): string {
  return "body-" + Math.random().toString(36).slice(2, 10);
}
