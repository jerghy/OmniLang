import type { SlotType } from "../nodes/SlotTextNode";
import type { CardKind } from "../nodes/CardElementNode";

export interface AutocompleteItem {
  label: string;
  description: string;
  kind: string;
}

export interface AutocompleteContext {
  parentKind: CardKind | null;
  slotType: SlotType | null;
}

// ── Statement-level completions (paragraph context) ──
const STATEMENT_ITEMS: AutocompleteItem[] = [
  { label: "let", description: "声明不可变变量", kind: "declare_var" },
  { label: "let mut", description: "声明可变变量", kind: "declare_var_mut" },
  { label: "set", description: "赋值变量", kind: "set_var" },
  { label: "if", description: "条件分支", kind: "if_block" },
  { label: "while", description: "while 循环", kind: "while" },
  { label: "for", description: "for..in 循环", kind: "for_in" },
  { label: "return", description: "返回", kind: "return" },
  { label: "await", description: "异步等待", kind: "await" },
  { label: "break", description: "跳出循环", kind: "break" },
  { label: "continue", description: "跳过本次", kind: "continue" },
  { label: "//", description: "注释", kind: "comment" },
];

// ── Type slot completions ──
const TYPE_ITEMS: AutocompleteItem[] = [
  { label: "i32", description: "32-bit integer", kind: "type" },
  { label: "i64", description: "64-bit integer", kind: "type" },
  { label: "f32", description: "32-bit float", kind: "type" },
  { label: "f64", description: "64-bit float", kind: "type" },
  { label: "String", description: "string type", kind: "type" },
  { label: "bool", description: "boolean", kind: "type" },
  { label: "Array<T>", description: "generic array", kind: "type" },
];

// ── Value/expression context — context-dependent, minimal defaults ──
const EXPRESSION_ITEMS: AutocompleteItem[] = [
  { label: "true", description: "boolean true", kind: "literal" },
  { label: "false", description: "boolean false", kind: "literal" },
  { label: "null", description: "null value", kind: "literal" },
];

/** Get filtered autocomplete items based on token prefix and cursor context. */
export function getAutocompleteItems(
  token: string,
  context: AutocompleteContext,
): AutocompleteItem[] {
  const candidates = getCandidatesForContext(context);
  if (!token) return candidates;
  const lower = token.toLowerCase();
  return candidates.filter(item => item.label.toLowerCase().startsWith(lower));
}

function getCandidatesForContext(context: AutocompleteContext): AutocompleteItem[] {
  const { slotType } = context;

  if (slotType === "type") return TYPE_ITEMS;
  if (slotType === "value" || slotType === "condition" || slotType === "arg") {
    return EXPRESSION_ITEMS;
  }

  // Default: statement-level items
  return STATEMENT_ITEMS;
}

/** Detect CardKind from autocomplete item kind string. */
export function detectCardKindFromItem(item: AutocompleteItem): CardKind | null {
  switch (item.kind) {
    case "declare_var":
    case "declare_var_mut":
      return "declare_var";
    case "set_var": return "set_var";
    case "if_block": return "if_block";
    case "while": return "while";
    case "for_in": return "for_in";
    case "return": return "return";
    case "await": return "await";
    case "break": return "break";
    case "continue": return "continue";
    case "comment": return "comment";
    default: return null;
  }
}
