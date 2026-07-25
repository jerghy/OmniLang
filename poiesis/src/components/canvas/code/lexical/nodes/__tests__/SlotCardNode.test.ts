import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import { SlotCardNode, $createSlotCardNode, $isSlotCardNode } from "../SlotCardNode";

describe("SlotCardNode (ElementNode, dual-mode)", () => {
  it("has correct type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("hello", "name", "s1");
      expect(SlotCardNode.getType()).toBe("slot-card");
      expect(node.getSlotKind()).toBe("name");
      expect(node.getValue()).toBe("hello");
      expect(node.getPid()).toBe("s1");
    });
  });

  it("is detected by $isSlotCardNode", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("x", "expression", "s2");
      expect($isSlotCardNode(node)).toBe(true);
      expect($isSlotCardNode(null)).toBe(false);
    });
  });

  it("is inline ElementNode with cursor access", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("count", "name", "s3");
      expect(node.isInline()).toBe(true);
      expect(node.canBeEmpty()).toBe(true);
      expect(node.canInsertTextBefore()).toBe(true);
      expect(node.canInsertTextAfter()).toBe(true);
    });
  });

  it("serializes to JSON with slot fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("counter", "name", "s4");
      const json = node.exportJSON();
      expect(json.type).toBe("slot-card");
      expect(json.slotKind).toBe("name");
      expect(json.value).toBe("counter");
      expect(json.pid).toBe("s4");
    });
  });

  it("deserializes from JSON", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = SlotCardNode.importJSON({
        type: "slot-card",
        slotKind: "expression",
        value: "42",
        pid: "s5",
        version: 1,
        children: [],
        format: 0,
        indent: 0,
        direction: null,
      } as any);
      expect($isSlotCardNode(node)).toBe(true);
      expect(node.getSlotKind()).toBe("expression");
      expect(node.getValue()).toBe("42");
    });
  });

  it("clones preserving all fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("result", "expression", "s6");
      const clone = SlotCardNode.clone(node);
      expect(clone.getSlotKind()).toBe("expression");
      expect(clone.getValue()).toBe("result");
      expect(clone.getPid()).toBe("s6");
    });
  });

  it("setValue updates the value", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("old", "expression", "s7");
      node.setValue("new");
      expect(node.getValue()).toBe("new");
    });
  });

  it("creates DOM with contenteditable=false in display mode", () => {
    const editor = createEditor({ nodes: [SlotCardNode] });
    editor.update(() => {
      const node = $createSlotCardNode("count", "name", "s8");
      const dom = node.createDOM({
        namespace: "test",
        theme: {} as any,
      } as any);
      expect(dom.getAttribute("contenteditable")).toBe("false");
      expect(dom.tagName).toBe("SPAN");
      expect(dom.getAttribute("data-slot-kind")).toBe("name");
      expect(dom.className).toContain("slot-card");
      expect(dom.className).toContain("slot-card--name");
      expect(dom.textContent).toBe("count");
    });
  });

  it("shows placeholder when value is empty", () => {
    const editor = createEditor({ nodes: [SlotCardNode] });
    editor.update(() => {
      const node = $createSlotCardNode("", "name", "s9");
      const dom = node.createDOM({
        namespace: "test",
        theme: {} as any,
      } as any);
      // Empty slot shows placeholder via CSS ::before, DOM may show empty
      expect(dom.textContent).toBe("");
    });
  });
});
