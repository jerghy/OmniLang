// editor/client/src/omni-gen/token-stream.ts
// TokenStream — a cursor over a Token[] array providing lookahead, consume, match, and expect.

import type { Token, TokenKind } from "./omni-tokenizer";

export class TokenStream {
  private tokens: Token[];
  private pos: number;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
    this.pos = 0;
  }

  /** Return the current position (for debugging / snapshots). */
  get position(): number {
    return this.pos;
  }

  /** Peek at the current token, or at an offset from it. */
  peek(offset: number = 0): Token {
    const idx = this.pos + offset;
    if (idx >= this.tokens.length) {
      return this.tokens[this.tokens.length - 1]; // EOF
    }
    return this.tokens[idx];
  }

  /** Consume and return the current token, advancing the cursor. */
  consume(): Token {
    const tok = this.tokens[this.pos];
    if (this.pos < this.tokens.length) {
      this.pos++;
    }
    return tok;
  }

  /** If the current token matches `kind`, consume and return it; otherwise return null. */
  match(kind: TokenKind): Token | null {
    if (this.pos < this.tokens.length && this.tokens[this.pos].kind === kind) {
      return this.consume();
    }
    return null;
  }

  /**
   * Expect the current token to have `kind`.
   * Returns the token if it matches.  If it doesn't match, returns a
   * synthetic token (kind === "EOF") and the caller should record an error.
   * (We don't throw — the parser accumulates errors and continues.)
   */
  expect(kind: TokenKind, message?: string): Token {
    if (this.pos < this.tokens.length && this.tokens[this.pos].kind === kind) {
      return this.consume();
    }
    // Return a synthetic EOF so the caller can detect mismatch
    return {
      kind: "EOF" as TokenKind,
      text: message || `Expected ${kind}`,
      line: this.tokens[this.pos]?.line ?? 0,
      col: this.tokens[this.pos]?.col ?? 0,
    };
  }

  /** Check if the cursor is at (or past) the EOF token. */
  isEof(): boolean {
    return this.pos >= this.tokens.length || this.tokens[this.pos].kind === "EOF";
  }

  /** Skip any consecutive NEWLINE tokens. */
  skipNewlines(): void {
    while (this.pos < this.tokens.length && this.tokens[this.pos].kind === "NEWLINE") {
      this.pos++;
    }
  }

  /**
   * Return the column of the first token on the current "content line" —
   * i.e. after skipping any NEWLINE tokens.  If a NEWLINE was just skipped,
   * the next token's col is the indent of the new line.
   */
  currentIndent(): number {
    // Look past NEWLINE tokens to find the first content token's col
    let idx = this.pos;
    while (idx < this.tokens.length && this.tokens[idx].kind === "NEWLINE") {
      idx++;
    }
    if (idx < this.tokens.length) {
      return this.tokens[idx].col;
    }
    return 1; // default indent
  }
}
