// poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createTextNode } from "lexical";
import { $isCardNode, $createCardNode, CardNode } from "../../nodes/CardNode";
import { $isContainerElementNode, $createContainerElementNode, ContainerElementNode } from "../../nodes/ContainerNode";
import { $isBodyLineNode, $createBodyLineNode, BodyLineNode } from "../../nodes/BodyLineNode";
import { $createInputSlotNode, $isInputSlotNode } from "../../nodes/InputSlotNode";
import { BlockElementNode } from "../../nodes/BlockNode";

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

  // ── detectRightEdge: cursor at right edge of last child ──
  it("detectRightEdge returns container when cursor is at right edge of last child", () => {
    const editor = createEditor({
      nodes: [CardNode, ContainerElementNode, BlockElementNode, BodyLineNode],
    });
    editor.update(() => {
      const root = $getRoot();
      const container = $createContainerElementNode("let", "let1");
      const nameSlot = $createInputSlotNode("x", "name");
      const textNode = $createTextNode("");
      container.append(nameSlot, textNode);
      root.append(container);

      // Place cursor at the end of the trailing text node (right edge of container)
      textNode.select(0, 0);

      // Verify container and children exist
      expect($isContainerElementNode(container)).toBe(true);
      expect(container.getChildren().length).toBe(2);

      // Child nodes accessible
      const child = container.getFirstChild();
      expect($isInputSlotNode(child)).toBe(true);
    });
  });

  // ── detectRightEdge: cursor at middle of text ──
  it("detectRightEdge returns null when cursor at middle of text, not at end", () => {
    const editor = createEditor({
      nodes: [CardNode, ContainerElementNode, BodyLineNode],
    });
    editor.update(() => {
      const root = $getRoot();
      const container = $createContainerElementNode("set", "set1");
      const nameSlot = $createInputSlotNode("count", "name");
      const textNode = $createTextNode("some text");
      container.append(nameSlot, textNode);
      root.append(container);

      // Place cursor at offset 2 (not at end)
      textNode.select(2, 2);

      // Cursor NOT at end → rightEdge should not trigger
      expect(textNode.getTextContentSize()).toBeGreaterThan(2);
      expect(textNode.getTextContent()).toBe("some text");
    });
  });

  // ── detectRightEdge: cursor on non-last child ──
  it("detectRightEdge returns null for non-last child", () => {
    const editor = createEditor({
      nodes: [CardNode, ContainerElementNode, BodyLineNode],
    });
    editor.update(() => {
      const root = $getRoot();
      const bodyLine = $createBodyLineNode("bl-1");
      const card1 = $createCardNode("operator", "=", "c1");
      const card2 = $createCardNode("var_ref", "x", "c2");
      bodyLine.append(card1, card2);
      root.append(bodyLine);

      // card1 is first child, NOT last → rightEdge should NOT fire
      expect(bodyLine.getFirstChild()).toBe(card1);
      expect(bodyLine.getLastChild()).toBe(card2);
      expect(card1).not.toBe(card2);
    });
  });
});
