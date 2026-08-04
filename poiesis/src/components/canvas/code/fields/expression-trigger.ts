// 表达式公式框的触发规则纯函数（表驱动，可单测）。
// 运算符集合以编译器 infix.rs 为权威：+ - * / % == != < > <= >= && || ?? is ?. 与一元 !。

export const SINGLE_CHAR_OPERATORS = ["+", "-", "*", "/", "%"];

export const OPERATOR_PREFIX_CANDIDATES: Record<string, string[]> = {
  "<": ["<", "<="],
  ">": [">", ">="],
  "=": ["=="],
  "!": ["!=", "!"],
  "&": ["&&"],
  "|": ["||"],
  "?": ["??", "?."],
};

export interface TriggerContext {
  kind: "operand" | "member" | "operator" | "none";
  token: string;
  tokenStart: number;
  storeName?: string;
}

const IDENT_CHAR = /[A-Za-z0-9_]/;
const OPERATOR_CHAR = /[+\-*/%<>=!&|?]/;

export function getTriggerContext(value: string, cursor: number): TriggerContext {
  const before = value.slice(0, cursor);
  // 成员访问：光标前一字符是 "."
  if (before.endsWith(".")) {
    const storeMatch = before.slice(0, -1).match(/[A-Za-z_]\w*$/);
    // 替换区间须覆盖整个 `Store.` 前缀（tokenStart = store 名起点）：否则选中候选时
    // replaceToken 的替换区间为空、退化为光标处插入，产生 `Store.Store.signal` 前缀重复
    const storeName = storeMatch?.[0];
    const tokenStart = storeName ? cursor - 1 - storeName.length : cursor;
    return { kind: "member", token: "", tokenStart, storeName };
  }
  // 标识符 token：从光标向前收集字母数字下划线
  let i = cursor;
  while (i > 0 && IDENT_CHAR.test(before[i - 1] ?? "")) i--;
  if (i < cursor) return { kind: "operand", token: before.slice(i, cursor), tokenStart: i };
  // 运算符串：从光标向前收集连续运算符字符——多字符粒度：光标停在运算符串中间
  // （如 `count <=` 光标压在 `<` 与 `=` 之间）时若只取光标左侧，token 截断成 "<"，
  // 选中候选只替换前半段、残留后半段 "="（"count <==" 污染）。故再向后扫到运算符串
  // 末尾：完整串（如 "<="）在候选表中无前缀 → 不弹层，天然避免截断替换。
  while (i > 0 && OPERATOR_CHAR.test(before[i - 1] ?? "")) i--;
  if (i < cursor) {
    let j = cursor;
    while (j < value.length && OPERATOR_CHAR.test(value[j] ?? "")) j++;
    return { kind: "operator", token: value.slice(i, j), tokenStart: i };
  }
  // 光标恰好停在运算符字符上（如 "count <" 的光标压在 "<" 上、该字符尚未键入值中）→
  // operator 上下文；同样向后收集完整运算符串（防御性兜底，当前均为单字符）。
  const at = value[cursor];
  if (at !== undefined && OPERATOR_CHAR.test(at)) {
    let j = cursor + 1;
    while (j < value.length && OPERATOR_CHAR.test(value[j] ?? "")) j++;
    return { kind: "operator", token: value.slice(cursor, j), tokenStart: cursor };
  }
  // 空/其它字符（右括号等）→ 操作数位置（允许空 token 弹全量候选）
  if (/[(\s,=]/.test(before[cursor - 1] ?? "")) {
    return { kind: "operand", token: "", tokenStart: cursor };
  }
  return { kind: "none", token: "", tokenStart: cursor };
}

export function operatorCandidates(prefix: string): Array<{ value: string; label: string }> {
  return (OPERATOR_PREFIX_CANDIDATES[prefix] ?? []).map((v) => ({ value: v, label: v }));
}

export function insertAt(value: string, cursor: number, insert: string): { value: string; cursor: number } {
  return { value: value.slice(0, cursor) + insert + value.slice(cursor), cursor: cursor + insert.length };
}

export function replaceToken(value: string, tokenStart: number, cursor: number, insert: string): { value: string; cursor: number } {
  const next = value.slice(0, tokenStart) + insert + value.slice(cursor);
  return { value: next, cursor: tokenStart + insert.length };
}

/** 轻量括号配对校验：跳过字符串字面量（单/双引号）内的括号；引号内 `\` 转义的下一个字符是字面内容，不结束引号。 */
export function countUnbalancedParens(value: string): number {
  let depth = 0;
  let quote: "'" | '"' | null = null;
  for (let i = 0; i < value.length; i++) {
    const ch = value[i]!;
    if (quote) {
      // 转义序列：`\` 后的字符（如 `\"`）属字面内容——跳过，不关闭引号
      if (ch === "\\") { i++; continue; }
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === "'" || ch === '"') { quote = ch; continue; }
    if (ch === "(") depth++;
    else if (ch === ")") depth--;
  }
  return depth;
}
