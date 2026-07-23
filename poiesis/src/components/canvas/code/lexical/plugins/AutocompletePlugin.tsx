// poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx
import { useCallback, useEffect, useRef, useState } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  $getSelection,
  $isRangeSelection,
  $isTextNode,
  $createTextNode,
  COMMAND_PRIORITY_LOW,
  KEY_ENTER_COMMAND,
  KEY_ESCAPE_COMMAND,
  KEY_ARROW_DOWN_COMMAND,
  KEY_ARROW_UP_COMMAND,
} from "lexical";
import { AutocompleteMenu } from "../autocomplete/AutocompleteMenu";
import { $isSlotTextNode, type SlotType } from "../nodes/SlotTextNode";
import { $isCardElementNode, type CardKind } from "../nodes/CardElementNode";
import { getAutocompleteItems, detectCardKindFromItem } from "../autocomplete/autocomplete-rules";
import type { AutocompleteItem } from "../autocomplete/autocomplete-rules";
import { uid } from "../../../../../core/utils/id";
import { $transformTextToCard } from "../transforms/text-to-card";

export function AutocompletePlugin(): React.ReactNode {
  const [editor] = useLexicalComposerContext();
  const [show, setShow] = useState(false);
  const [items, setItems] = useState<AutocompleteItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [menuPos, setMenuPos] = useState({ top: 0, left: 0 });
  const lastTokenRef = useRef<string>("");
  const contextRef = useRef<{ parentKind: CardKind | null; slotType: SlotType | null }>({
    parentKind: null,
    slotType: null,
  });

  const close = useCallback(() => {
    setShow(false);
    setSelectedIndex(0);
  }, []);

  // ── Detect token at cursor and trigger autocomplete ──
  const detectAndShow = useCallback(() => {
    editor.getEditorState().read(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection) || !selection.isCollapsed()) {
        return;
      }

      const anchor = selection.anchor;
      const node = anchor.getNode();

      // Determine context: are we in a slot or in a paragraph?
      let slotType: SlotType | null = null;
      let parentKind: CardKind | null = null;

      if ($isSlotTextNode(node)) {
        slotType = (node as any).__slot;
        // Walk up to find parent card
        let parent = node.getParent();
        while (parent) {
          if ($isCardElementNode(parent)) {
            parentKind = parent.__cardKind;
            break;
          }
          parent = parent.getParent();
        }
      } else if ($isTextNode(node)) {
        let parent = node.getParent();
        while (parent) {
          if ($isCardElementNode(parent)) {
            parentKind = parent.__cardKind;
            break;
          }
          parent = parent.getParent();
        }
      }

      // Extract token: text from start of current word to cursor
      const textContent = node.getTextContent();
      const offset = anchor.offset;
      // Find the start of the current token (walk back to space/punctuation)
      let tokenStart = offset;
      while (tokenStart > 0 && /[\w.]/.test(textContent[tokenStart - 1])) {
        tokenStart--;
      }
      const token = textContent.slice(tokenStart, offset);

      contextRef.current = { parentKind, slotType };

      const candidates = getAutocompleteItems(token, { parentKind, slotType });
      if (candidates.length > 0) {
        lastTokenRef.current = token;
        setItems(candidates);
        setSelectedIndex(0);
        setShow(true);
        // Position: approximate — below current line
        // In production, use getBoundingClientRect on the editor + caret position
        setMenuPos({
          top: 200 + (anchor.offset || 0),
          left: 60,
        });
      } else {
        setShow(false);
      }
    });
  }, [editor]);

  // ── Execute autocomplete selection ──
  const executeSelection = useCallback(
    (item: AutocompleteItem) => {
      close();
      const cardKind = detectCardKindFromItem(item);
      if (!cardKind) return;

      editor.update(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return;

        const anchor = selection.anchor;
        const node = anchor.getNode();
        const token = lastTokenRef.current;

        // Remove the typed token from the TextNode
        if ($isTextNode(node) || $isSlotTextNode(node)) {
          const textContent = node.getTextContent();
          const offset = anchor.offset;
          let tokenStart = offset;
          while (tokenStart > 0 && /[\w.]/.test(textContent[tokenStart - 1])) {
            tokenStart--;
          }
          const before = textContent.slice(0, tokenStart);
          const after = textContent.slice(offset);
          (node as any).setTextContent(before + after);
        }

        // Create the card
        const pid = uid("op");
        const card = $transformTextToCard(cardKind, token, pid);

        // Handle declare_var_mut: set mutable
        if (item.kind === "declare_var_mut") {
          card.__cardKind = "declare_var";
          // Insert "mut" text node after "let "
          const firstChild = card.getFirstChild();
          if (firstChild) {
            // Find "let " and insert "mut " after it
            let current = firstChild;
            while (current) {
              if (current.getTextContent() === "let") {
                const next = current.getNextSibling();
                if (next) {
                  next.insertBefore($createTextNode("mut "));
                }
                break;
              }
              const nextSibling = current.getNextSibling();
              if (!nextSibling) break;
              current = nextSibling;
            }
          }
        }

        // Insert the card at the current selection
        selection.insertNodes([card]);

        // Move cursor to the first slot in the card
        const firstSlot = findFirstSlot(card);
        if (firstSlot) {
          firstSlot.select(0, 0);
        }
      });
    },
    [editor, close],
  );

  // ── Keyboard command handlers ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent) => {
        if (show && items.length > 0) {
          event.preventDefault();
          event.stopPropagation();
          executeSelection(items[selectedIndex]);
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, show, items, selectedIndex, executeSelection]);

  useEffect(() => {
    return editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      () => {
        if (show) {
          close();
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, show, close]);

  useEffect(() => {
    return editor.registerCommand(
      KEY_ARROW_DOWN_COMMAND,
      (event: KeyboardEvent) => {
        if (show) {
          event.preventDefault();
          setSelectedIndex(i => Math.min(i + 1, items.length - 1));
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, show, items]);

  useEffect(() => {
    return editor.registerCommand(
      KEY_ARROW_UP_COMMAND,
      (event: KeyboardEvent) => {
        if (show) {
          event.preventDefault();
          setSelectedIndex(i => Math.max(i - 1, 0));
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, show]);

  // ── Register text-change listener ──
  useEffect(() => {
    return editor.registerTextContentListener(() => {
      detectAndShow();
    });
  }, [editor, detectAndShow]);

  if (!show) return null;

  return (
    <AutocompleteMenu
      items={items}
      selectedIndex={selectedIndex}
      onSelect={executeSelection}
      onClose={close}
      position={menuPos}
    />
  );
}

function findFirstSlot(card: any): any | null {
  const children = card.getChildren();
  for (const child of children) {
    if ($isSlotTextNode(child)) return child;
  }
  return null;
}
