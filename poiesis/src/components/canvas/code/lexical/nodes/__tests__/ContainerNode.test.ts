import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import {
  ContainerElementNode,
  $createContainerElementNode,
  $isContainerElementNode,
} from "../ContainerNode";

describe("ContainerElementNode", () => {
  it("creates with correct type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("let", "c1");
      expect(ContainerElementNode.getType()).toBe("container");
      expect(node.getContainerKind()).toBe("let");
      expect(node.getContainerPid()).toBe("c1");
    });
  });

  it("is detected correctly", () => {
    const editor = createEditor();
    editor.update(() => {
      expect($isContainerElementNode($createContainerElementNode("set", "c2"))).toBe(true);
      expect($isContainerElementNode(null)).toBe(false);
    });
  });

  it("supports all container kinds", () => {
    const editor = createEditor();
    editor.update(() => {
      const kinds: Array<[string, string]> = [
        ["let", "c1"],
        ["set", "c2"],
        ["return", "c3"],
        ["await", "c4"],
        ["call", "c5"],
        ["array", "c6"],
        ["map", "c7"],
      ];
      for (const [kind, pid] of kinds) {
        const node = $createContainerElementNode(kind as any, pid);
        expect(node.getContainerKind()).toBe(kind);
      }
    });
  });

  it("clones preserving kind and pid", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("await", "c8");
      const clone = ContainerElementNode.clone(node);
      expect(clone.getContainerKind()).toBe("await");
      expect(clone.getContainerPid()).toBe("c8");
    });
  });

  it("serializes to JSON with container kind and pid", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("call", "c9");
      const json = node.exportJSON();
      expect(json.type).toBe("container");
      expect(json.containerKind).toBe("call");
      expect(json.containerPid).toBe("c9");
    });
  });

  it("creates DOM with correct attributes", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("map", "c10");
      const dom = node.createDOM();
      expect(dom.className).toBe("code-container code-container--map");
      expect(dom.getAttribute("data-container-kind")).toBe("map");
      expect(dom.getAttribute("data-container-pid")).toBe("c10");
    });
  });

  it("is not inline", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("let", "c11");
      expect(node.isInline()).toBe(false);
    });
  });

  it("cannot be empty", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("let", "c12");
      expect(node.canBeEmpty()).toBe(false);
    });
  });

  it("can insert text before and after", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("let", "c13");
      expect(node.canInsertTextBefore()).toBe(true);
      expect(node.canInsertTextAfter()).toBe(true);
    });
  });

  it("updateDOM always returns true", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createContainerElementNode("let", "c14");
      const dom = node.createDOM();
      expect(node.updateDOM(node, dom)).toBe(true);
    });
  });

  it("imports from JSON", () => {
    const editor = createEditor();
    editor.update(() => {
      const json: any = {
        type: "container",
        containerKind: "return",
        containerPid: "c15",
        format: 0,
        indent: 0,
        direction: null,
      };
      const node = ContainerElementNode.importJSON(json);
      expect($isContainerElementNode(node)).toBe(true);
      expect(node.getContainerKind()).toBe("return");
      expect(node.getContainerPid()).toBe("c15");
    });
  });
});
