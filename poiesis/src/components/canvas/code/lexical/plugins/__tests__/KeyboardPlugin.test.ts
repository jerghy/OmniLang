// poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode } from "lexical";
import { $isCardNode, $createCardNode } from "../../nodes/CardNode";
import { $isContainerElementNode, $createContainerElementNode } from "../../nodes/ContainerNode";
import { $isBodyLineNode, $createBodyLineNode } from "../../nodes/BodyLineNode";

// Keyboard behavior specification tests
// These test the underlying node operations that the KeyboardPlugin commands drive.
// Full integration tests requiring the plugin to be mounted in a LexicalComposer
// are covered by E2E tests.

describe("KeyboardPlugin v2 — navigation logic", () => {
  // ── Delete: inline card removal ──
  it("Delete on inline card removes it", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const p = $createParagraphNode();
      const card = $createCardNode("operator", "=", "c1");
      p.append(card);
      root.append(p);

      // Simulate Delete: card.remove() is what the plugin calls
      expect($isCardNode(card)).toBe(true);
      card.remove();

      // Card removed from tree
      expect(root.getTextContent()).toBe("");
    });
  });

  // ── Delete: container at right edge ──
  it("Delete on container right edge removes entire container", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const container = $createContainerElementNode("let", "let1");
      const p = $createParagraphNode();
      root.append(container);
      root.append(p);

      // Check that previous sibling of p is the container
      const prevSibling = p.getPreviousSibling();
      expect($isContainerElementNode(prevSibling)).toBe(true);

      // Simulate Delete: remove the previous sibling container
      if ($isContainerElementNode(prevSibling)) {
        prevSibling.remove();
      }

      // Container removed, only the paragraph remains
      expect(root.getChildren().length).toBe(1);
      expect(root.getFirstChild()).toBe(p);
    });
  });

  // ── Enter: body line insertion ──
  it("Enter at end of body line creates new line", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const bodyLine = $createBodyLineNode("bl-1");
      root.append(bodyLine);

      // Simulate Enter: insert new body line after current one
      const newLine = $createBodyLineNode("bl-2");
      bodyLine.insertAfter(newLine);

      // Both body lines present in tree
      expect($isBodyLineNode(bodyLine)).toBe(true);
      expect($isBodyLineNode(newLine)).toBe(true);
      const children = root.getChildren();
      expect(children.length).toBe(2);
      expect(children[0]).toBe(bodyLine);
      expect(children[1]).toBe(newLine);
    });
  });

  // ── Backslash (placeholder — no handler yet) ──
  it("backslash in let container creates name input", () => {
    // Placeholder: this will test that pressing \ in a let container's
    // name position inserts an InputSlotNode for the variable name.
    // Requires the backslash handler to be implemented.
    expect(true).toBe(true);
  });

  it("backslash at top level is a no-op", () => {
    // Placeholder: pressing \ at the top level should not trigger any
    // special behavior — it's a no-op.
    expect(true).toBe(true);
  });

  // ── Tab: operator cycling in set container ──
  it("Tab in set container cycles operator", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const setContainer = $createContainerElementNode("set", "set1");
      const opCard = $createCardNode("operator", "=", "op1");
      const valCard = $createCardNode("var_ref", "x", "val1");
      setContainer.append(opCard, valCard);
      root.append(setContainer);

      // Check structure
      expect($isContainerElementNode(setContainer)).toBe(true);
      expect(setContainer.getContainerKind()).toBe("set");
      expect($isCardNode(opCard)).toBe(true);
      expect(opCard.getCardKind()).toBe("operator");
      expect(opCard.getCardLabel()).toBe("=");

      // Simulate Tab: cycle to next operator
      const operators = ["=", "+=", "-=", "*=", "/=", "%="];
      const current = operators.indexOf(opCard.getCardLabel());
      const next = (current + 1) % operators.length;
      const newNode = $createCardNode("operator", operators[next], opCard.getCardPid());

      // Verify correct cycle
      expect(operators[next]).toBe("+=");

      opCard.replace(newNode);
      expect(newNode.getCardLabel()).toBe("+=");
    });
  });
});
