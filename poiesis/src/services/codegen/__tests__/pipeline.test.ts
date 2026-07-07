import { describe, it, expect } from "vitest";
import { generateOmniSource } from "../pipeline";
import type { ElementData, WireData, CodeTab } from "../../../core/types";

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

describe("generateOmniSource", () => {
  it("should produce valid output with empty elements/wires and a basic component", () => {
    const result = generateOmniSource({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "App",
      forSave: true,
    });

    // No fatal errors with minimal valid input
    const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
    expect(fatalErrors).toHaveLength(0);

    // Value should be a non-null string
    expect(result.value).not.toBeNull();
    expect(typeof result.value).toBe("string");

    // The output should declare the component
    expect(result.value).toContain("component App");
  });

  it("should produce a ViewNode tree for a basic component with one view element", () => {
    const elements: ElementData[] = [
      {
        id: "el-1",
        nodeType: "view",
        tag: "div",
        props: {},
        children: [],
        parentId: null,
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "App",
      forSave: true,
    });

    // Phase A produced a ViewNode tree that survived to source emission
    const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
    expect(fatalErrors).toHaveLength(0);
    expect(result.value).not.toBeNull();
    expect(result.value!).toContain("component App");

    // The single root div should appear in the emitted view tree
    expect(result.value!).toContain("div");
    expect(result.value!).toContain("el-1");
  });

  it("should extract stores from codeblock elements (Phase B)", () => {
    const elements: ElementData[] = [
      {
        id: "store-1",
        nodeType: "codeblock",
        tag: "store",
        props: { name: "TodoStore" },
        parentId: null,
        source: "",
      },
      {
        id: "sig-1",
        nodeType: "codeblock",
        tag: "signal",
        props: { name: "items", typeAnnotation: "string[]" },
        parentId: "store-1",
        source: "",
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "App",
      forSave: true,
    });

    // Verify Phase B extracted and emitted the store definition
    const fatalErrors = result.errors.filter((e) => e.severity === "fatal");
    expect(fatalErrors).toHaveLength(0);
    expect(result.value).not.toBeNull();
    expect(result.value!).toContain("store TodoStore");
    expect(result.value!).toContain("signal items: string[]");
  });

  it("should return errors in the correct StageResult shape", () => {
    const result = generateOmniSource({
      designElements: [],
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "App",
      forSave: true,
    });

    // StageResult<T> = { value: T | null; errors: PipelineError[] }
    expect(result).toHaveProperty("value");
    expect(result).toHaveProperty("errors");
    expect(Array.isArray(result.errors)).toBe(true);

    // Each error should have the PipelineError shape if any exist
    if (result.errors.length > 0) {
      for (const err of result.errors) {
        expect(err).toHaveProperty("phase");
        expect(err).toHaveProperty("module");
        expect(err).toHaveProperty("severity");
        expect(err).toHaveProperty("code");
        expect(err).toHaveProperty("message");
      }
    }
  });

  it("should produce valid .omni source with imports and proper structure", () => {
    const elements: ElementData[] = [
      {
        id: "el-1",
        nodeType: "view",
        tag: "button",
        props: { label: "Click me" },
        children: [],
        parentId: null,
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: basicCodeTabs,
      componentName: "MyButton",
      forSave: true,
    });

    expect(result.value).not.toBeNull();
    // Should include the omniview import
    expect(result.value!).toContain("import omniview;");
    // Should have a component block
    expect(result.value!).toContain("component MyButton");
    // Should have a view block
    expect(result.value!).toContain("view {");
    // Should contain the button element
    expect(result.value!).toContain("button");
  });
});
