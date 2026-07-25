import type { InlineOp } from "./inline-op";

export type SlotValue =
  | { kind: "text"; value: string }
  | { kind: "expr"; children: InlineOp[] }
  | { kind: "empty" };

export function fromLegacyString(s: string | undefined): SlotValue {
  if (s === undefined || s === "") return { kind: "empty" };
  return { kind: "text", value: s };
}

export function slotToText(sv: SlotValue): string {
  switch (sv.kind) {
    case "text": return sv.value;
    case "expr": return sv.children.map(opToText).join(" ");
    case "empty": return "";
  }
}

function opToText(op: InlineOp): string {
  switch (op.kind) {
    case "var_ref": return op.name;
    case "literal": return op.value;
    case "binary": return `${slotToText(op.left)} ${op.op} ${slotToText(op.right)}`;
    case "unary": return `${op.op}${slotToText(op.operand)}`;
    case "call_fn": return `${op.fnName}(${op.args.join(", ")})`;
    case "member_access": return `${op.target}.${op.member}`;
    case "parens": return `(${op.inner.map(opToText).join(" ")})`;
  }
}
