/** FlowEngine 单元测试。*/

import { describe, it, expect } from "vitest";
import { executeFlow, validateFlow } from "../FlowEngine";
import type { ElementData, WireData } from "../../../../core/types";

function makeEl(overrides: Partial<ElementData> = {}): ElementData {
  return {
    id: "el-1",
    nodeType: "codeblock",
    tag: "signal",
    props: {},
    children: [],
    parentId: null,
    ...overrides,
  };
}

// ── executeFlow ─────────────────────────────────────────────────────

describe("executeFlow", () => {
  it("returns empty steps when there are no event nodes", () => {
    const elements = [
      makeEl({ id: "a", tag: "signal", nodeCategory: "variable" }),
    ];
    const result = executeFlow(elements, []);
    expect(result.steps).toHaveLength(0);
  });

  it("marks all codeblocks as unreachable when no event nodes", () => {
    const elements = [
      makeEl({ id: "a", tag: "signal", nodeCategory: "variable", nodeType: "codeblock" }),
    ];
    const result = executeFlow(elements, []);
    expect(result.unreachable).toContain("a");
  });

  it("finds event nodes as execution roots", () => {
    const elements = [
      makeEl({
        id: "evt",
        tag: "onPress",
        nodeCategory: "event",
        ports: [
          { name: "flowOut", direction: "out", portType: "flow", type: "void" },
        ],
      }),
      makeEl({
        id: "act",
        tag: "action",
        nodeCategory: "action",
        ports: [
          { name: "call", direction: "in", portType: "flow", type: "void" },
          { name: "flowOut", direction: "out", portType: "flow", type: "void" },
        ],
      }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "evt", fromPort: "flowOut", toId: "act", toPort: "call" },
    ];
    const result = executeFlow(elements, wires);
    expect(result.steps.length).toBeGreaterThanOrEqual(1);
    expect(result.unreachable).toHaveLength(0);
  });

  it("marks codeblocks unreachable when not connected to any event", () => {
    const elements = [
      makeEl({
        id: "evt",
        tag: "onPress",
        nodeCategory: "event",
        ports: [{ name: "flowOut", direction: "out", portType: "flow", type: "void" }],
      }),
      makeEl({ id: "orphan", tag: "signal", nodeCategory: "variable", nodeType: "codeblock" }),
    ];
    const result = executeFlow(elements, []);
    // The orphan signal is not visited
    expect(result.unreachable).toContain("orphan");
  });
});

// ── validateFlow ────────────────────────────────────────────────────

describe("validateFlow", () => {
  it("returns valid=true when there are no cycles", () => {
    const elements = [
      makeEl({
        id: "a",
        tag: "onPress",
        nodeCategory: "event",
        ports: [{ name: "out", direction: "out", portType: "flow", type: "void" }],
      }),
      makeEl({
        id: "b",
        tag: "action",
        nodeCategory: "action",
        ports: [
          { name: "in", direction: "in", portType: "flow", type: "void" },
        ],
      }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "a", fromPort: "out", toId: "b", toPort: "in" },
    ];
    const result = validateFlow(elements, wires);
    expect(result.valid).toBe(true);
    expect(result.cycles).toHaveLength(0);
  });

  it("detects flow cycles", () => {
    const elements = [
      makeEl({
        id: "a",
        tag: "branch",
        nodeCategory: "flow",
        ports: [
          { name: "in", direction: "in", portType: "flow", type: "void" },
          { name: "true", direction: "out", portType: "flow", type: "void" },
        ],
      }),
      makeEl({
        id: "b",
        tag: "action",
        nodeCategory: "action",
        ports: [
          { name: "in", direction: "in", portType: "flow", type: "void" },
          { name: "out", direction: "out", portType: "flow", type: "void" },
        ],
      }),
    ];
    const wires: WireData[] = [
      { id: "w1", fromId: "a", fromPort: "true", toId: "b", toPort: "in" },
      { id: "w2", fromId: "b", fromPort: "out", toId: "a", toPort: "in" },
    ];
    const result = validateFlow(elements, wires);
    expect(result.cycles.length).toBeGreaterThan(0);
  });

  it("finds orphan flow inputs (action nodes with no incoming flow)", () => {
    const elements = [
      makeEl({
        id: "act",
        tag: "action",
        nodeCategory: "action",
        ports: [
          { name: "call", direction: "in", portType: "flow", type: "void" },
        ],
      }),
    ];
    const result = validateFlow(elements, []);
    expect(result.orphanFlowIns).toContain("act");
  });

  it("valid=true for empty canvas", () => {
    const result = validateFlow([], []);
    expect(result.valid).toBe(true);
    expect(result.cycles).toHaveLength(0);
    expect(result.orphanFlowIns).toHaveLength(0);
  });
});
