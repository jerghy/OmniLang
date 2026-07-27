import { describe, it, expect } from "vitest";
import {
  parsePidAnnotation,
  parseTypeAnnotation,
  parseAnyAnnotation,
  isPidAnnotation as _isPidAnnotation,
  isTypeAnnotation as _isTypeAnnotation,
  isAnyAnnotation,
  emitPidAnnotation,
  PID_REGEX as _PID_REGEX,
  TYPE_REGEX as _TYPE_REGEX,
  VALID_TYPES,
} from "../annotation-registry";

describe("parsePidAnnotation", () => {
  it("extracts pid from // @pid:xxx", () => {
    expect(parsePidAnnotation("// @pid:a1")).toEqual({ pid: "a1" });
  });

  it("handles leading whitespace", () => {
    expect(parsePidAnnotation("    // @pid:b2")).toEqual({ pid: "b2" });
  });

  it("handles trailing content (inline pid)", () => {
    expect(parsePidAnnotation("signal count: i32 = 0; // @pid:sig_0")).toEqual({ pid: "sig_0" });
  });

  it("returns null for non-pid comments", () => {
    expect(parsePidAnnotation("// @if")).toBeNull();
    expect(parsePidAnnotation("// some comment")).toBeNull();
    expect(parsePidAnnotation("")).toBeNull();
  });

  it("returns null for old design annotations", () => {
    expect(parsePidAnnotation("// @token id=tk_1")).toBeNull();
    expect(parsePidAnnotation("// @class id=cls_1")).toBeNull();
    expect(parsePidAnnotation("// @variant id=var_1")).toBeNull();
  });
});

describe("parseTypeAnnotation", () => {
  it.each(["if", "elif", "else", "return", "code-block", "for", "while"])(
    "recognizes // @%s",
    (type) => {
      expect(parseTypeAnnotation(`// @${type}`)).toEqual({ nodeType: type });
    },
  );

  it("handles trailing whitespace", () => {
    expect(parseTypeAnnotation("// @if   ")).toEqual({ nodeType: "if" });
  });

  it("rejects unknown types", () => {
    expect(parseTypeAnnotation("// @unknown")).toBeNull();
    expect(parseTypeAnnotation("// @token")).toBeNull();
    expect(parseTypeAnnotation("// @pid:a1")).toBeNull();
  });

  it("rejects empty line", () => {
    expect(parseTypeAnnotation("")).toBeNull();
  });
});

describe("parseAnyAnnotation", () => {
  it("returns pid when line has @pid", () => {
    expect(parseAnyAnnotation("// @pid:a1")).toEqual({ pid: "a1" });
  });

  it("returns nodeType when line has @type", () => {
    expect(parseAnyAnnotation("// @if")).toEqual({ nodeType: "if" });
  });

  it("returns null for non-annotations", () => {
    expect(parseAnyAnnotation("// hello")).toBeNull();
  });
});

describe("is functions", () => {
  it("isAnyAnnotation returns true for both pid and type", () => {
    expect(isAnyAnnotation("// @pid:a1")).toBe(true);
    expect(isAnyAnnotation("// @if")).toBe(true);
    expect(isAnyAnnotation("// plain comment")).toBe(false);
  });
});

describe("emitPidAnnotation", () => {
  it("emits correct format", () => {
    expect(emitPidAnnotation("a1")).toBe("// @pid:a1");
  });
});

describe("VALID_TYPES", () => {
  it("contains all 7 types", () => {
    expect(VALID_TYPES.size).toBe(7);
    expect(VALID_TYPES.has("if")).toBe(true);
    expect(VALID_TYPES.has("elif")).toBe(true);
    expect(VALID_TYPES.has("else")).toBe(true);
    expect(VALID_TYPES.has("return")).toBe(true);
    expect(VALID_TYPES.has("code-block")).toBe(true);
    expect(VALID_TYPES.has("for")).toBe(true);
    expect(VALID_TYPES.has("while")).toBe(true);
  });
});
