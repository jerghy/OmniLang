import { describe, it, expect } from "vitest";
import { convertToOps } from "../convertToOps";
import { convertToNodes } from "../convertToNodes";

describe("convertToOps", () => {
  it("converts let container back to declare_var", () => {
    const nodes = [{
      kind: "let" as const, pid: "d1",
      children: [
        { kind: "mut", cardLabel: "mut", pid: "m1" } as any,
        { kind: "slot", slotKind: "name", value: "count", pid: "s1" } as any,
        { kind: "separator", cardLabel: ":", pid: "sep1" } as any,
        { kind: "type", cardLabel: "i32", pid: "t1" } as any,
        { kind: "operator", cardLabel: "=", pid: "op1" } as any,
        { kind: "slot", slotKind: "expression", value: "0", pid: "s2" } as any,
      ],
    }];
    const ops = convertToOps(nodes);
    expect(ops.length).toBe(1);
    expect(ops[0].kind).toBe("declare_var");
    if (ops[0].kind === "declare_var") {
      expect(ops[0].mutable).toBe(true);
      expect(ops[0].name).toBe("count");
      expect(ops[0].varType).toBe("i32");
      expect(ops[0].initValue).toBe("0");
    }
  });

  it("converts set container back to set_var", () => {
    const nodes = [{
      kind: "set" as const, pid: "s1",
      children: [
        { kind: "slot", slotKind: "name", value: "sum", pid: "s1" } as any,
        { kind: "operator", cardLabel: "+=", pid: "op1" } as any,
        { kind: "slot", slotKind: "expression", value: "1", pid: "s2" } as any,
      ],
    }];
    const ops = convertToOps(nodes);
    expect(ops[0].kind).toBe("set_var");
    if (ops[0].kind === "set_var") {
      expect(ops[0].name).toBe("sum");
      expect(ops[0].operator).toBe("+=");
      expect(ops[0].value).toBe("1");
    }
  });

  it("converts if BlockNode back to if_block", () => {
    const nodes = [{
      kind: "if" as const, pid: "if1",
      head: [
        { kind: "slot", slotKind: "condition", value: "x > 0", pid: "c1" } as any,
      ],
      body: [
        { pid: "bl1", children: [
          { kind: "return" as const, pid: "r1", children: [
            { kind: "slot", slotKind: "expression", value: "x", pid: "e1" } as any,
          ] },
        ] },
      ],
      tail: { pid: "tl1" },
      branches: [{
        kind: "else" as const, pid: "br1",
        head: [],
        body: [
          { pid: "bl2", children: [
            { kind: "return" as const, pid: "r2", children: [
              { kind: "slot", slotKind: "expression", value: "0", pid: "e2" } as any,
            ] },
          ] },
        ],
      }],
    }];
    const ops = convertToOps(nodes);
    expect(ops.length).toBe(1);
    expect(ops[0].kind).toBe("if_block");
    if (ops[0].kind === "if_block") {
      expect(ops[0].branches.length).toBe(2);
      expect(ops[0].branches[0].condition).toBe("x > 0");
      expect(ops[0].branches[1].condition).toBe("");
    }
  });

  it("converts while BlockNode back to while", () => {
    const nodes = [{
      kind: "while" as const, pid: "w1",
      head: [{ kind: "slot", slotKind: "condition", value: "i < n", pid: "c1" } as any],
      body: [],
      tail: { pid: "tl1" },
    }];
    const ops = convertToOps(nodes);
    expect(ops[0].kind).toBe("while");
    if (ops[0].kind === "while") {
      expect(ops[0].condition).toBe("i < n");
    }
  });

  it("converts for BlockNode back to for_in", () => {
    const nodes = [{
      kind: "for" as const, pid: "f1",
      head: [
        { kind: "slot", slotKind: "name", value: "item", pid: "s1" } as any,
        { kind: "separator", cardLabel: "in", pid: "in1" } as any,
        { kind: "slot", slotKind: "expression", value: "items", pid: "s2" } as any,
      ],
      body: [],
      tail: { pid: "tl1" },
    }];
    const ops = convertToOps(nodes);
    expect(ops[0].kind).toBe("for_in");
    if (ops[0].kind === "for_in") {
      expect(ops[0].itemName).toBe("item");
      expect(ops[0].collection).toBe("items");
    }
  });

  it("roundtrip: convertToNodes -> convertToOps is identity for key ops", () => {
    const original: any[] = [
      { kind: "declare_var", pid: "d1", mutable: true, name: "x", varType: "f64", initValue: "1.0", scope: "fn" },
      { kind: "set_var", pid: "s1", name: "x", operator: "=", value: "2" },
      { kind: "return", pid: "r1", value: "x" },
    ];
    const nodes = convertToNodes(original);
    const roundtripped = convertToOps(nodes);
    expect(roundtripped.length).toBe(3);
    expect(roundtripped[0].kind).toBe("declare_var");
    if (roundtripped[0].kind === "declare_var") {
      expect(roundtripped[0].mutable).toBe(true);
      expect(roundtripped[0].name).toBe("x");
      expect(roundtripped[0].varType).toBe("f64");
      expect(roundtripped[0].initValue).toBe("1.0");
    }
    expect(roundtripped[1].kind).toBe("set_var");
    if (roundtripped[1].kind === "set_var") {
      expect(roundtripped[1].name).toBe("x");
      expect(roundtripped[1].operator).toBe("=");
      expect(roundtripped[1].value).toBe("2");
    }
    expect(roundtripped[2].kind).toBe("return");
    if (roundtripped[2].kind === "return") {
      expect(roundtripped[2].value).toBe("x");
    }
  });
});
