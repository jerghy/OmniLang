// poiesis/src/components/canvas/code/lexical/transforms/__tests__/convertToNodes.test.ts
import { describe, it, expect } from "vitest";
import { convertToNodes } from "../convertToNodes";
import type { CodeOp } from "../../../../../../core/types/code-op";
import type { ContainerNode, BlockNode, InlineNode, InputSlot } from "../../../../../../core/types/editor-node";

describe("convertToNodes", () => {
  it("converts simple declare_var to let container with children", () => {
    const ops: CodeOp[] = [{
      kind: "declare_var", pid: "d1", mutable: false,
      name: "count", varType: "i32", initValue: "0", scope: "fn",
    }];
    const nodes = convertToNodes(ops);
    expect(nodes.length).toBe(1);

    const letNode = nodes[0] as ContainerNode;
    expect(letNode.kind).toBe("let");
    expect(letNode.children.length).toBeGreaterThanOrEqual(4);

    // children should include: name input, :, type, =, value input
    const labels = letNode.children.map(c =>
      c.kind === "slot" ? `slot:${(c as InputSlot).slotKind}` :
      "cardLabel" in c ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("slot:name");
    expect(labels).toContain(":");
    expect(labels).toContain("i32");
    expect(labels).toContain("=");
    expect(labels).toContain("slot:expression");
  });

  it("converts declare_var with mut", () => {
    const ops: CodeOp[] = [{
      kind: "declare_var", pid: "d2", mutable: true,
      name: "x", scope: "fn",
    }];
    const nodes = convertToNodes(ops);
    const letNode = nodes[0] as ContainerNode;
    const labels = letNode.children.map(c =>
      "cardLabel" in c ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("mut");
  });

  it("converts set_var to set container", () => {
    const ops: CodeOp[] = [{
      kind: "set_var", pid: "s1", name: "count", operator: "+=", value: "1",
    }];
    const nodes = convertToNodes(ops);
    const setNode = nodes[0] as ContainerNode;
    expect(setNode.kind).toBe("set");

    const labels = setNode.children.map(c =>
      c.kind === "slot" ? `slot:${(c as InputSlot).slotKind}` :
      "cardLabel" in c ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("slot:name");
    expect(labels).toContain("+=");
    expect(labels).toContain("slot:expression");
  });

  it("converts if_block to BlockNode", () => {
    const ops: CodeOp[] = [{
      kind: "if_block", pid: "if1",
      branches: [{
        condition: "x > 0",
        body: [{ kind: "return", pid: "r1", value: "x" }],
      }],
    }];
    const nodes = convertToNodes(ops);
    expect(nodes.length).toBe(1);
    const ifNode = nodes[0] as BlockNode;
    expect(ifNode.kind).toBe("if");
    expect(ifNode.head.length).toBeGreaterThan(0);
    expect(ifNode.body.length).toBe(1);
    expect(ifNode.body[0].children.length).toBeGreaterThan(0);
  });

  it("converts while loop to BlockNode", () => {
    const ops: CodeOp[] = [{
      kind: "while", pid: "w1", condition: "i < n",
      body: [{ kind: "break", pid: "b1" }],
    }];
    const nodes = convertToNodes(ops);
    const whileNode = nodes[0] as BlockNode;
    expect(whileNode.kind).toBe("while");
  });

  it("converts for_in to BlockNode with in separator", () => {
    const ops: CodeOp[] = [{
      kind: "for_in", pid: "f1", itemName: "item", collection: "items",
      body: [],
    }];
    const nodes = convertToNodes(ops);
    const forNode = nodes[0] as BlockNode;
    expect(forNode.kind).toBe("for");
    const labels = forNode.head.map(c =>
      "cardLabel" in c ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("in");
  });

  it("converts return to container", () => {
    const ops: CodeOp[] = [{ kind: "return", pid: "r1", value: "x + 1" }];
    const nodes = convertToNodes(ops);
    expect((nodes[0] as ContainerNode).kind).toBe("return");
  });

  it("converts break and continue to inline nodes", () => {
    const ops: CodeOp[] = [
      { kind: "break", pid: "b1" },
      { kind: "continue", pid: "c1" },
    ];
    const nodes = convertToNodes(ops);
    expect((nodes[0] as InlineNode).kind).toBe("break");
    expect((nodes[1] as InlineNode).kind).toBe("continue");
  });

  it("converts comment to inline node", () => {
    const ops: CodeOp[] = [{ kind: "comment", pid: "cm1", text: "TODO" }];
    const nodes = convertToNodes(ops);
    expect((nodes[0] as InlineNode).kind).toBe("comment");
    expect((nodes[0] as InlineNode).cardLabel).toBe("TODO");
  });

  it("converts call_fn to call container with fn_name and comma-separated args", () => {
    const ops: CodeOp[] = [{
      kind: "call_fn", pid: "cf1", fnName: "foo", args: ["a", "b", "c"],
    }];
    const nodes = convertToNodes(ops);
    const callNode = nodes[0] as ContainerNode;
    expect(callNode.kind).toBe("call");
    const labels = callNode.children.map(c =>
      c.kind === "slot" ? `slot:${(c as InputSlot).slotKind}` :
      "cardLabel" in c ? (c as InlineNode).cardLabel : c.kind
    );
    expect(labels).toContain("slot:fn_name");
    expect(labels).toContain(",");
    expect(labels.filter(l => l === "slot:expression").length).toBe(3);
  });

  it("converts await to await container with keyword + expression slot", () => {
    const ops: CodeOp[] = [{ kind: "await", pid: "aw1", value: "fetchData()" }];
    const nodes = convertToNodes(ops);
    const awaitNode = nodes[0] as ContainerNode;
    expect(awaitNode.kind).toBe("await");
    // First child is keyword card, second is expression slot
    const kwChild = awaitNode.children[0] as InlineNode;
    expect(kwChild.kind).toBe("keyword");
    expect(kwChild.cardLabel).toBe("await");
    const exprChild = awaitNode.children[1];
    expect(exprChild.kind).toBe("slot");
    expect((exprChild as InputSlot).slotKind).toBe("expression");
    expect((exprChild as InputSlot).value).toBe("fetchData()");
  });

  it("converts raw to inline comment node with source as label", () => {
    const ops: CodeOp[] = [{ kind: "raw", pid: "rw1", source: "# some raw text" }];
    const nodes = convertToNodes(ops);
    const rawNode = nodes[0] as InlineNode;
    expect(rawNode.kind).toBe("comment");
    expect(rawNode.cardLabel).toBe("# some raw text");
  });

  it("converts if_block with empty branches gracefully", () => {
    const ops: CodeOp[] = [{
      kind: "if_block", pid: "if0", branches: [],
    }];
    const nodes = convertToNodes(ops);
    expect(nodes.length).toBe(1);
    const ifNode = nodes[0] as BlockNode;
    expect(ifNode.kind).toBe("if");
    expect(ifNode.head.length).toBe(1); // keyword "if" card
    expect(ifNode.body.length).toBe(1);
  });

  it("handles empty ops array", () => {
    expect(convertToNodes([])).toEqual([]);
  });
});
