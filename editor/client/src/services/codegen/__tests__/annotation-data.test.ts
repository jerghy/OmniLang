// editor/client/src/omni-gen/__tests__/annotation-data.test.ts
// Unit tests for buildAnnotationData and createAnnotationStrategy.

import { describe, it, expect } from "vitest";
import { buildAnnotationData, createAnnotationStrategy } from "../annotation-data";
import type { ViewNode } from "../types";

// ── Helpers ──

function makeNode(overrides: Partial<ViewNode> & { path: string }): ViewNode {
  return {
    id: overrides.id || overrides.path.replace(/[^a-zA-Z0-9]/g, "_"),
    path: overrides.path,
    tag: overrides.tag || "div",
    props: overrides.props || {},
    children: overrides.children || [],
    kind: overrides.kind || "element",
    ...overrides,
  };
}

// ── buildAnnotationData ──

describe("buildAnnotationData", () => {
  it("produces codeTabDirective for logic tab", () => {
    const data = buildAnnotationData([], "logic");
    expect(data.codeTabDirective).toBe("// @omni-file type=logic");
  });

  it("produces codeTabDirective for event tab", () => {
    const data = buildAnnotationData([], "event");
    expect(data.codeTabDirective).toBe("// @omni-file type=event");
  });

  it("produces // @edit for a normal node", () => {
    const nodes: ViewNode[] = [makeNode({ path: "Comp.root" })];
    const data = buildAnnotationData(nodes, "logic");
    const ann = data.editAnnotations.get("Comp.root");
    expect(ann).toBeDefined();
    expect(ann!.line).toBe("// @edit");
    expect(ann!.indent).toBe(0);
  });

  it("produces // @edit folded for a folded node", () => {
    const nodes: ViewNode[] = [makeNode({ path: "Comp.folded", isFolded: true })];
    const data = buildAnnotationData(nodes, "logic");
    const ann = data.editAnnotations.get("Comp.folded");
    expect(ann).toBeDefined();
    expect(ann!.line).toBe("// @edit folded");
  });

  it("produces // @edit render=canvas for a canvas node", () => {
    const nodes: ViewNode[] = [makeNode({ path: "Comp.canvas", renderMode: "canvas" })];
    const data = buildAnnotationData(nodes, "logic");
    const ann = data.editAnnotations.get("Comp.canvas");
    expect(ann).toBeDefined();
    expect(ann!.line).toBe("// @edit render=canvas");
  });

  it("folded takes precedence over canvas", () => {
    const nodes: ViewNode[] = [
      makeNode({ path: "Comp.fc", isFolded: true, renderMode: "canvas" }),
    ];
    const data = buildAnnotationData(nodes, "logic");
    const ann = data.editAnnotations.get("Comp.fc");
    expect(ann).toBeDefined();
    expect(ann!.line).toBe("// @edit folded");
  });

  it("walks nested children with increasing depth", () => {
    const child: ViewNode = makeNode({ path: "Comp.root.child.0" });
    const nodes: ViewNode[] = [
      makeNode({ path: "Comp.root", children: [child] }),
    ];
    const data = buildAnnotationData(nodes, "event");
    expect(data.editAnnotations.get("Comp.root")!.indent).toBe(0);
    expect(data.editAnnotations.get("Comp.root.child.0")!.indent).toBe(1);
  });

  it("walks deeply nested trees", () => {
    const grandchild: ViewNode = makeNode({ path: "Comp.a.b.c.0" });
    const child: ViewNode = makeNode({ path: "Comp.a.b.0", children: [grandchild] });
    const nodes: ViewNode[] = [makeNode({ path: "Comp.a", children: [child] })];
    const data = buildAnnotationData(nodes, "logic");
    expect(data.editAnnotations.get("Comp.a")!.indent).toBe(0);
    expect(data.editAnnotations.get("Comp.a.b.0")!.indent).toBe(1);
    expect(data.editAnnotations.get("Comp.a.b.c.0")!.indent).toBe(2);
  });

  it("handles multiple root nodes", () => {
    const nodes: ViewNode[] = [
      makeNode({ path: "Comp.first" }),
      makeNode({ path: "Comp.second" }),
    ];
    const data = buildAnnotationData(nodes, "logic");
    expect(data.editAnnotations.has("Comp.first")).toBe(true);
    expect(data.editAnnotations.has("Comp.second")).toBe(true);
  });

  it("handles empty node list", () => {
    const data = buildAnnotationData([], "logic");
    expect(data.codeTabDirective).toBe("// @omni-file type=logic");
    expect(data.editAnnotations.size).toBe(0);
  });
});

// ── createAnnotationStrategy ──

describe("createAnnotationStrategy", () => {
  it("beforeFile returns codeTabDirective", () => {
    const data = buildAnnotationData([], "event");
    const strategy = createAnnotationStrategy(data);
    expect(strategy.beforeFile()).toBe("// @omni-file type=event");
  });

  it("beforeViewNode returns annotation for known node path", () => {
    const nodes: ViewNode[] = [makeNode({ path: "Comp.root" })];
    const data = buildAnnotationData(nodes, "logic");
    const strategy = createAnnotationStrategy(data);
    const result = strategy.beforeViewNode(nodes[0], 0);
    expect(result).toBe("// @edit");
  });

  it("beforeViewNode returns null for unknown node path", () => {
    const data = buildAnnotationData([], "logic");
    const strategy = createAnnotationStrategy(data);
    const node = makeNode({ path: "Comp.unknown" });
    const result = strategy.beforeViewNode(node, 0);
    expect(result).toBeNull();
  });

  it("beforeViewNode returns correct annotation for folded node", () => {
    const nodes: ViewNode[] = [makeNode({ path: "Comp.f", isFolded: true })];
    const data = buildAnnotationData(nodes, "logic");
    const strategy = createAnnotationStrategy(data);
    const result = strategy.beforeViewNode(nodes[0], 0);
    expect(result).toBe("// @edit folded");
  });
});
