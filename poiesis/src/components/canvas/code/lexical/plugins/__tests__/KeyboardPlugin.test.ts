// poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import { $isCardNode, $createCardNode } from "../../nodes/CardNode";
import { $isContainerElementNode, $createContainerElementNode } from "../../nodes/ContainerNode";
import { $isBodyLineNode, $createBodyLineNode } from "../../nodes/BodyLineNode";
import { $createInputSlotNode, $isInputSlotNode } from "../../nodes/InputSlotNode";

describe("KeyboardPlugin v2 — node operations", () => {
  // ── CardNode as ElementNode ──
  it("CardNode is inline ElementNode with cursor access", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $createCardNode("operator", "=", "c1");
      expect(card.isInline()).toBe(true);
      expect(card.canBeEmpty()).toBe(true);
      expect(card.canInsertTextBefore()).toBe(true);
      expect(card.canInsertTextAfter()).toBe(true);
    });
  });

  // ── Delete: container at right edge ──
  it("Delete at right edge of container removes entire container with children", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const container = $createContainerElementNode("let", "let1");
      const nameSlot = $createInputSlotNode("count", "name");
      container.append(nameSlot);
      root.append(container);

      // Verify container exists and has children
      expect($isContainerElementNode(container)).toBe(true);
      expect(container.getChildren().length).toBe(1);

      // Simulate Delete at right edge: remove container
      container.remove();

      // Container removed
      expect(root.getChildren().length).toBe(0);
    });
  });

  // ── Delete: empty body line ──
  it("Delete on empty body line removes it when siblings exist", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const line1 = $createBodyLineNode("bl-1");
      const line2 = $createBodyLineNode("bl-2");
      root.append(line1, line2);

      expect(root.getChildren().length).toBe(2);

      // Simulate Delete on line2: remove it
      line2.remove();

      expect(root.getChildren().length).toBe(1);
      expect(root.getFirstChild()).toBe(line1);
    });
  });

  // ── Enter: body line insertion ──
  it("Enter at end of body line creates new line", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const bodyLine = $createBodyLineNode("bl-1");
      root.append(bodyLine);

      const newLine = $createBodyLineNode("bl-2");
      bodyLine.insertAfter(newLine);

      expect(root.getChildren().length).toBe(2);
      expect($isBodyLineNode(root.getChildren()[1])).toBe(true);
    });
  });

  // ── Tab: operator cycling in set container ──
  it("Tab in set container cycles operator", () => {
    const editor = createEditor();
    editor.update(() => {
      const setContainer = $createContainerElementNode("set", "set1");
      const opCard = $createCardNode("operator", "=", "op1");
      const valSlot = $createInputSlotNode("x", "expression");
      setContainer.append(opCard, valSlot);

      const operators = ["=", "+=", "-=", "*=", "/=", "%="];
      const current = operators.indexOf(opCard.getCardLabel());
      const next = (current + 1) % operators.length;

      const newNode = $createCardNode("operator", operators[next], opCard.getCardPid());
      opCard.replace(newNode);

      expect(newNode.getCardLabel()).toBe("+=");
      expect(newNode.getCardKind()).toBe("operator");
    });
  });

  // ── Delete: card (browser native) ──
  it("Card can be removed via remove()", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const card = $createCardNode("type", "i32", "c2");
      root.append(card);

      expect($isCardNode(card)).toBe(true);
      card.remove();
      expect(root.getChildren().length).toBe(0);
    });
  });
});
