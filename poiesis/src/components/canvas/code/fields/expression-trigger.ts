// 表达式公式框的触发规则纯函数（表驱动，可单测）。
// 运算符集合以编译器 infix.rs 为权威：+ - * / % == != < > <= >= && || ?? is ?. 与一元 !。

export const SINGLE_CHAR_OPERATORS = ["+", "-", "*", "/", "%"];

export const OPERATOR_PREFIX_CANDIDATES: Record<string, string[]> = {
  "<": ["<", "<="],
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
    return { kind: "member", token: "", tokenStart: cursor, storeName: storeMatch?.[0] };
  }
  // 标识符 token：从光标向前收集字母数字下划线
  let i = cursor;
  while (i > 0 && IDENT_CHAR.test(before[i - 1]!)) i--;
  if (i < cursor) return { kind: "operand", token: before.slice(i, cursor), tokenStart: i };
  // 运算符串：从光标向前收集运算符字符
  while (i > 0 && OPERATOR_CHAR.test(before[i - 1]!)) i--;
  if (i < cursor) return { kind: "operator", token: before.slice(i, cursor), tokenStart: i };
  // 光标恰好停在运算符字符上（如 "count <" 的光标压在 "<" 上）→ operator 上下文
  const at = value[cursor];
  if (at !== undefined && OPERATOR_CHAR.test(at)) {
    return { kind: "operator", token: at, tokenStart: cursor };
  }
  // 空/其它字符（右括号等）→ 操作数位置（允许空 token 弹全量候选）
  if (/[(\s,=]/.test(before[cursor - 1]!)) {
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

/** 轻量括号配对校验：跳过字符串字面量（单/双引号）内的括号。 */
export function countUnbalancedParens(value: string): number {
  let depth = 0;
  let quote: "'" | '"' | null = null;
  for (let i = 0; i < value.length; i++) {
    const ch = value[i]!;
    if (quote) {
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === "'" || ch === '"') { quote = ch; continue; }
    if (ch === "(") depth++;
    else if (ch === ")") depth--;
  }
  return depth;
}
