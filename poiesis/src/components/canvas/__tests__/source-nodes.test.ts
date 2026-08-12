import { describe, it, expect } from "vitest";
import { collectSourceNodes, type SourceNode } from "../source-nodes";

describe("collectSourceNodes", () => {
  const defs = {
    componentName: "Card",
    componentDefs: {
      Card: {
        name: "Card",
        filePath: "src/components/Card.omni",
        signals: [
          { pid: "sig-1", name: "count", type: "i32", defaultValue: 0 },
          { pid: "sig-2", name: "label", type: "String", defaultValue: "hi" },
        ],
        computeds: [{ pid: "comp-1", name: "double", returnType: "i32", expression: "count * 2" }],
        events: [], fns: [], lifecycles: [], props: [],
      },
    },
    storeDefs: {
      TodoStore: {
        name: "TodoStore",
        signals: [{ pid: "ts-1", name: "items", type: "String[]", defaultValue: [] }],
        getters: [{ pid: "tg-1", name: "totalCount", returnType: "i32" }],
        actions: [], setters: [],
      },
    },
    inlineStoreDefs: {
      "Card.LocalStore": {
        name: "LocalStore",
        signals: [{ pid: "ls-1", name: "draft", type: "String", defaultValue: "" }],
        getters: [], actions: [], setters: [],
      },
    },
    storeReferences: { Card: ["TodoStore"] },
    debugValues: {},
  };

  it("收集组件信号与 computeds", () => {
    const nodes: SourceNode[] = collectSourceNodes(defs as any);
    const names = nodes.map((n) => n.name);
    expect(names).toContain("count");
    expect(names).toContain("label");
    expect(names).toContain("double");
    expect(nodes.find((n) => n.name === "count")).toMatchObject({ kind: "signal", type: "i32", value: 0, ownerKey: "Card" });
  });

  it("引用 store 信号/getters 带前缀名 + store ownerKey", () => {
    const nodes = collectSourceNodes(defs as any);
    const items = nodes.find((n) => n.name === "TodoStore.items");
    expect(items).toMatchObject({ kind: "store-signal", ownerKey: "TodoStore", value: [] });
    const total = nodes.find((n) => n.name === "TodoStore.totalCount");
    expect(total).toMatchObject({ kind: "store-getter", ownerKey: "TodoStore" });
  });

  it("inline store 信号不带前缀（组件内可直接引用）", () => {
    const nodes = collectSourceNodes(defs as any);
    const draft = nodes.find((n) => n.name === "draft");
    expect(draft).toMatchObject({ kind: "inline-signal", ownerKey: "Card.LocalStore" });
  });

  it("debugValues 覆盖值优先于 defaultValue", () => {
    const nodes = collectSourceNodes({ ...defs, debugValues: { Card: { count: { text: "42", valid: true } } } } as any);
    const count = nodes.find((n) => n.name === "count");
    expect(count?.value).toBe("42");
  });

  it("valid:false 的调试条目不参与（回落 defaultValue）", () => {
    const nodes = collectSourceNodes({ ...defs, debugValues: { Card: { count: { text: "999", valid: false } } } } as any);
    const count = nodes.find((n) => n.name === "count");
    expect(count?.value).toBe(0); // defaultValue 兜底，非 "999"
  });

  it("computed 无 defaultValue 且无调试覆盖 → value undefined", () => {
    const nodes = collectSourceNodes(defs as any);
    const double = nodes.find((n) => n.name === "double");
    expect(double?.value).toBeUndefined();
  });

  it("无组件 tab → 空数组", () => {
    expect(collectSourceNodes({ ...defs, componentName: null } as any)).toEqual([]);
  });
});
