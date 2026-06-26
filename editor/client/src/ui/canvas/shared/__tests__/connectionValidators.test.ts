/** Connection validators 单元测试。*/

import { describe, it, expect } from "vitest";
import {
  validateConnection,
  wouldCreateCycle,
  getAllPorts,
} from "../connectionValidators";
import type { ElementData, WireData } from "../../../../core/types";

function makeEl(overrides: Partial<ElementData> = {}): ElementData {
  return {
    id: "el-1",
    nodeType: "codeblock",
    tag: "signal",
    props: {},
    children: [],
    parentId: null,
    ports: [
      { name: "value", direction: "out", portType: "data", type: "i32" },
      { name: "set", direction: "in", portType: "data", type: "i32" },
    ],
    ...overrides,
  };
}

// ── getAllPorts ─────────────────────────────────────────────────────

describe("getAllPorts", () => {
  it("returns element ports as-is for codeblock nodes", () => {
    const el = makeEl({ nodeType: "codeblock" });
    const ports = getAllPorts(el);
    expect(ports).toHaveLength(2);
    expect(ports[0].name).toBe("value");
  });

  it("returns empty array when no ports and not a view", () => {
    const el = makeEl({ nodeType: "codeblock", ports: undefined });
    const ports = getAllPorts(el);
    expect(ports).toHaveLength(0);
  });

  it("returns only explicit ports — event injection moved to design mode", () => {
    // Events are now bound in design mode PropertyPanel, not via code graph
    const el = makeEl({
      nodeType: "view",
      tag: "Button",
      ports: undefined,
    });
    const ports = getAllPorts(el);
    // No automatic event port injection — returns empty array
    expect(ports).toHaveLength(0);
  });
});

// ── validateConnection ──────────────────────────────────────────────

describe("validateConnection", () => {
  it("returns no issues for a valid data connection", () => {
    const from = makeEl({
      id: "a",
      ports: [{ name: "value", direction: "out", portType: "data", type: "i32" }],
    });
    const to = makeEl({
      id: "b",
      ports: [{ name: "deps", direction: "in", portType: "data", type: "i32" }],
    });
    const issues = validateConnection(
      from, "value", to, "deps", [],
      getAllPorts(from), getAllPorts(to),
    );
    // No errors
    expect(issues.filter((i) => i.severity === "error")).toHaveLength(0);
  });

  it("rejects type-incompatible connections (void → i32)", () => {
    const from = makeEl({
      id: "a",
      ports: [{ name: "out", direction: "out", portType: "data", type: "void" }],
    });
    const to = makeEl({
      id: "b",
      ports: [{ name: "value", direction: "in", portType: "data", type: "i32" }],
    });
    const issues = validateConnection(
      from, "out", to, "value", [],
      getAllPorts(from), getAllPorts(to),
    );
    expect(issues.some((i) => i.code === "TYPE_INCOMPATIBLE")).toBe(true);
  });

  it("rejects duplicate wires", () => {
    const from = makeEl({
      id: "a",
      ports: [{ name: "value", direction: "out", portType: "data", type: "i32" }],
    });
    const to = makeEl({
      id: "b",
      ports: [{ name: "deps", direction: "in", portType: "data", type: "i32" }],
    });
    const existingWire: WireData = {
      id: "existing",
      fromId: "a",
      fromPort: "value",
      toId: "b",
      toPort: "deps",
    };
    const issues = validateConnection(
      from, "value", to, "deps", [existingWire],
      getAllPorts(from), getAllPorts(to),
    );
    expect(issues.some((i) => i.code === "DUPLICATE_WIRE")).toBe(true);
  });

  it("warns on fan-out (one output → multiple inputs)", () => {
    const from = makeEl({
      id: "a",
      ports: [{ name: "value", direction: "out", portType: "data", type: "i32" }],
    });
    const to = makeEl({
      id: "b",
      ports: [{ name: "deps", direction: "in", portType: "data", type: "i32" }],
    });
    const existingWire: WireData = {
      id: "existing",
      fromId: "a",
      fromPort: "value",
      toId: "c",
      toPort: "deps",
    };
    const issues = validateConnection(
      from, "value", to, "deps", [existingWire],
      getAllPorts(from), getAllPorts(to),
    );
    expect(issues.some((i) => i.code === "FAN_OUT_WARNING")).toBe(true);
  });

  it("warns when port not found", () => {
    const from = makeEl({
      id: "a",
      ports: [{ name: "value", direction: "out", portType: "data", type: "i32" }],
    });
    const to = makeEl({
      id: "b",
      ports: [{ name: "deps", direction: "in", portType: "data", type: "i32" }],
    });
    const issues = validateConnection(
      from, "nonexistent", to, "deps", [],
      getAllPorts(from), getAllPorts(to),
    );
    expect(issues.some((i) => i.code === "PORT_NOT_FOUND")).toBe(true);
  });
});

// ── wouldCreateCycle ────────────────────────────────────────────────

describe("wouldCreateCycle", () => {
  it("detects a simple cycle A→B→A", () => {
    const wires: WireData[] = [
      { id: "w1", fromId: "a", fromPort: "out", toId: "b", toPort: "in" },
    ];
    // Adding b→a would create a cycle a→b→a
    expect(wouldCreateCycle("b", "a", wires)).toBe(true);
  });

  it("returns false when no cycle exists", () => {
    const wires: WireData[] = [
      { id: "w1", fromId: "a", fromPort: "out", toId: "b", toPort: "in" },
    ];
    // Adding a→c is safe
    expect(wouldCreateCycle("a", "c", wires)).toBe(false);
  });

  it("returns false for an empty wire set", () => {
    expect(wouldCreateCycle("a", "b", [])).toBe(false);
  });
});
