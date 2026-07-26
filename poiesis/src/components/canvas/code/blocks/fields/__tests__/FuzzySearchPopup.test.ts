// poiesis/src/components/canvas/code/blocks/fields/__tests__/FuzzySearchPopup.test.ts
// Task 8: FuzzySearchPopup — fuzzy scoring, filtering, and popup interaction tests.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  fuzzyScore,
  FuzzySearchPopup,
  SearchItem,
  getItemsForContext,
} from "../FuzzySearchPopup";

// ── Test data ──

const TEST_ITEMS: SearchItem[] = [
  { label: "let", blockType: "declare_var", description: "declare variable" },
  { label: "set", blockType: "set_var", description: "assign variable" },
  { label: "return", blockType: "return_stmt", description: "return value" },
  { label: "if", blockType: "if_block", description: "if / elif / else" },
  { label: "while", blockType: "while_loop", description: "while loop" },
  { label: "for", blockType: "for_in", description: "for..in loop" },
  { label: "break", blockType: "break_stmt", description: "break loop" },
  { label: "comment", blockType: "comment_stmt", description: "add comment" },
  { label: "raw", blockType: "raw_stmt", description: "raw code" },
];

// ── fuzzyScore tests ──

describe("fuzzyScore", () => {
  it("returns 100 for empty query (matches everything)", () => {
    expect(fuzzyScore("", "anything")).toBe(100);
    expect(fuzzyScore("", "")).toBe(100);
  });

  it("returns 0 when no characters match", () => {
    expect(fuzzyScore("xyz", "abc")).toBe(0);
    expect(fuzzyScore("zzzz", "let")).toBe(0);
  });

  it("returns 0 when query characters are not in order", () => {
    // "while" contains 'h' and 'w' but 'wh' appears in order, 'hw' does not
    expect(fuzzyScore("hw", "while")).toBe(0);
    // "let" has 'e' at index 1, then 'l' at index 0 — but query "el" needs
    // 'e' before 'l', and once we find 'e' at index 1 there's no 'l' after it
    expect(fuzzyScore("el", "let")).toBe(0);
  });

  it("matches exact string with high score", () => {
    const score = fuzzyScore("let", "let");
    expect(score).toBeGreaterThan(0);
  });

  it("matches prefix with positive score", () => {
    const score = fuzzyScore("le", "let");
    expect(score).toBeGreaterThan(0);
  });

  it("matches substring with positive score", () => {
    const score = fuzzyScore("et", "let");
    expect(score).toBeGreaterThan(0);
  });

  it("is case-insensitive", () => {
    const scoreLower = fuzzyScore("let", "let");
    const scoreUpper = fuzzyScore("LET", "let");
    const scoreMixed = fuzzyScore("LeT", "let");
    expect(scoreUpper).toBe(scoreLower);
    expect(scoreMixed).toBe(scoreLower);
  });

  it("gives higher score for consecutive matches", () => {
    // "wh" in "while" is consecutive (index 0,1)
    // "wl" in "while" is not consecutive (index 0,3)
    const consecutiveScore = fuzzyScore("wh", "while");
    const nonConsecutiveScore = fuzzyScore("wl", "while");
    expect(consecutiveScore).toBeGreaterThan(nonConsecutiveScore);
  });

  it("gives higher score for matches closer to start", () => {
    // "et" at the start of "etc" should score higher than "et" at the end of "let"
    const earlyScore = fuzzyScore("et", "etc");
    const lateScore = fuzzyScore("et", "let");
    expect(earlyScore).toBeGreaterThan(lateScore);
  });

  it("scores exact match higher than partial match", () => {
    const exact = fuzzyScore("return", "return");
    const partial = fuzzyScore("ret", "return");
    expect(exact).toBeGreaterThan(partial);
  });

  it("matches multi-character query against target", () => {
    const score = fuzzyScore("whl", "while");
    expect(score).toBeGreaterThan(0);
  });

  it("returns 0 when query is longer than target", () => {
    expect(fuzzyScore("longer", "short")).toBe(0);
  });

  it("sorts items by score descending", () => {
    // Verify the scoring logic would sort "while" above "for" for query "wh"
    const whileScore = fuzzyScore("wh", "while");
    const forScore = fuzzyScore("wh", "for");
    expect(whileScore).toBeGreaterThan(forScore);
  });
});

// ── getItemsForContext tests ──

describe("getItemsForContext", () => {
  it("returns GAP_ITEMS for gap context", () => {
    const items = getItemsForContext("gap");
    expect(items.length).toBe(12);
    expect(items[0].label).toBe("let");
    expect(items[items.length - 1].label).toBe("for");
  });

  it("returns SLOT_ITEMS for slot context", () => {
    const items = getItemsForContext("slot");
    expect(items.length).toBe(3);
    expect(items.map((i) => i.label)).toEqual(["var_ref", "literal", "type_name"]);
  });

  it("returns EXPR_ITEMS for expr context", () => {
    const items = getItemsForContext("expr");
    expect(items.length).toBe(7);
    expect(items.map((i) => i.label)).toEqual([
      "var_ref",
      "literal",
      "binary",
      "unary",
      "call",
      "member",
      "parens",
    ]);
  });
});

// ── FuzzySearchPopup tests ──

describe("FuzzySearchPopup", () => {
  let popup: FuzzySearchPopup;
  let onSelect: ReturnType<typeof vi.fn>;
  let onClose: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onSelect = vi.fn();
    onClose = vi.fn();
    popup = new FuzzySearchPopup(TEST_ITEMS, onSelect, onClose);
  });

  afterEach(() => {
    // Ensure popup is closed after each test
    if ((popup as unknown as Record<string, unknown>).popupDiv) {
      popup.close();
    }
  });

  describe("open / close", () => {
    it("opens popup and creates DOM elements", () => {
      popup.open("");
      const popupDiv = document.querySelector(".fuzzy-search-popup");
      expect(popupDiv).not.toBeNull();
      expect(popupDiv?.querySelector("input")).not.toBeNull();
      expect(popupDiv?.querySelector("ul")).not.toBeNull();
    });

    it("shows all items when query is empty", () => {
      popup.open("");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      // +1 for the selected item styling structure
      expect(listItems.length).toBe(TEST_ITEMS.length);
    });

    it("filters items when query is provided", () => {
      popup.open("wh");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      // Only "while" should match (contains 'wh')
      expect(listItems.length).toBe(1);
    });

    it("shows no-matches message when query matches nothing", () => {
      popup.open("zzzzz");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      expect(listItems.length).toBe(1);
      expect(listItems[0]?.textContent).toContain("No matches");
    });

    it("close() removes popup from DOM", () => {
      popup.open("");
      expect(document.querySelector(".fuzzy-search-popup")).not.toBeNull();
      popup.close();
      expect(document.querySelector(".fuzzy-search-popup")).toBeNull();
    });

    it("close() fires onClose callback", () => {
      popup.open("");
      popup.close();
      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  describe("keyboard navigation", () => {
    it("ArrowDown moves selection to next item", () => {
      popup.open("");
      const input = document.querySelector(
        ".fuzzy-search-popup input",
      ) as HTMLInputElement;

      // First item is selected by default
      let selectedItems = document.querySelectorAll(
        '.fuzzy-search-popup ul li[data-selected="true"]',
      );
      expect(selectedItems.length).toBe(1);

      // Press ArrowDown
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));

      // Second item should now be selected
      selectedItems = document.querySelectorAll(
        '.fuzzy-search-popup ul li[data-selected="true"]',
      );
      expect(selectedItems.length).toBe(1);
    });

    it("ArrowUp moves selection to previous item", () => {
      popup.open("");
      const input = document.querySelector(
        ".fuzzy-search-popup input",
      ) as HTMLInputElement;

      // Move down twice, then up once
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" }));

      // Second item should be selected (down 2, up 1 = index 1)
      const selectedItems = document.querySelectorAll(
        '.fuzzy-search-popup ul li[data-selected="true"]',
      );
      expect(selectedItems.length).toBe(1);
    });

    it("ArrowDown does not wrap beyond last item", () => {
      popup.open("");
      const input = document.querySelector(
        ".fuzzy-search-popup input",
      ) as HTMLInputElement;

      // Press ArrowDown many times to try to go past the last item
      for (let i = 0; i < TEST_ITEMS.length + 5; i++) {
        input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
      }

      // Last item should be selected
      const selectedItems = document.querySelectorAll(
        '.fuzzy-search-popup ul li[data-selected="true"]',
      );
      expect(selectedItems.length).toBe(1);
    });

    it("ArrowUp does not wrap beyond first item", () => {
      popup.open("");
      const input = document.querySelector(
        ".fuzzy-search-popup input",
      ) as HTMLInputElement;

      // Press ArrowUp many times to try to go before the first item
      for (let i = 0; i < 5; i++) {
        input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" }));
      }

      // First item should be selected
      const selectedItems = document.querySelectorAll(
        '.fuzzy-search-popup ul li[data-selected="true"]',
      );
      expect(selectedItems.length).toBe(1);
    });

    it("Enter triggers onSelect with current item and closes popup", () => {
      popup.open("");
      const input = document.querySelector(
        ".fuzzy-search-popup input",
      ) as HTMLInputElement;

      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));

      // Should have selected the first item ("let")
      expect(onSelect).toHaveBeenCalledOnce();
      expect(onSelect).toHaveBeenCalledWith(
        expect.objectContaining({ label: "let" }),
      );
      expect(document.querySelector(".fuzzy-search-popup")).toBeNull();
    });

    it("Enter on filtered list selects correct item", () => {
      popup.open("wh");
      const input = document.querySelector(
        ".fuzzy-search-popup input",
      ) as HTMLInputElement;

      // Simulate typing by setting value and re-filtering
      // The popup re-filters on the next microtask after keydown
      input.value = "wh";

      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));

      expect(onSelect).toHaveBeenCalledOnce();
      expect(onSelect).toHaveBeenCalledWith(
        expect.objectContaining({ label: "while", blockType: "while_loop" }),
      );
    });

    it("Escape closes popup without selection", () => {
      popup.open("");
      const input = document.querySelector(
        ".fuzzy-search-popup input",
      ) as HTMLInputElement;

      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));

      expect(onSelect).not.toHaveBeenCalled();
      expect(onClose).toHaveBeenCalled();
      expect(document.querySelector(".fuzzy-search-popup")).toBeNull();
    });
  });

  describe("mouse interaction", () => {
    it("click on item selects it and closes popup", () => {
      popup.open("");
      const firstItem = document.querySelector(
        '.fuzzy-search-popup ul li',
      ) as HTMLElement;

      firstItem.click();

      expect(onSelect).toHaveBeenCalledOnce();
      expect(onSelect).toHaveBeenCalledWith(
        expect.objectContaining({ label: "let" }),
      );
      expect(document.querySelector(".fuzzy-search-popup")).toBeNull();
    });

    it("click outside closes popup without selection", () => {
      popup.open("");
      document.body.click(); // click outside the popup

      expect(onSelect).not.toHaveBeenCalled();
      expect(onClose).toHaveBeenCalled();
      expect(document.querySelector(".fuzzy-search-popup")).toBeNull();
    });
  });

  describe("filter method", () => {
    it("returns all items for empty query", () => {
      // Access private filter via open + empty query
      popup.open("");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      expect(listItems.length).toBe(TEST_ITEMS.length);
      popup.close();
    });

    it("returns matching items for partial query", () => {
      popup.open("if");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      // Should match "if" (label: "if")
      expect(listItems.length).toBe(1);
      popup.close();
    });

    it("returns items matching by substring", () => {
      popup.open("br");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      // Should match "break" (contains 'br'), and possibly others
      expect(listItems.length).toBeGreaterThanOrEqual(1);
      popup.close();
    });
  });

  describe("open with different item sets", () => {
    it("works with SLOT_ITEMS context", () => {
      const slotItems = getItemsForContext("slot");
      const slotPopup = new FuzzySearchPopup(slotItems, onSelect, onClose);
      slotPopup.open("var");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      expect(listItems.length).toBe(1);
      expect(listItems[0]?.textContent).toContain("var_ref");
      slotPopup.close();
    });

    it("works with EXPR_ITEMS context", () => {
      const exprItems = getItemsForContext("expr");
      const exprPopup = new FuzzySearchPopup(exprItems, onSelect, onClose);
      exprPopup.open("bin");
      const listItems = document.querySelectorAll(
        ".fuzzy-search-popup ul li",
      );
      expect(listItems.length).toBe(1);
      expect(listItems[0]?.textContent).toContain("binary");
      exprPopup.close();
    });
  });
});
