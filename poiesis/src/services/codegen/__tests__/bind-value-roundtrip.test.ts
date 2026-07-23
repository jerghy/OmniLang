// poiesis/src/services/codegen/__tests__/bind-value-roundtrip.test.ts
// Tests the bind:value two-way binding round-trip: parse → store → emit

import { describe, test, expect } from "vitest";
import { generateOmniSource } from "../pipeline";
import type { ElementData, CodeTab } from "../../../core/types";

// Minimal code tab matching existing pipeline tests
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

describe("bind:value round-trip", () => {
  test("element with bindings emits bind:value={sig}", () => {
    const elements: ElementData[] = [
      {
        id: "el-1",
        nodeType: "view",
        tag: "input",
        props: { type: "text", placeholder: "Enter todo" },
        bindings: { value: "newTodo" },
        children: [],
        parentId: null,
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "TestComponent",
    });

    const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
    expect(fatalErrors).toHaveLength(0);
    expect(result.value).not.toBeNull();
    expect(result.value!).toContain("bind:value={newTodo}");
  });

  test("element without bindings does not produce bind:value", () => {
    const elements: ElementData[] = [
      {
        id: "el-1",
        nodeType: "view",
        tag: "input",
        props: { type: "text" },
        children: [],
        parentId: null,
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "NoBindComponent",
    });

    const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
    expect(fatalErrors).toHaveLength(0);
    expect(result.value).not.toBeNull();
    expect(result.value!).not.toContain("bind:value");
  });

  test("wire bindings take precedence over element.bindings", () => {
    const elements: ElementData[] = [
      {
        id: "sig-1",
        nodeType: "codeblock",
        tag: "signal",
        props: { name: "count", type: "i32", defaultValue: "0" },
        source: "",
        parentId: null,
      },
      {
        id: "el-1",
        nodeType: "view",
        tag: "input",
        props: { type: "text" },
        bindings: { value: "other" },
        children: [],
        parentId: null,
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [
        {
          id: "w1",
          fromId: "sig-1",
          fromPort: "value",
          toId: "el-1",
          toPort: "value",
        },
      ],
      codeTabs: basicCodeTabs,
      componentName: "WireTest",
    });

    const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
    expect(fatalErrors).toHaveLength(0);
    expect(result.value).not.toBeNull();
    // Wire binding "count" wins over element binding "other"
    expect(result.value!).toContain("bind:value={count}");
    expect(result.value!).not.toContain("bind:value={other}");
  });
});
