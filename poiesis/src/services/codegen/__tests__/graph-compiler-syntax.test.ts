import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import { parseCodeFlowGraph } from "../graph-parser";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

function el(id: string, tag: string, props: Record<string, unknown>, source = ""): CodeblockElementData {
  return { id, nodeType: "codeblock", tag, props, parentId: null, source };
}

function wire(fromId: string, toId: string): WireData {
  return { id: `w-${fromId}-${toId}`, fromId, fromPort: "flow-out", toId, toPort: "flow-in" };
}

describe("compileCodeFlowGraph — syntax nodes", () => {
  it("emits let/assign/store-call/call in a linear fn chain", () => {
    const elements = [
      el("fn1", "fn", { name: "handleAdd", params: "text: String" }),
      el("l1", "let", { name: "items", mutable: false, value: "[]" }),
      el("a1", "assign", { target: "count", operator: "=", value: "count + step" }),
      el("s1", "store-call", { storeName: "TodoStore", action: "add", args: "text" }),
      el("c1", "call", { fnName: "close", args: "", emit: false }),
      el("r1", "return", {}, ""),
    ];
    const wires = [
      wire("fn1", "l1"), wire("l1", "a1"), wire("a1", "s1"), wire("s1", "c1"), wire("c1", "r1"),
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("fn handleAdd(text: String) {");
    expect(out).toContain("// @pid:l1\n  let items = [];");
    expect(out).toContain("count = count + step;");
    expect(out).toContain("TodoStore.add(text);");
    expect(out).toContain("close();");
    expect(out).toContain("return;");
  });

  it("emits syntax nodes inside condition branches with branch indent", () => {
    const elements = [
      el("fn1", "fn", { name: "check", params: "" }),
      el("cond1", "condition", { branches: [
        { id: "br0", type: "if", condition: "x > 0" },
        { id: "br1", type: "else", condition: "" },
      ] }),
      el("l1", "let", { name: "y", mutable: true, value: "1" }),
      el("a1", "assign", { target: "y", operator: "+=", value: "1" }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "cond1", toPort: "flow-in" },
      { id: "w2", fromId: "cond1", fromPort: "br0", toId: "l1", toPort: "flow-in" },
      { id: "w3", fromId: "cond1", fromPort: "br1", toId: "a1", toPort: "flow-in" },
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("if x > 0 {");
    expect(out).toContain("  let mut y = 1;");
    // Note: 既有 condition emitter 输出 `}\n  else {`（分两行，graph-parser 可解析，round-trip 测试覆盖）
    // 因此不断言单行 "} else {"，而断言 else 分支行本身。
    expect(out).toContain("  else {");
    expect(out).toContain("  y += 1;");
  });

  it("emits emit-call with the emit prefix", () => {
    const elements = [
      el("fn1", "fn", { name: "select", params: "" }),
      el("c1", "call", { fnName: "onSelect", args: "item", emit: true }),
    ];
    const wires = [wire("fn1", "c1")];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("emit onSelect(item);");
  });
});

/** while + body 链 + 续接：fn → while → [body: assign] → flow-out 续接 return */
const ELEMS: CodeblockElementData[] = [
  { id: "fn-1", nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "f", params: "", returnType: "void" }, source: "" },
  { id: "w-2", nodeType: "codeblock", tag: "while", parentId: null, props: { condition: "n < 10" }, source: "while n < 10 {" },
  { id: "a-3", nodeType: "codeblock", tag: "assign", parentId: null, props: { target: "n", operator: "=", value: "n + 1" }, source: "n = n + 1;" },
  { id: "ret-4", nodeType: "codeblock", tag: "return", parentId: null, props: {}, source: "n" },
];
const WIRES = [
  { id: "w1", fromId: "fn-1", fromPort: "flow-out", toId: "w-2", toPort: "flow-in" },
  { id: "w2", fromId: "w-2", fromPort: "br0", toId: "a-3", toPort: "flow-in" },
  { id: "w3", fromId: "w-2", fromPort: "flow-out", toId: "ret-4", toPort: "flow-in" },
];

describe("graph-compiler 循环发射", () => {
  it("while：@pid 注释 + 块头 + body 缩进块 + flow-out 续接同缩进", () => {
    const out = compileCodeFlowGraph(ELEMS, WIRES, "fn-1");
    expect(out).toContain("// @pid:w-2");
    expect(out).toContain("while n < 10 {");
    expect(out).toContain("  n = n + 1;"); // body 缩进 2
    expect(out).toContain("}");
    expect(out).toContain("return n;"); // 续接与 while 同缩进（0）
    expect(out.indexOf("while")).toBeLessThan(out.indexOf("return"));
  });
  it("for-in 发射 `for item in items {`", () => {
    const elems = ELEMS.map((e) => e.id === "w-2" ? { ...e, tag: "for-in" as const, props: { varName: "item", iterable: "items" }, source: "for item in items {" } : e);
    const out = compileCodeFlowGraph(elems, WIRES, "fn-1");
    expect(out).toContain("for item in items {");
  });
  it("无 body wire 时块头空块（`while c { }` 不发射孤儿 body）", () => {
    const wires = WIRES.filter((w) => !(w.fromId === "w-2" && w.fromPort === "br0"));
    const out = compileCodeFlowGraph(ELEMS, wires, "fn-1");
    // 闭括号与块头同缩进（镜像 condition 模式），fn 体内为缩进 2 的 `  }`
    expect(out).toContain("while n < 10 {\n  }");
  });
});

describe("graph-compiler throw/await 发射", () => {
  it("throw/await 编译-再解析闭环", () => {
    const graph = parseCodeFlowGraph(`// @pid:t1
throw new Error("x");
// @pid:a1
await cleanup();
`);
    const compiled = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
    expect(compiled).toContain(`throw new Error("x");`);
    expect(compiled).toContain("await cleanup();");
    const graph2 = parseCodeFlowGraph(compiled);
    expect(graph2.elements.map((e) => e.tag)).toEqual(["throw", "await"]);
  });

  it("throw/await 在 fn 链中按序发射（pid 注释 + 后继链）", () => {
    const elements = [
      el("fn1", "fn", { name: "f", params: "" }),
      el("t1", "throw", { value: `new Error("x")` }),
      el("a1", "await", { value: "cleanup()" }),
      el("r1", "return", {}, ""),
    ];
    const wires = [
      wire("fn1", "t1"), wire("t1", "a1"), wire("a1", "r1"),
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("// @pid:t1\n  throw new Error(\"x\");");
    expect(out).toContain("// @pid:a1\n  await cleanup();");
    expect(out.indexOf("throw")).toBeLessThan(out.indexOf("await"));
    expect(out.indexOf("await")).toBeLessThan(out.indexOf("return"));
  });
});
