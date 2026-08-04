import { describe, it, expect } from "vitest";
import { buildFlowScope, parseFnParamsStr, filterCandidates, validateSyntaxProps, type FlowScope } from "../flow-scope";

const compDef = {
  name: "App", filePath: "App.omni",
  signals: [{ pid: "p1", name: "count", type: "i32" }, { pid: "p2", name: "items", type: "Array<String>" }],
  props: [{ pid: "p3", name: "title", type: "String" }],
  computeds: [{ pid: "p4", name: "doneCount", returnType: "i32", expression: "" }],
  events: [{ pid: "p5", name: "onSelect" }],
  fns: [{ pid: "p6", name: "close", params: [{ name: "x", type: "i32" }], returnType: "" }],
};

const storeDefs = {
  TodoStore: {
    pid: "s1", name: "TodoStore", filePath: "TodoStore.omni", imports: [],
    signals: [{ pid: "s2", name: "list", type: "Array<String>" }],
    getters: [], actions: [{ pid: "s3", name: "add", params: [{ name: "text", type: "String" }] }],
  },
} as unknown as Parameters<typeof buildFlowScope>[0]["storeDefs"];

const fnEls = [
  { id: "l1", nodeType: "codeblock", tag: "let", parentId: null, props: { name: "total", mutable: false, value: "0" } },
  { id: "l2", nodeType: "codeblock", tag: "let", parentId: null, props: { name: "x" } },
] as unknown as Parameters<typeof buildFlowScope>[0]["fnElements"];

describe("parseFnParamsStr", () => {
  it("解析 name: Type 逗号格式", () => {
    expect(parseFnParamsStr("text: String, count: i32")).toEqual([
      { name: "text", type: "String" }, { name: "count", type: "i32" },
    ]);
  });
  it("空串返回空数组", () => {
    expect(parseFnParamsStr("")).toEqual([]);
  });
});

describe("buildFlowScope", () => {
  const scope = buildFlowScope({
    componentDef: compDef as never,
    storeDefs, inlineStoreDefs: {}, storesUsed: ["TodoStore"],
    fnParamsStr: "text: String",
    fnElements: fnEls,
  });

  it("收集 signals/props/computeds/events/fns", () => {
    expect(scope.signals.map(s => s.name)).toEqual(["count", "items"]);
    expect(scope.props[0].name).toBe("title");
    expect(scope.computeds[0].name).toBe("doneCount");
    expect(scope.events[0].name).toBe("onSelect");
    expect(scope.fns[0].name).toBe("close");
  });
  it("收集 store actions 与 signals（storeDefs 与 inlineStoreDefs 合并）", () => {
    expect(scope.stores[0].name).toBe("TodoStore");
    expect(scope.stores[0].actions[0].name).toBe("add");
    expect(scope.stores[0].signals[0].name).toBe("list");
  });
  it("收集 fn 参数与 let 变量", () => {
    expect(scope.fnParams[0].name).toBe("text");
    expect(scope.letVars.map(v => v.name)).toEqual(["total", "x"]);
  });
  it("fnElements 缺省时 letVars 为空", () => {
    const s2 = buildFlowScope({ componentDef: undefined, storeDefs: {}, inlineStoreDefs: {}, storesUsed: [], fnElements: [] });
    expect(s2.letVars).toEqual([]);
    expect(s2.stores).toEqual([]);
  });
});

describe("filterCandidates", () => {
  it("空 query 返回全部（限 8 项）", () => {
    const c = Array.from({ length: 10 }, (_, i) => ({ name: `v${i}` }));
    expect(filterCandidates("", c)).toHaveLength(8);
  });
  it("前缀匹配", () => {
    const c = [{ name: "count" }, { name: "doneCount" }, { name: "title" }];
    expect(filterCandidates("count", c).map(x => x.name)).toEqual(["count"]);
    expect(filterCandidates("ti", c).map(x => x.name)).toEqual(["title"]);
  });
  it("无匹配返回空", () => {
    expect(filterCandidates("zzz", [{ name: "count" }])).toEqual([]);
  });
});

describe("validateSyntaxProps", () => {
  it("let 非法变量名报错", () => {
    const r = validateSyntaxProps("let", { name: "1bad", mutable: false }, {} as FlowScope);
    expect(r.some(x => x.field === "name")).toBe(true);
  });
  it("let 合法通过", () => {
    expect(validateSyntaxProps("let", { name: "count" }, {} as FlowScope)).toEqual([]);
  });
  it("store-call action 不在所选 store 时报错", () => {
    const scope = buildFlowScope({ componentDef: undefined, storeDefs, inlineStoreDefs: {}, storesUsed: ["TodoStore"], fnElements: [] });
    const r = validateSyntaxProps("store-call", { storeName: "TodoStore", action: "nope" }, scope);
    expect(r.some(x => x.field === "action")).toBe(true);
  });
  it("store-call 未知 store 不报 action 错（跳过校验）", () => {
    const scope = buildFlowScope({ componentDef: undefined, storeDefs: {}, inlineStoreDefs: {}, storesUsed: [], fnElements: [] });
    expect(validateSyntaxProps("store-call", { storeName: "UnknownStore", action: "x" }, scope)).toEqual([]);
  });
  it("assign 空 target 报错", () => {
    const r = validateSyntaxProps("assign", { target: "", operator: "=" }, {} as FlowScope);
    expect(r.some(x => x.field === "target")).toBe(true);
  });
});

it("storesUsed 纳入无 def 的 store（引用方向：store→组件）", () => {
  const s = buildFlowScope({ componentDef: undefined, storeDefs: {}, inlineStoreDefs: {}, storesUsed: ["NoDefStore"], fnElements: [] });
  expect(s.stores.map((x) => x.name)).toContain("NoDefStore");
});

it("同名 store 冲突时 storeDefs 优先于 inlineStoreDefs", () => {
  const s = buildFlowScope({
    componentDef: undefined,
    storeDefs: { A: { pid: "a1", name: "A", filePath: "A.omni", imports: [], signals: [], getters: [], actions: [{ pid: "a2", name: "global" }] } },
    inlineStoreDefs: { A: { pid: "b1", name: "A", filePath: "B.omni", imports: [], signals: [], getters: [], actions: [{ pid: "b2", name: "inline" }] } },
    storesUsed: [], fnElements: [],
  } as unknown as Parameters<typeof buildFlowScope>[0]);
  expect(s.stores[0].actions.map((a) => a.name)).toEqual(["global"]);
});

it("call 空函数名报错，非空通过", () => {
  expect(validateSyntaxProps("call", { fnName: "" }, {} as FlowScope).some((x) => x.field === "fnName")).toBe(true);
  expect(validateSyntaxProps("call", { fnName: "close" }, {} as FlowScope)).toEqual([]);
});

it("assign target 支持 Store.信号，拒绝多段路径", () => {
  expect(validateSyntaxProps("assign", { target: "ThemeStore.mode" }, {} as FlowScope)).toEqual([]);
  expect(validateSyntaxProps("assign", { target: "a.b.c" }, {} as FlowScope).some((x) => x.field === "target")).toBe(true);
});
