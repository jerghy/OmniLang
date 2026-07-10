// poiesis/src/services/codegen/__tests__/save-inline-store.test.ts
// Integration tests: save pipeline emits inline stores inside component bodies.

import { describe, it, expect } from "vitest";
import { generateOmniFiles, generateOmniSource } from "../pipeline";
import type { StoreDefinition } from "../../../core/types/store";
import type { CodeTab } from "../../../core/types";

// Shared minimal code tab used across tests so the pipeline always has a main tab.
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

describe("save pipeline inline stores", () => {
  it("generateOmniSource includes inline stores in component output", () => {
    const inlineStore: StoreDefinition = {
      pid: "s1",
      name: "Test.CartStore",
      filePath: "src/Test.omni",
      imports: [],
      signals: [
        { pid: "s1_n1", name: "items", type: "Array<String>", defaultValue: "[]" },
        { pid: "s1_n2", name: "count", type: "i32", defaultValue: "0" },
      ],
      getters: [],
      actions: [],
    };

    const result = generateOmniSource({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "Test",
      inlineStores: [inlineStore],
    });

    expect(result.value).not.toBeNull();
    expect(result.value!).toContain("store CartStore");
    expect(result.value!).toContain("signal items: Array<String>");
    expect(result.value!).toContain("signal count: i32");
    // Inline store should appear inside the component body
    const compOpen = result.value!.indexOf("component Test {");
    const storeOpen = result.value!.indexOf("store CartStore");
    expect(compOpen).toBeGreaterThanOrEqual(0);
    expect(storeOpen).toBeGreaterThan(compOpen);
  });

  it("generateOmniFiles includes inline stores in component output", () => {
    const inlineStore: StoreDefinition = {
      pid: "s1",
      name: "Test.CartStore",
      filePath: "src/Test.omni",
      imports: [],
      signals: [{ pid: "s1_n1", name: "items", type: "Array<String>", defaultValue: "[]" }],
      getters: [],
      actions: [],
    };

    const result = generateOmniFiles({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "Test",
      rootOrder: [],
      filePath: "src/Test.omni",
      inlineStores: [inlineStore],
    });

    expect(result.value).not.toBeNull();
    const source = result.value!["src/Test.omni"] ?? "";
    expect(source).toContain("store CartStore");
    expect(source).toContain("signal items: Array<String>");
    // Inline store should appear inside the component body
    const compOpen = source.indexOf("component Test {");
    const storeOpen = source.indexOf("store CartStore");
    expect(compOpen).toBeGreaterThanOrEqual(0);
    expect(storeOpen).toBeGreaterThan(compOpen);
  });

  it("generateOmniFiles emits multiple inline stores inside component", () => {
    const storeA: StoreDefinition = {
      pid: "s1",
      name: "Test.StoreA",
      filePath: "src/Test.omni",
      imports: [],
      signals: [{ pid: "s1_n1", name: "x", type: "String", defaultValue: "" }],
      getters: [],
      actions: [],
    };
    const storeB: StoreDefinition = {
      pid: "s2",
      name: "Test.StoreB",
      filePath: "src/Test.omni",
      imports: [],
      signals: [{ pid: "s2_n1", name: "y", type: "i32", defaultValue: "0" }],
      getters: [],
      actions: [],
    };

    const result = generateOmniFiles({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "Test",
      rootOrder: [],
      filePath: "src/Test.omni",
      inlineStores: [storeA, storeB],
    });

    expect(result.value).not.toBeNull();
    const source = result.value!["src/Test.omni"] ?? "";
    expect(source).toContain("store StoreA");
    expect(source).toContain("store StoreB");
    expect(source).toContain("signal x: String");
    expect(source).toContain("signal y: i32");
  });

  it("generateOmniSource works without inline stores (backward compat)", () => {
    const result = generateOmniSource({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "App",
    });

    expect(result.value).not.toBeNull();
    expect(result.value!).toContain("component App");
    // No inline store block
    expect(result.value!).not.toContain("store ");
  });

  it("generateOmniFiles works without inlineStores (backward compat)", () => {
    const result = generateOmniFiles({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "App",
      rootOrder: [],
      filePath: "src/main.omni",
    });

    expect(result.value).not.toBeNull();
    const source = result.value!["src/main.omni"] ?? "";
    expect(source).toContain("component App");
  });

  it("inline stores with actions and getters survive full pipeline", () => {
    const inlineStore: StoreDefinition = {
      pid: "s1",
      name: "Test.FullStore",
      filePath: "src/Test.omni",
      imports: [],
      signals: [{ pid: "s1_n1", name: "items", type: "Array<String>", defaultValue: "[]" }],
      getters: [
        { pid: "s1_g1", name: "count", returnType: "i32", expression: "items.len()" },
      ],
      actions: [
        {
          pid: "s1_a1",
          name: "addItem",
          params: [{ name: "item", type: "String" }],
          returnType: undefined,
          isAsync: false,
          elements: [],
          wires: [],
        },
      ],
    };

    const result = generateOmniFiles({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "Test",
      rootOrder: [],
      filePath: "src/Test.omni",
      inlineStores: [inlineStore],
    });

    expect(result.value).not.toBeNull();
    const source = result.value!["src/Test.omni"] ?? "";
    expect(source).toContain("store FullStore");
    expect(source).toContain("signal items: Array<String>");
    expect(source).toContain("getter count: i32");
    expect(source).toContain("action addItem(item: String)");
  });

  it("does not generate standalone file for inline stores", () => {
    const inlineStore: StoreDefinition = {
      pid: "s1",
      name: "Test.CartStore",
      filePath: "src/Test.omni",
      imports: [],
      signals: [{ pid: "s1_n1", name: "items", type: "Array<String>", defaultValue: "[]" }],
      getters: [],
      actions: [],
    };

    const result = generateOmniFiles({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "Test",
      rootOrder: [],
      filePath: "src/Test.omni",
      inlineStores: [inlineStore],
    });

    expect(result.value).not.toBeNull();
    // Inline stores should NOT produce a standalone file under src/stores/
    const standaloneKeys = Object.keys(result.value!).filter((k) => k.startsWith("src/stores/"));
    expect(standaloneKeys).toHaveLength(0);
    // Only the component file should exist
    expect(result.value!).toHaveProperty("src/Test.omni");
  });
});
