import { describe, it, expect } from "vitest";
import { parseMarkup, CAPSULE_START, CAPSULE_END } from "../expression-trigger";
import { getTriggerContext, operatorCandidates, insertAt, replaceToken, countUnbalancedParens, SINGLE_CHAR_OPERATORS, tokenizeFormula, toMarkup } from "../expression-trigger";
import { isBoundary, OPERATOR_PREFIX_CANDIDATES } from "../expression-trigger";
import { escapeString, unescapeString, serializeSegments } from "../expression-trigger";

describe("parseMarkup", () => {
  it("解析胶囊标记段", () => {
    const segs = parseMarkup(`${CAPSULE_START}count${CAPSULE_END} + 1`);
    expect(segs).toEqual([
      { kind: "capsule", text: "count", start: 0, end: CAPSULE_START.length + 5 + CAPSULE_END.length },
      { kind: "text", text: " + 1", start: segs[0]!.end, end: segs[0]!.end + 4 },
    ]);
  });
  it("结构字符独立成段（连续合并）", () => {
    const segs = parseMarkup(`a(1, 2)`);
    expect(segs.map((s) => [s.kind, s.text])).toEqual([
      ["text", "a"],
      ["struct", "("],
      ["text", "1"],
      ["struct", ","],
      ["text", " 2"],
      ["struct", ")"],
    ]);
  });
  it("结构字符集覆盖 ()[]{}.,:", () => {
    const segs = parseMarkup(`[10].{x:2}`);
    expect(segs.filter((s) => s.kind === "struct").map((s) => s.text).join("")).toBe("[].{:}");
  });
  it("未闭合标记容错：截断到末尾", () => {
    const segs = parseMarkup(`${CAPSULE_START}abc`);
    expect(segs[0]).toEqual({ kind: "capsule", text: "abc", start: 0, end: 3 + CAPSULE_START.length });
  });
});

describe("SINGLE_CHAR_OPERATORS", () => {
  it("单字符运算符集合：+ - * / %", () => {
    expect(SINGLE_CHAR_OPERATORS).toEqual(["+", "-", "*", "/", "%"]);
  });
});

describe("getTriggerContext", () => {
  it("空输入 → none", () => {
    expect(getTriggerContext("", 0)).toEqual({ kind: "none", token: "", tokenStart: 0 });
  });
  it("操作数位置输入字母 → operand", () => {
    expect(getTriggerContext("cou", 3)).toEqual({ kind: "operand", token: "cou", tokenStart: 0 });
  });
  it("运算符后的新操作数 → operand（token 空串）", () => {
    expect(getTriggerContext("count + ", 8)).toEqual({ kind: "operand", token: "", tokenStart: 8 });
  });
  it("运算符字符 → operator", () => {
    expect(getTriggerContext("count <", 6)).toEqual({ kind: "operator", token: "<", tokenStart: 6 });
  });
  it("光标在运算符串中间（<= 的 < 与 = 之间）→ token 为完整运算符串（不截断）", () => {
    // 修复前只收集光标左侧：token 截断成 "<"，选中候选仅替换前半段、残留 "="（"count <==" 污染）。
    // at-cursor 分支（光标前是空格，压在 "<" 上）：向前收集完整串
    expect(getTriggerContext("count <=", 6)).toEqual({ kind: "operator", token: "<=", tokenStart: 6 });
    // 回扫分支（光标前已键入 "<"，光标夹在 < 与 = 之间）：回扫 + 前扫合并完整串
    expect(getTriggerContext("a <=", 3)).toEqual({ kind: "operator", token: "<=", tokenStart: 2 });
  });
  it("单个运算符字符（+）→ operator 且 token 为单字符", () => {
    expect(getTriggerContext("count +", 6)).toEqual({ kind: "operator", token: "+", tokenStart: 6 });
  });
  it("成员点（Store.）→ member，tokenStart 为 store 名起点（替换整段 Store. 前缀）", () => {
    expect(getTriggerContext("ThemeStore.", 11)).toEqual({ kind: "member", token: "", tokenStart: 0, storeName: "ThemeStore" });
  });
  it("数字字面量 → operand", () => {
    expect(getTriggerContext("count + 5", 9)).toEqual({ kind: "operand", token: "5", tokenStart: 8 });
  });
  it("左括号后 → operand", () => {
    expect(getTriggerContext("max(", 4)).toEqual({ kind: "operand", token: "", tokenStart: 4 });
  });
  it("非表达式字符（右括号/分号）→ none", () => {
    expect(getTriggerContext("a)", 2)).toEqual({ kind: "none", token: "", tokenStart: 2 });
  });
});

describe("operatorCandidates", () => {
  it("前缀过滤：< → [<, <=]", () => {
    expect(operatorCandidates("<").map((o) => o.value)).toEqual(["<", "<="]);
  });
  it("> → [>, >=]（infix.rs 有 Gt/GtEq）", () => {
    expect(operatorCandidates(">").map((o) => o.value)).toEqual([">", ">="]);
  });
  it("= → 只提供 ==（不提供赋值 =）", () => {
    expect(operatorCandidates("=").map((o) => o.value)).toEqual(["=="]);
  });
  it("! → [!=, !]", () => {
    expect(operatorCandidates("!").map((o) => o.value)).toEqual(["!=", "!"]);
  });
  it("未收录前缀（如 .）→ []", () => {
    expect(operatorCandidates(".")).toEqual([]);
  });
});

describe("insertAt / replaceToken", () => {
  it("insertAt 光标处插入", () => {
    expect(insertAt("count ", 6, "+ ")).toEqual({ value: "count + ", cursor: 8 });
  });
  it("replaceToken 替换当前 token", () => {
    expect(replaceToken("cou", 0, 3, "count")).toEqual({ value: "count", cursor: 5 });
  });
  it("replaceToken 替换中间 token（保留两侧）", () => {
    expect(replaceToken("a + cou + b", 4, 7, "count")).toEqual({ value: "a + count + b", cursor: 9 });
  });
});

describe("countUnbalancedParens（轻量括号配对校验，spec §4.5）", () => {
  it("配对完整 → 0", () => {
    expect(countUnbalancedParens("max(count, 0) + 1")).toBe(0);
  });
  it("多一个左括号 → 1", () => {
    expect(countUnbalancedParens("(count + 1")).toBe(1);
  });
  it("多一个右括号 → -1", () => {
    expect(countUnbalancedParens("count + 1)")).toBe(-1);
  });
  it("字符串字面量内的括号不计数（简单引号跳过）", () => {
    expect(countUnbalancedParens('"(" + ")"')).toBe(0);
  });
  it("转义引号：\\ 后的字符是字面内容，不结束引号", () => {
    // 值为 "a\"b(" —— 引号内 \ 转义的 " 不关闭引号，其后 ( 仍属字符串内容不计数
    expect(countUnbalancedParens('"a\\"b("')).toBe(0);
    expect(countUnbalancedParens('"\\""')).toBe(0);
  });
  it("空串 → 0", () => {
    expect(countUnbalancedParens("")).toBe(0);
  });
});

function kinds(value: string): string[] {
  return tokenizeFormula(value).map((t) => `${t.kind}:${t.text}`);
}

describe("tokenizeFormula", () => {
  it("混合表达式：标识符/运算符/数字/括号", () => {
    expect(kinds("count + 1")).toEqual(["identifier:count", "operator:+", "number:1"]);
  });
  it("多字符运算符最长匹配", () => {
    expect(kinds("a <= b")).toEqual(["identifier:a", "operator:<=", "identifier:b"]);
    expect(kinds("a == b && c")).toEqual(["identifier:a", "operator:==", "identifier:b", "operator:&&", "identifier:c"]);
  });
  it("member：Store.signal 整体一个 token", () => {
    expect(kinds("TodoStore.total > 0")).toEqual(["member:TodoStore.total", "operator:>", "number:0"]);
  });
  it("字符串字面量（含空格/中文/运算符字符）整体一个 token", () => {
    expect(kinds(`msg == "a + b 你好"`)).toEqual(["identifier:msg", "operator:==", "string:a + b 你好"]);
  });
  it("未知字符收集为 unknown 保留原文", () => {
    expect(kinds("a @ b")).toEqual(["identifier:a", "unknown:@", "identifier:b"]);
  });
  it("括号独立 token", () => {
    expect(kinds("f(x)")).toEqual(["identifier:f", "paren:(", "identifier:x", "paren:)"]);
  });
  it("is 关键字识别为运算符", () => {
    expect(kinds("x is i32")).toEqual(["identifier:x", "operator:is", "identifier:i32"]);
  });
  it("token 偏移可定位（start/end 拼接还原原文）", () => {
    const toks = tokenizeFormula("a <= 2");
    expect(toks.map((t) => [t.start, t.end])).toEqual([[0, 1], [2, 4], [5, 6]]);
    expect(toks.map((t) => "a <= 2".slice(t.start, t.end)).join("")).toBe("a<=2");
  });
  it("独立点归 paren：Store.（成员补全典型中间态）→ identifier + paren 单字符", () => {
    // Task 3 契约（brief Step 3）：`.` 从 unknown 归入 paren（结构字符），toMarkup 原样输出。
    // 回归注记保留：修复前 "." 不被任何前置分支消费、又落在 unknown 停止字符类内 → 空 token 死循环
    expect(kinds("Store.")).toEqual(["identifier:Store", "paren:."]);
  });
  it("独立点归 paren：a . b / .5 / a..b（逐字符单点）", () => {
    expect(kinds("a . b")).toEqual(["identifier:a", "paren:.", "identifier:b"]);
    expect(kinds(".5")).toEqual(["paren:.", "number:5"]);
    expect(kinds("a..b")).toEqual(["identifier:a", "paren:.", "paren:.", "identifier:b"]);
  });
  it("未闭合字符串尾部反斜杠：不越界（end 不超 length）", () => {
    // 回归：修复前 `"abc\` 转义分支 i += 2 无边界检查 → end=6 > length=5
    expect(tokenizeFormula('"abc\\')).toEqual([{ kind: "string", text: "abc\\", start: 0, end: 5 }]);
  });
});

describe("serializeSegments 字面量判定与拼接", () => {
  const c = (s: string) => `${CAPSULE_START}${s}${CAPSULE_END}`;
  it("纯数字 → number 原样", () => {
    expect(serializeSegments(`${c("count")} + 1`)).toBe("count + 1");
  });
  it("带小数点数字原样（点属结构字符，拼接一致）", () => {
    expect(serializeSegments(`1.5`)).toBe("1.5");
  });
  it("true/false/null 原样", () => {
    expect(serializeSegments(`true`)).toBe("true");
    expect(serializeSegments(`false`)).toBe("false");
    expect(serializeSegments(`null`)).toBe("null");
  });
  it("普通文本 → 字符串加双引号（trim + 转义）", () => {
    expect(serializeSegments(`hello world`)).toBe('"hello world"');
    expect(serializeSegments(`user-settings`)).toBe('"user-settings"');
  });
  it("字符串内嵌引号转义", () => {
    expect(serializeSegments(`say "hi"`)).toBe('"say \\"hi\\""');
  });
  it("空白段原样保留（胶囊间距不变）", () => {
    expect(serializeSegments(`${c("count")} + ${c("count")}`)).toBe("count + count");
  });
  it("点号后的自由文本段 → member 原样（不判字符串）", () => {
    expect(serializeSegments(`${c("sizeBox")}.measure()`)).toBe("sizeBox.measure()");
    expect(serializeSegments(`${c("mirrorInput")}.value`)).toBe("mirrorInput.value");
  });
  it("member 链含参数与数字", () => {
    expect(serializeSegments(`${c("items")}.get(${c("items")}.len() - 1)`)).toBe("items.get(items.len() - 1)");
  });
  it("胶囊原样、结构字符原样", () => {
    expect(serializeSegments(`${c("+")}${c("count")}`)).toBe("+count");
  });
  it("escapeString/unescapeString 互逆", () => {
    expect(unescapeString(escapeString(`a\\b"c`))).toBe(`a\\b"c`);
  });
});

describe("toMarkup 反向结构化 + round-trip", () => {
  const c = (s: string) => `${CAPSULE_START}${s}${CAPSULE_END}`;
  it("标识符/运算符 → 胶囊；数字/空白原样", () => {
    // brief 期望含多余裸 `+`（`${c("count")} + ${c("+")} 1`）——与 brief 自身 toMarkup 实现及 round-trip 契约矛盾，已修正（见 task-3-report）
    expect(toMarkup("count + 1")).toBe(`${c("count")} ${c("+")} 1`);
  });
  it("字符串剥引号反转义", () => {
    expect(toMarkup('"hello"')).toBe("hello");
    expect(toMarkup('"say \\"hi\\""')).toBe(`say "hi"`);
  });
  it("member token 拆分三段（引用胶囊+点+member 段）", () => {
    expect(toMarkup("mirrorInput.value")).toBe(`${c("mirrorInput")}.value`);
    expect(toMarkup("Store.signal")).toBe(`${c("Store")}.signal`);
  });
  it("方法链：member 拆段、.height 为点+胶囊（height 是独立 identifier token）", () => {
    expect(toMarkup("sizeBox.measure().height")).toBe(`${c("sizeBox")}.measure().${c("height")}`);
  });
  it("结构字符原样（括号/逗号/方括号/花括号/冒号）；结构体 key 是标识符 → 胶囊", () => {
    expect(toMarkup("foo(a, b)")).toBe(`${c("foo")}(${c("a")}, ${c("b")})`);
    expect(toMarkup("[10, 20]")).toBe(`[10, 20]`);
    expect(toMarkup("Point { x: 0 }")).toBe(`${c("Point")} { ${c("x")}: 0 }`);
  });
  it("is 运算符成胶囊", () => {
    expect(toMarkup("x is String")).toBe(`${c("x")} ${c("is")} ${c("String")}`);
  });
  it("round-trip 双向一致：serialize(toMarkup(t)) === t", () => {
    const cases = ["count + 1", "mirrorInput.value", '"hello world"', "user-settings", "items.get(items.len() - 1)", "sizeBox.measure().height", "Store.signal", "total > 0 && total < 10", 'say "hi" + name'];
    for (const t of cases) expect(serializeSegments(toMarkup(t))).toBe(t);
  });
});

describe("isBoundary 边界触发", () => {
  const c = (s: string) => `${CAPSULE_START}${s}${CAPSULE_END}`;
  it("开头/空白后/胶囊后/结构字符后 → 边界", () => {
    expect(isBoundary("", 0)).toBe(true);
    expect(isBoundary("count ", 6)).toBe(true);
    expect(isBoundary(`${c("count")}/`, c("count").length)).toBe(true); // slash 前是胶囊标记尾
    expect(isBoundary("(", 1)).toBe(true);
  });
  it("词中间（字母数字后）→ 非边界（字符串内容）", () => {
    expect(isBoundary("user", 4)).toBe(false);
    expect(isBoundary("a/b", 2)).toBe(false);
  });
});
describe("OPERATOR_PREFIX_CANDIDATES 增补", () => {
  it("单字符运算符弹自身", () => {
    expect(OPERATOR_PREFIX_CANDIDATES["+"]).toEqual(["+"]);
    expect(OPERATOR_PREFIX_CANDIDATES["-"]).toEqual(["-"]);
    expect(OPERATOR_PREFIX_CANDIDATES["/"]).toEqual(["/"]);
    expect(OPERATOR_PREFIX_CANDIDATES["%"]).toEqual(["%"]);
  });
  it("多字符前缀不变（infix.rs 权威）", () => {
    expect(OPERATOR_PREFIX_CANDIDATES["<"]).toEqual(["<", "<="]);
    expect(OPERATOR_PREFIX_CANDIDATES["="]).toEqual(["=="]);
    expect(OPERATOR_PREFIX_CANDIDATES["?"]).toEqual(["??", "?."]);
  });
});
