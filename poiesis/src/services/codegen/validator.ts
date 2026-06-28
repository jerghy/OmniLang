// editor/client/src/omni-gen/validator.ts
import { rustCompiler } from "../../commands/rustCompiler";
import type { ValidationResult } from "./types";

export function validate(source: string): ValidationResult {
  const result = rustCompiler.compileToIR(source);
  return {
    valid: result.success,
    errors: (result.errors || []) as ValidationResult["errors"],
  };
}
