// poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
// v3: Simplified 2-state cursor (pipe | and card highlight).
// No more container right-edge detection or container-level delete.
// Enter activates SlotCardNode editing, or creates new body line.
import { useEffect } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  KEY_DELETE_COMMAND,
  KEY_BACKSPACE_COMMAND,
  KEY_ENTER_COMMAND,
  KEY_TAB_COMMAND,
  COMMAND_PRIORITY_LOW,
  $getSelection,
  $isRangeSelection,
  $isTextNode,
  type RangeSelection,
  type LexicalNode,
} from "lexical";
import { $isCardNode, $createCardNode } from "../nodes/CardNode";
import { $isSlotCardNode, type SlotCardNode } from "../nodes/SlotCardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBodyLineNode, $createBodyLineNode } from "../nodes/BodyLineNode";

export function KeyboardPlugin(): null {
  const [editor] = useLexicalComposerContext();

  // ── Delete ──
  // v3: No container right-edge logic. Browser handles card deletion natively.
  // We only intercept to prevent Lexical from doing its own paragraph deletion.
  useEffect(() => {
    return editor.registerCommand(
      KEY_DELETE_COMMAND,
      (_event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // If the selection has a highlighted card (contenteditable="false" span),
        // the browser will handle deletion natively. Let it through.
        // Lexical's default Delete behavior would try to delete a character,
        // which fails on contenteditable="false" elements anyway.
        return false; // let browser handle
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Backspace ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_BACKSPACE_COMMAND,
      (_event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Backspace at very start of a container → no-op
        const anchor = selection.anchor;
        const anchorNode = anchor.getNode();
        if (anchor.offset === 0 && $isTextNode(anchorNode)) {
          const parent = anchorNode.getParent();
          if (parent !== null && $isContainerElementNode(parent)) {
            const firstChild = parent.getFirstChild();
            if (firstChild === anchorNode) {
              return true; // swallow
            }
          }
        }

        return false; // let browser handle
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Enter: activate SlotCardNode editing, or create new body line ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent | null) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Check if a SlotCardNode is highlighted (selected)
        const anchorNode = selection.anchor.getNode();
        const slotCard = findSlotCardAtCursor(anchorNode);
        if (slotCard !== null) {
          event?.preventDefault();
          // Activate editing on the slot card's DOM element
          const editorEl = document.querySelector('[contenteditable="true"]');
          const slotDom = editorEl?.querySelector(
            `[data-slot-pid="${slotCard.getPid()}"]`
          ) as HTMLElement | null;
          if (slotDom) {
            slotCard.activateEditing(editor, slotDom);
            return true;
          }
          // Fall through to body-line creation if DOM query failed
        }

        // Enter inside a body line or at the right edge of one → create new line below
        const bodyLine = findAncestorBodyLine(selection);
        if (bodyLine !== null) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          bodyLine.insertAfter(newLine);
          newLine.selectStart();
          return true;
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Tab: cycle operator in set container ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_TAB_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;
        const anchorNode = selection.anchor.getNode();

        const cardNode = findCardAtCursor(anchorNode, selection);
        if (cardNode !== null && $isCardNode(cardNode) && cardNode.getCardKind() === "operator") {
          const parent = cardNode.getParent();
          if (
            parent !== null &&
            $isContainerElementNode(parent) &&
            parent.getContainerKind() === "set"
          ) {
            event.preventDefault();
            const operators = ["=", "+=", "-=", "*=", "/=", "%="];
            const current = operators.indexOf(cardNode.getCardLabel());
            const next = (current + 1) % operators.length;
            const newNode = $createCardNode(
              "operator",
              operators[next],
              cardNode.getCardPid(),
            );
            cardNode.replace(newNode);
            newNode.selectNext();
            return true;
          }
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  return null;
}

// ── Helpers ──

function findAncestorBodyLine(selection: RangeSelection): LexicalNode | null {
  const anchorNode = selection.anchor.getNode();
  let node: LexicalNode | null = anchorNode;
  while (node !== null) {
    if ($isBodyLineNode(node)) return node;
    node = node.getParent();
  }
  return null;
}

/**
 * Find a SlotCardNode at or near the cursor position.
 */
function findSlotCardAtCursor(anchorNode: LexicalNode): SlotCardNode | null {
  if ($isSlotCardNode(anchorNode)) return anchorNode;

  const parent = anchorNode.getParent();
  if (parent === null) return null;

  // Check if we're inside a SlotCardNode (it traps cursor in display mode)
  if ($isSlotCardNode(parent)) return parent;

  return null;
}

/**
 * Find a CardNode at or near the cursor position.
 */
function findCardAtCursor(
  anchorNode: LexicalNode,
  selection: RangeSelection,
): LexicalNode | null {
  if ($isCardNode(anchorNode)) return anchorNode;

  const parent = anchorNode.getParent();
  if (parent === null) return null;

  const offset = selection.anchor.offset;

  if (offset === 0) {
    const prev = anchorNode.getPreviousSibling();
    if ($isCardNode(prev)) return prev;
  }

  if (offset === anchorNode.getTextContentSize()) {
    const next = anchorNode.getNextSibling();
    if ($isCardNode(next)) return next;
  }

  if ($isCardNode(parent)) return parent;

  return null;
}
