// 类型感知调试值解析：把用户输入文本按声明类型转成 OmniLang 字面量。
// 规则（spec 4.3）：String 加引号转义 / bool 严格 / 数字校验 / Array 逗号分隔逐项转义 /
// 字面量透传识别（以 [ { " ' 开头或整体为数字/bool → 原样过）/ struct 数组拒绝 / 未知类型透传。

export interface ParsedDebugValue {
  literal: string;
  valid: boolean;
}

const INT_RE = /^-?\d+$/;
const FLOAT_RE = /^-?\d+(\.\d+)?$/;
const BOOL_RE = /^(true|false)$/;
// 字面量透传：以数组/对象/引号开头，或整体是数字/bool
function isLiteralText(text: string): boolean {
  const t = text.trim();
  if (t.length === 0) return false;
  const first = t[0];
  if (first === "[" || first === "{" || first === '"' || first === "'") return true;
  return FLOAT_RE.test(t) || BOOL_RE.test(t);
}

function escapeString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function elementTypeOf(arrayType: string): string | null {
  // "Array<String>" → "String"；"String[]" → "String"
  const m = arrayType.match(/^Array<(.+)>$/);
  if (m) return m[1];
  const m2 = arrayType.match(/^(.+)\[\]$/);
  return m2 ? m2[1] : null;
}

function parseArrayItems(elemType: string, text: string): { literal: string; valid: boolean } {
  const items = text.split(",").map((s) => s.trim()).filter((s) => s.length > 0);
  const literals: string[] = [];
  for (const item of items) {
    const parsed = parseDebugValue(elemType, item);
    if (!parsed.valid) return { literal: "", valid: false };
    literals.push(parsed.literal);
  }
  return { literal: `[${literals.join(", ")}]`, valid: true };
}

export function parseDebugValue(type: string, text: string): ParsedDebugValue {
  const t = text.trim();
  if (t.length === 0) {
    // 空输入：String → ""，其余 → 无效（避免覆盖成空）
    return type === "String" ? { literal: '""', valid: true } : { literal: "", valid: false };
  }
  // 数组类型：先识别已写字面量整体透传；否则逐项解析（struct 元素拒绝）
  const elemType = elementTypeOf(type);
  if (elemType) {
    if (isLiteralText(t)) return { literal: t, valid: true };
    if (elemType !== "String" && elemType !== "i32" && elemType !== "i64" && elemType !== "f64" && elemType !== "f32" && elemType !== "bool") {
      return { literal: "", valid: false };
    }
    return parseArrayItems(elemType, t);
  }
  // 标量类型：数字/bool 必须通过各自校验，不能被透传绕过
  if (type === "String") {
    if (isLiteralText(t)) return { literal: t, valid: true };
    return { literal: `"${escapeString(t)}"`, valid: true };
  }
  if (type === "bool") return BOOL_RE.test(t) ? { literal: t, valid: true } : { literal: "", valid: false };
  if (type === "i32" || type === "i64") {
    // 整数类型：拒绝小数（3.5 不得冒充 i32）
    return INT_RE.test(t) ? { literal: t, valid: true } : { literal: "", valid: false };
  }
  if (type === "f64" || type === "f32") {
    return FLOAT_RE.test(t) ? { literal: t, valid: true } : { literal: "", valid: false };
  }
  // 未知类型：原样透传不校验
  return { literal: t, valid: true };
}
