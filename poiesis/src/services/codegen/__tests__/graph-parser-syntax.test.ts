// poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";

describe("parseCodeFlowGraph — syntax nodes", () => {
  it("parses let statement into a let node with extracted props", () => {
    const fnBody = `// @pid:l1
let mut items: Array<String> = [];`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("let");
    expect(el.props).toEqual({ name: "items", mutable: true, varType: "Array<String>", value: "[]" });
    expect(el.source).toBe("let mut items: Array<String> = [];");
  });

  it("parses signal assignment into an assign node", () => {
    const fnBody = `// @pid:a1
count = count + step;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "count", operator: "=", value: "count + step" });
  });

  it("parses compound assignment operator", () => {
    const fnBody = `// @pid:a1
y += 1;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "y", operator: "+=", value: "1" });
  });

  it("parses store call into a store-call node", () => {
    const fnBody = `// @pid:s1
TodoStore.add(text);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("store-call");
    expect(result.elements[0].props).toEqual({ storeName: "TodoStore", action: "add", args: "text" });
  });

  it("parses store signal write as assign (not store-call)", () => {
    const fnBody = `// @pid:a1
ThemeStore.mode = "dark";`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("assign");
    expect(result.elements[0].props).toEqual({ target: "ThemeStore.mode", operator: "=", value: '"dark"' });
  });

  it("parses emit into a call node with emit=true", () => {
    const fnBody = `// @pid:c1
emit onSelect(item);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: true, fnName: "onSelect", args: "item" });
  });

  it("parses plain function call into a call node", () => {
    const fnBody = `// @pid:c1
close();`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: false, fnName: "close", args: "" });
  });

  it("parses array method call (lowercase) into a call node", () => {
    const fnBody = `// @pid:c1
items.push(x);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("call");
    expect(result.elements[0].props).toEqual({ emit: false, fnName: "items.push", args: "x" });
  });

  it("parses let without initial value (name survives round-trip)", () => {
    // LetInspector clearing the initial value emits `let total;` (no `=`).
    // The regex must not fall back to the placeholder name "x".
    const fnBody = `// @pid:l1
let total;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("let");
    expect(el.props).toEqual({ name: "total", mutable: false });
    // formatSyntaxNode round-trip keeps the no-value form
    expect(el.source).toBe("let total;");
  });

  it("parses let mut without initial value", () => {
    const fnBody = `// @pid:l1
let mut x;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("let");
    expect(el.props).toEqual({ name: "x", mutable: true });
    expect(el.source).toBe("let mut x;");
  });

  it("joins multi-line statements instead of truncating to the first line", () => {
    const fnBody = `// @pid:s1
TodoStore.add(
  text
);`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("store-call");
    expect(el.props).toEqual({ storeName: "TodoStore", action: "add", args: "text" });
    // Content from the continuation lines survives the round-trip
    expect(el.source).toBe("TodoStore.add(text);");
  });

  it("preserves original source when syntax props cannot be parsed", () => {
    // Unmatched text must not be replaced with placeholder defaults —
    // keep seg.code verbatim so nothing is silently rewritten.
    const fnBody = `// @pid:s1
TodoStore.add(
  text
;`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(1);
    const el = result.elements[0];
    expect(el.tag).toBe("store-call");
    expect(el.props).toEqual({});
    expect(el.source).toBe(`TodoStore.add(
  text
;`);
  });

  it("keeps for-loop as code-block (starts with 'for', not 'let')", () => {
    const fnBody = `// @pid:cb1
for (let i = 0; i < n; i++) {
  process(i);
}`;
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements[0].tag).toBe("code-block");
  });

  it("wires syntax nodes linearly like code-blocks", () => {
    const fnBody = `// @pid:l1
let x = 0;
// @pid:a1
x = x + 1;
// @pid:c1
close();`;
    const result = parseCodeFlowGraph(fnBody);
    const lw = result.wires.find((w) => w.fromId === "l1" && w.fromPort === "flow-out");
    expect(lw?.toId).toBe("a1");
    const aw = result.wires.find((w) => w.fromId === "a1" && w.fromPort === "flow-out");
    expect(aw?.toId).toBe("c1");
  });
});

function parseProps(src: string) {
  const g = parseCodeFlowGraph(src);
  const el = g.elements.find((e) => e.tag === "while" || e.tag === "for-in");
  return el ? { tag: el.tag, props: el.props } : null;
}

describe("graph-parser 循环段", () => {
  it("while 段头识别为 while 节点并提取 condition", () => {
    const r = parseProps(`fn f() {
  // @pid:w-1
  while n < 10 {
    // @pid:b-2
    n = n + 1;
  }
}`);
    expect(r).not.toBeNull();
    expect(r!.tag).toBe("while");
    expect(r!.props.condition).toBe("n < 10");
  });
  it("for-in 段头识别并提取 varName/iterable", () => {
    const r = parseProps(`fn f() {
  // @pid:fi-1
  for item in items {
    // @pid:b-2
    total = total + item;
  }
}`);
    expect(r!.tag).toBe("for-in");
    expect(r!.props.varName).toBe("item");
    expect(r!.props.iterable).toBe("items");
  });
  it("member 引用经 tokenizer 空格化（TodoStore . items）后归一化（round-trip 保存不退化）", () => {
    // RED（Task 12 E2E）：磁盘 `for item in TodoStore.items {` 经 omni tokenizer 重读后
    // 变成 "TodoStore . items"（点周围空格），parseSyntaxProps 原样保留 → 再保存写回
    // 带空格表达式（round-trip 退化，WASM 可能拒绝）。
    const r = parseProps(`fn f() {
  // @pid:fi-1
  for item in TodoStore . items {
    // @pid:b-2
    total = total + item;
  }
}`);
    expect(r!.tag).toBe("for-in");
    expect(r!.props.iterable).toBe("TodoStore.items");
  });

  it("字符串字面量内的点空格不受归一化影响", () => {
    const r = parseProps(`fn f() {
  // @pid:w-1
  while label == "a . b" {
    // @pid:b-2
    x = 1;
  }
}`);
    expect(r!.tag).toBe("while");
    expect(r!.props.condition).toBe('label == "a . b"');
  });

  it("let initializer 的 member 引用同样归一化（round-trip 契约全覆盖）", () => {
    // RED（审查 round 1）：compactSpacedMembers 漏 case "let" 的 value——
    // `let x = TodoStore . items.length;` 解析后 value 仍带空格（同类 round-trip 退化）。
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:l-1
  let x = TodoStore . items.length;
}`);
    const el = g.elements.find((e) => e.id === "l-1");
    expect(el?.props.value).toBe("TodoStore.items.length");
  });

  it("let initializer 字符串字面量内的点空格不受归一化影响", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:l-1
  let s = "a . b";
}`);
    const el = g.elements.find((e) => e.id === "l-1");
    expect(el?.props.value).toBe('"a . b"');
  });

  it("多级 member 链（TodoStore . items . length）逐层收敛归一化", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:l-1
  let x = TodoStore . items . length;
}`);
    const el = g.elements.find((e) => e.id === "l-1");
    expect(el?.props.value).toBe("TodoStore.items.length");
  });

  it("C 风格括号循环（`for (…`) 保持 code-block（防御性，非 OmniLang 语法）", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:c-1
  for (let i = 0; i < n; i++) {
    x = i;
  }
}`);
    const el = g.elements.find((e) => e.id === "c-1");
    expect(el?.tag).toBe("code-block");
  });
});

describe("graph-parser throw/await 段", () => {
  it("throw/await 段解析为语法节点", () => {
    const { elements, wires } = parseCodeFlowGraph(`// @pid:t1
throw new Error("boom");
// @pid:a1
await async_sleep(20);
`);
    const tags = elements.map((e) => e.tag);
    expect(tags).toEqual(["throw", "await"]);
    // 值保留单词间空格（compactSpacedMembers 语义：new Error("boom") 不得变成 newError("boom")）
    expect(elements[0].props?.value).toBe(`new Error("boom")`);
    expect(elements[1].props?.value).toBe("async_sleep(20)");
    // 值经 formatSyntaxNode 重写为规范化 source（round-trip 不退化）
    expect(elements[0].source).toBe(`throw new Error("boom");`);
    expect(elements[1].source).toBe("await async_sleep(20);");
    expect(wires).toHaveLength(1); // throw → await
  });

  it("await 段头优先于 call 判定（`await foo()` 不得落入 call）", () => {
    const g = parseCodeFlowGraph(`// @pid:a1
await fetch_data();
`);
    expect(g.elements[0].tag).toBe("await");
    expect(g.elements[0].props?.value).toBe("fetch_data()");
  });

  it("字符串字面量内的点空格不受 throw/await 值归一化影响", () => {
    const g = parseCodeFlowGraph(`// @pid:t1
throw new Error("a . b");
`);
    expect(g.elements[0].tag).toBe("throw");
    expect(g.elements[0].props?.value).toBe('new Error("a . b")');
  });
});

describe("graph-parser 生命周期段（G6）", () => {
  it("@mount 段（防御路径）：入口元素 tag/hookType + body 语句 + wire", () => {
    const r = parseCodeFlowGraph(`@mount {
  initRows();
}
`);
    expect(r.elements[0]).toMatchObject({ tag: "lifecycle", props: { hookType: "mount" } });
    expect(r.elements[0].id).toMatch(/^lifecycle_mount_/);
    expect(r.elements).toHaveLength(2);
    expect(r.elements[1].tag).toBe("call");
    expect(r.wires).toHaveLength(1);
    expect(r.wires[0]?.fromId).toBe(r.elements[0].id);
  });

  it("@unmount 段：hookType=unmount（deps 恒无）", () => {
    const r = parseCodeFlowGraph(`@unmount {
  cleanup();
}
`);
    expect(r.elements[0]).toMatchObject({ tag: "lifecycle", props: { hookType: "unmount" } });
    expect((r.elements[0].props as Record<string, unknown>).deps).toBeUndefined();
    expect(r.wires).toHaveLength(1);
  });

  it("空 body 生命周期：仅入口元素 + 无 wire", () => {
    const r = parseCodeFlowGraph(`@mount {
}
`);
    expect(r.elements).toHaveLength(1);
    expect(r.elements[0].tag).toBe("lifecycle");
    expect(r.wires).toHaveLength(0);
  });

  it("段内 // @pid 注释取入口 id（分段路径）", () => {
    const r = parseCodeFlowGraph(`// @pid:lc-stable-1
@effect(searchText, setFilter) {
  filterTasks(searchText);
}
`);
    expect(r.elements[0].id).toBe("lc-stable-1");
    expect(r.elements[0].tag).toBe("lifecycle");
    expect(r.elements[0].props).toMatchObject({ hookType: "effect", deps: "searchText, setFilter" });
    expect(r.elements).toHaveLength(2);
    expect(r.wires).toHaveLength(1);
  });

  it("零段回退负分支：多语句未注解 body 保持 0 元素（不吞并语句）", () => {
    const r = parseCodeFlowGraph("let a = 1;\nlet b = 2;");
    expect(r.elements).toHaveLength(0);
    expect(r.wires).toHaveLength(0);
  });
});
