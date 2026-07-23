// poiesis/src/components/canvas/code/card-editor/autocomplete-rules.ts
// Context-aware autocomplete rules with prefix filtering.
// Migrated from lexical/autocomplete/autocomplete-rules.ts

import type { CodeOp } from "../../../../core/types/code-op";

export interface AutocompleteItem {
  label: string;
  description: string;
  kind: string;
}

export interface AutocompleteContext {
  parentKind: CodeOp["kind"] | null;
  slotType: string | null;
}

// ── Statement-level completions ──
const STATEMENT_ITEMS: AutocompleteItem[] = [
  { label: "let", description: "不可变变量", kind: "declare_var" },
  { label: "let mut", description: "可变变量", kind: "declare_var_mut" },
  { label: "set", description: "赋值", kind: "set_var" },
  { label: "if", description: "条件分支", kind: "if_block" },
  { label: "while", description: "while 循环", kind: "while" },
  { label: "for", description: "for..in 循环", kind: "for_in" },
  { label: "return", description: "返回", kind: "return" },
  { label: "await", description: "异步等待", kind: "await" },
  { label: "break", description: "跳出循环", kind: "break" },
  { label: "continue", description: "跳过本次", kind: "continue" },
  { label: "//", description: "注释", kind: "comment" },
];

/** Get filtered autocomplete items based on token prefix. */
export function getAutocompleteItems(
  token: string,
  _context: AutocompleteContext,
): AutocompleteItem[] {
  if (!token) return STATEMENT_ITEMS;
  const lower = token.toLowerCase();
  return STATEMENT_ITEMS.filter((item) => item.label.toLowerCase().startsWith(lower));
}

/** Map autocomplete item kind to CodeOp kind. */
export function detectCardKindFromItem(item: AutocompleteItem): CodeOp["kind"] | null {
  switch (item.kind) {
    case "declare_var":
    case "declare_var_mut":
      return "declare_var";
    case "set_var":
      return "set_var";
    case "if_block":
      return "if_block";
    case "while":
      return "while";
    case "for_in":
      return "for_in";
    case "return":
      return "return";
    case "await":
      return "await";
    case "break":
      return "break";
    case "continue":
      return "continue";
    case "comment":
      return "comment";
    default:
      return null;
  }
}
