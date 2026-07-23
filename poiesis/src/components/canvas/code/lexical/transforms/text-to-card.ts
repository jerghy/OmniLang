// poiesis/src/components/canvas/code/lexical/transforms/text-to-card.ts
import { $createTextNode, $createLineBreakNode } from "lexical";
import { $createCardElementNode, type CardElementNode } from "../nodes/CardElementNode";
import { $createSlotTextNode, type SlotType } from "../nodes/SlotTextNode";
import type { CardKind } from "../nodes/CardElementNode";

/**
 * Per-card-kind factory that builds the CardElementNode's internal child structure.
 * Each function receives the card node and appends TextNode / SlotTextNode children.
 */
const CARD_BUILDERS: Record<CardKind, (card: CardElementNode) => void> = {
  declare_var: (card) => {
    // let name : Type = value
    card.append(
      $createTextNode("let"),
      $createTextNode(" "),
      $createSlotTextNode("", "name"),
      $createTextNode(" : "),
      $createSlotTextNode("", "type"),
      $createTextNode(" = "),
      $createSlotTextNode("", "value"),
    );
  },

  set_var: (card) => {
    // name operator value
    card.append(
      $createSlotTextNode("", "name"),
      $createTextNode(" = "),
      $createSlotTextNode("", "value"),
    );
  },

  if_block: (card) => {
    // if condition {
    card.append(
      $createTextNode("if"),
      $createTextNode(" "),
      $createSlotTextNode("", "condition"),
      $createTextNode(" {"),
      $createLineBreakNode(),
      $createTextNode("}"),
    );
  },

  while: (card) => {
    card.append(
      $createTextNode("while"),
      $createTextNode(" "),
      $createSlotTextNode("", "condition"),
      $createTextNode(" {"),
      $createLineBreakNode(),
      $createTextNode("}"),
    );
  },

  for_in: (card) => {
    card.append(
      $createTextNode("for"),
      $createTextNode(" "),
      $createSlotTextNode("", "item"),
      $createTextNode(" in "),
      $createSlotTextNode("", "collection"),
      $createTextNode(" {"),
      $createLineBreakNode(),
      $createTextNode("}"),
    );
  },

  return: (card) => {
    card.append(
      $createTextNode("return"),
      $createTextNode(" "),
      $createSlotTextNode("", "value"),
    );
  },

  call_fn: (card) => {
    card.append(
      $createSlotTextNode("", "fnName"),
      $createTextNode("("),
      $createSlotTextNode("", "arg"),
      $createTextNode(")"),
    );
  },

  await: (card) => {
    card.append(
      $createTextNode("await"),
      $createTextNode(" "),
      $createSlotTextNode("", "value"),
    );
  },

  break: (card) => {
    card.append($createTextNode("break"));
  },

  continue: (card) => {
    card.append($createTextNode("continue"));
  },

  comment: (card) => {
    card.append(
      $createTextNode("//"),
      $createTextNode(" "),
      $createSlotTextNode("", "value"),
    );
  },

  raw: (card) => {
    card.append($createSlotTextNode("", "value"));
  },
};

/** Create a new CardElementNode with pre-populated child nodes for the given kind. */
export function $transformTextToCard(
  cardKind: CardKind,
  _text: string,
  pid: string,
): CardElementNode {
  const card = $createCardElementNode(cardKind, pid);
  const builder = CARD_BUILDERS[cardKind];
  if (builder) {
    builder(card);
  }
  return card;
}
