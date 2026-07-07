// poiesis/src/services/codegen/validator.ts
import { rustCompiler } from "../../commands/rustCompiler";
import type { ValidationResult } from "./types";

export function validate(source: string): ValidationResult {
  try {
    const result = rustCompiler.compileToIR(source);
    return {
      valid: result.success,
      errors: (result.errors || []) as ValidationResult["errors"],
    };
  } catch (e) {
    return {
      valid: false,
      errors: [{
        severity: "error",
        code: "E_VALIDATION_CRASH",
        message: e instanceof Error ? e.message : String(e),
        line: 0,
        col: 0,
        end_line: 0,
        end_col: 0,
      }],
    };
  }
}
