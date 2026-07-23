import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import {
  BlockElementNode,
  $createBlockElementNode,
  $isBlockElementNode,
} from "../BlockNode";

describe("BlockElementNode", () => {
  it("creates with correct type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("if", "b1");
      expect(BlockElementNode.getType()).toBe("block");
      expect(node.getBlockKind()).toBe("if");
      expect(node.getBlockPid()).toBe("b1");
    });
  });

  it("is detected correctly", () => {
    const editor = createEditor();
    editor.update(() => {
      expect($isBlockElementNode($createBlockElementNode("while", "b2"))).toBe(true);
      expect($isBlockElementNode(null)).toBe(false);
    });
  });

  it("supports all block kinds", () => {
    const editor = createEditor();
    editor.update(() => {
      const kinds: Array<[string, string]> = [
        ["if", "b1"],
        ["while", "b2"],
        ["for", "b3"],
      ];
      for (const [kind, pid] of kinds) {
        const node = $createBlockElementNode(kind as any, pid);
        expect(node.getBlockKind()).toBe(kind);
      }
    });
  });

  it("clones preserving kind and pid", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("for", "b4");
      const clone = BlockElementNode.clone(node);
      expect(clone.getBlockKind()).toBe("for");
      expect(clone.getBlockPid()).toBe("b4");
    });
  });

  it("serializes to JSON with block kind and pid", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("while", "b5");
      const json = node.exportJSON();
      expect(json.type).toBe("block");
      expect(json.blockKind).toBe("while");
      expect(json.blockPid).toBe("b5");
    });
  });

  it("creates DOM with correct attributes", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("if", "b6");
      const dom = node.createDOM({ namespace: "test", theme: {} });
      expect(dom.className).toBe("code-block-card code-block-card--if");
      expect(dom.getAttribute("data-block-kind")).toBe("if");
      expect(dom.getAttribute("data-block-pid")).toBe("b6");
    });
  });

  it("is not inline", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("if", "b7");
      expect(node.isInline()).toBe(false);
    });
  });

  it("cannot be empty", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("if", "b8");
      expect(node.canBeEmpty()).toBe(false);
    });
  });

  it("can insert text before and after", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("if", "b9");
      expect(node.canInsertTextBefore()).toBe(true);
      expect(node.canInsertTextAfter()).toBe(true);
    });
  });

  it("updateDOM always returns true", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createBlockElementNode("if", "b10");
      const dom = node.createDOM({ namespace: "test", theme: {} });
      expect(node.updateDOM(dom)).toBe(true);
    });
  });

  it("imports from JSON", () => {
    const editor = createEditor();
    editor.update(() => {
      const json: any = {
        type: "block",
        blockKind: "for",
        blockPid: "b11",
        format: 0,
        indent: 0,
        direction: null,
      };
      const node = BlockElementNode.importJSON(json);
      expect($isBlockElementNode(node)).toBe(true);
      expect(node.getBlockKind()).toBe("for");
      expect(node.getBlockPid()).toBe("b11");
    });
  });
});
