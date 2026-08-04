import { describe, it, expect } from "vitest";
import { getTriggerContext, operatorCandidates, insertAt, replaceToken, countUnbalancedParens, SINGLE_CHAR_OPERATORS } from "../expression-trigger";

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
  it("空串 → 0", () => {
    expect(countUnbalancedParens("")).toBe(0);
  });
});
