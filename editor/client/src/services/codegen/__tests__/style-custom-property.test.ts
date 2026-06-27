// editor/client/src/services/codegen/__tests__/style-custom-property.test.ts
// End-to-end tests for CSS custom property (--name) support in the WASM compiler.
// These tests compile .omni source through the Rust WASM compiler and verify
// that CSS custom properties are handled correctly in both static style blocks
// and dynamic inline styles.

import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { resolveStyle } from "../style-resolver";
import { parseStylesFromAST, generateOmniSource } from "../pipeline";
import { collectStylesFromEntries } from "../collect-styles";
import { parseOmniSource } from "../omni-parser";
import type { StyleRegistry } from "../../../core/types";
import type { ViewNode } from "../types";
import type { ElementData } from "../../../core/types";
import { rustCompiler } from "../../../api/rustCompiler";
import { useStyleRegistryStore } from "../../../core/state/styleRegistry";

beforeAll(async () => {
  await rustCompiler.init();
});

beforeEach(() => {
  useStyleRegistryStore.setState({ registries: {} });
});

beforeAll(async () => {
  await rustCompiler.init();
});

describe("WASM -- CSS custom properties", () => {
  it("parses --custom-property in style block", () => {
    const src = [
      "component Test {",
      "    style {",
      '        .foo { --my-color: #333; color: var(--my-color); }',
      "    }",
      '    view { <div class: foo/> }',
      "}",
    ].join("\n");

    const result = rustCompiler.compileToOmniView(src, true);
    expect(result.success).toBe(true);
    expect(result.files!.css).toContain("--my-color");
    expect(result.files!.css).toContain("var(--my-color)");
  });

  it("generates setProperty for dynamic -- inline styles", () => {
    const src = [
      "component Test {",
      '    signal s: String = "#333";',
      '    view { <div style: { --bg: s }/> }',
      "}",
    ].join("\n");

    const result = rustCompiler.compileToOmniView(src, true);
    expect(result.success).toBe(true);
    expect(result.files!.js).toContain('setProperty("--bg"');
  });

  it("returns error for malformed source", () => {
    const result = rustCompiler.compileToOmniView("invalid @@ garbage", true);
    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.length).toBeGreaterThan(0);
  });
});

describe("resolveStyle — CSS custom properties", () => {
  it("routes --prefixed keys to cssRules not htmlAttrs", () => {
    const node: ViewNode = {
      id: "1",
      tag: "Container",
      path: "root.container",
      props: { "--primary-color": "#333", padding: "16px" },
      children: [],
      kind: "element",
    };
    const result = resolveStyle(node);
    expect(result.cssRules["--primary-color"]).toBe("#333");
    expect(result.htmlAttrs["--primary-color"]).toBeUndefined();
  });

  it("backward compatible: non-prefixed keys still map through STYLE_PROP_CSS_MAP", () => {
    const node: ViewNode = {
      id: "2",
      tag: "Container",
      path: "root.container",
      props: { background_color: "#eee" },
      children: [],
      kind: "element",
    };
    const result = resolveStyle(node);
    expect(result.cssRules["background"]).toBe("#eee");
  });
});

describe("collectStylesFromEntries", () => {
  it("generates componentStyle with class entries", () => {
    const registry: StyleRegistry = {
      classes: [{
        id: "s1", name: "card",
        properties: [{ id: "p1", name: "padding", segments: [{ type: "text", value: "16px" }] }],
      }],
      variables: [],
    };
    const elements = [{ id: "a", style: { classRefs: ["card"], inlineProperties: [] } }];
    const result = collectStylesFromEntries(elements, registry);
    expect(result.componentStyle["card"]).toEqual({ padding: '"16px"' });
    expect(result.elementClassMap.get("a")).toEqual(["card"]);
  });

  it("generates variable entries with _v- prefix class", () => {
    const registry: StyleRegistry = {
      classes: [],
      variables: [{
        id: "s1", name: "--primary-color",
        segments: [{ type: "text", value: "#333" }],
      }],
    };
    const elements = [{ id: "a", style: { classRefs: ["_v-primary-color"], inlineProperties: [] } }];
    const result = collectStylesFromEntries(elements, registry);
    expect(result.componentStyle["_v-primary-color"]).toEqual({ "--primary-color": '"#333"' });
    expect(result.elementClassMap.get("a")).toEqual(["_v-primary-color"]);
  });

  it("generates inline styles separately", () => {
    const registry: StyleRegistry = { classes: [], variables: [] };
    const elements = [{
      id: "a",
      style: { classRefs: [], inlineProperties: [{ id: "p1", name: "padding", segments: [{ type: "text", value: "8px" }] }] },
    }];
    const result = collectStylesFromEntries(elements, registry);
    expect(result.elementInlineMap.get("a")).toEqual({ padding: '"8px"' });
    expect(result.elementClassMap.get("a")).toBeUndefined();
  });

  it("skips elements without style", () => {
    const registry: StyleRegistry = { classes: [], variables: [] };
    const elements = [{ id: "a" }];
    const result = collectStylesFromEntries(elements, registry);
    expect(Object.keys(result.componentStyle)).toHaveLength(0);
    expect(result.elementClassMap.size).toBe(0);
    expect(result.elementInlineMap.size).toBe(0);
  });

  it("handles multiple elements with mixed entry types", () => {
    const registry: StyleRegistry = {
      classes: [{
        id: "s1", name: "btn",
        properties: [{ id: "p1", name: "color", segments: [{ type: "text", value: "red" }] }],
      }],
      variables: [{
        id: "s2", name: "--gap",
        segments: [{ type: "text", value: "8px" }],
      }],
    };
    const elements = [
      { id: "a", style: { classRefs: ["btn"], inlineProperties: [] } },
      {
        id: "b",
        style: {
          classRefs: ["_v-gap"],
          inlineProperties: [{ id: "p3", name: "display", segments: [{ type: "text", value: "flex" }] }],
        },
      },
    ];
    const result = collectStylesFromEntries(elements, registry);
    expect(result.componentStyle["btn"]).toEqual({ color: '"red"' });
    expect(result.componentStyle["_v-gap"]).toEqual({ "--gap": '"8px"' });
    expect(result.elementClassMap.get("a")).toEqual(["btn"]);
    expect(result.elementClassMap.get("b")).toEqual(["_v-gap"]);
    expect(result.elementInlineMap.get("b")).toEqual({ display: '"flex"' });
  });
});

describe("parseStylesFromAST", () => {
  it("restores class type from style { } block", () => {
    const componentAST: any = {
      name: "test",
      style: [
        { class_name: "card", properties: { padding: { value: "16px" }, color: { value: "#333" } } },
      ],
    };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: {}, nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: { class: "card" }, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const el = elements.get("a")!;
    expect(el.style?.classRefs).toContain("card");
    const registry = useStyleRegistryStore.getState().getRegistry("test");
    expect(registry.classes).toHaveLength(1);
    expect(registry.classes[0].name).toBe("card");
    expect(registry.classes[0].properties).toHaveLength(2);
  });

  it("restores variable type from _v- prefixed class", () => {
    const componentAST: any = {
      name: "test",
      style: [
        { class_name: "_v-primary-color", properties: { "--primary-color": { value: "#4F46E5" } } },
      ],
    };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: {}, nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: { class: "_v-primary-color" }, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const el = elements.get("a")!;
    expect(el.style?.classRefs).toContain("_v-primary-color");
    const registry = useStyleRegistryStore.getState().getRegistry("test");
    expect(registry.variables).toHaveLength(1);
    expect(registry.variables[0].name).toBe("--primary-color");
  });

  it("restores inline type from style: attribute", () => {
    const componentAST: any = { name: "test", style: [] };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: {}, nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: {}, inline_style: { padding: "8px" }, children: [] };

    parseStylesFromAST(componentAST, elements);
    const el = elements.get("a")!;
    expect(el.style?.inlineProperties).toHaveLength(1);
    expect(el.style?.inlineProperties[0]).toEqual({
      id: expect.any(String),
      name: "padding",
      segments: [{ type: "text", value: "8px" }],
    });
  });

  it("backward compat: props style keys → inline properties when no style block", () => {
    const componentAST: any = { name: "test", style: undefined };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: { padding: "16px", label: "Submit", background_color: "#eee" }, nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: {}, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const el = elements.get("a")!;
    expect(el.style?.inlineProperties.length).toBeGreaterThan(0);
    const propNames = el.style!.inlineProperties.map(p => p.name);
    expect(propNames).toContain("padding");
    expect(propNames).toContain("background_color");
    expect(propNames).not.toContain("label");
  });
});

describe("Integration: StyleEntry codegen and round-trip", () => {
  it("round-trips class and variable through generate -> parse", () => {
    // Populate the registry with class and variable definitions
    useStyleRegistryStore.getState().addClass("Test", {
      id: "s_c1",
      name: "card",
      properties: [
        { id: "p1", name: "padding", segments: [{ type: "text", value: "16px" }] },
        { id: "p2", name: "color", segments: [{ type: "text", value: "#333" }] },
      ],
    });
    useStyleRegistryStore.getState().addVariable("Test", {
      id: "s_v1",
      name: "--primary-color",
      segments: [{ type: "text", value: "#4F46E5" }],
    });

    const elements: ElementData[] = [
      {
        id: "root",
        nodeType: "view",
        tag: "div",
        props: {},
        children: ["child1"],
        parentId: null,
      },
      {
        id: "child1",
        nodeType: "view",
        tag: "div",
        props: {},
        children: [],
        parentId: "root",
        style: { classRefs: ["card", "_v-primary-color"], inlineProperties: [] },
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "", graph: { elements: [], wires: [] }, variables: [] }],
      componentName: "Test",
    });

    expect(result.errors.some(e => e.severity === "fatal")).toBe(false);
    expect(result.value).not.toBeNull();

    // Reset registry for parse (parseStylesFromAST will repopulate it)
    useStyleRegistryStore.setState({ registries: {} });

    const parsed = parseOmniSource(result.value!);
    const hardErrors = parsed.errors.filter(e => e.severity === "error");
    expect(hardErrors).toHaveLength(0);
    expect(parsed.designElements).not.toBeNull();

    const parsedChild = parsed.designElements!.find(el => el.id === "child1");
    expect(parsedChild).toBeDefined();
    expect(parsedChild!.style?.classRefs).toBeDefined();
    expect(parsedChild!.style!.classRefs!.length).toBe(2);
    expect(parsedChild!.style!.classRefs).toContain("card");
    expect(parsedChild!.style!.classRefs).toContain("_v-primary-color");
  });

  it("quotes inline style values in generated .omni source", () => {
    const elements: ElementData[] = [
      {
        id: "root",
        nodeType: "view",
        tag: "div",
        props: {},
        children: [],
        parentId: null,
        style: {
          classRefs: [],
          inlineProperties: [
            { id: "p1", name: "padding", segments: [{ type: "text", value: "16px" }] },
          ],
        },
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "", graph: { elements: [], wires: [] }, variables: [] }],
      componentName: "Test",
    });

    expect(result.errors.some(e => e.severity === "fatal")).toBe(false);
    expect(result.value).not.toBeNull();

    // The source emitter uses compiled segments from collectStylesFromEntries:
    // text values appear quoted, so inline style becomes: style: { padding: "16px" }
    expect(result.value!).toContain("style: {");
    expect(result.value!).toContain('padding: "16px"');
  });

  it("collectStylesFromEntries handles mixed class+variable+inline entries on same element", () => {
    const registry: StyleRegistry = {
      classes: [{
        id: "s1", name: "btn",
        properties: [
          { id: "p1", name: "color", segments: [{ type: "text", value: "red" }] },
          { id: "p2", name: "padding", segments: [{ type: "text", value: "8px" }] },
        ],
      }],
      variables: [{
        id: "s2", name: "--gap",
        segments: [{ type: "text", value: "8px" }],
      }],
    };
    const elements = [{
      id: "a",
      style: {
        classRefs: ["btn", "_v-gap"],
        inlineProperties: [{ id: "p3", name: "display", segments: [{ type: "text", value: "flex" }] }],
      },
    }];

    const result = collectStylesFromEntries(elements, registry);

    expect(result.componentStyle["btn"]).toEqual({ color: '"red"', padding: '"8px"' });
    expect(result.componentStyle["_v-gap"]).toEqual({ "--gap": '"8px"' });
    expect(result.elementClassMap.get("a")).toEqual(["btn", "_v-gap"]);
    expect(result.elementInlineMap.get("a")).toEqual({ display: '"flex"' });
  });

  it("parses class: attribute when it is the last attribute before >", () => {
    // Regression test: parser must capture class names when class: is the
    // last attribute before the closing > without a trailing space.
    // Each element must be on its own line for parseOmniSource.
    const src = [
      "component Test {",
      "    view {",
      '        <div class: foo/>',
      "    }",
      "}",
    ].join("\n");
    const parsed = parseOmniSource(src);
    const el = parsed.designElements?.find(e => e.props.className === "foo");
    expect(el).toBeDefined();
  });

  it("parses class: attribute with multiple names before >", () => {
    // Same as above but with multiple space-separated class names
    const src = [
      "component Test {",
      "    view {",
      '        <div class: foo bar baz/>',
      "    }",
      "}",
    ].join("\n");
    const parsed = parseOmniSource(src);
    const el = parsed.designElements?.find(e => e.props.className === "foo bar baz");
    expect(el).toBeDefined();
  });

  it("parses class: attribute before other attributes (not last)", () => {
    // Verify that existing behavior (class: before other attrs) still works
    const src = [
      "component Test {",
      "    view {",
      '        <div class: foo id="bar"/>',
      "    }",
      "}",
    ].join("\n");
    const parsed = parseOmniSource(src);
    const el = parsed.designElements?.find(e => e.props.className === "foo");
    expect(el).toBeDefined();
  });

  it("round-trips inline style through generate -> parse", () => {
    const elements: ElementData[] = [
      {
        id: "root",
        nodeType: "view",
        tag: "div",
        props: {},
        children: [],
        parentId: null,
        style: {
          classRefs: [],
          inlineProperties: [
            { id: "p1", name: "padding", segments: [{ type: "text", value: "16px" }] },
            { id: "p2", name: "color", segments: [{ type: "text", value: "#333" }] },
          ],
        },
      },
    ];

    const result = generateOmniSource({
      designElements: elements,
      designWires: [],
      codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "", graph: { elements: [], wires: [] }, variables: [] }],
      componentName: "Test",
    });

    expect(result.errors.some(e => e.severity === "fatal")).toBe(false);
    expect(result.value).not.toBeNull();

    // Reset registry for parse
    useStyleRegistryStore.setState({ registries: {} });

    const parsed = parseOmniSource(result.value!);
    const hardErrors = parsed.errors.filter(e => e.severity === "error");
    expect(hardErrors).toHaveLength(0);
    expect(parsed.designElements).not.toBeNull();

    const parsedEl = parsed.designElements!.find(el => el.id === "root");
    expect(parsedEl).toBeDefined();

    // Inline styles are parsed back into the element's inlineProperties
    const inlineProps = parsedEl!.style?.inlineProperties ?? [];
    const propValues = inlineProps.map(p => ({
      name: p.name,
      value: p.segments.map(s => s.type === "text" ? s.value : "").join(""),
    }));
    expect(propValues).toContainEqual({ name: "padding", value: "16px" });
    expect(propValues).toContainEqual({ name: "color", value: "#333" });
  });

  it("backward compatible: old props-based styles become inline properties", () => {
    // Old-format .omni source without style {} block — style keys in props
    const src = [
      "component Test {",
      "    view {",
      '        <div id="root" padding="16px" color="#333" label="Submit"/>',
      "    }",
      "}",
    ].join("\n");

    const parsed = parseOmniSource(src);
    const el = parsed.designElements?.find(e => e.id === "root");
    expect(el).toBeDefined();
    expect(el!.style?.inlineProperties).toBeDefined();
    expect(el!.style!.inlineProperties!.length).toBeGreaterThanOrEqual(1);
    const propNames = el!.style!.inlineProperties.map(p => p.name);
    expect(propNames).toContain("padding");
    expect(propNames).toContain("color");
    // label is NOT a style key → should NOT appear in styles
    expect(propNames).not.toContain("label");
  });
});
