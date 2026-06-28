// editor/client/src/omni-gen/omni-parser.ts
// Thin facade: tokenizes an OmniLang source string and runs the recursive-descent parser.

import { tokenize } from "./omni-tokenizer";
import { TokenStream } from "./token-stream";
import { Parser } from "./parser";
import type { ParseResult } from "./types";

// ── Public API ──

export function parseOmniSource(source: string): ParseResult {
  const tokens = tokenize(source);
  const stream = new TokenStream(tokens);
  const parser = new Parser(stream);
  return parser.parse();
}

// ── Re-exports for convenience ──

export type { ParserState, ParseContext, ViewElementAttrs } from "./parser-types";
export type { ParseContext as _ParseContext } from "./parser-types";
export { Parser } from "./parser";
