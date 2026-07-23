// poiesis/src/components/canvas/code/lexical/transforms/__tests__/card-to-text.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode, $isParagraphNode } from "lexical";
import { $createCardElementNode, $isCardElementNode } from "../../nodes/CardElementNode";
import { $createSlotTextNode } from "../../nodes/SlotTextNode";
import { $degradeCardToText } from "../card-to-text";

describe("$degradeCardToText", () => {
  it("replaces card with paragraph containing text representation", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const card = $createCardElementNode("declare_var", "op_x");
      card.append(
        $createSlotTextNode("sum", "name"),
      );
      root.append(card);

      expect($isCardElementNode(root.getFirstChild())).toBe(true);

      $degradeCardToText(card);

      // Should now be a ParagraphNode
      const newChild = root.getFirstChild();
      expect($isParagraphNode(newChild)).toBe(true);
    });
  });

  it("preserves text content from card slots", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const card = $createCardElementNode("declare_var", "op_y");
      card.append(
        $createSlotTextNode("count", "name"),
      );
      root.append(card);

      $degradeCardToText(card);

      const paragraph = root.getFirstChild()!;
      const text = paragraph.getTextContent();
      expect(text).toContain("count");
    });
  });
});
