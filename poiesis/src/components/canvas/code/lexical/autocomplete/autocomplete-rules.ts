export interface AutocompleteItem {
  label: string;
  description: string;
  /** The EditorNode kind this item creates */
  cardKind: "container" | "block" | "inline";
  /** Sub-kind for container/block/inline */
  subKind: string;
}

export interface AutocompleteContext {
  /** What container/block we're inside (null = top-level) */
  parentKind: string | null;
  /** What preceded the cursor (used for . dot detection) */
  precedingCardKind: string | null;
}

// ── Statement-level (top-level) ──
const STATEMENT_ITEMS: AutocompleteItem[] = [
  { label: "let", description: "声明不可变变量", cardKind: "container", subKind: "let" },
  { label: "set", description: "赋值变量", cardKind: "container", subKind: "set" },
  { label: "if", description: "条件分支", cardKind: "block", subKind: "if" },
  { label: "while", description: "while 循环", cardKind: "block", subKind: "while" },
  { label: "for", description: "for..in 循环", cardKind: "block", subKind: "for" },
  { label: "return", description: "返回", cardKind: "container", subKind: "return" },
  { label: "await", description: "异步等待", cardKind: "container", subKind: "await" },
  { label: "call", description: "函数调用", cardKind: "container", subKind: "call" },
  { label: "break", description: "跳出循环", cardKind: "inline", subKind: "break" },
  { label: "continue", description: "跳过本次", cardKind: "inline", subKind: "continue" },
  { label: "//", description: "注释", cardKind: "inline", subKind: "comment" },
];

// ── Inside let container ──
const LET_CHILDREN: AutocompleteItem[] = [
  { label: "mut", description: "可变修饰", cardKind: "inline", subKind: "mut" },
  { label: ":", description: "类型标注", cardKind: "inline", subKind: "separator" },
  { label: "=", description: "初始化", cardKind: "inline", subKind: "operator" },
];

// ── Inside set container ──
const SET_CHILDREN: AutocompleteItem[] = [
  { label: "=", description: "赋值", cardKind: "inline", subKind: "operator" },
  { label: "+=", description: "加等于", cardKind: "inline", subKind: "operator" },
  { label: "-=", description: "减等于", cardKind: "inline", subKind: "operator" },
  { label: "*=", description: "乘等于", cardKind: "inline", subKind: "operator" },
  { label: "/=", description: "除等于", cardKind: "inline", subKind: "operator" },
  { label: "%=", description: "取余等于", cardKind: "inline", subKind: "operator" },
];

// ── Type completions ──
const TYPE_ITEMS: AutocompleteItem[] = [
  { label: "i32", description: "32-bit integer", cardKind: "inline", subKind: "type" },
  { label: "i64", description: "64-bit integer", cardKind: "inline", subKind: "type" },
  { label: "f32", description: "32-bit float", cardKind: "inline", subKind: "type" },
  { label: "f64", description: "64-bit float", cardKind: "inline", subKind: "type" },
  { label: "String", description: "string type", cardKind: "inline", subKind: "type" },
  { label: "bool", description: "boolean", cardKind: "inline", subKind: "type" },
];

// ── Comparison operators (condition context) ──
const COMPARISON_ITEMS: AutocompleteItem[] = [
  { label: ">", description: "大于", cardKind: "inline", subKind: "operator" },
  { label: "<", description: "小于", cardKind: "inline", subKind: "operator" },
  { label: ">=", description: "大于等于", cardKind: "inline", subKind: "operator" },
  { label: "<=", description: "小于等于", cardKind: "inline", subKind: "operator" },
  { label: "==", description: "等于", cardKind: "inline", subKind: "operator" },
  { label: "!=", description: "不等于", cardKind: "inline", subKind: "operator" },
];

// ── Arithmetic operators (expression context) ──
const ARITHMETIC_ITEMS: AutocompleteItem[] = [
  { label: "+", description: "加", cardKind: "inline", subKind: "operator" },
  { label: "-", description: "减", cardKind: "inline", subKind: "operator" },
  { label: "*", description: "乘", cardKind: "inline", subKind: "operator" },
  { label: "/", description: "除", cardKind: "inline", subKind: "operator" },
  { label: "%", description: "取余", cardKind: "inline", subKind: "operator" },
];

// ── Logic operators ──
const LOGIC_ITEMS: AutocompleteItem[] = [
  { label: "&&", description: "且", cardKind: "inline", subKind: "operator" },
  { label: "||", description: "或", cardKind: "inline", subKind: "operator" },
  { label: "!", description: "非", cardKind: "inline", subKind: "operator" },
];

// ── Literals ──
const LITERAL_ITEMS: AutocompleteItem[] = [
  { label: "true", description: "true", cardKind: "inline", subKind: "literal" },
  { label: "false", description: "false", cardKind: "inline", subKind: "literal" },
  { label: "null", description: "null", cardKind: "inline", subKind: "literal" },
];

// ── Dot access ──
const DOT_METHODS: AutocompleteItem[] = [
  { label: ".len()", description: "长度", cardKind: "inline", subKind: "dot" },
  { label: ".push()", description: "添加元素", cardKind: "inline", subKind: "dot" },
  { label: ".pop()", description: "移除末尾", cardKind: "inline", subKind: "dot" },
  { label: ".get()", description: "获取元素", cardKind: "inline", subKind: "dot" },
  { label: ".to_string()", description: "转字符串", cardKind: "inline", subKind: "dot" },
];

/** Get autocomplete items filtered by token prefix and cursor context. */
export function getAutocompleteItems(
  token: string,
  context: AutocompleteContext,
): AutocompleteItem[] {
  const candidates = getCandidatesForContext(context);
  if (!token) return [];
  const lower = token.toLowerCase();
  return candidates.filter(item => item.label.toLowerCase().startsWith(lower));
}

function getCandidatesForContext(context: AutocompleteContext): AutocompleteItem[] {
  // After a dot, show method completions
  if (context.precedingCardKind === "dot") {
    return DOT_METHODS;
  }

  // Inside containers
  switch (context.parentKind) {
    case "let":
      return [...LET_CHILDREN, ...TYPE_ITEMS, ...LITERAL_ITEMS];
    case "set":
      return [...SET_CHILDREN, ...ARITHMETIC_ITEMS, ...LITERAL_ITEMS];
    case "return":
    case "await":
      return [...ARITHMETIC_ITEMS, ...LOGIC_ITEMS, ...LITERAL_ITEMS];
    case "if":
    case "while":
      // In block head = condition context
      return [...COMPARISON_ITEMS, ...LOGIC_ITEMS, ...LITERAL_ITEMS];
    case "for":
      return [...LITERAL_ITEMS];
    case "call":
      return [...ARITHMETIC_ITEMS, ...LITERAL_ITEMS];
    case "array":
    case "map":
      return [...LITERAL_ITEMS];
    default:
      return STATEMENT_ITEMS;
  }
}

/** Map autocomplete item to node construction parameters. */
export function detectCardKindFromItem(item: AutocompleteItem): {
  nodeType: "container" | "block" | "inline";
  kind: string;
} {
  return { nodeType: item.cardKind, kind: item.subKind };
}
