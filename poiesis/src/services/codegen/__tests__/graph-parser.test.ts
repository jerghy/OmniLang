// poiesis/src/services/codegen/__tests__/graph-parser.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph, normalizeAnnotations } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

// ---------------------------------------------------------------------------
// Helper: find a wire by fromId and fromPort
// ---------------------------------------------------------------------------
function findWire(
  wires: WireData[],
  fromId: string,
  fromPort: string,
): WireData | undefined {
  return wires.find((w) => w.fromId === fromId && w.fromPort === fromPort);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("parseCodeFlowGraph", () => {
  // -----------------------------------------------------------------------
  // Edge: empty / missing input
  // -----------------------------------------------------------------------
  it("returns empty graph for empty fnBody", () => {
    const result = parseCodeFlowGraph("");
    expect(result.elements).toEqual([]);
    expect(result.wires).toEqual([]);
  });

  it("returns empty graph for whitespace-only fnBody", () => {
    const result = parseCodeFlowGraph("   \n  \n  ");
    expect(result.elements).toEqual([]);
    expect(result.wires).toEqual([]);
  });

  it("returns empty graph for fnBody with only PID comments", () => {
    const result = parseCodeFlowGraph("// @pid:foo\n// @pid:bar");
    expect(result.elements).toEqual([]);
    expect(result.wires).toEqual([]);
  });

  // -----------------------------------------------------------------------
  // Simple chain: code-block → return
  // -----------------------------------------------------------------------
  it("parses a simple code-block chain with return", () => {
    const fnBody = `// @pid:code1
let x = 1
// @pid:ret1
return x`;

    const result = parseCodeFlowGraph(fnBody);

    // Elements
    expect(result.elements).toHaveLength(2);
    const [codeEl, retEl] = result.elements;

    expect(codeEl.id).toBe("code1");
    expect(codeEl.tag).toBe("let");
    expect(codeEl.props).toEqual({ name: "x", mutable: false, value: "1" });
    expect(codeEl.source).toBe("let x = 1;");

    expect(retEl.id).toBe("ret1");
    expect(retEl.tag).toBe("return");
    expect(retEl.source).toBe("x");

    // Wires: code-block → return
    expect(result.wires).toHaveLength(1);
    const w = findWire(result.wires, "code1", "flow-out");
    expect(w).toBeDefined();
    expect(w!.toId).toBe("ret1");
    expect(w!.toPort).toBe("flow-in");
  });

  // -----------------------------------------------------------------------
  // Simple chain: fn → code-block → return
  // -----------------------------------------------------------------------
  it("parses an fn → code-block → return chain", () => {
    const fnBody = `// @pid:fn1
fn main() {
// @pid:code1
  // 步骤一
// @pid:ret1
  return result
}`;

    const result = parseCodeFlowGraph(fnBody);

    // Elements
    expect(result.elements).toHaveLength(3);
    const [fnEl, codeEl, retEl] = result.elements;

    expect(fnEl.id).toBe("fn1");
    expect(fnEl.tag).toBe("fn");
    expect(fnEl.props.name).toBe("main");

    expect(codeEl.id).toBe("code1");
    expect(codeEl.tag).toBe("code-block");
    expect(codeEl.source).toBe("  // 步骤一");

    expect(retEl.id).toBe("ret1");
    expect(retEl.tag).toBe("return");
    expect(retEl.source).toBe("result");

    // Wires: fn → code-block → return
    expect(result.wires).toHaveLength(2);
    const w1 = findWire(result.wires, "fn1", "flow-out");
    expect(w1).toBeDefined();
    expect(w1!.toId).toBe("code1");

    const w2 = findWire(result.wires, "code1", "flow-out");
    expect(w2).toBeDefined();
    expect(w2!.toId).toBe("ret1");
  });

  // -----------------------------------------------------------------------
  // Condition with if/else branches
  // -----------------------------------------------------------------------
  it("parses condition with if/else: wires br0/br1 and flow-out correctly", () => {
    const fnBody = `// @pid:cond1
if cond {
// @pid:then1
  doThen()
// @pid:else1
} else {
  doElse()
}
// @pid:post1
continue()`;

    const result = parseCodeFlowGraph(fnBody);

    // Elements
    const condEl = result.elements.find((e) => e.id === "cond1");
    expect(condEl).toBeDefined();
    expect(condEl!.tag).toBe("condition");
    const branches = condEl!.props.branches as Array<{ type: string; condition: string }>;
    expect(branches).toHaveLength(2);
    expect(branches[0].type).toBe("if");
    expect(branches[0].condition).toBe("cond");
    expect(branches[1].type).toBe("else");
    expect(branches[1].condition).toBe("");

    // Branch 0 (if): cond1.br0 → then1
    const br0Wire = findWire(result.wires, "cond1", "br0");
    expect(br0Wire).toBeDefined();
    expect(br0Wire!.toId).toBe("then1");
    expect(br0Wire!.toPort).toBe("flow-in");

    // Branch 1 (else): cond1.br1 → else1
    const br1Wire = findWire(result.wires, "cond1", "br1");
    expect(br1Wire).toBeDefined();
    expect(br1Wire!.toId).toBe("else1");
    expect(br1Wire!.toPort).toBe("flow-in");

    // Flow-out: cond1.flow-out → post1
    const flowOutWire = findWire(result.wires, "cond1", "flow-out");
    expect(flowOutWire).toBeDefined();
    expect(flowOutWire!.toId).toBe("post1");
    expect(flowOutWire!.toPort).toBe("flow-in");
  });

  // -----------------------------------------------------------------------
  // Condition with only 'if' (no else)
  // -----------------------------------------------------------------------
  it("parses if-only condition (no else branch)", () => {
    const fnBody = `// @pid:cond1
if cond {
// @pid:then1
  doThen()
}
// @pid:post1
continue()`;

    const result = parseCodeFlowGraph(fnBody);

    // br0 → then1
    const br0Wire = findWire(result.wires, "cond1", "br0");
    expect(br0Wire).toBeDefined();
    expect(br0Wire!.toId).toBe("then1");

    // flow-out → post1
    const flowOutWire = findWire(result.wires, "cond1", "flow-out");
    expect(flowOutWire).toBeDefined();
    expect(flowOutWire!.toId).toBe("post1");

    // br1 should NOT exist
    const br1Wire = findWire(result.wires, "cond1", "br1");
    expect(br1Wire).toBeUndefined();
  });

  // -----------------------------------------------------------------------
  // Multiple segments inside one branch
  // -----------------------------------------------------------------------
  it("chains multiple segments within a single branch", () => {
    const fnBody = `// @pid:cond1
if cond {
// @pid:step1
  stepOne()
// @pid:step2
  stepTwo()
}
// @pid:post1
continue()`;

    const result = parseCodeFlowGraph(fnBody);

    // Branch 0 (if) should have 2 segments: step1 → step2
    const br0Wire = findWire(result.wires, "cond1", "br0");
    expect(br0Wire).toBeDefined();
    expect(br0Wire!.toId).toBe("step1");

    const chainWire = findWire(result.wires, "step1", "flow-out");
    expect(chainWire).toBeDefined();
    expect(chainWire!.toId).toBe("step2");

    // Flow-out → post1
    const flowOutWire = findWire(result.wires, "cond1", "flow-out");
    expect(flowOutWire).toBeDefined();
    expect(flowOutWire!.toId).toBe("post1");

    // Total wires: br0 + flow-out + internal chain = 3
    expect(result.wires).toHaveLength(3);
  });

  // -----------------------------------------------------------------------
  // Round-trip: compileCodeFlowGraph → parseCodeFlowGraph
  // -----------------------------------------------------------------------
  it("round-trips: compile → parse returns equivalent graph", () => {
    // Build a graph manually
    const elements: CodeblockElementData[] = [
      {
        id: "fn1",
        nodeType: "codeblock",
        tag: "fn",
        parentId: null,
        props: { name: "test", params: "" },
        source: "",
      },
      {
        id: "code1",
        nodeType: "codeblock",
        tag: "code-block",
        parentId: null,
        props: {},
        source: "  // 初始化变量", // code1（原 "  let x = 1"）
      },
      {
        id: "cond1",
        nodeType: "codeblock",
        tag: "condition",
        parentId: null,
        props: {
          branches: [
            { id: "br0", type: "if", condition: "x > 0" },
            { id: "br1", type: "else", condition: "" },
          ],
        },
        source: "",
      },
      {
        id: "then1",
        nodeType: "codeblock",
        tag: "code-block",
        parentId: null,
        props: {},
        source: "  // then 分支", // then1（原 "  process(x)"）
      },
      {
        id: "else1",
        nodeType: "codeblock",
        tag: "code-block",
        parentId: null,
        props: {},
        source: "  // else 分支", // else1（原 "  skip()"）
      },
      {
        id: "post1",
        nodeType: "codeblock",
        tag: "code-block",
        parentId: null,
        props: {},
        source: "  // 收尾", // post1（原 "  done()"）
      },
      {
        id: "ret1",
        nodeType: "codeblock",
        tag: "return",
        parentId: null,
        props: {},
        source: "x",
      },
    ];

    const wires: WireData[] = [
      { id: "w1", fromId: "fn1", fromPort: "flow-out", toId: "code1", toPort: "flow-in" },
      { id: "w2", fromId: "code1", fromPort: "flow-out", toId: "cond1", toPort: "flow-in" },
      { id: "w3", fromId: "cond1", fromPort: "br0", toId: "then1", toPort: "flow-in" },
      { id: "w4", fromId: "cond1", fromPort: "br1", toId: "else1", toPort: "flow-in" },
      { id: "w5", fromId: "cond1", fromPort: "flow-out", toId: "post1", toPort: "flow-in" },
      { id: "w6", fromId: "post1", fromPort: "flow-out", toId: "ret1", toPort: "flow-in" },
    ];

    // Compile to text
    const compiled = compileCodeFlowGraph(elements, wires, "fn1");
    expect(compiled).toBeTruthy();

    // Parse back
    const parsed = parseCodeFlowGraph(compiled);

    // Verify parsed elements match original structure
    // (IDs may differ because compiler uses pid or element id)
    // Check tags, props, and source, not IDs
    const parsedByTag = (tag: string) =>
      parsed.elements.filter((e) => e.tag === tag);
    const origByTag = (tag: string) =>
      elements.filter((e) => e.tag === tag);

    expect(parsedByTag("fn").length).toBe(origByTag("fn").length);
    expect(parsedByTag("code-block").length).toBe(origByTag("code-block").length);
    expect(parsedByTag("condition").length).toBe(origByTag("condition").length);
    expect(parsedByTag("return").length).toBe(origByTag("return").length);

    // Condition should have correct branches
    const parsedCond = parsedByTag("condition")[0];
    const pBranches = parsedCond?.props.branches as Array<{
      type: string;
      condition: string;
    }>;
    expect(pBranches).toBeDefined();
    expect(pBranches).toHaveLength(2);
    expect(pBranches[0].type).toBe("if");
    expect(pBranches[0].condition).toBe("x > 0");
    expect(pBranches[1].type).toBe("else");

    // Verify wire structure: check that condition.br0 and br1 exist
    const parsedCondId = parsedCond!.id;
    const br0 = findWire(parsed.wires, parsedCondId, "br0");
    const br1 = findWire(parsed.wires, parsedCondId, "br1");
    const flowOut = findWire(parsed.wires, parsedCondId, "flow-out");

    expect(br0).toBeDefined();
    expect(br1).toBeDefined();
    expect(flowOut).toBeDefined();

    // Ensure flow-out does NOT point to a branch-internal node
    const branchTargets = new Set([br0!.toId, br1!.toId]);
    expect(branchTargets.has(flowOut!.toId)).toBe(false);
  });

  // -----------------------------------------------------------------------
  // Missing PID comments — segments with no PID should be skipped by
  // splitByPid (they're treated as raw code that belongs to the preceding
  // segment). The function should not crash.
  // -----------------------------------------------------------------------
  it("handles code without PID comments gracefully", () => {
    const fnBody = `fn test() {
  let x = 1
  return x
}`;

    // No PID comments — splitByPid returns empty segments
    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toEqual([]);
    expect(result.wires).toEqual([]);
  });

  // -----------------------------------------------------------------------
  // Condition with if/elif/else
  // -----------------------------------------------------------------------
  it("parses condition with if/elif/else branches", () => {
    const fnBody = `// @pid:cond1
if a {
// @pid:if1
  handleA()
// @pid:elif1
} else if b {
  handleB()
// @pid:else1
} else {
  handleC()
}
// @pid:post1
done()`;

    const result = parseCodeFlowGraph(fnBody);

    // Condition should have 3 branches
    const condEl = result.elements.find((e) => e.tag === "condition");
    const branches = condEl!.props.branches as Array<{ type: string }>;
    expect(branches).toHaveLength(3);
    expect(branches[0].type).toBe("if");
    expect(branches[1].type).toBe("elif");
    expect(branches[2].type).toBe("else");

    // br0 → if1
    expect(findWire(result.wires, condEl!.id, "br0")?.toId).toBe("if1");
    // br1 → elif1
    expect(findWire(result.wires, condEl!.id, "br1")?.toId).toBe("elif1");
    // br2 → else1
    expect(findWire(result.wires, condEl!.id, "br2")?.toId).toBe("else1");
    // flow-out → post1
    expect(findWire(result.wires, condEl!.id, "flow-out")?.toId).toBe("post1");
  });

  // -----------------------------------------------------------------------
  // return expression extraction
  // -----------------------------------------------------------------------
  it("extracts return expressions correctly", () => {
    const fnBody = `// @pid:code1
doSomething()
// @pid:ret1
return someValue`;

    const result = parseCodeFlowGraph(fnBody);
    const retEl = result.elements.find((e) => e.tag === "return");
    expect(retEl?.source).toBe("someValue");
  });

  it("handles return with semicolon", () => {
    const fnBody = `// @pid:code1
doSomething()
// @pid:ret1
return someValue;`;

    const result = parseCodeFlowGraph(fnBody);
    const retEl = result.elements.find((e) => e.tag === "return");
    expect(retEl?.source).toBe("someValue");
  });

  it("handles bare return (no expression)", () => {
    const fnBody = `// @pid:code1
doSomething()
// @pid:ret1
return`;

    const result = parseCodeFlowGraph(fnBody);
    const retEl = result.elements.find((e) => e.tag === "return");
    expect(retEl?.source).toBe("");
  });
});

// ---------------------------------------------------------------------------
// Tests for // @<type> annotation support
// ---------------------------------------------------------------------------

describe("parseCodeFlowGraph with // @<type> annotations", () => {
  it("parses manual @if and @return annotations", () => {
    const fnBody = [
      "// @if",
      "if (x > 0) {",
      "  // @return",
      "  return x;",
      "}",
      "// @return",
      "return 0;",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(3);
    const ifEl = result.elements.find((e) => e.tag === "condition");
    expect(ifEl).toBeDefined();
    const returnEls = result.elements.filter((e) => e.tag === "return");
    expect(returnEls).toHaveLength(2);
    // Should have wires connecting them
    expect(result.wires.length).toBeGreaterThanOrEqual(2);
  });

  it("parses sequential @code-block nodes with linear flow-out", () => {
    const fnBody = [
      "// @code-block",
      "step1();",
      "// @code-block",
      "step2();",
      "// @code-block",
      "step3();",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    const codeBlocks = result.elements.filter((e) => e.tag === "code-block");
    expect(codeBlocks).toHaveLength(3);
    // Linear flow-out: step1 → step2, step2 → step3
    const flowWires = result.wires.filter((w) => w.fromPort === "flow-out");
    expect(flowWires).toHaveLength(2);
  });

  it("parses if/elif/else with manual annotations", () => {
    const fnBody = [
      "// @if",
      "if (x > 0) {",
      "  // @code-block",
      "  doA();",
      "}",
      "// @elif",
      "else if (x == 0) {",
      "  // @code-block",
      "  doB();",
      "}",
      "// @else",
      "else {",
      "  // @code-block",
      "  doC();",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    const conditions = result.elements.filter((e) => e.tag === "condition");
    expect(conditions).toHaveLength(1);
    // Should have br0, br1, br2 wires
    const brWires = result.wires.filter((w) => w.fromPort.startsWith("br"));
    expect(brWires).toHaveLength(3);
  });

  it("returns pidMap for @type annotations", () => {
    const fnBody = [
      "// @if",
      "if (x > 0) {",
      "  // @return",
      "  return x;",
      "}",
      "// @return",
      "return 0;",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    expect(result.pidMap).toBeDefined();
    expect(result.pidMap.size).toBe(3); // @if + 2 × @return
    // Verify pidMap keys use correct line indices
    for (const [key, pid] of result.pidMap) {
      expect(key).toMatch(/^(if|return):\d+$/);
      expect(pid).toMatch(/^(if|return)-[a-z0-9-]+$/);
    }
  });

  it("normalizeAnnotations replaces @type with @pid in source", () => {
    const fnBody = [
      "// @code-block",
      "step1();",
      "// @return",
      "return x;",
    ].join("\n");

    const { pidMap } = parseCodeFlowGraph(fnBody);
    const normalized = normalizeAnnotations(fnBody, pidMap);

    // After normalization, all // @<type> should be replaced with // @pid:xxx
    expect(normalized).not.toContain("// @code-block");
    expect(normalized).not.toContain("// @return");
    expect(normalized).toContain("// @pid:");
    // Code lines should remain unchanged
    expect(normalized).toContain("step1();");
    expect(normalized).toContain("return x;");
  });

  it("normalizeAnnotations leaves @pid-only source unchanged", () => {
    const fnBody = [
      "// @pid:abc",
      "step1();",
      "// @pid:def",
      "return x;",
    ].join("\n");

    const { pidMap } = parseCodeFlowGraph(fnBody);
    const normalized = normalizeAnnotations(fnBody, pidMap);

    expect(normalized).toBe(fnBody);
  });

  it("parses for-loop annotation as code-block", () => {
    const fnBody = [
      "// @for",
      "for (let i = 0; i < n; i++) {",
      "  process(i);",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    const els = result.elements.filter((e) => e.tag === "code-block");
    expect(els).toHaveLength(1);
  });

  it("parses while-loop annotation as code-block", () => {
    const fnBody = [
      "// @while",
      "while (cond) {",
      "  work();",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    const els = result.elements.filter((e) => e.tag === "code-block");
    expect(els).toHaveLength(1);
  });
});
