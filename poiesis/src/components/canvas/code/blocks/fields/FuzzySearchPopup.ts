// poiesis/src/components/canvas/code/blocks/fields/FuzzySearchPopup.ts
// Task 8: Standalone DOM popup for fuzzy block search.
//
// FuzzySearchPopup shows a floating dropdown with VS Code Command Palette-style
// fuzzy filtering. When the user types in a slot/gap, KeyboardHandler calls
// onFuzzySearch(query, context), which triggers this popup.
//
// The popup sends its result via the onSelect callback, which the KeyboardHandler
// or parent component binds to create the selected block / set slot value.

// ── Public types ──

export interface SearchItem {
  label: string;
  blockType: string;
  description: string;
}

// ── Context-aware search items ──

/** Items shown when user is in a gap between statement blocks. */
const GAP_ITEMS: SearchItem[] = [
  { label: "let", blockType: "declare_var", description: "declare variable" },
  { label: "set", blockType: "set_var", description: "assign variable" },
  { label: "return", blockType: "return_stmt", description: "return value" },
  { label: "call", blockType: "call_fn", description: "call function" },
  { label: "await", blockType: "await_expr", description: "await expression" },
  { label: "break", blockType: "break_stmt", description: "break loop" },
  { label: "continue", blockType: "continue_stmt", description: "continue loop" },
  { label: "comment", blockType: "comment_stmt", description: "add comment" },
  { label: "raw", blockType: "raw_stmt", description: "raw code" },
  { label: "if", blockType: "if_block", description: "if / elif / else" },
  { label: "while", blockType: "while_loop", description: "while loop" },
  { label: "for", blockType: "for_in", description: "for..in loop" },
];

/** Items shown when user is on a text slot (e.g. variable name, type). */
const SLOT_ITEMS: SearchItem[] = [
  { label: "var_ref", blockType: "var_ref", description: "variable reference" },
  { label: "literal", blockType: "literal", description: "literal value (number, string, bool)" },
  { label: "type_name", blockType: "type_name", description: "type annotation" },
];

/** Items shown when user is on an expression slot (e.g. init value, condition). */
const EXPR_ITEMS: SearchItem[] = [
  { label: "var_ref", blockType: "var_ref", description: "variable reference" },
  { label: "literal", blockType: "literal", description: "literal value" },
  { label: "binary", blockType: "binary", description: "binary operation (a + b)" },
  { label: "unary", blockType: "unary", description: "unary operation (!x, -y)" },
  { label: "call", blockType: "call_fn", description: "function call in expression" },
  { label: "member", blockType: "member_access", description: "member access (a.b)" },
  { label: "parens", blockType: "parens", description: "parenthesized expression" },
];

/** Returns the appropriate search items for the given editing context. */
export function getItemsForContext(context: "slot" | "gap" | "expr"): SearchItem[] {
  switch (context) {
    case "gap": return GAP_ITEMS;
    case "slot": return SLOT_ITEMS;
    case "expr": return EXPR_ITEMS;
  }
}

// ── Fuzzy scoring algorithm ──

/**
 * Score how well `query` matches `target`.
 *
 * Rules:
 * - Query characters must appear in order within target (case-insensitive).
 * - Consecutive matching characters earn a higher bonus.
 * - Matches earlier in the string score higher.
 * - Returns 0 if not all query characters are found in order.
 * - Returns a positive integer score otherwise.
 */
export function fuzzyScore(query: string, target: string): number {
  if (!query) return 100;

  const q = query.toLowerCase();
  const t = target.toLowerCase();

  let qi = 0;
  let score = 0;
  let consecutive = 0;
  let firstMatchIndex = -1;

  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      if (firstMatchIndex === -1) firstMatchIndex = ti;
      consecutive++;
      qi++;
      // Base points per match + consecutive bonus
      score += 10 + consecutive * 5;
    } else {
      consecutive = 0;
    }
  }

  // Not all query characters matched in order
  if (qi < q.length) return 0;

  // Bonus for matching earlier in the string
  if (firstMatchIndex >= 0) {
    score += Math.max(0, 50 - firstMatchIndex * 2);
  }

  return score;
}

// ── Popup styling constants ──

const POPUP_STYLES = {
  container: `
    position: fixed;
    z-index: 1000;
    background: #ffffff;
    border: 1px solid #d0d0d0;
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.15);
    width: 320px;
    max-height: 400px;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 14px;
  `,
  input: `
    width: 100%;
    padding: 10px 14px;
    border: none;
    border-bottom: 1px solid #eee;
    outline: none;
    font-size: 14px;
    box-sizing: border-box;
    font-family: inherit;
  `,
  list: `
    list-style: none;
    margin: 0;
    padding: 4px 0;
    max-height: 344px;
    overflow-y: auto;
  `,
  item: (isSelected: boolean) => `
    padding: 8px 14px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: ${isSelected ? "#e8f0fe" : "transparent"};
  `,
  itemLabel: `
    font-weight: 600;
    color: #222;
  `,
  itemDesc: `
    font-size: 12px;
    color: #888;
  `,
  noMatch: `
    padding: 12px 14px;
    color: #999;
    font-style: italic;
  `,
};

// ── FuzzySearchPopup ──

export class FuzzySearchPopup {
  private selectedIndex = 0;
  private inputEl: HTMLInputElement | null = null;
  private popupDiv: HTMLDivElement | null = null;
  private listEl: HTMLUListElement | null = null;
  private filteredItems: SearchItem[] = [];
  private items: SearchItem[];
  private readonly onSelect: (item: SearchItem) => void;
  private readonly onClose: () => void;
  private readonly boundKeyDown: (e: KeyboardEvent) => void;
  private readonly boundOutsideClick: (e: MouseEvent) => void;

  constructor(
    items: SearchItem[],
    onSelect: (item: SearchItem) => void,
    onClose: () => void,
  ) {
    this.items = items;
    this.onSelect = onSelect;
    this.onClose = onClose;
    this.boundKeyDown = this.onKeyDown.bind(this);
    this.boundOutsideClick = this.onOutsideClick.bind(this);
  }

  // ── Public API ──

  /** Open popup with optional initial query. */
  open(query: string): void {
    this.closeExisting();

    this.popupDiv = document.createElement("div");
    this.popupDiv.className = "fuzzy-search-popup";
    this.popupDiv.style.cssText = POPUP_STYLES.container;

    this.inputEl = document.createElement("input");
    this.inputEl.type = "text";
    this.inputEl.style.cssText = POPUP_STYLES.input;
    this.inputEl.placeholder = "Search block types...";
    this.inputEl.value = query;

    this.listEl = document.createElement("ul");
    this.listEl.style.cssText = POPUP_STYLES.list;

    this.popupDiv.appendChild(this.inputEl);
    this.popupDiv.appendChild(this.listEl);
    document.body.appendChild(this.popupDiv);

    // Position near the center of the viewport (or near cursor if available)
    this.popupDiv.style.left = `${Math.max(16, window.innerWidth / 2 - 160)}px`;
    this.popupDiv.style.top = `${Math.max(16, window.innerHeight / 2 - 200)}px`;

    this.filterAndRender(query);

    // Focus the input so keyboard events are captured
    this.inputEl.focus();
    this.inputEl.select();

    // Bind events
    this.inputEl.addEventListener("keydown", this.boundKeyDown);
    document.addEventListener("click", this.boundOutsideClick);
  }

  /** Close popup and fire onClose callback. */
  close(): void {
    if (this.popupDiv && this.popupDiv.parentElement) {
      this.inputEl?.removeEventListener("keydown", this.boundKeyDown);
      document.removeEventListener("click", this.boundOutsideClick);
      this.popupDiv.parentElement.removeChild(this.popupDiv);
    }
    this.popupDiv = null;
    this.inputEl = null;
    this.listEl = null;
    this.onClose();
  }

  // ── Filtering ──

  /** Filter items by fuzzyScore > 0, sort by score descending. */
  private filter(query: string): SearchItem[] {
    if (!query) return [...this.items];
    return this.items
      .map((item) => ({ item, score: fuzzyScore(query, item.label) }))
      .filter((x) => x.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((x) => x.item);
  }

  /** Filter and re-render the list. */
  private filterAndRender(query: string): void {
    this.filteredItems = this.filter(query);
    this.selectedIndex = 0;
    this.renderList();
  }

  // ── Rendering ──

  /** Render the filtered items list into the popup. */
  private renderList(): void {
    if (!this.listEl) return;
    this.listEl.innerHTML = "";

    if (this.filteredItems.length === 0) {
      const li = document.createElement("li");
      li.textContent = "No matches";
      li.style.cssText = POPUP_STYLES.noMatch;
      this.listEl.appendChild(li);
      return;
    }

    for (let i = 0; i < this.filteredItems.length; i++) {
      const item = this.filteredItems[i];
      const li = document.createElement("li");
      li.dataset.index = String(i);
      if (i === this.selectedIndex) {
        li.dataset.selected = "true";
      }
      li.style.cssText = POPUP_STYLES.item(i === this.selectedIndex);

      const labelSpan = document.createElement("span");
      labelSpan.textContent = item.label;
      labelSpan.style.cssText = POPUP_STYLES.itemLabel;

      const descSpan = document.createElement("span");
      descSpan.textContent = item.description;
      descSpan.style.cssText = POPUP_STYLES.itemDesc;

      li.appendChild(labelSpan);
      li.appendChild(descSpan);

      // Hover updates selection
      li.addEventListener("mouseenter", () => {
        this.selectedIndex = i;
        this.renderList();
      });

      // Click selects
      li.addEventListener("click", () => {
        this.selectCurrent();
      });

      this.listEl.appendChild(li);
    }

    // Scroll selected item into view (safely handle jsdom / non-browser environments)
    const selectedEl = this.listEl.querySelector(
      `[data-index="${this.selectedIndex}"]`,
    );
    if (selectedEl && typeof selectedEl.scrollIntoView === "function") {
      selectedEl.scrollIntoView({ block: "nearest" });
    }
  }

  // ── Selection ──

  /** Select current item -> call onSelect -> close. */
  private selectCurrent(): void {
    if (
      this.selectedIndex >= 0 &&
      this.selectedIndex < this.filteredItems.length
    ) {
      this.onSelect(this.filteredItems[this.selectedIndex]);
    }
    this.close();
  }

  // ── Keyboard handling ──

  /** Handle keyboard: up/down to navigate, Enter to select, Esc to close. */
  private onKeyDown(e: KeyboardEvent): void {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        if (this.selectedIndex < this.filteredItems.length - 1) {
          this.selectedIndex++;
          this.renderList();
        }
        break;

      case "ArrowUp":
        e.preventDefault();
        if (this.selectedIndex > 0) {
          this.selectedIndex--;
          this.renderList();
        }
        break;

      case "Enter":
        e.preventDefault();
        this.selectCurrent();
        break;

      case "Escape":
        e.preventDefault();
        this.close();
        break;

      default:
        // For character input, the native input element handles the value
        // change. We re-filter on the next microtask so the input's value
        // has been updated.
        setTimeout(() => {
          if (this.inputEl) {
            this.filterAndRender(this.inputEl.value);
          }
        }, 0);
    }
  }

  /** Close popup when clicking outside. */
  private onOutsideClick(e: MouseEvent): void {
    if (this.popupDiv && !this.popupDiv.contains(e.target as Node)) {
      this.close();
    }
  }

  /** Safely close an existing popup instance before re-opening. */
  private closeExisting(): void {
    if (this.popupDiv) {
      this.inputEl?.removeEventListener("keydown", this.boundKeyDown);
      document.removeEventListener("click", this.boundOutsideClick);
      if (this.popupDiv.parentElement) {
        this.popupDiv.parentElement.removeChild(this.popupDiv);
      }
      this.popupDiv = null;
      this.inputEl = null;
      this.listEl = null;
    }
  }
}
