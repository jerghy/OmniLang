// design/binding-sources.test.ts（M3 T3）：buildBindingSources 来源组装——
// 信号（组件/store/inline 归一 signal）+ fn 出口签名 + 画布元素。
import { describe, expect, test } from "vitest";
import { buildBindingSources } from "../binding-sources";
import type { BindingSourceState } from "../binding-sources";

const state: BindingSourceState = {
  componentName: "Counter",
  componentDefs: {
    Counter: {
      name: "Counter",
      filePath: "Counter.omni",
      signals: [{ pid: "p1", name: "count", type: "i32", defaultValue: 0 }],
      computeds: [{ pid: "p2", name: "double", returnType: "i32", body: "count * 2" }],
    } as never,
  },
  storeDefs: {
    Cart: {
      name: "Cart",
      signals: [{ pid: "p3", name: "items", type: "i32", defaultValue: 0 }],
    } as never,
  },
  inlineStoreDefs: {
    "Counter.Store": {
      name: "Store",
      signals: [{ pid: "p4", name: "note", type: "string", defaultValue: "" }],
    } as never,
  },
  storeReferences: { Cart: ["Counter"] },
  debugValues: {},
  elements: [
    { id: "el1", nodeType: "codeblock", tag: "fn", props: { name: "newFn" } },
    { id: "el2", nodeType: "codeblock", tag: "fn", props: { name: "fmt", params: "n: i32", returnType: "string" } },
    { id: "el3", nodeType: "codeblock", tag: "fn" }, // 未命名 → 跳过
    { id: "el4", nodeType: "view", tag: "input", props: { label: "搜索框\n（第二行忽略）" } },
    { id: "el5", nodeType: "view", tag: "button" }, // 无 label → 兜底 id
    { id: "el6", nodeType: "conditional" }, // 非 view → 跳过
  ],
};

describe("buildBindingSources", () => {
  test("信号来源归一为 signal kind：组件 / store 前缀 / inline；id 复用 SourceNode pid 编码", () => {
    const sigs = buildBindingSources(state).filter((s) => s.kind === "signal");
    expect(sigs).toHaveLength(4);
    expect(sigs.map((s) => s.name)).toEqual(["count", "double", "Cart.items", "note"]);
    // id 必须按 pid 唯一（跨作用域同名不碰撞）：sig:pid / computed:pid / store:名.pid / inline:键.pid
    expect(sigs.map((s) => s.id)).toEqual(["sig:p1", "computed:p2", "store:Cart.p3", "inline:Counter.Store.p4"]);
    expect(new Set(sigs.map((s) => s.id)).size).toBe(4);
  });

  test("fn 出口带签名类型（params/returnType 拼串，缺省 void）", () => {
    const fns = buildBindingSources(state).filter((s) => s.kind === "fn");
    expect(fns.map((s) => s.name)).toEqual(["newFn", "fmt"]);
    expect(fns[0].type).toBe("() => void");
    expect(fns[1].type).toBe("(n: i32) => string");
  });

  test("画布元素取 label 首行、兜底 id；非 view 跳过", () => {
    const els = buildBindingSources(state).filter((s) => s.kind === "element");
    expect(els.map((s) => s.name)).toEqual(["搜索框", "el5"]);
    expect(els[1].type).toBe("button");
    expect(els.map((s) => s.id)).toEqual(["el:el4", "el:el5"]);
  });
});
