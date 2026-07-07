// poiesis/src/services/codegen/parser-finalize.ts
// Finalization (post-processing) functions extracted from the Parser class.

import type { ParserInternals } from "./parser";
import type { ElementData } from "../../core/types/element";
import type { ParseResult } from "./types";
import { parseStylesFromAST } from "./style-parser";
import { flushChain } from "./parser-helpers";

// ── Finalization (post-processing) ──

export function finalizeParse(internals: ParserInternals): void {
  flushChain(internals.ctx);

  // Parse styles from captured AST (style blocks + view tree).
  // Same logic as the old parser.
  if (internals.ctx.elements.length > 0) {
    type ViewTreeNode = {
      id: string;
      props: Record<string, unknown>;
      inline_style: Record<string, string> | null;
      children: ViewTreeNode[];
    };
    function buildViewTreeFromElements(
      el: ElementData,
      elements: ElementData[],
    ): ViewTreeNode {
      // Only view elements have the structure we need for tree building
      if (el.nodeType !== "view") {
        return { id: el.id, props: {}, inline_style: null, children: [] };
      }
      let inline_style: Record<string, string> | null = null;
      const styleStr = el.props.style as string | undefined;
      if (styleStr && typeof styleStr === "string") {
        const cleaned = styleStr
          .replace(/^\s*\{\s*/, "")
          .replace(/\s*\}\s*$/, "");
        if (cleaned.length > 0) {
          const pairs: Record<string, string> = {};
          for (const part of cleaned.split(";")) {
            const t = part.trim();
            if (t.length === 0) continue;
            const colonIdx = t.indexOf(":");
            if (colonIdx > 0) {
              pairs[t.slice(0, colonIdx).trim()] = t.slice(colonIdx + 1).trim();
            }
          }
          if (Object.keys(pairs).length > 0) {
            inline_style = pairs;
          }
        }
      }

      return {
        id: el.id,
        props: { ...el.props, class: (el.props.className as string) || "" },
        inline_style,
        children: (el.children || [])
          .map((cid: string) =>
            elements.find((e: ElementData) => e.id === cid),
          )
          .filter((c: ElementData | undefined): c is ElementData => c !== undefined)
          .map((c: ElementData) => buildViewTreeFromElements(c, elements)),
      };
    }

    const rootElements = internals.ctx.elements.filter(
      (e: ElementData) => e.nodeType === "view" && !e.parentId,
    );
    if (rootElements.length > 0) {
      const elementMap = new Map(
        internals.ctx.elements.map((e: ElementData) => [e.id, e]),
      );
      const componentNode = {
        name: internals.ctx.currentComponent || undefined,
        style: internals.ctx.hasStyleBlock
          ? internals.ctx.parsedStyleRules
          : undefined,
        view: buildViewTreeFromElements(rootElements[0], internals.ctx.elements),
      };
      parseStylesFromAST(componentNode as Parameters<typeof parseStylesFromAST>[0], elementMap);
    }
  }
}

// ── Build result ──

export function buildParseResult(ctx: ParserInternals["ctx"]): ParseResult {
  return {
    designElements:
      ctx.elements.length > 0 ? ctx.elements : null,
    designWires: ctx.wires,
    codeTabs: ctx.codeTabs,
    imports: ctx.imports,
    errors: ctx.errors,
    componentMeta: ctx.componentMeta,
    props: ctx.props.length > 0 ? ctx.props : undefined,
    events: ctx.events.length > 0 ? ctx.events : undefined,
    designName: ctx.designName || undefined,
    designTokens: ctx.designTokens.length > 0 ? ctx.designTokens : undefined,
    designClasses: ctx.designClasses.length > 0 ? ctx.designClasses : undefined,
    designVariants: ctx.designVariants.length > 0 ? ctx.designVariants : undefined,
  };
}
