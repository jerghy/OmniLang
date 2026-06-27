// editor/client/src/omni-gen/parser-types.ts
// Shared types and utilities for the OmniLang recursive-descent parser.

import type { ElementData, WireData } from "../../store/editor";
import type { CodeTab } from "../../codegraph/types";
import type { ParseResult } from "./types";
import type { Token } from "./omni-tokenizer";

// ── Parser state ──

export type ParserState =
  | "ROOT"
  | "IN_COMPONENT"
  | "IN_STORE"
  | "IN_VIEW"
  | "IN_CHAIN"
  | "IN_FN"
  | "IN_STYLE";

// ── Parse context ──

export interface ParseContext {
  state: ParserState;
  stateStack: ParserState[];
  braceDepth: number;
  braceDepthStack: number[];
  elements: ElementData[];
  wires: WireData[];
  codeTabs: CodeTab[];
  imports: string[];
  errors: ParseResult["errors"];
  codeTabType: "logic" | "event";
  currentComponent: string;
  currentStore: string;
  chainName: string;
  chainElements: ElementData[];
  chainWires: WireData[];
  viewParentStack: string[];
  pendingEditFlags: string | null;
  parsedStyleRules: Array<{ class_name: string; properties: Record<string, string> }>;
  hasStyleBlock: boolean;
  props: Array<{ name: string; type: string; default?: unknown }>;
  events: Array<{ name: string }>;
  componentMeta: Record<string, unknown> | null;
}

// ── Parsed attribute shape ──

export interface ViewElementAttrs {
  id: string | null;
  className: string;
  attrs: Record<string, string>;
  events: Record<string, string>;
  styleStr: string;
  isSelfClosing: boolean;
}

// ── Utility functions ──

/**
 * Parse key=value pairs from a string.
 * Supports: id=abc123, category=Custom, children=true, description="quoted string"
 */
export function parseKeyValueAttrs(input: string): Record<string, string> {
  const result: Record<string, string> = {};
  const regex = /(\w+)=(?:"([^"]*)"|(\S+))/g;
  let match;
  while ((match = regex.exec(input)) !== null) {
    result[match[1]] = match[2] !== undefined ? match[2] : match[3];
  }
  return result;
}

/**
 * Check if a token looks like the start of a next attribute binding
 * (attribute + EQUALS or COLON on the very next token).
 */
export function isAttrBound(next: Token): boolean {
  return next?.kind === "EQUALS" || next?.kind === "COLON";
}
