// poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
// v2 cursor navigation: card highlight, pipe between cards, block at container edge
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
} from "lexical";
import { $isCardNode, $createCardNode } from "../nodes/CardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBlockElementNode } from "../nodes/BlockNode";
import { $isBodyLineNode, $createBodyLineNode } from "../nodes/BodyLineNode";

export function KeyboardPlugin(): null {
  const [editor] = useLexicalComposerContext();

  // ── Delete: remove inline card, container at right edge, or block ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_DELETE_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        const anchorNode = selection.anchor.getNode();
        const parent = anchorNode.getParent();

        // Delete inline card (card highlight state)
        if ($isCardNode(anchorNode)) {
          event.preventDefault();
          anchorNode.remove();
          return true;
        }

        // Delete container when cursor is positioned after it (pipe state)
        const prevSibling = anchorNode.getPreviousSibling();
        if ($isContainerElementNode(prevSibling)) {
          event.preventDefault();
          prevSibling.remove();
          return true;
        }

        // Delete block element or its parent block (block at edge state)
        const blockNode = $isBlockElementNode(anchorNode)
          ? anchorNode
          : parent !== null && $isBlockElementNode(parent)
            ? parent
            : null;
        if (blockNode !== null) {
          event.preventDefault();
          blockNode.remove();
          return true;
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Backspace: remove inline card ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_BACKSPACE_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;
        const anchorNode = selection.anchor.getNode();
        if ($isCardNode(anchorNode)) {
          event.preventDefault();
          anchorNode.remove();
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Enter: new body line inside body section ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent | null) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;
        const anchorNode = selection.anchor.getNode();

        // Enter inside a body line: insert new body line below
        if ($isBodyLineNode(anchorNode)) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          anchorNode.insertAfter(newLine);
          newLine.selectStart();
          return true;
        }
        // Cursor inside a body line's descendant (e.g. inside a card in a body line)
        const parent = anchorNode.getParent();
        if (parent !== null && $isBodyLineNode(parent)) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          parent.insertAfter(newLine);
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

        // If cursor is on an operator card inside a set container, cycle operator
        if ($isCardNode(anchorNode) && anchorNode.getCardKind() === "operator") {
          const parent = anchorNode.getParent();
          if (parent !== null && $isContainerElementNode(parent) && parent.getContainerKind() === "set") {
            event.preventDefault();
            const operators = ["=", "+=", "-=", "*=", "/=", "%="];
            const current = operators.indexOf(anchorNode.getCardLabel());
            const next = (current + 1) % operators.length;
            // Replace with new operator card
            const newNode = $createCardNode("operator", operators[next], anchorNode.getCardPid());
            anchorNode.replace(newNode);
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
