import { describe, it, expect } from "vitest";
import { fromLegacyString, slotToText } from "../slot-value";

describe("fromLegacyString", () => {
  it("returns empty for undefined", () => {
    expect(fromLegacyString(undefined)).toEqual({ kind: "empty" });
  });
  it("returns empty for empty string", () => {
    expect(fromLegacyString("")).toEqual({ kind: "empty" });
  });
  it("returns text slot for non-empty string", () => {
    expect(fromLegacyString("count")).toEqual({ kind: "text", value: "count" });
  });
});

describe("slotToText", () => {
  it("renders text slot as its value", () => {
    expect(slotToText({ kind: "text", value: "count" })).toBe("count");
  });
  it("renders empty slot as empty string", () => {
    expect(slotToText({ kind: "empty" })).toBe("");
  });
  it("renders expression with var_ref and literal", () => {
    expect(slotToText({
      kind: "expr",
      children: [
        { kind: "var_ref", name: "x" },
        { kind: "literal", value: "0", literalType: "number" },
      ],
    })).toBe("x 0");
  });
  it("renders binary expression", () => {
    expect(slotToText({
      kind: "expr",
      children: [{ kind: "binary", op: "+", left: { kind: "text", value: "x" }, right: { kind: "text", value: "1" } }],
    })).toBe("x + 1");
  });
});
