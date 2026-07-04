// editor/client/src/services/codegen/parser-helpers.ts
// Pure utility functions extracted from the Parser class.

import type { Token } from "./omni-tokenizer";
import type { TokenStream } from "./token-stream";
import type { ParseContext, ViewElementAttrs } from "./parser-types";
import type { ParserInternals } from "./parser";
import type { ElementData } from "../../core/types/element";
import { useComponentRegistry } from "../../store/componentRegistry";
import { Logger } from "../../lib/utils/logger";

const log = new Logger({ prefix: "omni-parser" });

// ── Stream helpers ──

export function skipUntilSemicolon(stream: TokenStream): void {
  while (
    !stream.isEof() &&
    stream.peek().kind !== "SEMICOLON" &&
    stream.peek().kind !== "NEWLINE"
  ) {
    stream.consume();
  }
}

// ── Error helpers ──

export function pushError(
  ctx: ParseContext,
  severity: string,
  code: string,
  message: string,
  tok: Token,
): void {
  ctx.errors.push({
    severity,
    code,
    message,
    line: tok.line,
    col: tok.col,
    end_line: tok.line,
    end_col: tok.col + tok.text.length,
  });
}

// ── Element creation ──

export function createElement(
  internals: ParserInternals,
  id: string,
  tag: string,
  attrs: ViewElementAttrs,
  indent: number,
): ElementData {
  const { ctx, indentMap } = internals;

  // Auto-rename duplicate IDs
  let finalId = id;
  if (ctx.elements.some((e) => e.id === finalId)) {
    log.warn(`Duplicate element ID "${finalId}"; assigning unique ID`);
    let counter = 1;
    let newId = `${finalId}__dup${counter}`;
    while (ctx.elements.some((e) => e.id === newId)) {
      counter++;
      newId = `${finalId}__dup${counter}`;
    }
    finalId = newId;
  }

  // Parent inference by indent (same as old parser)
  while (ctx.viewParentStack.length > 0) {
    const parentId =
      ctx.viewParentStack[ctx.viewParentStack.length - 1];
    const parent = ctx.elements.find((e) => e.id === parentId);
    if (
      parent &&
      getElementIndent(indentMap, parent) !== undefined &&
      getElementIndent(indentMap, parent)! >= indent
    ) {
      ctx.viewParentStack.pop();
    } else {
      break;
    }
  }

  const parentId =
    ctx.viewParentStack.length > 0
      ? ctx.viewParentStack[ctx.viewParentStack.length - 1]
      : null;

  // Detect custom component references
  const isPascal = /^[A-Z]/.test(tag);
  const isCustom =
    isPascal && !!useComponentRegistry.getState().definitions[tag];

  const el: ElementData = {
    id: finalId,
    nodeType: "view",
    tag,
    props: {
      ...attrs.attrs,
      ...attrs.events,
      ...(attrs.className ? { className: attrs.className } : {}),
    },
    children: [],
    parentId,
    ...(isCustom ? { componentRef: tag } : {}),
  };

  // Set parent/child relationships
  if (parentId) {
    const parent = ctx.elements.find((e) => e.id === parentId);
    if (parent) parent.children.push(finalId);
  }

  // Store indent on the element (for parent inference)
  setElementIndent(indentMap, el, indent);

  // Handle inline style if present
  if (attrs.styleStr) {
    el.props.style = attrs.styleStr;
  }

  return el;
}

// ── Element indent tracking ──

export function getElementIndent(
  map: Map<string, number>,
  el: ElementData,
): number | undefined {
  return map.get(el.id);
}

export function setElementIndent(
  map: Map<string, number>,
  el: ElementData,
  indent: number,
): void {
  map.set(el.id, indent);
}

// ── Chain flush ──

export function flushChain(ctx: ParseContext): void {
  if (ctx.chainElements.length === 0) return;
  const tab =
    ctx.codeTabs.find((t) => t.id === "main") || ctx.codeTabs[0];
  for (const el of ctx.chainElements) tab.graph.elements.push(el);
  for (const w of ctx.chainWires) tab.graph.wires.push(w);
  ctx.chainElements = [];
  ctx.chainWires = [];
}

// ── Codeblock element factory ──

export function codeblockEl(
  id: string,
  tag: string,
  props: Record<string, unknown>,
): ElementData {
  return {
    id,
    nodeType: "codeblock",
    tag,
    props: props as Record<string, unknown>,
    children: [],
    parentId: null,
  };
}
