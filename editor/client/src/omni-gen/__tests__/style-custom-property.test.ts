// editor/client/src/omni-gen/__tests__/style-custom-property.test.ts
// End-to-end tests for CSS custom property (--name) support in the WASM compiler.
// These tests compile .omni source through the Rust WASM compiler and verify
// that CSS custom properties are handled correctly in both static style blocks
// and dynamic inline styles.

import { describe, it, expect, beforeAll } from "vitest";
import { resolveStyle } from "../style-resolver";
import { collectStylesFromEntries, parseStylesFromAST } from "../pipeline";
import type { StyleEntry } from "../../constants/style-properties";
import type { ViewNode } from "../types";
import type { ElementData } from "../../store/editor";
import { rustCompiler } from "../../api/rustCompiler";

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
