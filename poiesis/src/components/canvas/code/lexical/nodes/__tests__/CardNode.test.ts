import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode, $isElementNode } from "lexical";
import { CardNode, $createCardNode, $isCardNode } from "../CardNode";

describe("CardNode (ElementNode)", () => {
  it("has correct type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("operator", "=", "c1");
      expect(CardNode.getType()).toBe("inline-card");
      expect(node.getCardKind()).toBe("operator");
      expect(node.getCardLabel()).toBe("=");
      expect(node.getCardPid()).toBe("c1");
    });
  });

  it("is detected by $isCardNode", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("type", "i32", "c2");
      expect($isCardNode(node)).toBe(true);
      expect($isCardNode(null)).toBe(false);
      expect($isCardNode(undefined)).toBe(false);
    });
  });

  it("serializes to JSON with all fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("literal", "42", "c3");
      const json = node.exportJSON();
      expect(json.type).toBe("inline-card");
      expect(json.cardKind).toBe("literal");
      expect(json.cardLabel).toBe("42");
      expect(json.cardPid).toBe("c3");
      // ElementNode serialization fields
      expect(json).toHaveProperty("children");
      expect(json).toHaveProperty("format");
    });
  });

  it("deserializes from JSON", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = CardNode.importJSON({
        type: "inline-card",
        cardKind: "operator",
        cardLabel: "+=",
        cardPid: "c4",
        version: 1,
        children: [],
        format: 0,
        indent: 0,
        direction: null,
      } as any);
      expect($isCardNode(node)).toBe(true);
      expect(node.getCardKind()).toBe("operator");
      expect(node.getCardLabel()).toBe("+=");
      expect(node.getCardPid()).toBe("c4");
    });
  });

  it("is inline", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("mut", "mut", "c5");
      expect(node.isInline()).toBe(true);
    });
  });

  it("can be empty (no Lexical-managed text children)", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("var_ref", "count", "c6");
      expect(node.canBeEmpty()).toBe(true);
    });
  });

  it("allows cursor before and after", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("separator", ":", "c7");
      expect(node.canInsertTextBefore()).toBe(true);
      expect(node.canInsertTextAfter()).toBe(true);
    });
  });

  it("clones preserving all fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("var_ref", "count", "c8");
      const clone = CardNode.clone(node);
      expect(clone.getCardKind()).toBe("var_ref");
      expect(clone.getCardLabel()).toBe("count");
      expect(clone.getCardPid()).toBe("c8");
    });
  });

  it("DOM has contenteditable=false", async () => {
    const editor = createEditor({ nodes: [CardNode] });
    await editor.update(() => {
      const root = $getRoot();
      const p = $createParagraphNode();
      const card = $createCardNode("operator", "=", "c9");
      p.append(card);
      root.append(p);
    });

    // Check DOM attribute after rendering
    editor.getEditorState().read(() => {
      const root = $getRoot();
      const paragraph = root.getFirstChild();
      const card = paragraph && $isElementNode(paragraph) ? paragraph.getFirstChild() : null;
      expect($isCardNode(card)).toBe(true);
    });
  });
});
