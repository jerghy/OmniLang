// 成真/成假合成规则（spec 4.4 表）：点选分支 → 合成使条件为真/假的值。
import { describe, it, expect } from "vitest";
import { synthesizeConditionValue } from "../synthesize-condition";

describe("synthesizeConditionValue", () => {
  it("裸 bool：成真 true / 成假 false", () => {
    expect(synthesizeConditionValue({ sig: "isActive", op: null }, "true")).toEqual({ literal: "true", valid: true });
    expect(synthesizeConditionValue({ sig: "isActive", op: null }, "false")).toEqual({ literal: "false", valid: true });
  });

  it("!sig 取反：成真 false / 成假 true（F1 修复）", () => {
    expect(synthesizeConditionValue({ sig: "badgeVisible", op: null, negated: true }, "true"))
      .toEqual({ literal: "false", valid: true });
    expect(synthesizeConditionValue({ sig: "badgeVisible", op: null, negated: true }, "false"))
      .toEqual({ literal: "true", valid: true });
  });

  it("==：成真取字面量", () => {
    expect(synthesizeConditionValue({ sig: "status", op: "==", literal: "loading" }, "true"))
      .toEqual({ literal: "loading", valid: true });
  });

  it("== 成假：bool 取反", () => {
    expect(synthesizeConditionValue({ sig: "flag", op: "==", literal: "true" }, "false"))
      .toEqual({ literal: "false", valid: true });
    expect(synthesizeConditionValue({ sig: "flag", op: "==", literal: "false" }, "false"))
      .toEqual({ literal: "true", valid: true });
  });

  it("== 成假：数字 0 取 1，非 0 取 0", () => {
    expect(synthesizeConditionValue({ sig: "count", op: "==", literal: "0" }, "false")).toEqual({ literal: "1", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "==", literal: "5" }, "false")).toEqual({ literal: "0", valid: true });
  });

  it("== 成假：字符串非空取空、空取 x", () => {
    expect(synthesizeConditionValue({ sig: "status", op: "==", literal: "loading" }, "false")).toEqual({ literal: "", valid: true });
    expect(synthesizeConditionValue({ sig: "status", op: "==", literal: "" }, "false")).toEqual({ literal: "x", valid: true });
  });

  it("!=：成真取「成假」值，成假取字面量", () => {
    expect(synthesizeConditionValue({ sig: "count", op: "!=", literal: "0" }, "true")).toEqual({ literal: "1", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "!=", literal: "0" }, "false")).toEqual({ literal: "0", valid: true });
  });

  it("比较链：> n 成真 n+1、成假 n", () => {
    expect(synthesizeConditionValue({ sig: "count", op: ">", literal: "10" }, "true")).toEqual({ literal: "11", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: ">", literal: "10" }, "false")).toEqual({ literal: "10", valid: true });
  });

  it("比较链：>= 成真 n、成假 n-1；< 成真 n-1、成假 n；<= 成真 n、成假 n+1", () => {
    expect(synthesizeConditionValue({ sig: "count", op: ">=", literal: "10" }, "true")).toEqual({ literal: "10", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: ">=", literal: "10" }, "false")).toEqual({ literal: "9", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<", literal: "10" }, "true")).toEqual({ literal: "9", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<", literal: "10" }, "false")).toEqual({ literal: "10", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<=", literal: "10" }, "true")).toEqual({ literal: "10", valid: true });
    expect(synthesizeConditionValue({ sig: "count", op: "<=", literal: "10" }, "false")).toEqual({ literal: "11", valid: true });
  });

  it("无字面量比较（sig > 但无 literal）→ invalid", () => {
    expect(synthesizeConditionValue({ sig: "count", op: ">" }, "true").valid).toBe(false);
  });

  it("store qualified sig 不参与合成逻辑差异（sig 仅作标识）", () => {
    expect(synthesizeConditionValue({ sig: "NotificationStore.unreadCount", op: "==", literal: "0" }, "true"))
      .toEqual({ literal: "0", valid: true });
  });
});
