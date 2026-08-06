// 成真/成假合成器（spec 4.4）：点选 @if 分支 → 合成使该条件为真/假的值。
// 只支持单信号简单比较；复合条件/函数调用由调用方判定为 invalid 置灰。

export type ComparisonOp = "==" | "!=" | ">" | ">=" | "<" | "<=" | null;

export interface ConditionComparison {
  /** 信号 key，可带 Store. 限定（如 NotificationStore.unreadCount） */
  sig: string;
  op: ComparisonOp;
  /** 比较字面量（原始文本，不带引号）："loading" / 0 / false */
  literal?: string;
}

export type TruthGoal = "true" | "false";

const NUM_RE = /^-?\d+(\.\d+)?$/;
const BOOL_RE = /^(true|false)$/;

/** 求「x == literal」为假的值（即 != literal 为真的值） */
function falsifyEquals(literal: string): string {
  if (BOOL_RE.test(literal)) return literal === "true" ? "false" : "true";
  if (NUM_RE.test(literal)) return Number(literal) === 0 ? "1" : "0";
  // 字符串
  return literal === "" ? "x" : "";
}

export function synthesizeConditionValue(
  cmp: ConditionComparison,
  goal: TruthGoal,
): { literal: string; valid: boolean } {
  const { op, literal } = cmp;
  if (op === null) {
    return { literal: goal === "true" ? "true" : "false", valid: true };
  }
  if (literal === undefined) {
    // 无字面量：无法合成，一律视为复杂，交由 UI 置灰。
    // 注意：literal 为空字符串 "" 是有意义的（== 成假 → "x"，spec 4.4 表），不在此拦截。
    return { literal: "", valid: false };
  }
  switch (op) {
    case "==":
      return goal === "true"
        ? { literal, valid: true }
        : { literal: falsifyEquals(literal), valid: true };
    case "!=":
      return goal === "true"
        ? { literal: falsifyEquals(literal), valid: true }
        : { literal, valid: true };
    case ">":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal: String(Number(literal) + 1), valid: true }
        : { literal, valid: true };
    case ">=":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal, valid: true }
        : { literal: String(Number(literal) - 1), valid: true };
    case "<":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal: String(Number(literal) - 1), valid: true }
        : { literal, valid: true };
    case "<=":
      if (!NUM_RE.test(literal)) return { literal: "", valid: false };
      return goal === "true"
        ? { literal, valid: true }
        : { literal: String(Number(literal) + 1), valid: true };
    default:
      return { literal: "", valid: false };
  }
}
