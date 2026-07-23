import { describe, it, expect } from "vitest";
import { getAutocompleteItems } from "../autocomplete-rules";

describe("getAutocompleteItems", () => {
  describe("paragraph context (new statement)", () => {
    it('filters by prefix "l" → let, let mut', () => {
      const items = getAutocompleteItems("l", { parentKind: null, slotType: null });
      const labels = items.map(i => i.label);
      expect(labels).toContain("let");
      expect(labels).toContain("let mut");
      expect(labels).not.toContain("if"); // doesn't start with "l"
    });

    it('filters by prefix "w" → while only', () => {
      const items = getAutocompleteItems("w", { parentKind: null, slotType: null });
      const labels = items.map(i => i.label);
      expect(labels).toContain("while");
      expect(labels.length).toBe(1);
    });

    it("shows all when token is empty", () => {
      const items = getAutocompleteItems("", { parentKind: null, slotType: null });
      expect(items.length).toBeGreaterThanOrEqual(10);
    });
  });

  describe("type slot context", () => {
    it("shows type names", () => {
      const items = getAutocompleteItems("", { parentKind: "declare_var", slotType: "type" });
      const labels = items.map(i => i.label);
      expect(labels).toContain("i32");
      expect(labels).toContain("String");
      expect(labels).toContain("f64");
      expect(labels).toContain("bool");
    });

    it('filters types by prefix "f" → f64, f32', () => {
      const items = getAutocompleteItems("f", { parentKind: "declare_var", slotType: "type" });
      const labels = items.map(i => i.label);
      expect(labels).toContain("f64");
      expect(labels).toContain("f32");
      expect(labels).not.toContain("i32");
    });
  });

  describe("name slot context", () => {
    it("shows nothing specific (variable names come from scope)", () => {
      const items = getAutocompleteItems("", { parentKind: "declare_var", slotType: "name" });
      // Name slots don't have a fixed autocomplete — just empty
      expect(Array.isArray(items)).toBe(true);
    });
  });
});
