// poiesis/src/components/canvas/code/lexical/transforms/convertToNodes.ts
import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine } from "../../../../../core/types/editor-node";
import { uid } from "../../../../../core/utils/id";

export function convertToNodes(ops: CodeOp[]): EditorNode[] {
  return ops.map(op => convertOneOp(op));
}

function convertOneOp(op: CodeOp): EditorNode {
  switch (op.kind) {
    case "declare_var": return convertDeclareVar(op);
    case "set_var": return convertSetVar(op);
    case "if_block": return convertIfBlock(op);
    case "while": return convertWhile(op);
    case "for_in": return convertForIn(op);
    case "return": return convertReturn(op);
    case "call_fn": return convertCallFn(op);
    case "await": return convertAwait(op);
    case "break": return inline("break", "break", op.pid);
    case "continue": return inline("continue", "continue", op.pid);
    case "comment": return inline("comment", op.text, op.pid);
    // raw → "comment" kind; EditorNode lacks "raw" variant by design
    case "raw": return inline("comment", op.source, op.pid);
  }
}

// ── Helpers ──

function slot(slotKind: InputSlot["slotKind"], value = "", pid?: string): InputSlot {
  return { kind: "slot", slotKind, value, pid: pid ?? uid("sl") };
}

function inline(kind: InlineNode["kind"], cardLabel: string, pid?: string): InlineNode {
  return { kind, cardLabel, pid: pid ?? uid("in") };
}

function container(kind: ContainerNode["kind"], children: EditorNode[], pid: string): ContainerNode {
  return { kind, pid, children };
}

function block(kind: BlockNode["kind"], head: EditorNode[], body: BodyLine[], tailPid: string, pid: string, branches?: BlockNode["branches"]): BlockNode {
  return { kind, pid, head, body, tail: { pid: tailPid }, branches };
}

function bodyLine(children: EditorNode[]): BodyLine {
  return { pid: uid("bl"), children };
}

// ── Op converters ──

function convertDeclareVar(op: Extract<CodeOp, { kind: "declare_var" }>): ContainerNode {
  const children: EditorNode[] = [];
  children.push(inline("keyword", "let"));  // keyword card
  if (op.mutable) children.push(inline("mut", "mut"));
  children.push(slot("name", op.name));
  if (op.varType) {
    children.push(inline("separator", ":"));
    children.push(inline("type", op.varType));
  }
  if (op.initValue !== undefined) {
    children.push(inline("operator", "="));
    children.push(slot("expression", op.initValue));
  }
  return container("let", children, op.pid);
}

function convertSetVar(op: Extract<CodeOp, { kind: "set_var" }>): ContainerNode {
  return container("set", [
    inline("keyword", "set"),
    slot("name", op.name),
    inline("operator", op.operator),
    slot("expression", op.value),
  ], op.pid);
}

function convertIfBlock(op: Extract<CodeOp, { kind: "if_block" }>): BlockNode {
  if (op.branches.length === 0) {
    return block("if", [inline("keyword", "if")], [bodyLine([])], uid("tl"), op.pid);
  }
  const first = op.branches[0];
  const branches = op.branches.length > 1
    ? op.branches.slice(1).map(br => ({
        kind: br.condition ? "elif" as const : "else" as const,
        pid: uid("br"),
        head: br.condition
          ? [inline("keyword", br.condition ? "elif" : "else"), slot("condition", br.condition)]
          : [inline("keyword", "else")],
        body: br.body.map(bodyOp => bodyLine([convertOneOp(bodyOp)])),
      }))
    : undefined;

  return block(
    "if",
    first.condition
      ? [inline("keyword", "if"), slot("condition", first.condition)]
      : [inline("keyword", "if")],
    first.body.length > 0
      ? first.body.map(bodyOp => bodyLine([convertOneOp(bodyOp)]))
      : [bodyLine([])],
    uid("tl"),
    op.pid,
    branches,
  );
}

function convertWhile(op: Extract<CodeOp, { kind: "while" }>): BlockNode {
  return block(
    "while",
    [inline("keyword", "while"), slot("condition", op.condition)],
    op.body.length > 0
      ? op.body.map(o => bodyLine([convertOneOp(o)]))
      : [bodyLine([])],
    uid("tl"),
    op.pid,
  );
}

function convertForIn(op: Extract<CodeOp, { kind: "for_in" }>): BlockNode {
  return block(
    "for",
    [
      inline("keyword", "for"),
      slot("name", op.itemName),
      inline("separator", "in"),
      slot("expression", op.collection),
    ],
    op.body.length > 0
      ? op.body.map(o => bodyLine([convertOneOp(o)]))
      : [bodyLine([])],
    uid("tl"),
    op.pid,
  );
}

function convertReturn(op: Extract<CodeOp, { kind: "return" }>): ContainerNode {
  return container("return", [
    inline("keyword", "return"),
    slot("expression", op.value ?? ""),
  ], op.pid);
}

function convertCallFn(op: Extract<CodeOp, { kind: "call_fn" }>): ContainerNode {
  const children: EditorNode[] = [inline("keyword", "call"), slot("fn_name", op.fnName)];
  for (let i = 0; i < op.args.length; i++) {
    if (i > 0) children.push(inline("separator", ","));
    children.push(slot("expression", op.args[i]));
  }
  return container("call", children, op.pid);
}

function convertAwait(op: Extract<CodeOp, { kind: "await" }>): ContainerNode {
  return container("await", [
    inline("keyword", "await"),
    slot("expression", op.value),
  ], op.pid);
}
