// poiesis/src/components/canvas/code/lexical/transforms/__tests__/text-to-card.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import { $transformTextToCard } from "../text-to-card";
import { $isCardElementNode } from "../../nodes/CardElementNode";
import { $isSlotTextNode } from "../../nodes/SlotTextNode";

describe("$transformTextToCard", () => {
  it("creates declare_var card with slots", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $transformTextToCard("declare_var", "let x = 1", "op_a");
      expect($isCardElementNode(card)).toBe(true);
      expect(card.getChildrenSize()).toBeGreaterThan(0);

      // Find the name slot
      const children = card.getChildren();
      const slotNodes = children.filter(c => $isSlotTextNode(c));
      expect(slotNodes.length).toBe(3); // name, type, value
    });
  });

  it("creates set_var card with target and value slots", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $transformTextToCard("set_var", "count = count + 1", "op_b");
      const children = card.getChildren();
      const slotNodes = children.filter(c => $isSlotTextNode(c));
      expect(slotNodes.length).toBe(2); // name, value
    });
  });

  it("creates if_block card with condition and empty body", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $transformTextToCard("if_block", "if x > 0", "op_c");
      expect($isCardElementNode(card)).toBe(true);
      // if_block has condition slot + { } markers
      const children = card.getChildren();
      const slotNodes = children.filter(c => $isSlotTextNode(c));
      expect(slotNodes.length).toBe(1); // condition
    });
  });

  it("creates while card with condition and empty body", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $transformTextToCard("while", "while i < n", "op_d");
      const children = card.getChildren();
      const slotNodes = children.filter(c => $isSlotTextNode(c));
      expect(slotNodes.length).toBe(1); // condition
    });
  });
});
