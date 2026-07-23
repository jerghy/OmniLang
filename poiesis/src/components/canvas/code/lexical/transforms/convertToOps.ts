import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine } from "../../../../../core/types/editor-node";

export function convertToOps(nodes: EditorNode[]): CodeOp[] {
  return nodes.map(node => convertOneNode(node));
}

function convertOneNode(node: EditorNode): CodeOp {
  switch (node.kind) {
    case "let": return convertLetContainer(node as ContainerNode);
    case "set": return convertSetContainer(node as ContainerNode);
    case "return": return convertReturnContainer(node as ContainerNode);
    case "await": return convertAwaitContainer(node as ContainerNode);
    case "call": return convertCallContainer(node as ContainerNode);
    case "if": return convertIfBlock(node as BlockNode);
    case "while": return convertWhileBlock(node as BlockNode);
    case "for": return convertForBlock(node as BlockNode);
    case "break": return { kind: "break", pid: node.pid };
    case "continue": return { kind: "continue", pid: node.pid };
    case "comment": return { kind: "comment", pid: node.pid, text: (node as InlineNode).cardLabel };
    default:
      console.warn(`convertToOps: unknown node kind "${(node as any).kind}", falling back to raw`);
      return { kind: "raw", pid: node.pid, source: "" };
  }
}

// ── Type guard ──

function isInlineNode(c: EditorNode): c is InlineNode {
  return "cardLabel" in c;
}

// ── Extract helpers ──

function findSlot(children: EditorNode[], slotKind: string): InputSlot | undefined {
  return children.find(c => c.kind === "slot" && (c as InputSlot).slotKind === slotKind) as InputSlot | undefined;
}

function hasInline(children: EditorNode[], cardLabel: string): boolean {
  return children.some(c => isInlineNode(c) && c.cardLabel === cardLabel);
}

// ── Container converters ──

function convertLetContainer(node: ContainerNode): CodeOp {
  const mutable = hasInline(node.children, "mut");
  const nameSlot = findSlot(node.children, "name");
  const valueSlot = findSlot(node.children, "expression");

  // type is the inline whose cardLabel is after the ":" separator
  let varType: string | undefined;
  const sepIdx = node.children.findIndex(c => isInlineNode(c) && c.cardLabel === ":");
  if (sepIdx >= 0 && sepIdx + 1 < node.children.length) {
    const next = node.children[sepIdx + 1];
    if (isInlineNode(next)) {
      varType = next.cardLabel;
    }
  }

  return {
    kind: "declare_var",
    pid: node.pid,
    mutable,
    name: nameSlot?.value ?? "x",
    varType,
    initValue: valueSlot?.value,
    scope: "fn",
  };
}

function convertSetContainer(node: ContainerNode): CodeOp {
  const nameSlot = findSlot(node.children, "name");
  // operator is the inline with label matching an operator pattern
  let operator: Extract<CodeOp, { kind: "set_var" }>["operator"] = "=";
  for (const child of node.children) {
    if (!isInlineNode(child)) continue;
    const label = child.cardLabel;
    if (/^(=|\+=|-=|\*=|\/=|%=)$/.test(label)) {
      operator = label as Extract<CodeOp, { kind: "set_var" }>["operator"];
      break;
    }
  }
  const valueSlot = findSlot(node.children, "expression");
  return {
    kind: "set_var",
    pid: node.pid,
    name: nameSlot?.value ?? "x",
    operator,
    value: valueSlot?.value ?? "",
  };
}

function convertReturnContainer(node: ContainerNode): CodeOp {
  const valSlot = findSlot(node.children, "expression");
  return {
    kind: "return",
    pid: node.pid,
    value: valSlot?.value || undefined,
  };
}

function convertAwaitContainer(node: ContainerNode): CodeOp {
  const valSlot = findSlot(node.children, "expression");
  return { kind: "await", pid: node.pid, value: valSlot?.value ?? "" };
}

function convertCallContainer(node: ContainerNode): CodeOp {
  const fnSlot = findSlot(node.children, "fn_name");
  const args: string[] = [];
  for (const child of node.children) {
    if (child.kind === "slot" && (child as InputSlot).slotKind === "expression") {
      args.push((child as InputSlot).value);
    }
  }
  return { kind: "call_fn", pid: node.pid, fnName: fnSlot?.value ?? "", args };
}

// ── Block converters ──

function convertIfBlock(node: BlockNode): CodeOp {
  const branches = [{ condition: extractCondition(node.head), body: convertBody(node.body) }];
  if (node.branches) {
    for (const br of node.branches) {
      branches.push({
        condition: br.kind === "elif" ? extractCondition(br.head) : "",
        body: convertBody(br.body),
      });
    }
  }
  return { kind: "if_block", pid: node.pid, branches };
}

function convertWhileBlock(node: BlockNode): CodeOp {
  return {
    kind: "while",
    pid: node.pid,
    condition: extractCondition(node.head),
    body: convertBody(node.body),
  };
}

function convertForBlock(node: BlockNode): CodeOp {
  const itemSlot = findSlot(node.head, "name");
  const collSlot = findSlot(node.head, "expression");
  return {
    kind: "for_in",
    pid: node.pid,
    itemName: itemSlot?.value ?? "item",
    collection: collSlot?.value ?? "",
    body: convertBody(node.body),
  };
}

function extractCondition(head: EditorNode[]): string {
  const condSlot = findSlot(head, "condition");
  return condSlot?.value ?? "";
}

function convertBody(bodyLines: BodyLine[]): CodeOp[] {
  return bodyLines.flatMap(line => line.children.map(child => convertOneNode(child)));
}
