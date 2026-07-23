import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import { SlotTextNode, $isSlotTextNode, $createSlotTextNode } from "../SlotTextNode";

describe("SlotTextNode", () => {
  it("creates a SlotTextNode with text and slot type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotTextNode("sum", "name");
      expect($isSlotTextNode(node)).toBe(true);
      expect(node.__text).toBe("sum");
      expect(node.__slot).toBe("name");
    });
  });

  it("serializes to JSON with slot info", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotTextNode("f64", "type");
      const json = node.exportJSON();
      expect(json.slot).toBe("type");
      expect(json.type).toBe("slot-text");
    });
  });

  it("clones preserving slot type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotTextNode("0.0", "value");
      const clone = SlotTextNode.clone(node);
      expect(clone.__slot).toBe("value");
      expect(clone.__text).toBe("0.0");
    });
  });
});
