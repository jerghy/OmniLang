import type { SlotValue } from "./slot-value";

export type InlineOp =
  | { kind: "var_ref"; name: string }
  | { kind: "literal"; value: string; literalType: "number" | "string" | "bool" }
  | { kind: "binary"; op: string; left: SlotValue; right: SlotValue }
  | { kind: "unary"; op: string; operand: SlotValue }
  | { kind: "call_fn"; fnName: string; args: string[] }
  | { kind: "member_access"; target: string; member: string }
  | { kind: "parens"; inner: InlineOp[] };
