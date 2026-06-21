// editor/client/src/omni-gen/__tests__/view-tree-builder.test.ts
import { describe, it, expect } from "vitest";
import { buildViewTree } from "../view-tree-builder";
import type { ElementData } from "../../store/editor";

describe("buildViewTree", () => {
  it("builds nested view tree from flat elements", () => {
    const elements: ElementData[] = [
      { id: "root", nodeType: "view", tag: "Container", props: {}, children: ["child1"], parentId: null },
      { id: "child1", nodeType: "view", tag: "Button", props: { label: "Click" }, children: [], parentId: "root" },
    ];

    const { roots } = buildViewTree(elements, "App");

    expect(roots).toHaveLength(1);
    expect(roots[0].path).toBe("App");
    expect(roots[0].children).toHaveLength(1);
    expect(roots[0].children[0].tag).toBe("Button");
    expect(roots[0].children[0].path).toBe("App.button.0");
  });

  it("collects PascalCase tags for imports", () => {
    const elements: ElementData[] = [
      { id: "r", nodeType: "view", tag: "Container", props: {}, children: ["h"], parentId: null },
      { id: "h", nodeType: "view", tag: "MyHeader", props: {}, children: [], parentId: "r" },
    ];

    const { pascalCaseTags } = buildViewTree(elements, "App");

    expect(pascalCaseTags).toContain("MyHeader");
    expect(pascalCaseTags).not.toContain("Container"); // in TAG_HTML_MAP
  });
});
