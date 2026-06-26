// editor/client/src/omni-gen/__tests__/chain-emitter.test.ts
import { describe, it, expect } from "vitest";
import { emitChains } from "../chain-emitter";
import type { ElementData, WireData, CodeTab } from "../../core/types";
import type { LoweredCode } from "../types";

describe("emitChains", () => {
  // ── Basic: one customFn + one literal + one wire ─────────────

  it("produces a chain block for a function with a literal body", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "increment", returnType: "void" }, children: [], parentId: null, config: { fnName: "increment", fnReturnType: "void" }, position: { x: 100, y: 200 } },
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 42 }, children: [], parentId: null, position: { x: 50, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "lit1", toPort: "value" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "increment", params: [], returnType: "void", body: "42" },
    ];

    const result = emitChains(tab, functions);

    expect(result.size).toBe(1);
    const block = result.get("increment")!;
    expect(block.functionName).toBe("increment");

    // Should contain fn1 @node line with pos and fnName
    expect(block.nodeLines).toHaveLength(2);
    expect(block.nodeLines[0]).toContain("@node");
    expect(block.nodeLines[0]).toContain("fn1");
    expect(block.nodeLines[0]).toContain("customFn");
    expect(block.nodeLines[0]).toContain("fnName=increment");
    expect(block.nodeLines[0]).toContain("pos=100,200");

    // Should contain lit1 @node line with value and pos
    expect(block.nodeLines[1]).toContain("@node");
    expect(block.nodeLines[1]).toContain("lit1");
    expect(block.nodeLines[1]).toContain("literal");
    expect(block.nodeLines[1]).toContain("value=42");
    expect(block.nodeLines[1]).toContain("pos=50,50");

    // Should contain the wire line
    expect(block.wireLines).toHaveLength(1);
    expect(block.wireLines[0]).toContain("@wire");
    expect(block.wireLines[0]).toContain("w1");
    // fn node skips port name on from side
    expect(block.wireLines[0]).toContain("fn1 -> lit1.value");
  });

  // ── Empty tab ────────────────────────────────────────────────

  it("returns empty map for empty tab", () => {
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: [], wires: [] },
      variables: [],
    };
    const result = emitChains(tab, []);
    expect(result.size).toBe(0);
  });

  // ── Multiple functions ───────────────────────────────────────

  it("produces multiple chain blocks for multiple functions", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "inc" }, children: [], parentId: null, config: { fnName: "inc" }, position: { x: 0, y: 0 } },
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 1 }, children: [], parentId: null, position: { x: 50, y: 50 } },
      { id: "fn2", nodeType: "codeblock", tag: "customFn", props: { name: "dec" }, children: [], parentId: null, config: { fnName: "dec" }, position: { x: 0, y: 100 } },
      { id: "lit2", nodeType: "codeblock", tag: "literal", props: { value: 99 }, children: [], parentId: null, position: { x: 50, y: 150 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "lit1", toPort: "value" },
      { id: "w2", fromId: "fn2", fromPort: "body", toId: "lit2", toPort: "value" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "inc", params: [], returnType: "void", body: "1" },
      { name: "dec", params: [], returnType: "void", body: "99" },
    ];

    const result = emitChains(tab, functions);

    expect(result.size).toBe(2);
    expect(result.has("inc")).toBe(true);
    expect(result.has("dec")).toBe(true);

    const incBlock = result.get("inc")!;
    expect(incBlock.functionName).toBe("inc");
    expect(incBlock.nodeLines.some((l) => l.includes("fn1"))).toBe(true);

    const decBlock = result.get("dec")!;
    expect(decBlock.functionName).toBe("dec");
    expect(decBlock.nodeLines.some((l) => l.includes("fn2"))).toBe(true);
  });

  // ── Single function with multiple nodes and wires ────────────

  it("captures all reachable nodes and wires via BFS", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "compute" }, children: [], parentId: null, config: { fnName: "compute" }, position: { x: 0, y: 0 } },
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: 10 }, children: [], parentId: null, position: { x: 50, y: 50 } },
      { id: "lit2", nodeType: "codeblock", tag: "literal", props: { value: 20 }, children: [], parentId: null, position: { x: 50, y: 100 } },
      { id: "add1", nodeType: "codeblock", tag: "add", props: {}, children: [], parentId: null, position: { x: 150, y: 75 } },
      { id: "set1", nodeType: "codeblock", tag: "setVariable", props: { name: "result" }, children: [], parentId: null, position: { x: 250, y: 75 } },
    ];
    const wires: WireData[] = [
      { id: "w0", fromId: "fn1", fromPort: "body", toId: "set1", toPort: "body" },
      { id: "w1", fromId: "lit1", fromPort: "value", toId: "add1", toPort: "a" },
      { id: "w2", fromId: "lit2", fromPort: "value", toId: "add1", toPort: "b" },
      { id: "w3", fromId: "add1", fromPort: "result", toId: "set1", toPort: "value" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [{ name: "result", type: "i32" }],
    };
    const functions: LoweredCode["functions"] = [
      { name: "compute", params: [], returnType: "i32", body: "result = 10 + 20;" },
    ];

    const result = emitChains(tab, functions);

    expect(result.size).toBe(1);
    const block = result.get("compute")!;

    // Should have all 5 nodes
    expect(block.nodeLines).toHaveLength(5);
    // Should have all 4 wires
    expect(block.wireLines).toHaveLength(4);

    // fn1 wire should skip port name
    const fnWire = block.wireLines.find((l) => l.includes("w0"));
    expect(fnWire).toBeTruthy();
    expect(fnWire).toContain("fn1 -> set1.body");
  });

  // ── Format node with various attribute types ─────────────────

  it("formats nodes with required and optional attributes", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "test" }, children: [], parentId: null, config: { fnName: "test", fnReturnType: "i32" }, position: { x: 0, y: 0 } },
      { id: "sig1", nodeType: "codeblock", tag: "signal", props: { name: "counter", typeAnnotation: "i32" }, children: [], parentId: null, position: { x: 50, y: 50 } },
      { id: "gv1", nodeType: "codeblock", tag: "getVariable", props: { name: "counter" }, children: [], parentId: null, position: { x: 100, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "gv1", toPort: "body" },
      { id: "w2", fromId: "sig1", fromPort: "value", toId: "gv1", toPort: "value" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [{ name: "counter", type: "i32" }],
    };
    const functions: LoweredCode["functions"] = [
      { name: "test", params: [], returnType: "i32", body: "" },
    ];

    const result = emitChains(tab, functions);

    expect(result.size).toBe(1);
    const block = result.get("test")!;

    // customFn node with returnType
    const fnLine = block.nodeLines.find((l) => l.includes("fn1"));
    expect(fnLine).toContain("fnName=test");
    expect(fnLine).toContain("returnType=i32");

    // signal node with type annotation mapped to "type" attr
    const sigLine = block.nodeLines.find((l) => l.includes("sig1"));
    expect(sigLine).toContain("name=counter");
    // typeAnnotation goes into "type" attr
    expect(sigLine).toContain("type=i32");

    // getVariable node
    const gvLine = block.nodeLines.find((l) => l.includes("gv1"));
    expect(gvLine).toContain("name=counter");
  });

  // ── Function not found in graph returns empty block ──────────

  it("returns no chain block when function has no matching element", () => {
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: [], wires: [] },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "orphan", params: [], returnType: "void", body: "" },
    ];
    const result = emitChains(tab, functions);
    expect(result.size).toBe(0);
  });

  // ── string literal value is quoted ───────────────────────────

  it("quotes string literal values in node attrs", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "greet" }, children: [], parentId: null, config: { fnName: "greet" }, position: { x: 0, y: 0 } },
      { id: "lit1", nodeType: "codeblock", tag: "literal", props: { value: "hello" }, children: [], parentId: null, position: { x: 50, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "lit1", toPort: "value" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "greet", params: [], returnType: "void", body: "" },
    ];
    const result = emitChains(tab, functions);
    expect(result.size).toBe(1);
    const block = result.get("greet")!;
    const litLine = block.nodeLines.find((l) => l.includes("lit1"));
    expect(litLine).toContain('value="hello"');
  });

  // ── Compare node with op in config (not props) ──────────────

  it("resolves compare.op from config fallback", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "cmp" }, children: [], parentId: null, config: { fnName: "cmp" }, position: { x: 0, y: 0 } },
      { id: "cmp1", nodeType: "codeblock", tag: "compare", props: { a: 1, b: 2 }, children: [], parentId: null, config: { op: ">=" }, position: { x: 50, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "cmp1", toPort: "body" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "cmp", params: [], returnType: "bool", body: "" },
    ];
    const result = emitChains(tab, functions);
    expect(result.size).toBe(1);
    const block = result.get("cmp")!;
    const cmpLine = block.nodeLines.find((l) => l.includes("cmp1"));
    expect(cmpLine).toBeTruthy();
    expect(cmpLine).toContain("op=>=");
  });

  // ── store node ──────────────────────────────────────────────

  it("formats store node with name attribute", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "test" }, children: [], parentId: null, config: { fnName: "test" }, position: { x: 0, y: 0 } },
      { id: "s1", nodeType: "codeblock", tag: "store", props: { name: "todos" }, children: [], parentId: null, position: { x: 50, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "s1", toPort: "body" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "test", params: [], returnType: "void", body: "" },
    ];
    const result = emitChains(tab, functions);
    const block = result.get("test")!;
    const sLine = block.nodeLines.find((l) => l.includes("s1"));
    expect(sLine).toContain("name=todos");
  });

  // ── computed node ───────────────────────────────────────────

  it("formats computed node with name and optional returnType", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "test" }, children: [], parentId: null, config: { fnName: "test" }, position: { x: 0, y: 0 } },
      { id: "comp1", nodeType: "codeblock", tag: "computed", props: { name: "fullName", returnType: "string" }, children: [], parentId: null, position: { x: 50, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "comp1", toPort: "body" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "test", params: [], returnType: "void", body: "" },
    ];
    const result = emitChains(tab, functions);
    const block = result.get("test")!;
    const cLine = block.nodeLines.find((l) => l.includes("comp1"));
    expect(cLine).toContain("name=fullName");
    expect(cLine).toContain("returnType=string");
  });

  // ── fn node ─────────────────────────────────────────────────

  it("formats fn node with source attribute", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "test" }, children: [], parentId: null, config: { fnName: "test" }, position: { x: 0, y: 0 } },
      { id: "fn2", nodeType: "codeblock", tag: "fn", props: { source: "fetchData()" }, children: [], parentId: null, position: { x: 50, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "fn2", toPort: "body" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "test", params: [], returnType: "void", body: "" },
    ];
    const result = emitChains(tab, functions);
    const block = result.get("test")!;
    const fnLine = block.nodeLines.find((l) => l.includes("fn2"));
    expect(fnLine).toContain("source=fetchData()");
  });

  // ── updateUIProperty node ──────────────────────────────────

  it("formats updateUIProperty node with targetId and property", () => {
    const els: ElementData[] = [
      { id: "fn1", nodeType: "codeblock", tag: "customFn", props: { name: "test" }, children: [], parentId: null, config: { fnName: "test" }, position: { x: 0, y: 0 } },
      { id: "u1", nodeType: "codeblock", tag: "updateUIProperty", props: { targetId: "btn1", property: "text" }, children: [], parentId: null, position: { x: 50, y: 50 } },
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "body", toId: "u1", toPort: "body" },
    ];
    const tab: CodeTab = {
      id: "main",
      type: "logic",
      label: "Main",
      icon: "doc",
      graph: { elements: els, wires },
      variables: [],
    };
    const functions: LoweredCode["functions"] = [
      { name: "test", params: [], returnType: "void", body: "" },
    ];
    const result = emitChains(tab, functions);
    const block = result.get("test")!;
    const uLine = block.nodeLines.find((l) => l.includes("u1"));
    expect(uLine).toContain("targetId=btn1");
    expect(uLine).toContain("property=text");
  });
});
