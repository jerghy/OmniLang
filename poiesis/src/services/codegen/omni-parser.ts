// poiesis/src/omni-gen/omni-parser.ts
// Thin facade: tokenizes an OmniLang source string and runs the recursive-descent parser.

import { tokenize } from "./omni-tokenizer";
import { TokenStream } from "./token-stream";
import { Parser } from "./parser";
import type { ParseResult } from "./types";
import type { PidMap, PidEntry } from "../../core/types/pid";

// ── Public API ──

export function parseOmniSource(source: string): ParseResult {
  const tokens = tokenize(source);
  const stream = new TokenStream(tokens);
  const parser = new Parser(stream);
  const result = parser.parse();

  // Parse // @pid:xxx comments and build a PidMap
  const pidMap = parsePidAnnotations(source);
  result.pidMap = pidMap;

  return result;
}

/**
 * Parse `// @pid:xxx` comments from raw .omni source text.
 * Returns a PidMap with the parsed PIDs.
 *
 * If no `@pid` comments are found, returns an empty PidMap.
 */
export function parsePidAnnotations(source: string): PidMap {
  const entries: PidEntry[] = [];
  const lines = source.split("\n");

  // Regex: optional whitespace, then // @pid:<pid>  — capture only the first non-whitespace token
  const pidRe = /^\s*\/\/\s*@pid:(\S+)/;

  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(pidRe);
    if (match) {
      const pid = match[1].trim();
      entries.push({
        pid,
        blockType: "view", // default — caller can refine
        blockName: "",
        sourceFile: "",
        startLine: i + 1,    // 1-based
        endLine: i + 1,
      });
    }
  }

  // Build PidMap from entries
  const pidMap: PidMap = { entries: {}, lineToPid: {} };
  for (const entry of entries) {
    pidMap.entries[entry.pid] = entry;
    for (let line = entry.startLine; line <= entry.endLine; line++) {
      if (pidMap.lineToPid[line] && pidMap.lineToPid[line] !== entry.pid) {
        console.warn(`[parser] Line ${line} has conflicting PID annotations: ${pidMap.lineToPid[line]} vs ${entry.pid}`);
      }
      pidMap.lineToPid[line] = entry.pid;
    }
  }

  return pidMap;
}

// ── Re-exports for convenience ──

export type { ParserState, ParseContext, ViewElementAttrs } from "./parser-types";
export type { ParseContext as _ParseContext } from "./parser-types";
export { Parser } from "./parser";
