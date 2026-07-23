import { describe, it, expect, expectTypeOf } from "vitest";
import type {
  EditorNode,
  ContainerNode,
  BlockNode,
  InlineNode,
  InputSlot,
  BodyLine,
  TailLine,
  Branch,
  ContainerKind,
  BlockKind,
  InlineKind,
  SlotKind,
} from "../editor-node";

describe("EditorNode type system", () => {
  // ── Container nodes ──

  it("should create a let container node", () => {
    const node: ContainerNode = {
      kind: "let",
      pid: "pid-1",
      children: [],
    };
    expect(node.kind).toBe("let");
    expect(node.pid).toBe("pid-1");
    expect(node.children).toEqual([]);
  });

  it("should create a set container node with inline children", () => {
    const node: ContainerNode = {
      kind: "set",
      pid: "pid-2",
      children: [
        { kind: "var_ref", cardLabel: "x", pid: "pid-3" },
        { kind: "operator", cardLabel: "=", pid: "pid-4" },
        { kind: "literal", cardLabel: "42", pid: "pid-5" },
      ],
    };
    expect(node.kind).toBe("set");
    expect(node.children).toHaveLength(3);
  });

  it("should create all container kinds", () => {
    const kinds: ContainerKind[] = ["let", "set", "return", "await", "call", "array", "map"];
    for (const kind of kinds) {
      const node: ContainerNode = { kind, pid: `pid-${kind}`, children: [] };
      expect(node.kind).toBe(kind);
    }
  });

  // ── Block nodes ──

  it("should create an if block with head, body, and tail", () => {
    const node: BlockNode = {
      kind: "if",
      pid: "pid-if-1",
      head: [
        { kind: "slot", slotKind: "condition", value: "x > 0", pid: "pid-slot-1" },
      ],
      body: [
        { pid: "pid-body-1", children: [] },
      ],
      tail: { pid: "pid-tail-1" },
    };
    expect(node.kind).toBe("if");
    expect(node.head).toHaveLength(1);
    expect(node.body).toHaveLength(1);
    expect(node.tail.pid).toBe("pid-tail-1");
  });

  it("should create a while block without branches", () => {
    const node: BlockNode = {
      kind: "while",
      pid: "pid-while-1",
      head: [],
      body: [],
      tail: { pid: "pid-tail-2" },
    };
    expect(node.kind).toBe("while");
    expect(node.branches).toBeUndefined();
  });

  it("should create a for block", () => {
    const node: BlockNode = {
      kind: "for",
      pid: "pid-for-1",
      head: [
        { kind: "slot", slotKind: "expression", value: "item in items", pid: "pid-slot-2" },
      ],
      body: [
        { pid: "pid-body-2", children: [] },
      ],
      tail: { pid: "pid-tail-3" },
    };
    expect(node.kind).toBe("for");
    expect(node.head[0].kind).toBe("slot");
  });

  it("should support all block kinds via BlockKind helper", () => {
    const kinds: BlockKind[] = ["if", "while", "for"];
    expect(kinds).toHaveLength(3);
  });

  // ── Branches (elif / else) ──

  it("should create an if block with elif and else branches", () => {
    const node: BlockNode = {
      kind: "if",
      pid: "pid-if-2",
      head: [
        { kind: "slot", slotKind: "condition", value: "score > 90", pid: "pid-slot-3" },
      ],
      body: [{ pid: "pid-body-3", children: [] }],
      tail: { pid: "pid-tail-4" },
      branches: [
        {
          kind: "elif",
          pid: "pid-elif-1",
          head: [
            { kind: "slot", slotKind: "condition", value: "score > 70", pid: "pid-slot-4" },
          ],
          body: [{ pid: "pid-body-4", children: [] }],
        },
        {
          kind: "else",
          pid: "pid-else-1",
          head: [],
          body: [{ pid: "pid-body-5", children: [] }],
        },
      ],
    };
    const branches = node.branches;
    expect(branches).toBeDefined();
    if (branches) {
      expect(branches).toHaveLength(2);
      expect(branches[0].kind).toBe("elif");
      expect(branches[1].kind).toBe("else");
      expect(branches[0].head).toHaveLength(1);
    }
  });

  it("should create a Branch with correct structure", () => {
    const branch: Branch = {
      kind: "elif",
      pid: "pid-branch-1",
      head: [],
      body: [{ pid: "pid-branch-body-1", children: [] }],
    };
    expect(branch.kind).toBe("elif");
    expectTypeOf(branch).toHaveProperty("kind");
    expectTypeOf(branch).toHaveProperty("pid");
    expectTypeOf(branch).toHaveProperty("head");
    expectTypeOf(branch).toHaveProperty("body");
  });

  // ── Inline nodes ──

  it("should create inline nodes of various kinds", () => {
    const nodes: InlineNode[] = [
      { kind: "mut", cardLabel: "mut", pid: "pid-mut-1" },
      { kind: "break", cardLabel: "break", pid: "pid-break-1" },
      { kind: "continue", cardLabel: "continue", pid: "pid-cont-1" },
      { kind: "operator", cardLabel: "+=", pid: "pid-op-1" },
      { kind: "type", cardLabel: "i32", pid: "pid-type-1" },
      { kind: "var_ref", cardLabel: "counter", pid: "pid-var-1" },
      { kind: "literal", cardLabel: "42", pid: "pid-lit-1" },
      { kind: "dot", cardLabel: ".", pid: "pid-dot-1" },
      { kind: "separator", cardLabel: ",", pid: "pid-sep-1" },
      { kind: "comment", cardLabel: "// TODO", pid: "pid-comment-1" },
    ];

    expect(nodes).toHaveLength(10);
    expect(nodes[0].kind).toBe("mut");
    expect(nodes[4].cardLabel).toBe("i32");
    expect(nodes[9].cardLabel).toBe("// TODO");
  });

  it("should support all inline kinds via InlineKind helper", () => {
    const kinds: InlineKind[] = [
      "mut", "break", "continue", "operator", "type",
      "var_ref", "literal", "dot", "separator", "comment",
    ];
    expect(kinds).toHaveLength(10);
  });

  // ── Input slots ──

  it("should create an InputSlot of various slot kinds", () => {
    const slots: InputSlot[] = [
      { kind: "slot", slotKind: "name", value: "", pid: "pid-slot-name" },
      { kind: "slot", slotKind: "type_value", value: "i32", pid: "pid-slot-type" },
      { kind: "slot", slotKind: "expression", value: "a + b", pid: "pid-slot-expr" },
      { kind: "slot", slotKind: "condition", value: "x > 0", pid: "pid-slot-cond" },
      { kind: "slot", slotKind: "fn_name", value: "myFunc", pid: "pid-slot-fn" },
    ];

    expect(slots).toHaveLength(5);
    expect(slots[0].slotKind).toBe("name");
    expect(slots[3].value).toBe("x > 0");
    expect(slots[4].slotKind).toBe("fn_name");
  });

  it("should support all slot kinds via SlotKind helper", () => {
    const kinds: SlotKind[] = ["name", "type_value", "expression", "condition", "fn_name"];
    expect(kinds).toHaveLength(5);
  });

  // ── EditorNode union ──

  it("should accept all node types as EditorNode", () => {
    const nodes: EditorNode[] = [
      { kind: "let", pid: "p1", children: [] },
      { kind: "if", pid: "p2", head: [], body: [], tail: { pid: "p-tail" } },
      { kind: "mut", cardLabel: "mut", pid: "p3" },
      { kind: "slot", slotKind: "name", value: "", pid: "p4" },
    ];

    expect(nodes).toHaveLength(4);
    expect(nodes[0].kind).toBe("let");
    expect(nodes[1].kind).toBe("if");
    expect(nodes[2].kind).toBe("mut");
    expect(nodes[3].kind).toBe("slot");
  });

  it("should allow discriminated union narrowing on EditorNode", () => {
    const node: EditorNode = { kind: "let", pid: "p1", children: [] };

    if (node.kind === "let" || node.kind === "set" || node.kind === "return" || node.kind === "await" || node.kind === "call" || node.kind === "array" || node.kind === "map") {
      // ContainerNode — has children
      expectTypeOf(node).toHaveProperty("children");
      expect(node.children).toEqual([]);
    }
  });

  // ── BodyLine / TailLine ──

  it("should create BodyLine and TailLine", () => {
    const bodyLine: BodyLine = {
      pid: "pid-bl-1",
      children: [
        { kind: "var_ref", cardLabel: "result", pid: "pid-inline-1" },
        { kind: "operator", cardLabel: "=", pid: "pid-inline-2" },
        { kind: "literal", cardLabel: "true", pid: "pid-inline-3" },
      ],
    };
    expect(bodyLine.pid).toBe("pid-bl-1");
    expect(bodyLine.children).toHaveLength(3);

    const tail: TailLine = { pid: "pid-tail-5" };
    expect(tail.pid).toBe("pid-tail-5");
  });

  // ── Structural type assertions ──

  it("should have correct ContainerNode type structure", () => {
    expectTypeOf<ContainerNode>().toHaveProperty("kind");
    expectTypeOf<ContainerNode>().toHaveProperty("pid");
    expectTypeOf<ContainerNode>().toHaveProperty("children");
  });

  it("should have correct BlockNode type structure", () => {
    expectTypeOf<BlockNode>().toHaveProperty("kind");
    expectTypeOf<BlockNode>().toHaveProperty("pid");
    expectTypeOf<BlockNode>().toHaveProperty("head");
    expectTypeOf<BlockNode>().toHaveProperty("body");
    expectTypeOf<BlockNode>().toHaveProperty("tail");
  });

  it("should have correct InlineNode type structure", () => {
    expectTypeOf<InlineNode>().toHaveProperty("kind");
    expectTypeOf<InlineNode>().toHaveProperty("cardLabel");
    expectTypeOf<InlineNode>().toHaveProperty("pid");
  });

  it("should have correct InputSlot type structure", () => {
    expectTypeOf<InputSlot>().toHaveProperty("kind");
    expectTypeOf<InputSlot>().toHaveProperty("slotKind");
    expectTypeOf<InputSlot>().toHaveProperty("value");
    expectTypeOf<InputSlot>().toHaveProperty("pid");
  });
});
