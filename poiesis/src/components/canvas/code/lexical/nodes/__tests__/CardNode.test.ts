import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import { CardNode, $createCardNode, $isCardNode } from "../CardNode";

describe("CardNode", () => {
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

  it("clones preserving all fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("var_ref", "count", "c6");
      const clone = CardNode.clone(node);
      expect(clone.__cardKind).toBe("var_ref");
      expect(clone.__cardLabel).toBe("count");
      expect(clone.__cardPid).toBe("c6");
    });
  });
});
