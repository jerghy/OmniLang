import { describe, it, expect } from "vitest";
import { parseMarkup, CAPSULE_START, CAPSULE_END } from "../expression-trigger";
import { getTriggerContext, operatorCandidates, insertAt, replaceToken, countUnbalancedParens, SINGLE_CHAR_OPERATORS, tokenizeFormula } from "../expression-trigger";

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
  it("独立点不挂死：Store.（成员补全典型中间态）→ identifier + unknown 单字符", () => {
    // 回归：修复前 "." 不被任何前置分支消费、又落在 unknown 停止字符类内 → 空 token 死循环
    expect(kinds("Store.")).toEqual(["identifier:Store", "unknown:."]);
  });
  it("独立点不挂死：a . b / .5 / a..b（空转防护，逐字符单点 unknown）", () => {
    expect(kinds("a . b")).toEqual(["identifier:a", "unknown:.", "identifier:b"]);
    expect(kinds(".5")).toEqual(["unknown:.", "number:5"]);
    expect(kinds("a..b")).toEqual(["identifier:a", "unknown:.", "unknown:.", "identifier:b"]);
  });
  it("未闭合字符串尾部反斜杠：不越界（end 不超 length）", () => {
    // 回归：修复前 `"abc\` 转义分支 i += 2 无边界检查 → end=6 > length=5
    expect(tokenizeFormula('"abc\\')).toEqual([{ kind: "string", text: "abc\\", start: 0, end: 5 }]);
  });
});
