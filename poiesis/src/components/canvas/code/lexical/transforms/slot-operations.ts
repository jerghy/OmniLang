// poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts
// \ (backslash) input slot inference and creation.
// Extracted from AutocompletePlugin for testability and separation of concerns.
import {
  $isRootNode,
  $isElementNode,
  type RangeSelection,
  type LexicalNode,
} from "lexical";
import type { SlotKind } from "../../../../../core/types/editor-node";
import { $isCardNode } from "../nodes/CardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBlockElementNode } from "../nodes/BlockNode";
import { $createInputSlotNode, type InputSlotNode } from "../nodes/InputSlotNode";

export interface CursorContext {
  parentKind: string | null;
  precedingCardKind: string | null;
  precedingCardLabel: string | null;
  grandparentKind: string | null;
  isTopLevel: boolean;
}

/**
 * Build cursor context from the current range selection.
 * Must be called inside editor.update() or editorState.read().
 */
export function buildCursorContext(selection: RangeSelection): CursorContext {
  const anchor = selection.anchor;
  const anchorNode = anchor.getNode();
  const offset = anchor.offset;

  const parent = anchorNode.getParent();
  const grandparent = parent?.getParent() ?? null;

  const parentKind = getKindName(parent);
  const grandparentKind = getKindName(grandparent);

  // Determine if we're at the top level (root or paragraph under root)
  const isTopLevel =
    parent === null ||
    $isRootNode(parent) ||
    ($isRootNode(grandparent) && parent?.getType() === "paragraph");

  // Find the preceding card (card before the cursor)
  let precedingCardKind: string | null = null;
  let precedingCardLabel: string | null = null;

  if (offset === 0 && $isCardNode(anchorNode.getPreviousSibling())) {
    const prev = anchorNode.getPreviousSibling()!;
    precedingCardKind = prev.getType() === "inline-card"
      ? (prev as any).getCardKind() : null;
    precedingCardLabel = prev.getType() === "inline-card"
      ? (prev as any).getCardLabel() : null;
  } else {
    // Walk backwards through siblings to find the nearest card
    let sibling = anchorNode.getPreviousSibling();
    while (sibling !== null) {
      if ($isCardNode(sibling)) {
        precedingCardKind = sibling.getCardKind();
        precedingCardLabel = sibling.getCardLabel();
        break;
      }
      sibling = sibling.getPreviousSibling();
    }
  }

  return {
    parentKind,
    precedingCardKind,
    precedingCardLabel,
    grandparentKind,
    isTopLevel,
  };
}

/**
 * Determine the slot kind based on cursor context.
 * Returns null if no slot should be created at this position.
 */
export function inferSlotKind(ctx: CursorContext): SlotKind | null {
  // let / set at start -> variable name
  if ((ctx.parentKind === "let" || ctx.parentKind === "set") &&
      ctx.precedingCardKind === null) {
    return "name";
  }

  // for at start -> item variable name
  if (ctx.parentKind === "for" && ctx.precedingCardKind === null) {
    return "name";
  }

  // After ':' -> type annotation
  if (ctx.precedingCardLabel === ":") {
    return "type_value";
  }

  // After 'in' -> collection expression
  if (ctx.precedingCardLabel === "in") {
    return "expression";
  }

  // After assignment operator -> value expression
  if (ctx.precedingCardLabel === "=" ||
      /^[+\-*/%]=$/.test(ctx.precedingCardLabel ?? "")) {
    return "expression";
  }

  // if / while head -> condition
  if (ctx.parentKind === "if" || ctx.parentKind === "while") {
    return "condition";
  }

  // call at start -> function name
  if (ctx.parentKind === "call" && ctx.precedingCardKind === null) {
    return "fn_name";
  }

  // return / await -> expression
  if (ctx.parentKind === "return" || ctx.parentKind === "await") {
    return "expression";
  }

  // Inside a body line (grandparent is a block) -> expression
  if (ctx.grandparentKind !== null && ctx.parentKind === null) {
    return "expression";
  }

  return null;
}

/**
 * Check if a backslash slot can be created at the current cursor position.
 */
export function canCreateSlot(ctx: CursorContext): boolean {
  if (ctx.isTopLevel) return false;
  return inferSlotKind(ctx) !== null;
}

/**
 * Create an InputSlotNode at the cursor position.
 * Returns the created node or null if slot cannot be created.
 * Must be called inside editor.update().
 */
export function createSlotAtCursor(
  ctx: CursorContext,
  parentNode: LexicalNode | null,
): InputSlotNode | null {
  const slotKind = inferSlotKind(ctx);
  if (!slotKind || parentNode === null || !$isElementNode(parentNode)) return null;

  const slot = $createInputSlotNode("", slotKind);
  parentNode.append(slot);
  return slot;
}

// -- Private helpers --

function getKindName(node: LexicalNode | null): string | null {
  if (node === null) return null;
  if ($isContainerElementNode(node)) return node.getContainerKind();
  if ($isBlockElementNode(node)) return node.getBlockKind();
  return null;
}
