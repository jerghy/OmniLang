// Stub: WASM module not built yet.
// This module is only imported inside a try/catch block in code-op-parser.ts
// and falls back to the JS regex parser when unavailable.
export function parse_code_block(_source: string): string {
  throw new Error("WASM module not built — run 'npm run build:wasm'");
}
