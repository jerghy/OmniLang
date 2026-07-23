import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import { CardElementNode, $isCardElementNode, $createCardElementNode } from "../CardElementNode";
import { SlotTextNode, $createSlotTextNode } from "../SlotTextNode";
import { $isRootOrShadowRoot } from "lexical";

describe("CardElementNode", () => {
  it("creates a CardElementNode with kind and pid", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardElementNode("declare_var", "op_001");
      expect($isCardElementNode(node)).toBe(true);
      expect(node.__cardKind).toBe("declare_var");
      expect(node.__cardPid).toBe("op_001");
    });
  });

  it("can contain SlotTextNode children", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $createCardElementNode("declare_var", "op_001");
      card.append(
        $createTextNode("let"),
        $createTextNode(" "),
        $createSlotTextNode("sum", "name"),
        $createTextNode(" : "),
        $createSlotTextNode("f64", "type"),
        $createTextNode(" = "),
        $createSlotTextNode("0.0", "value"),
      );
      expect(card.getChildrenSize()).toBe(7);
    });
  });

  it("can contain nested CardElementNode for if/while body", () => {
    const editor = createEditor();
    editor.update(() => {
      const outer = $createCardElementNode("while", "op_002");
      const bodyCard = $createCardElementNode("set_var", "op_003");
      bodyCard.append(
        $createSlotTextNode("count", "name"),
        $createTextNode(" = "),
        $createSlotTextNode("count + 1", "value"),
      );
      outer.append(bodyCard);
      expect(outer.getFirstChild()).not.toBeNull();
      const child = outer.getFirstChild()!;
      expect($isCardElementNode(child)).toBe(true);
    });
  });

  it("serializes to JSON with cardKind and pid", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardElementNode("return", "op_004");
      node.append($createTextNode("return"), $createTextNode(" "), $createSlotTextNode("result", "value"));
      const json = node.exportJSON();
      expect(json.type).toBe("card");
      expect(json.cardKind).toBe("return");
      expect(json.cardPid).toBe("op_004");
    });
  });
});
