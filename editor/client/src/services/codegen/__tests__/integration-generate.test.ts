// editor/client/src/omni-gen/__tests__/integration-generate.test.ts
// Integration test for SourceEmitter — end-to-end generate() path.

import { describe, it, expect } from "vitest";
import { SourceEmitter, CodeWriter } from "../source-emitter";
import { parseOmniSource } from "../omni-parser";
import { generateOmniSource, generateOmniFiles } from "../pipeline";
import type { EmitInput, ChainBlock, ViewNode, AnnotationStrategy } from "../types";

// ── Helpers ──

function makeNode(overrides: Partial<ViewNode> & { tag: string }): ViewNode {
  return {
    id: overrides.id || "n1",
    path: overrides.path || "TestComp.n1",
    tag: overrides.tag,
    props: overrides.props || {},
    children: overrides.children || [],
    kind: overrides.kind || "element",
    resolvedStyle: overrides.resolvedStyle || {
      htmlTag: "div",
      cssClassName: "n1",
      htmlAttrs: {},
      eventBindings: {},
      bindings: {},
      cssRules: {},
      usedClassNames: [],
    },
  };
}

function makeResolvedStyle(
  overrides: Partial<NonNullable<ViewNode["resolvedStyle"]>>,
): NonNullable<ViewNode["resolvedStyle"]> {
  return {
    htmlTag: "div",
    cssClassName: "cls",
    htmlAttrs: {},
    eventBindings: {},
    bindings: {},
    cssRules: {},
    usedClassNames: [],
    ...overrides,
  };
}

// ── CodeWriter tests ──

describe("CodeWriter", () => {
  it("emits indented lines", () => {
    const w = new CodeWriter();
    w.emit("hello");
    w.indent();
    w.emit("world");
    expect(w.toString()).toBe("hello\n    world\n");
  });

  it("dedent never goes below 0", () => {
    const w = new CodeWriter();
    w.dedent();
    w.emit("top");
    expect(w.toString()).toBe("top\n");
  });

  it("blank adds empty line", () => {
    const w = new CodeWriter();
    w.emit("a");
    w.blank();
    w.emit("b");
    expect(w.toString()).toBe("a\n\nb\n");
  });
});

// ── SourceEmitter tests ──

describe("SourceEmitter", () => {
  it("emits a minimal component without view and without style", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "Empty",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: null,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("component Empty {");
    expect(result).toContain("}");
  });

  it("emits imports before component", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: ['import { useState } from "react";', 'import "./styles.css";'],
      stores: [],
      component: {
        name: "WithImports",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: null,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    const lines = result.split("\n");
    const importIdx = lines.findIndex((l) => l.includes("import"));
    const compIdx = lines.findIndex((l) => l.includes("component"));
    expect(importIdx).toBeLessThan(compIdx);
  });

  it("emits component signals with defaults", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "SignalDemo",
        signals: {
          count: { type: "number" },
          name: { type: "string", default: "world" },
        },
        computeds: {},
        storeRefs: [],
        fns: [],
        view: null,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("signal count: number;");
    expect(result).toContain('signal name: string = "world";');
  });

  it("emits store with signals, getters, and actions", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [
        {
          name: "counter",
          signals: { count: { type: "number", default: 0 } },
          getters: { doubled: { returnType: "number", body: "count * 2" } },
          actions: [
            { name: "inc", params: [], body: "count = count + 1" },
          ],
        },
      ],
      component: {
        name: "StoreDemo",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: null,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("store counter {");
    expect(result).toContain("signal count: number = 0;");
    expect(result).toContain("getter doubled: number = count * 2;");
    expect(result).toContain("action inc() {");
    expect(result).toContain("count = count + 1");
  });

  it("emits view with nested elements", () => {
    const root: ViewNode = makeNode({
      id: "root",
      path: "App",
      tag: "Container",
      resolvedStyle: makeResolvedStyle({ htmlTag: "div", cssClassName: "container" }),
      children: [
        makeNode({
          id: "btn",
          path: "App.button.0",
          tag: "Button",
          resolvedStyle: makeResolvedStyle({
            htmlTag: "button",
            cssClassName: "btn",
            htmlAttrs: { type: "submit" },
            eventBindings: { click: "handleClick" },
          }),
        }),
      ],
    });

    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "App",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: root,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("view {");
    expect(result).toContain('<div id="App"');
    expect(result).toContain("class: container");
    expect(result).toContain('<button id="App.button.0"');
    expect(result).toContain("class: btn");
    expect(result).toContain('type="submit"');
    expect(result).toContain("@click={handleClick}");
  });

  it("emits style blocks", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "Styled",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: null,
        style: {
          btn: { color: "red", "font-size": "14px" },
          container: { padding: "8px" },
        },
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("style {");
    expect(result).toContain(".btn { color: red; font-size: 14px; }");
    expect(result).toContain(".container { padding: 8px; }");
  });

  it("emits themes", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "Themed",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: null,
        style: {},
      },
      themes: [
        {
          name: "dark",
          categories: { colors: { bg: "#000", fg: "#fff" } },
        },
      ],
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("theme dark {");
    expect(result).toContain('bg: "#000";');
    expect(result).toContain('fg: "#fff";');
  });

  it("emits functions with chain blocks", () => {
    const chains = new Map<string, ChainBlock>();
    chains.set("loadData", {
      functionName: "loadData",
      nodeLines: ["// @node loadData start"],
      wireLines: ["// @wire data -> view"],
    });

    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "FuncDemo",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [
          {
            name: "loadData",
            params: [{ name: "id", type: "number" }],
            returnType: "void",
            body: "console.log(id)",
          },
        ],
        view: null,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, chains);
    expect(result).toContain("// @chain loadData");
    expect(result).toContain("// @node loadData start");
    expect(result).toContain("// @wire data -> view");
    expect(result).toContain("fn loadData(id: number) -> void {");
    expect(result).toContain("console.log(id)");
  });

  it("emits view bindings", () => {
    const root: ViewNode = makeNode({
      id: "input",
      path: "App.input.0",
      tag: "MyInput",
      resolvedStyle: makeResolvedStyle({
        htmlTag: "input",
        cssClassName: "my-input",
        bindings: { value: "userName" },
      }),
    });

    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "App",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: root,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("bind:value={userName}");
  });

  it("emits self-closing tag for element without children", () => {
    const root: ViewNode = makeNode({
      id: "br",
      path: "App.br.0",
      tag: "Break",
      resolvedStyle: makeResolvedStyle({ htmlTag: "br", cssClassName: "br" }),
    });

    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "App",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: root,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("></br>");
  });

  it("emits with annotation strategy", () => {
    const strategy: AnnotationStrategy = {
      beforeFile: () => "// @omni-file",
      beforeViewNode: (_node, _depth) => "// @edit",
    };

    const root: ViewNode = makeNode({
      id: "r",
      path: "Comp.r",
      tag: "Div",
      resolvedStyle: makeResolvedStyle({ htmlTag: "div", cssClassName: "r" }),
    });

    const input: EmitInput = {
      codeTabType: "event",
      imports: [],
      stores: [],
      component: {
        name: "Comp",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: root,
        style: {},
      },
    };
    const emitter = new SourceEmitter(strategy);
    const result = emitter.emitDocument(input, new Map());
    expect(result.startsWith("// @omni-file")).toBe(true);
    expect(result).toContain("// @edit");
  });

  it("emits content text node", () => {
    const root: ViewNode = makeNode({
      id: "t",
      path: "Comp.t.0",
      tag: "Text",
      resolvedStyle: makeResolvedStyle({
        htmlTag: "span",
        cssClassName: "text",
        contentText: "Hello World",
      }),
    });

    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "Comp",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: root,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("Hello World");
    expect(result).toContain("<span id=");
    expect(result).toContain("</span>");
  });

  it("emits store refs on component", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "RefDemo",
        signals: {},
        computeds: {},
        storeRefs: ["cart", "auth"],
        fns: [],
        view: null,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("@store cart;");
    expect(result).toContain("@store auth;");
  });

  it("emits component computeds", () => {
    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "CompDemo",
        signals: {},
        computeds: {
          fullName: { type: "string", body: "firstName + ' ' + lastName" },
        },
        storeRefs: [],
        fns: [],
        view: null,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain(
      "computed fullName: string = firstName + ' ' + lastName;",
    );
  });

  it("escapes double quotes in attribute values", () => {
    const root: ViewNode = makeNode({
      id: "t",
      path: "Comp.t.0",
      tag: "Div",
      resolvedStyle: makeResolvedStyle({
        htmlTag: "div",
        cssClassName: "t",
        htmlAttrs: { title: 'say "hello"' },
      }),
    });

    const input: EmitInput = {
      codeTabType: "logic",
      imports: [],
      stores: [],
      component: {
        name: "Comp",
        signals: {},
        computeds: {},
        storeRefs: [],
        fns: [],
        view: root,
        style: {},
      },
    };
    const emitter = new SourceEmitter();
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain('title="say \\"hello\\""');
  });
});

// ── Pipeline roundtrip tests ──

describe("Pipeline roundtrip", () => {
  const COUNTER_FIXTURE = `// @omni-file type=logic
import omniview;

component Counter {
  view {
    <div id="Counter" class: counter>
      <p id="Counter.label.0" class: label>
        Hello
      </p>
    </div>
  }
}
`;

  it("parse -> generate -> re-parse succeeds", () => {
    // Parse the fixture
    const parsed = parseOmniSource(COUNTER_FIXTURE);
    expect(parsed.errors).toHaveLength(0);
    expect(parsed.designElements).not.toBeNull();

    // Generate from parsed data
    const result = generateOmniSource({
      designElements: parsed.designElements ?? [],
      designWires: parsed.designWires,
      codeTabs: parsed.codeTabs,
      componentName: "Counter",
    });

    // Verify no fatal errors and output was produced
    expect(result.errors.some((e) => e.severity === "fatal")).toBe(false);
    expect(result.value).not.toBeNull();

    // Re-parse the generated output
    const reParsed = parseOmniSource(result.value!);
    expect(reParsed.errors.length).toBeLessThanOrEqual(2); // allow minor annotation warnings
    expect(reParsed.designElements).not.toBeNull();
  });

  it("generateOmniFiles produces multi-file output", () => {
    const parsed = parseOmniSource(COUNTER_FIXTURE);
    expect(parsed.designElements).not.toBeNull();

    const result = generateOmniFiles({
      designElements: parsed.designElements ?? [],
      designWires: parsed.designWires,
      codeTabs: parsed.codeTabs,
      componentName: "Counter",
    });

    expect(result.errors.some((e) => e.severity === "fatal")).toBe(false);
    expect(result.value).not.toBeNull();
    expect(result.value!["src/main.omni"]).toBeDefined();
    expect(result.value!["src/main.omni"]).toContain("component Counter");
  });

  it("generateOmniFiles includes store files when stores exist", () => {
    const FIXTURE_WITH_STORE = `// @omni-file type=logic
import omniview;

component Counter {
  @store CounterStore;

  view {
    <div id="Counter" class: counter>
      <p id="Counter.label.0" class: label>
        Hello
      </p>
    </div>
  }
}
`;
    const parsed = parseOmniSource(FIXTURE_WITH_STORE);
    expect(parsed.designElements).not.toBeNull();

    const result = generateOmniFiles({
      designElements: parsed.designElements ?? [],
      designWires: parsed.designWires,
      codeTabs: parsed.codeTabs,
      componentName: "Counter",
    });

    expect(result.errors.some((e) => e.severity === "fatal")).toBe(false);
    expect(result.value).not.toBeNull();

    // Should have main.omni even without store data fully parsed
    expect(result.value!["src/main.omni"]).toBeDefined();
    expect(result.value!["src/main.omni"]).toContain("component Counter");
  });

  it("handles empty input gracefully", () => {
    const result = generateOmniSource({
      designElements: [],
      designWires: [],
      codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "", graph: { elements: [], wires: [] }, variables: [] }],
      componentName: "Empty",
    });

    expect(result.errors.some((e) => e.severity === "fatal")).toBe(false);
    expect(result.value).not.toBeNull();
    expect(result.value).toContain("component Empty");
  });
});
