// editor/client/src/omni-gen/__tests__/source-emitter-annotation.test.ts
// Integration test: SourceEmitter + AnnotationStrategy produced by annotation-data.

import { describe, it, expect } from "vitest";
import { SourceEmitter } from "../source-emitter";
import { buildAnnotationData, createAnnotationStrategy } from "../annotation-data";
import type { EmitInput, ViewNode } from "../types";

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
    ...overrides,
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

// ── SourceEmitter + AnnotationStrategy integration ──

describe("SourceEmitter annotation integration", () => {
  it("emits // @omni-file at the top of the file", () => {
    const root: ViewNode = makeNode({
      id: "r",
      path: "Comp.r",
      tag: "Div",
      resolvedStyle: makeResolvedStyle({ htmlTag: "div", cssClassName: "r" }),
    });

    const viewNodes = [root];
    const annotationData = buildAnnotationData(viewNodes, "logic");
    const strategy = createAnnotationStrategy(annotationData);

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

    const emitter = new SourceEmitter(strategy);
    const result = emitter.emitDocument(input, new Map());
    const lines = result.split("\n");

    // @omni-file should be first line
    expect(lines[0]).toBe("// @omni-file type=logic");
  });

  it("emits // @omni-file type=event for event tab", () => {
    const root: ViewNode = makeNode({
      id: "r",
      path: "Comp.r",
      tag: "Div",
      resolvedStyle: makeResolvedStyle({ htmlTag: "div", cssClassName: "r" }),
    });

    const viewNodes = [root];
    const annotationData = buildAnnotationData(viewNodes, "event");
    const strategy = createAnnotationStrategy(annotationData);

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
    const lines = result.split("\n");
    expect(lines[0]).toBe("// @omni-file type=event");
  });

  it("emits // @edit before each view element tag", () => {
    const root: ViewNode = makeNode({
      id: "root",
      path: "Comp",
      tag: "Container",
      resolvedStyle: makeResolvedStyle({ htmlTag: "div", cssClassName: "container" }),
      children: [
        makeNode({
          id: "btn",
          path: "Comp.button.0",
          tag: "Button",
          resolvedStyle: makeResolvedStyle({
            htmlTag: "button",
            cssClassName: "btn",
          }),
        }),
      ],
    });

    const viewNodes = [root];
    const annotationData = buildAnnotationData(viewNodes, "logic");
    const strategy = createAnnotationStrategy(annotationData);

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

    const emitter = new SourceEmitter(strategy);
    const result = emitter.emitDocument(input, new Map());

    // Should have // @edit before opening <tag of each node
    const lines = result.split("\n");
    const viewStart = lines.findIndex((l) => l.trim() === "view {");
    const viewLines = lines.slice(viewStart);

    const editLines = viewLines.filter((l) => l.trim() === "// @edit");
    expect(editLines.length).toBe(2);

    // Each // @edit should be followed by an opening tag on the next line
    for (let i = 0; i < viewLines.length - 1; i++) {
      const trimmed = viewLines[i].trim();
      if (trimmed === "// @edit") {
        const nextLine = viewLines[i + 1].trim();
        expect(nextLine.startsWith("<")).toBe(true);
      }
    }
  });

  it("emits // @edit folded for folded nodes", () => {
    const root: ViewNode = makeNode({
      id: "r",
      path: "Comp.r",
      tag: "Div",
      resolvedStyle: makeResolvedStyle({ htmlTag: "div", cssClassName: "r" }),
      isFolded: true,
    });

    const viewNodes = [root];
    const annotationData = buildAnnotationData(viewNodes, "logic");
    const strategy = createAnnotationStrategy(annotationData);

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

    const emitter = new SourceEmitter(strategy);
    const result = emitter.emitDocument(input, new Map());
    expect(result).toContain("// @edit folded");
  });

  it("handles deeply nested view tree with annotations", () => {
    const inner: ViewNode = makeNode({
      id: "inner",
      path: "Comp.outer.inner.0",
      tag: "Span",
      resolvedStyle: makeResolvedStyle({ htmlTag: "span", cssClassName: "inner" }),
    });
    const outer: ViewNode = makeNode({
      id: "outer",
      path: "Comp.outer",
      tag: "Div",
      resolvedStyle: makeResolvedStyle({ htmlTag: "div", cssClassName: "outer" }),
      children: [inner],
    });

    const viewNodes = [outer];
    const annotationData = buildAnnotationData(viewNodes, "logic");
    const strategy = createAnnotationStrategy(annotationData);

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
        view: outer,
        style: {},
      },
    };

    const emitter = new SourceEmitter(strategy);
    const result = emitter.emitDocument(input, new Map());

    // Both nodes should have // @edit before their tags
    const htmlTagMatches = result.match(/\/\/ @edit/g);
    expect(htmlTagMatches).toHaveLength(2);

    // Verify ordering: // @edit -> <div> for outer, then // @edit -> <span> for inner
    const lines = result.split("\n");
    let outerEditLine = -1;
    let innerEditLine = -1;
    for (let i = 0; i < lines.length - 1; i++) {
      if (lines[i].trim() === "// @edit") {
        if (lines[i + 1].includes('id="outer"')) outerEditLine = i;
        if (lines[i + 1].includes('id="inner"')) innerEditLine = i;
      }
    }
    expect(outerEditLine).toBeGreaterThan(-1);
    expect(innerEditLine).toBeGreaterThan(-1);

    // outer annotation comes before inner
    expect(outerEditLine).toBeLessThan(innerEditLine);
  });
});
