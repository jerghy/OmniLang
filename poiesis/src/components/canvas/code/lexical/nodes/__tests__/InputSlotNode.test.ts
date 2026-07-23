import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import { InputSlotNode, $createInputSlotNode, $isInputSlotNode } from "../InputSlotNode";

describe("InputSlotNode", () => {
  it("has correct type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createInputSlotNode("hello", "name");
      expect(InputSlotNode.getType()).toBe("input-slot");
      expect(node.getSlotKind()).toBe("name");
      expect(node.getTextContent()).toBe("hello");
    });
  });

  it("is detected by $isInputSlotNode", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createInputSlotNode("x", "expression");
      expect($isInputSlotNode(node)).toBe(true);
    });
  });

  it("can update slot kind", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createInputSlotNode("test", "name");
      node.setSlotKind("condition");
      expect(node.getSlotKind()).toBe("condition");
    });
  });

  it("serializes to JSON with slotKind", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createInputSlotNode("counter", "name");
      const json = node.exportJSON();
      expect(json.type).toBe("input-slot");
      expect(json.slotKind).toBe("name");
      expect(json.text).toBe("counter");
    });
  });

  it("clones preserving slotKind", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createInputSlotNode("result", "expression");
      const clone = InputSlotNode.clone(node);
      expect(clone.getSlotKind()).toBe("expression");
      expect(clone.getTextContent()).toBe("result");
    });
  });

  it("creates DOM with slot kind data attribute", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createInputSlotNode("val", "condition");
      const dom = node.createDOM({ namespace: "test", theme: {} });
      expect(dom.className).toBe("input-slot input-slot--condition");
      expect(dom.getAttribute("data-slot-kind")).toBe("condition");
    });
  });
});
