// poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
// v2 cursor keyboard: browser handles ←→↑↓ natively via contenteditable="false" cards.
// We handle: Delete/Backspace at container right-edge, Enter for new body lines, Tab for operator cycling.
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
  $isElementNode,
  type RangeSelection,
  type LexicalNode,
} from "lexical";
import { $isCardNode, $createCardNode } from "../nodes/CardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBlockElementNode } from "../nodes/BlockNode";
import { $isBodyLineNode, $createBodyLineNode } from "../nodes/BodyLineNode";

export function KeyboardPlugin(): null {
  const [editor] = useLexicalComposerContext();

  // ── Delete ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_DELETE_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        const rightEdgeNode = detectRightEdge(selection);
        if (rightEdgeNode !== null) {
          event.preventDefault();
          if ($isContainerElementNode(rightEdgeNode)) {
            // Container right-edge block cursor → delete whole container
            rightEdgeNode.remove();
            return true;
          }
          if ($isBodyLineNode(rightEdgeNode)) {
            // Body line right-edge → delete this line (keep at least one child)
            const parent = rightEdgeNode.getParent();
            if (parent !== null) {
              const siblingCount = parent.getChildren().filter(
                (c) => $isBodyLineNode(c),
              ).length;
              if (siblingCount > 1) {
                rightEdgeNode.remove();
              }
            }
            return true;
          }
        }

        // If a card is selected (browser's atomic selection via contenteditable="false"),
        // the browser will handle deletion natively. Return false to let it through.
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Backspace ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_BACKSPACE_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Backspace at very start of a container → no-op (cannot delete container prefix)
        const anchor = selection.anchor;
        const anchorNode = anchor.getNode();
        if (anchor.offset === 0 && $isTextNode(anchorNode)) {
          const parent = anchorNode.getParent();
          if (parent !== null && $isContainerElementNode(parent)) {
            const firstChild = parent.getFirstChild();
            if (firstChild === anchorNode) {
              event.preventDefault();
              return true; // swallow backspace at container start
            }
          }
        }

        // Otherwise let browser handle (card deletion, text deletion, etc.)
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Enter: new body line ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent | null) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Enter at body line right-edge or inside body line → insert new line below
        const withinBodyLine = findAncestorBodyLine(selection);
        if (withinBodyLine !== null) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          withinBodyLine.insertAfter(newLine);
          newLine.selectStart();
          return true;
        }

        // Enter after a container (right-edge) → insert paragraph after
        const rightEdge = detectRightEdge(selection);
        if (rightEdge !== null && $isContainerElementNode(rightEdge)) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          rightEdge.insertAfter(newLine);
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

        // Check if the node at cursor or its neighboring sibling is an operator card
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

/**
 * Detect if the collapsed cursor is at the rightmost edge of a container or body line.
 * Returns the container/body-line node if so, or null.
 */
function detectRightEdge(
  selection: RangeSelection,
): LexicalNode | null {
  if (!selection.isCollapsed()) return null;

  const anchor = selection.anchor;
  const anchorNode = anchor.getNode();

  // Cursor must be at the very end of the anchor node
  if (anchor.offset !== anchorNode.getTextContentSize()) return null;

  // Walk up: is this node the last child of a container or body line?
  let node: LexicalNode | null = anchorNode;
  while (node !== null) {
    const parent: LexicalNode | null = node.getParent();
    if (parent === null) return null;
    if (!$isElementNode(parent)) return null;

    const lastChild = parent.getLastChild();
    if (lastChild !== node) return null; // not the last child → not at right edge

    if ($isContainerElementNode(parent)) return parent;
    if ($isBodyLineNode(parent)) return parent;
    if ($isBlockElementNode(parent)) return null; // don't delete block from inside

    node = parent;
  }
  return null;
}

/**
 * Find the closest ancestor BodyLineNode that contains the selection.
 */
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
 * Find a CardNode at or near the cursor position.
 * Since cards are contenteditable="false", the cursor may be at a text node
 * adjacent to the card rather than inside it.
 */
function findCardAtCursor(
  anchorNode: LexicalNode,
  selection: RangeSelection,
): LexicalNode | null {
  // Direct hit: cursor is somehow on a card node
  if ($isCardNode(anchorNode)) return anchorNode;

  const parent = anchorNode.getParent();
  if (parent === null) return null;

  const offset = selection.anchor.offset;

  // Cursor at start of text node → check previous sibling
  if (offset === 0) {
    const prev = anchorNode.getPreviousSibling();
    if ($isCardNode(prev)) return prev;
  }

  // Cursor at end of text node → check next sibling
  if (offset === anchorNode.getTextContentSize()) {
    const next = anchorNode.getNextSibling();
    if ($isCardNode(next)) return next;
  }

  // Check if parent itself is a card node
  if ($isCardNode(parent)) return parent;

  return null;
}
