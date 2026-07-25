// poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot } from "lexical";
import { $isCardNode, $createCardNode } from "../../nodes/CardNode";
import { $isSlotCardNode, $createSlotCardNode, SlotCardNode } from "../../nodes/SlotCardNode";
import { $isBodyLineNode, $createBodyLineNode } from "../../nodes/BodyLineNode";
import { $createContainerElementNode } from "../../nodes/ContainerNode";

describe("KeyboardPlugin v3 — simplified 2-state cursor", () => {
  // ── CardNode as inline ElementNode ──
  it("CardNode is inline ElementNode (visual-only, no cursor)", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $createCardNode("operator", "=", "c1");
      expect(card.isInline()).toBe(true);
      expect(card.canBeEmpty()).toBe(true);
      expect(card.canInsertTextBefore()).toBe(false);
      expect(card.canInsertTextAfter()).toBe(false);
    });
  });

  // ── SlotCardNode is inline ElementNode ──
  it("SlotCardNode is inline ElementNode with cursor access", () => {
    const editor = createEditor();
    editor.update(() => {
      const slot = $createSlotCardNode("count", "name", "s1");
      expect(slot.isInline()).toBe(true);
      expect(slot.canBeEmpty()).toBe(true);
      expect(slot.canInsertTextBefore()).toBe(true);
      expect(slot.canInsertTextAfter()).toBe(true);
    });
  });

  // ── Delete: remove a highlighted card ──
  it("Delete removes a card via remove()", () => {
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

  // ── Delete: remove a slot card ──
  it("Delete removes a slot card via remove()", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const slot = $createSlotCardNode("x", "name", "s2");
      root.append(slot);
      expect($isSlotCardNode(slot)).toBe(true);
      slot.remove();
      expect(root.getChildren().length).toBe(0);
    });
  });

  // ── Enter: body line → new line ──
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
      const valSlot = $createSlotCardNode("x", "expression", "s3");
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

  // ── SlotCardNode: activateEditing sets contenteditable ──
  it("SlotCardNode.activateEditing makes DOM editable", () => {
    const editor = createEditor({ nodes: [SlotCardNode] });
    editor.update(() => {
      const slot = $createSlotCardNode("hello", "name", "s4");
      const dom = slot.createDOM({ namespace: "test", theme: {} as any } as any);
      expect(dom.getAttribute("contenteditable")).toBe("false");

      const cleanup = slot.activateEditing(editor, dom);
      expect(dom.getAttribute("contenteditable")).toBe("true");
      expect(dom.classList.contains("slot-card--editing")).toBe(true);

      cleanup();
      expect(dom.getAttribute("contenteditable")).toBe("false");
      expect(dom.classList.contains("slot-card--editing")).toBe(false);
    });
  });

  // ── No more container right-edge or container-level delete ──
  it("Container removal is not special — just remove() like any node", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const container = $createContainerElementNode("let", "let1");
      const slot = $createSlotCardNode("x", "name", "s5");
      container.append(slot);
      root.append(container);

      // No special right-edge logic. Just remove the container
      container.remove();
      expect(root.getChildren().length).toBe(0);
    });
  });
});
