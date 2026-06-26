// editor/client/src/omni-gen/__tests__/validator.test.ts
import { describe, it, expect, vi } from "vitest";
import { validate } from "../validator";

// Mock rustCompiler — we test the wrapper logic, not the WASM
vi.mock("../../../api/rustCompiler", () => ({
  rustCompiler: {
    compileToIR: vi.fn(),
  },
}));

import { rustCompiler } from "../../../api/rustCompiler";

describe("validate", () => {
  it("returns valid=true when compileToIR succeeds", () => {
    (rustCompiler.compileToIR as any).mockReturnValue({
      success: true,
      ir: { version: "1", components: {}, stores: {}, source_map: {} },
      errors: [],
    });

    const result = validate("component App { view { <div></div> } }");

    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });

  it("returns valid=false with errors when compileToIR fails", () => {
    (rustCompiler.compileToIR as any).mockReturnValue({
      success: false,
      errors: [{ severity: "error", code: "E001", message: "unexpected token", line: 1, col: 10, end_line: 1, end_col: 15 }],
    });

    const result = validate("invalid @@@ omni code");

    expect(result.valid).toBe(false);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0].code).toBe("E001");
    expect(result.errors[0].severity).toBe("error");
    expect(result.errors[0].message).toBe("unexpected token");
    expect(result.errors[0].line).toBe(1);
    expect(result.errors[0].col).toBe(10);
  });
});
