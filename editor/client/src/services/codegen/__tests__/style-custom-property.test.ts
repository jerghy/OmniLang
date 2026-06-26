// editor/client/src/omni-gen/__tests__/style-custom-property.test.ts
// End-to-end tests for CSS custom property (--name) support in the WASM compiler.
// These tests compile .omni source through the Rust WASM compiler and verify
// that CSS custom properties are handled correctly in both static style blocks
// and dynamic inline styles.

import { describe, it, expect, beforeAll } from "vitest";
import { resolveStyle } from "../style-resolver";
import { parseStylesFromAST, generateOmniSource } from "../pipeline";
import { collectStylesFromEntries } from "../collect-styles";
import { parseOmniSource } from "../omni-parser";
import type { StyleEntry } from "../../../core/types";
import type { ViewNode } from "../types";
import type { ElementData } from "../../../core/types";
import { rustCompiler } from "../../../api/rustCompiler";

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
    const styles: StyleEntry[] = [
      { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "16px" }] },
    ];
    const elements = [{ id: "a", styles }];
    const result = collectStylesFromEntries(elements);
    expect(result.componentStyle["card"]).toEqual({ padding: "16px" });
    expect(result.elementClassMap.get("a")).toEqual(["card"]);
  });

  it("generates variable entries with _v- prefix class", () => {
    const styles: StyleEntry[] = [
      { id: "s1", type: "variable", name: "--primary-color", properties: [{ name: "value", value: "#333" }] },
    ];
    const elements = [{ id: "a", styles }];
    const result = collectStylesFromEntries(elements);
    expect(result.componentStyle["_v-primary-color"]).toEqual({ "--primary-color": "#333" });
    expect(result.elementClassMap.get("a")).toEqual(["_v-primary-color"]);
  });

  it("generates inline styles separately", () => {
    const styles: StyleEntry[] = [
      { id: "s1", type: "inline", name: "", properties: [{ name: "padding", value: "8px" }] },
    ];
    const elements = [{ id: "a", styles }];
    const result = collectStylesFromEntries(elements);
    expect(result.elementInlineMap.get("a")).toEqual({ padding: "8px" });
    expect(result.elementClassMap.get("a")).toBeUndefined();
  });

  it("skips elements without styles", () => {
    const elements = [{ id: "a" }];
    const result = collectStylesFromEntries(elements);
    expect(Object.keys(result.componentStyle)).toHaveLength(0);
    expect(result.elementClassMap.size).toBe(0);
    expect(result.elementInlineMap.size).toBe(0);
  });

  it("handles multiple elements with mixed entry types", () => {
    const elA: StyleEntry[] = [
      { id: "s1", type: "class", name: "btn", properties: [{ name: "color", value: "red" }] },
    ];
    const elB: StyleEntry[] = [
      { id: "s2", type: "variable", name: "--gap", properties: [{ name: "value", value: "8px" }] },
      { id: "s3", type: "inline", name: "", properties: [{ name: "display", value: "flex" }] },
    ];
    const elements = [
      { id: "a", styles: elA },
      { id: "b", styles: elB },
    ];
    const result = collectStylesFromEntries(elements);
    expect(result.componentStyle["btn"]).toEqual({ color: "red" });
    expect(result.componentStyle["_v-gap"]).toEqual({ "--gap": "8px" });
    expect(result.elementClassMap.get("a")).toEqual(["btn"]);
    expect(result.elementClassMap.get("b")).toEqual(["_v-gap"]);
    expect(result.elementInlineMap.get("b")).toEqual({ display: "flex" });
  });
});

describe("parseStylesFromAST", () => {
  it("restores class type from style { } block", () => {
    const componentAST: any = {
      style: [
        { class_name: "card", properties: { padding: { value: "16px" }, color: { value: "#333" } } },
      ],
    };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: {}, styles: [], nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: { class: "card" }, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles).toHaveLength(1);
    expect(styles[0].type).toBe("class");
    expect(styles[0].name).toBe("card");
    expect(styles[0].properties).toHaveLength(2);
  });

  it("restores variable type from _v- prefixed class", () => {
    const componentAST: any = {
      style: [
        { class_name: "_v-primary-color", properties: { "--primary-color": { value: "#4F46E5" } } },
      ],
    };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: {}, styles: [], nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: { class: "_v-primary-color" }, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles).toHaveLength(1);
    expect(styles[0].type).toBe("variable");
    expect(styles[0].name).toBe("--primary-color");
  });

  it("restores inline type from style: attribute", () => {
    const componentAST: any = { style: [] };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: {}, styles: [], nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: {}, inline_style: { padding: "8px" }, children: [] };

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles).toHaveLength(1);
    expect(styles[0].type).toBe("inline");
    expect(styles[0].properties[0]).toEqual({ name: "padding", value: "8px" });
  });

  it("backward compat: props style keys → inline StyleEntry when no style block", () => {
    const componentAST: any = { style: undefined };
    const elements = new Map<string, ElementData>([["a", { id: "a", props: { padding: "16px", label: "Submit", background_color: "#eee" }, styles: [], nodeType: "view", tag: "div", children: [], parentId: null }]]);
    componentAST.view = { id: "a", props: {}, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles).toHaveLength(1);
    expect(styles[0].type).toBe("inline");
    const propNames = styles[0].properties.map(p => p.name);
    expect(propNames).toContain("padding");
    expect(propNames).toContain("background_color");
    expect(propNames).not.toContain("label");
  });
});

describe("Integration: StyleEntry codegen and round-trip", () => {
  it("round-trips class and variable StyleEntry through generate -> parse", () => {
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
        styles: [
          { id: "s_c1", type: "class", name: "card", properties: [{ name: "padding", value: "16px" }, { name: "color", value: "#333" }] },
          { id: "s_v1", type: "variable", name: "--primary-color", properties: [{ name: "value", value: "#4F46E5" }] },
        ],
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

    const parsed = parseOmniSource(result.value!);
    // Only check for hard errors — non-fatal warnings can vary by parser version
    const hardErrors = parsed.errors.filter(e => e.severity === "error");
    expect(hardErrors).toHaveLength(0);
    expect(parsed.designElements).not.toBeNull();

    const parsedChild = parsed.designElements!.find(el => el.id === "child1");
    expect(parsedChild).toBeDefined();
    expect(parsedChild!.styles).toBeDefined();
    expect(parsedChild!.styles!.length).toBe(2);

    // Verify class entry
    const classEntry = parsedChild!.styles!.find(s => s.type === "class")!;
    expect(classEntry.name).toBe("card");
    expect(classEntry.properties).toHaveLength(2);
    expect(classEntry.properties).toContainEqual({ name: "padding", value: "16px" });
    expect(classEntry.properties).toContainEqual({ name: "color", value: "#333" });

    // Verify variable entry: name and value preserved (property name may differ after round-trip
    // because the style block stores --primary-color: #4F46E5, not value: #4F46E5)
    const varEntry = parsedChild!.styles!.find(s => s.type === "variable")!;
    expect(varEntry.name).toBe("--primary-color");
    expect(varEntry.properties).toEqual(
      expect.arrayContaining([expect.objectContaining({ value: "#4F46E5" })])
    );
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
        styles: [
          { id: "s_i1", type: "inline", name: "", properties: [{ name: "padding", value: "16px" }] },
        ],
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

    // The source emitter wraps inline style values in double quotes:
    // style: { padding: "16px" }
    expect(result.value!).toContain("style: {");
    expect(result.value!).toContain('padding: "16px"');
  });

  it("collectStylesFromEntries handles mixed class+variable+inline entries on same element", () => {
    const styles: StyleEntry[] = [
      { id: "s1", type: "class", name: "btn", properties: [{ name: "color", value: "red" }, { name: "padding", value: "8px" }] },
      { id: "s2", type: "variable", name: "--gap", properties: [{ name: "value", value: "8px" }] },
      { id: "s3", type: "inline", name: "", properties: [{ name: "display", value: "flex" }] },
    ];
    const elements = [{ id: "a", styles }];

    const result = collectStylesFromEntries(elements);

    // componentStyle should have btn class and _v-gap (from variable)
    expect(result.componentStyle["btn"]).toEqual({ color: "red", padding: "8px" });
    expect(result.componentStyle["_v-gap"]).toEqual({ "--gap": "8px" });

    // elementClassMap should have both class names
    expect(result.elementClassMap.get("a")).toEqual(["btn", "_v-gap"]);

    // elementInlineMap should have inline styles only
    expect(result.elementInlineMap.get("a")).toEqual({ display: "flex" });
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

  it("round-trips inline StyleEntry through generate -> parse", () => {
    const elements: ElementData[] = [
      {
        id: "root",
        nodeType: "view",
        tag: "div",
        props: {},
        children: [],
        parentId: null,
        styles: [
          { id: "s_i1", type: "inline", name: "", properties: [{ name: "padding", value: "16px" }, { name: "color", value: "#333" }] },
        ],
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

    const parsed = parseOmniSource(result.value!);
    const hardErrors = parsed.errors.filter(e => e.severity === "error");
    expect(hardErrors).toHaveLength(0);
    expect(parsed.designElements).not.toBeNull();

    const parsedEl = parsed.designElements!.find(el => el.id === "root");
    expect(parsedEl).toBeDefined();

    // Inline styles are parsed back into the element's styles array
    const inlineEntry = parsedEl!.styles?.find(s => s.type === "inline");
    expect(inlineEntry).toBeDefined();
    expect(inlineEntry!.properties).toContainEqual({ name: "padding", value: "16px" });
    expect(inlineEntry!.properties).toContainEqual({ name: "color", value: "#333" });
  });

  it("backward compatible: old props-based styles become inline StyleEntry", () => {
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
    expect(el!.styles).toBeDefined();
    expect(el!.styles!.length).toBeGreaterThanOrEqual(1);
    // padding and color are style keys → should be in inline StyleEntry
    const inlineProps = el!.styles!.filter(s => s.type === "inline").flatMap(s => s.properties);
    const propNames = inlineProps.map(p => p.name);
    expect(propNames).toContain("padding");
    expect(propNames).toContain("color");
    // label is NOT a style key → should NOT appear in styles
    expect(propNames).not.toContain("label");
  });
});
