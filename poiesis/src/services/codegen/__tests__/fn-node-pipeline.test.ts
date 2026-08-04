import { describe, it, expect } from "vitest";
import { generateOmniSource } from "../pipeline";
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
});
