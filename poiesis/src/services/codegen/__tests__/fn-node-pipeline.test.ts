import { describe, it, expect } from "vitest";
import { generateOmniSource } from "../pipeline";
import { parseOmniSource } from "../omni-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { ElementData, CodeTab } from "../../../core/types";

const basicCodeTabs: CodeTab[] = [
  {
    id: "main",
    type: "logic",
    label: "Main",
    icon: "code",
    graph: { elements: [], wires: [] },
    variables: [],
  },
];

describe("fn node through generateOmniSource", () => {
  it("emits fn chain from codeblock elements with fn tag", () => {
    const elements: ElementData[] = [
      {
        id: "fn1", nodeType: "codeblock", tag: "fn",
        props: { name: "handleAdd", params: "text: String" }, parentId: null, source: "",
      },
      {
        id: "l1", nodeType: "codeblock", tag: "let",
        props: { name: "items", mutable: false, value: "[]" }, parentId: null, source: "",
      },
      {
        id: "a1", nodeType: "codeblock", tag: "assign",
        props: { target: "count", operator: "=", value: "count + step" }, parentId: null, source: "",
      },
      {
        id: "s1", nodeType: "codeblock", tag: "store-call",
        props: { storeName: "TodoStore", action: "add", args: "text" }, parentId: null, source: "",
      },
      {
        id: "c1", nodeType: "codeblock", tag: "call",
        props: { fnName: "onSelect", args: "", emit: true }, parentId: null, source: "",
      },
      {
        id: "r1", nodeType: "codeblock", tag: "return",
        props: {}, parentId: null, source: "",
      },
    ];
    const wires = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "l1", toPort: "flow-in" },
      { id: "w2", fromId: "l1", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
      { id: "w3", fromId: "a1", fromPort: "flow-out", toId: "s1", toPort: "flow-in" },
      { id: "w4", fromId: "s1", fromPort: "flow-out", toId: "c1", toPort: "flow-in" },
      { id: "w5", fromId: "c1", fromPort: "flow-out", toId: "r1", toPort: "flow-in" },
    ];
    const result = generateOmniSource({
      designElements: elements,
      designWires: wires,
      codeTabs: basicCodeTabs,
      componentName: "App",
    });
    expect(result.errors.filter((e) => e.severity === "fatal")).toHaveLength(0);
    expect(result.value).toContain("fn handleAdd(text: String)");
    expect(result.value).toContain("let items = [];");
    expect(result.value).toContain("count = count + step;");
    expect(result.value).toContain("TodoStore.add(text);");
    expect(result.value).toContain("emit onSelect();");
  });

  it("fn 声明前发射 // @pid:<fnId> 注释（刷新后 fn 入口 pid 稳定）", () => {
    // 回归：保存链路把 compileCodeFlowGraph 的 pid 注释裁进 innerBody 后不再补发 →
    // 磁盘 fn 前无 @pid 注释 → 重解析 fn 入口合成随机 uid pid → 位置丢失 + 每次刷新漂移。
    const elements: ElementData[] = [
      {
        id: "fn-stable-001", nodeType: "codeblock", tag: "fn",
        props: { name: "newFn", params: "" }, parentId: null, source: "",
      },
      {
        id: "a1", nodeType: "codeblock", tag: "assign",
        props: { target: "newSignal", operator: "=", value: "\"demo\"" }, parentId: null, source: "",
      },
    ];
    const wires = [
      { id: "w1", fromId: "fn-stable-001", fromPort: "flow-out", toId: "a1", toPort: "flow-in" },
    ];
    const result = generateOmniSource({
      designElements: elements,
      designWires: wires,
      codeTabs: basicCodeTabs,
      componentName: "App",
    });
    expect(result.errors.filter((e) => e.severity === "fatal")).toHaveLength(0);
    // fn 声明行之前必须紧跟其 pid 注释（供下次解析恢复稳定 id）
    expect(result.value).toMatch(/\/\/ @pid:fn-stable-001\n\s*fn newFn\(\) -> void \{/);
  });

  it("空参 fn 签名不被幻影参数污染（E2E 缺陷 1 回归）", () => {
    // 覆盖两种历史形状：修复前解析侧的空格伪影（" "）与已被污染的签名（": String"）。
    // 防御性过滤应使两种输入都发射干净的 `fn handleAdd() -> void`。
    for (const params of [" ", ": String"]) {
      const elements: ElementData[] = [
        {
          id: "fn1", nodeType: "codeblock", tag: "fn",
          props: { name: "handleAdd", params }, parentId: null, source: "",
        },
        {
          id: "r1", nodeType: "codeblock", tag: "return",
          props: {}, parentId: null, source: "",
        },
      ];
      const wires = [
        { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "r1", toPort: "flow-in" },
      ];
      const result = generateOmniSource({
        designElements: elements,
        designWires: wires,
        codeTabs: basicCodeTabs,
        componentName: "App",
      });
      expect(result.errors.filter((e) => e.severity === "fatal")).toHaveLength(0);
      expect(result.value).toContain("fn handleAdd() -> void");
      expect(result.value).not.toContain("handleAdd(: String");
      expect(result.value).not.toContain("handleAdd( )");
    }
  });
});

describe("lifecycle 节点经 generateOmniSource（G6 注入闭环）", () => {
  it("lifecycle 元素注入：@effect(deps)/@mount 发射格式 + pid 注释前置", () => {
    const elements: ElementData[] = [
      {
        id: "lc-effect-1", nodeType: "codeblock", tag: "lifecycle",
        props: { hookType: "effect", deps: "searchText,setFilter" }, parentId: null, source: "",
      },
      {
        id: "cb-1", nodeType: "codeblock", tag: "call",
        props: { fnName: "filterTasks", args: "searchText", emit: false }, parentId: null, source: "",
      },
      {
        id: "lc-mount-1", nodeType: "codeblock", tag: "lifecycle",
        props: { hookType: "mount" }, parentId: null, source: "",
      },
      {
        id: "cb-2", nodeType: "codeblock", tag: "call",
        props: { fnName: "initRows", args: "", emit: false }, parentId: null, source: "",
      },
    ];
    const wires = [
      { id: "w1", fromId: "lc-effect-1", fromPort: "flow-out", toId: "cb-1", toPort: "flow-in" },
      { id: "w2", fromId: "lc-mount-1", fromPort: "flow-out", toId: "cb-2", toPort: "flow-in" },
    ];
    const result = generateOmniSource({
      designElements: elements,
      designWires: wires,
      codeTabs: basicCodeTabs,
      componentName: "App",
    });
    expect(result.errors.filter((e) => e.severity === "fatal")).toHaveLength(0);
    // 发射格式：pid 注释前置 + deps 括号仅 effect + body 紧凑语句
    expect(result.value).toMatch(/\/\/ @pid:lc-effect-1\n\s*@effect\(searchText,setFilter\) \{/);
    expect(result.value).toContain("filterTasks(searchText);");
    expect(result.value).toMatch(/\/\/ @pid:lc-mount-1\n\s*@mount \{/);
    expect(result.value).toContain("initRows();");
  });
});

describe("生命周期闭环：parse → 图 → compile → 再解析 fixpoint（G6）", () => {
  it("含 @effect/@mount 组件：registry/图按 pid 键、发射格式、编译产物嵌回后重解析再编译逐字节相等", () => {
    const src = `component Main {
  signal searchText: String = "";
  @effect(searchText, setFilter) {
    filterTasks(searchText);
  }
  @mount {
    initRows();
  }
  view {
    <div id="root"></div>
  }
}`;
    const r = parseOmniSource(src);
    const registry = r.lifecycleRegistry!["Main"];
    expect(registry.map((l) => l.hookType)).toEqual(["effect", "mount"]);
    const effectGraph = r.lifecycleGraphs![registry[0].pid];
    const compiled = compileCodeFlowGraph(effectGraph.elements, effectGraph.wires, effectGraph.elements[0].id)!;
    // 发射格式断言
    expect(compiled).toMatch(/^\/\/ @pid:/);
    expect(compiled).toContain("@effect(searchText,setFilter) {");
    expect(compiled).toContain("filterTasks(searchText);");

    // 再解析 fixpoint：编译产物嵌回组件 → 重解析 → 再编译逐字节相等（id 稳定 + 语句紧凑）
    const reparseSrc = `component Main {
  signal searchText: String = "";
${compiled}
  @mount {
    initRows();
  }
  view {
    <div id="root"></div>
  }
}`;
    const r2 = parseOmniSource(reparseSrc);
    const pid2 = r2.lifecycleRegistry!["Main"][0].pid;
    const graph2 = r2.lifecycleGraphs![pid2];
    const compiled2 = compileCodeFlowGraph(graph2.elements, graph2.wires, graph2.elements[0].id)!;
    expect(compiled2).toBe(compiled);
  });
});
