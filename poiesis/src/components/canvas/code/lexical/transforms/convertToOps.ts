import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine } from "../../../../../core/types/editor-node";
import { uid } from "../../../../../core/utils/id";

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
      return { kind: "raw", pid: node.pid, source: "" };
  }
}

// ── Extract helpers ──

function findSlot(children: EditorNode[], slotKind: string): InputSlot | undefined {
  return children.find(c => c.kind === "slot" && (c as InputSlot).slotKind === slotKind) as InputSlot | undefined;
}

function findInline(children: EditorNode[], cardLabel?: string): InlineNode | undefined {
  return children.find(c => {
    if (c.kind !== "inline") return false;
    if (cardLabel !== undefined) return (c as InlineNode).cardLabel === cardLabel;
    return true;
  }) as InlineNode | undefined;
}

function hasInline(children: EditorNode[], cardLabel: string): boolean {
  return children.some(c => c.kind === "inline" && (c as InlineNode).cardLabel === cardLabel);
}

// ── Container converters ──

function convertLetContainer(node: ContainerNode): CodeOp {
  const mutable = hasInline(node.children, "mut");
  const nameSlot = findSlot(node.children, "name");
  const valueSlot = findSlot(node.children, "expression");

  // type is the inline whose cardLabel is after the ":" separator
  let varType: string | undefined;
  for (const child of node.children) {
    if (child.kind !== "inline") continue;
    const label = (child as InlineNode).cardLabel;
    if (label !== "mut" && label !== "=" && label !== ":" && label !== "+=" && label !== "-=" && label !== "*=" && label !== "/=" && label !== "%=") {
      // This could be our type — after the ":" separator
      // Only take it if preceded by ":"
      const sepIdx = node.children.findIndex(c => c.kind === "inline" && (c as InlineNode).cardLabel === ":");
      const typeIdx = node.children.indexOf(child);
      if (sepIdx >= 0 && typeIdx === sepIdx + 1) {
        varType = label;
      }
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
  let operator = "=" as CodeOp["operator"];
  for (const child of node.children) {
    if (child.kind !== "inline") continue;
    const label = (child as InlineNode).cardLabel;
    if (/^(=|\+=|-=|\*=|\/=|%=)$/.test(label)) {
      operator = label as any;
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
