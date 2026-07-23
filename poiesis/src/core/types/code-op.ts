// poiesis/src/core/types/code-op.ts
// Structured operation types for CodeCard Editor.
// Each operation maps to a single OmniLang statement/expression inside a code-block.

export type ScopeKind = "fn" | "container";

// ── Base ──

interface CodeOpBase {
  pid: string; // 编辑器内部 ID，不写入 .omni
}

// ── Concrete operations ──

export interface DeclareVarOp extends CodeOpBase {
  kind: "declare_var";
  mutable: boolean;
  name: string;
  varType?: string;
  initValue?: string;
  scope: ScopeKind;
}

export interface SetVarOp extends CodeOpBase {
  kind: "set_var";
  name: string;
  operator: "=" | "+=" | "-=" | "*=" | "/=" | "%=";
  value: string;
}

export interface IfBlockOp extends CodeOpBase {
  kind: "if_block";
  branches: {
    condition: string;
    body: CodeOp[];
  }[];
}

export interface WhileLoopOp extends CodeOpBase {
  kind: "while";
  condition: string;
  body: CodeOp[];
}

export interface ForLoopOp extends CodeOpBase {
  kind: "for_in";
  itemName: string;
  collection: string;
  body: CodeOp[];
}

export interface ReturnExprOp extends CodeOpBase {
  kind: "return";
  value?: string;
}

export interface CallFnOp extends CodeOpBase {
  kind: "call_fn";
  fnName: string;
  args: string[];
}

export interface AwaitExprOp extends CodeOpBase {
  kind: "await";
  value: string;
}

export interface BreakStmtOp extends CodeOpBase {
  kind: "break";
}

export interface ContinueStmtOp extends CodeOpBase {
  kind: "continue";
}

export interface CommentOp extends CodeOpBase {
  kind: "comment";
  text: string;
}

export interface RawCodeOp extends CodeOpBase {
  kind: "raw";
  source: string;
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
