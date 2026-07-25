import { describe, it, expect } from "vitest";
import { fromLegacyString, slotToText } from "../slot-value";

describe("CodeOp backward compatibility", () => {
  it("old DeclareVarOp string name converts to SlotValue text", () => {
    const sv = fromLegacyString("count");
    expect(sv).toEqual({ kind: "text", value: "count" });
    expect(slotToText(sv)).toBe("count");
  });
  it("old IfBlockOp string condition converts to SlotValue text", () => {
    const sv = fromLegacyString("x > 0");
    expect(sv).toEqual({ kind: "text", value: "x > 0" });
  });
  it("old ReturnExprOp undefined value converts to empty", () => {
    expect(fromLegacyString(undefined)).toEqual({ kind: "empty" });
  });
});
