// poiesis/src/components/canvas/code/lexical/transforms/card-to-text.ts
import { $createParagraphNode, $createTextNode } from "lexical";
import type { CardElementNode } from "../nodes/CardElementNode";

/**
 * Replace a CardElementNode with a plain ParagraphNode containing the card's
 * text content. Used when user deletes all slots in a card (Backspace at boundary).
 */
export function $degradeCardToText(cardNode: CardElementNode): void {
  const textContent = cardNode.getTextContent();
  const paragraph = $createParagraphNode();
  paragraph.append($createTextNode(textContent));
  cardNode.replace(paragraph);
}
