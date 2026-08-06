/** 胶囊标记：textarea 中胶囊段用私有区字符包裹（React Mentions 同款）。 */
export const CAPSULE_START = "\uE000";
export const CAPSULE_END = "\uE001";

/** 结构字符：非胶囊的表达式标点，着色渲染、提交原样。 */
export const STRUCT_CHARS = new Set(["(", ")", "[", "]", "{", "}", ",", ":", "."]);

export interface Segment {
  kind: "capsule" | "struct" | "text";
  /** 段内容（capsule 不含标记字符） */
  text: string;
  /** 在 markup 中的起止偏移（覆盖层定位/光标定位用） */
  start: number;
  end: number;
}

/** 标记文本 → 段序列：胶囊标记段 / 连续结构字符段 / 连续普通文本段。 */
export function parseMarkup(markup: string): Segment[] {
  const segments: Segment[] = [];
  let i = 0;
  while (i < markup.length) {
    if (markup[i] === CAPSULE_START) {
      const end = markup.indexOf(CAPSULE_END, i + 1);
      const endPos = end === -1 ? markup.length : end + 1;
      segments.push({ kind: "capsule", text: markup.slice(i + 1, end === -1 ? endPos : end), start: i, end: endPos });
      i = endPos;
      continue;
    }
    if (STRUCT_CHARS.has(markup[i]!)) {
      let j = i;
      while (j < markup.length && STRUCT_CHARS.has(markup[j]!)) j++;
      segments.push({ kind: "struct", text: markup.slice(i, j), start: i, end: j });
      i = j;
      continue;
    }
    let j = i;
    while (j < markup.length && markup[j] !== CAPSULE_START && !STRUCT_CHARS.has(markup[j]!)) j++;
    segments.push({ kind: "text", text: markup.slice(i, j), start: i, end: j });
    i = j;
  }
  return segments;
}

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

export interface FormulaToken {
  kind: "identifier" | "operator" | "number" | "string" | "paren" | "member" | "unknown";
  text: string;
  start: number;
  end: number;
}

const STRING_QUOTE = /['"]/;

/** 表达式分词（Notion 式 token 化编辑的数据基础）。
 *  运算符以 infix.rs 为权威：两字符（<= >= == != && || ?? ?.）优先，单字符兜底；
 *  `is` 关键字识别为运算符；`Store.signal` 整体 member token；
 *  引号字符串跳过内部运算符字符与转义；未知字符逐字符收集。 */
export function tokenizeFormula(value: string): FormulaToken[] {
  const tokens: FormulaToken[] = [];
  let i = 0;
  while (i < value.length) {
    const ch = value[i]!;
    // 空白：跳过
    if (/\s/.test(ch)) { i++; continue; }
    // 字符串：引号包裹整段（含转义）
    if (STRING_QUOTE.test(ch)) {
      const start = i;
      i++;
      while (i < value.length) {
        if (value[i] === "\\") {
          // 转义边界防护：尾部反斜杠（未闭合字符串，如 `"abc\`）无后继字符可跳——只推进 1，防止 end 越界
          if (i + 1 < value.length) i += 2; else i++;
          continue;
        }
        if (value[i] === ch) { i++; break; }
        i++;
      }
      // text 不含引号本身（测试契约）；start/end 覆盖整段含引号（替换/定位用）；未闭合时取到末尾
      const textEnd = i > start + 1 && value[i - 1] === ch ? i - 1 : i;
      tokens.push({ kind: "string", text: value.slice(start + 1, textEnd), start, end: i });
      continue;
    }
    // 数字
    if (/[0-9]/.test(ch)) {
      const start = i;
      while (i < value.length && /[0-9.]/.test(value[i]!)) i++;
      tokens.push({ kind: "number", text: value.slice(start, i), start, end: i });
      continue;
    }
    // 标识符 / member（Store.signal） / is 运算符
    if (/[A-Za-z_]/.test(ch)) {
      const start = i;
      while (i < value.length && /[A-Za-z0-9_]/.test(value[i]!)) i++;
      // member：ident . ident（如 TodoStore.total）
      if (value[i] === "." && /[A-Za-z_]/.test(value[i + 1] ?? "")) {
        i++;
        while (i < value.length && /[A-Za-z0-9_]/.test(value[i]!)) i++;
        tokens.push({ kind: "member", text: value.slice(start, i), start, end: i });
      } else if (value.slice(start, i) === "is") {
        tokens.push({ kind: "operator", text: "is", start, end: i });
      } else {
        tokens.push({ kind: "identifier", text: value.slice(start, i), start, end: i });
      }
      continue;
    }
    // 括号
    if (ch === "(" || ch === ")") { tokens.push({ kind: "paren", text: ch, start: i, end: i + 1 }); i++; continue; }
    // 运算符：两字符优先（OPERATOR_PREFIX_CANDIDATES 表内长度 2 项），单字符兜底
    if (/[+\-*/%<>=!&|?]/.test(ch)) {
      const two = value.slice(i, i + 2);
      const start = i;
      if (two === "<=" || two === ">=" || two === "==" || two === "!=" || two === "&&" || two === "||" || two === "??" || two === "?.") {
        i += 2;
      } else {
        i++;
      }
      tokens.push({ kind: "operator", text: value.slice(start, i), start, end: i });
      continue;
    }
    // 未知字符（逐字符收集连续段）
    const start = i;
    while (i < value.length && !/[A-Za-z0-9_\s()+\-*/%<>=!&|?.'"]/.test(value[i]!)) i++;
    // 空转防护：独立 "." 等字符落在停止字符类内、又不被任何前置分支消费（member 需 ident.ident、
    // 数字需数字先行、?. 需 ? 先行）——至少推进 1 字符，杜绝空 token 死循环（如 `Store.` 成员补全中间态）
    if (start === i) i++;
    tokens.push({ kind: "unknown", text: value.slice(start, i), start, end: i });
  }
  return tokens;
}

/** 自由文本段提交分类：空白原样 / number / bool / null / member（`.` 后）原样 / string 加引号。 */
export type LiteralClass = "blank" | "number" | "bool" | "null" | "string" | "member";

export function escapeString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

export function unescapeString(s: string): string {
  return s.replace(/\\"/g, '"').replace(/\\\\/g, "\\");
}

/** 字面量判定：trim 后为空 → blank；前段以 `.` 结尾 → member；纯数字 → number；true/false/null 原样；其余 string。 */
export function classifyLiteral(text: string, prevStructDot: boolean): LiteralClass {
  const t = text.trim();
  if (t === "") return "blank";
  if (prevStructDot) return "member";
  if (/^\d+(\.\d+)?$/.test(t)) return "number";
  if (t === "true" || t === "false") return "bool";
  if (t === "null") return "null";
  return "string";
}

/** 标记文本 → omni 表达式（提交拼接）：胶囊/结构字符原样，自由文本按判定转换。 */
export function serializeSegments(markup: string): string {
  const segs = parseMarkup(markup);
  let out = "";
  for (let k = 0; k < segs.length; k++) {
    const s = segs[k]!;
    if (s.kind === "capsule" || s.kind === "struct") { out += s.text; continue; }
    const prev = k > 0 ? segs[k - 1]! : null;
    const cls = classifyLiteral(s.text, prev?.kind === "struct" && prev.text.endsWith("."));
    if (cls === "string") {
      // 纯运算符/数字/空白组成的表达式片段（无标识符/引号字符，如 " + 1"）→ 原样保留，不判字符串
      // （brief 参考实现逐字执行会对该类片段加引号产出 `count"+ 1"`，与测试契约矛盾，见 task-2-report）
      out += /[A-Za-z_"]/.test(s.text) ? `"${escapeString(s.text.trim())}"` : s.text;
    } else out += s.text;
  }
  return out;
}
