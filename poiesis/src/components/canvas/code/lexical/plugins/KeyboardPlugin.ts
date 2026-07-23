// poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
import { useEffect } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  $getSelection,
  $isRangeSelection,
  $isTextNode,
  COMMAND_PRIORITY_LOW,
  KEY_TAB_COMMAND,
  KEY_BACKSPACE_COMMAND,
} from "lexical";
import { $isSlotTextNode } from "../nodes/SlotTextNode";
import type { SlotTextNode } from "../nodes/SlotTextNode";
import { $isCardElementNode } from "../nodes/CardElementNode";
import { $degradeCardToText } from "../transforms/card-to-text";

export function KeyboardPlugin(): null {
  const [editor] = useLexicalComposerContext();

  // ── Tab: jump to next slot ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_TAB_COMMAND,
      (event: KeyboardEvent) => {
        // Only handle if not in autocomplete mode
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        const node = selection.anchor.getNode();
        if ($isSlotTextNode(node) || $isTextNode(node)) {
          event.preventDefault();
          // Find next SlotTextNode in the current card or next card
          const current = node;
          let next = current.getNextSibling();
          while (next) {
            if ($isSlotTextNode(next)) {
              next.select(0, 0);
              return true;
            }
            next = next.getNextSibling();
          }
          // No more slots in this card — move to next sibling
          const parent = current.getParent();
          if (parent) {
            const nextSibling = parent.getNextSibling();
            if (nextSibling) {
              nextSibling.selectStart();
              return true;
            }
          }
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Backspace at empty slot boundary: degrade card to text ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_BACKSPACE_COMMAND,
      () => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection) || !selection.isCollapsed()) return false;

        const node = selection.anchor.getNode();
        const offset = selection.anchor.offset;

        // If cursor is at start of an empty slot, degrade parent card
        if (offset === 0 && $isSlotTextNode(node)) {
          const allSlotsEmpty = checkAllSlotsEmpty(node);
          if (allSlotsEmpty) {
            let cardParent = node.getParent();
            while (cardParent) {
              if ($isCardElementNode(cardParent)) {
                $degradeCardToText(cardParent);
                return true;
              }
              cardParent = cardParent.getParent();
            }
          }
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  return null;
}

// ── Delete at end of empty slot: also degrade card to text ──
function checkAllSlotsEmpty(node: SlotTextNode): boolean {
  const parent = node.getParent();
  if (!parent) return true;
  const children = parent.getChildren();
  for (const child of children) {
    if ($isSlotTextNode(child) && child.getTextContent() !== "") {
      return false;
    }
  }
  return true;
}
