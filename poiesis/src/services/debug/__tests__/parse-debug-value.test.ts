// 类型感知解析器：按声明类型把输入文本转成 OmniLang 字面量；字面量透传识别。
import { describe, it, expect } from "vitest";
import { parseDebugValue } from "../parse-debug-value";

describe("parseDebugValue", () => {
  it("String：任意文本加引号并转义", () => {
    expect(parseDebugValue("String", 'hello "world"')).toEqual({ literal: '"hello \\"world\\""', valid: true });
  });

  it("String：空输入 = 空字符串字面量", () => {
    expect(parseDebugValue("String", "")).toEqual({ literal: '""', valid: true });
  });

  it("bool：仅 true/false 合法", () => {
    expect(parseDebugValue("bool", "true")).toEqual({ literal: "true", valid: true });
    expect(parseDebugValue("bool", "false")).toEqual({ literal: "false", valid: true });
    expect(parseDebugValue("bool", "yes").valid).toBe(false);
  });

  it("i32/f64：数字校验", () => {
    expect(parseDebugValue("i32", "42")).toEqual({ literal: "42", valid: true });
    expect(parseDebugValue("f64", "-3.14")).toEqual({ literal: "-3.14", valid: true });
    expect(parseDebugValue("i32", "abc").valid).toBe(false);
    expect(parseDebugValue("i32", "3.5").valid).toBe(false);
  });

  it("Array<String>：逗号分隔转数组字面量", () => {
    expect(parseDebugValue("Array<String>", 'a, b"c')).toEqual({ literal: '["a", "b\\"c"]', valid: true });
  });

  it("Array<i32>：逐项数字校验", () => {
    expect(parseDebugValue("Array<i32>", "1, 2, 3")).toEqual({ literal: "[1, 2, 3]", valid: true });
    expect(parseDebugValue("Array<i32>", "1, x").valid).toBe(false);
  });

  it("Array<String>：`String[]` 语法糖同解析", () => {
    expect(parseDebugValue("String[]", "a, b").literal).toBe('["a", "b"]');
  });

  it("字面量透传：已写 OmniLang 字面量原样过", () => {
    expect(parseDebugValue("Array<String>", '["a", "b"]')).toEqual({ literal: '["a", "b"]', valid: true });
    expect(parseDebugValue("Map<String, i32>", '{ "a": 1 }')).toEqual({ literal: '{ "a": 1 }', valid: true });
    expect(parseDebugValue("String", "3")).toEqual({ literal: "3", valid: true }); // 数字开头整体透传
  });

  it("struct 数组：文本输入拒绝（用代码样本胶囊）", () => {
    expect(parseDebugValue("RowItem[]", "a, b").valid).toBe(false);
  });

  it("未知类型：原样透传不校验", () => {
    expect(parseDebugValue("Map<String, i32>", "anything")).toEqual({ literal: "anything", valid: true });
  });
});
