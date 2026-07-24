import { describe, it, expect } from "vitest";
import { getAutocompleteItems } from "../autocomplete-rules";

describe("autocomplete rules v2", () => {
  it("returns statement items at top level", () => {
    const items = getAutocompleteItems("l", { parentKind: null, precedingCardKind: null });
    const labels = items.map(i => i.label);
    expect(labels).toContain("let");
  });

  it("returns mut inside let container", () => {
    const items = getAutocompleteItems("m", { parentKind: "let", precedingCardKind: null });
    expect(items.some(i => i.label === "mut")).toBe(true);
  });

  it("returns type completions inside let", () => {
    const items = getAutocompleteItems("i", { parentKind: "let", precedingCardKind: null });
    expect(items.some(i => i.label === "i32")).toBe(true);
  });

  it("returns operators inside set", () => {
    const items = getAutocompleteItems("+", { parentKind: "set", precedingCardKind: null });
    expect(items.some(i => i.label === "+=")).toBe(true);
  });

  it("returns comparison operators in if head", () => {
    const items = getAutocompleteItems(">", { parentKind: "if", precedingCardKind: null });
    expect(items.some(i => i.label === ">=")).toBe(true);
  });

  it("returns method completions after dot", () => {
    const items = getAutocompleteItems(".len", { parentKind: null, precedingCardKind: "dot" });
    expect(items.some(i => i.label === ".len()")).toBe(true);
  });

  it("returns literals inside for container", () => {
    const items = getAutocompleteItems("t", { parentKind: "for", precedingCardKind: null });
    const labels = items.map(i => i.label);
    expect(labels).toContain("true");
    expect(labels).not.toContain("false");
    expect(labels).not.toContain("null");
  });

  it("returns arithmetic operators in return context", () => {
    const items = getAutocompleteItems("+", { parentKind: "return", precedingCardKind: null });
    expect(items.some(i => i.label === "+")).toBe(true);
  });

  it("returns logic operators in while context", () => {
    const items = getAutocompleteItems("&", { parentKind: "while", precedingCardKind: null });
    expect(items.some(i => i.label === "&&")).toBe(true);
  });

  it("returns empty when no token (prevent Delete-key spam)", () => {
    const items = getAutocompleteItems("", { parentKind: null, precedingCardKind: null });
    expect(items).toEqual([]);
  });
});
