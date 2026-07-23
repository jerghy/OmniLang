// poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  $getSelection,
  $isRangeSelection,
  $isTextNode,
  KEY_ENTER_COMMAND,
  KEY_ESCAPE_COMMAND,
  KEY_ARROW_DOWN_COMMAND,
  KEY_ARROW_UP_COMMAND,
  COMMAND_PRIORITY_LOW,
  $getRoot,
  $createTextNode,
} from "lexical";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  getAutocompleteItems,
  detectCardKindFromItem,
  type AutocompleteItem,
  type AutocompleteContext,
} from "../autocomplete/autocomplete-rules";
import { $createCardNode } from "../nodes/CardNode";
import { $createContainerElementNode, $isContainerElementNode } from "../nodes/ContainerNode";
import { $createBlockElementNode, $isBlockElementNode } from "../nodes/BlockNode";
import { $createBodyLineNode } from "../nodes/BodyLineNode";
import { $createInputSlotNode } from "../nodes/InputSlotNode";
import type { SlotKind } from "../../../../../core/types/editor-node";
import { uid } from "../../../../../core/utils/id";
import { AutocompleteMenu } from "../autocomplete/AutocompleteMenu";

interface Props {
  /** Called when backslash key creates a new input slot */
  onCreateSlot?: () => void;
}

export function AutocompletePlugin({ onCreateSlot }: Props): React.ReactNode {
  const [editor] = useLexicalComposerContext();
  const [acState, setAcState] = useState<{
    visible: boolean;
    items: AutocompleteItem[];
    index: number;
  }>({ visible: false, items: [], index: 0 });
  const typingTokenRef = useRef("");

  // ── Menu position computed from cursor ──
  const [menuPos, setMenuPos] = useState({ top: 60, left: 60 });
  const updateMenuPosition = useCallback(() => {
    const sel = window.getSelection();
    if (sel && sel.rangeCount > 0) {
      const range = sel.getRangeAt(0);
      const rect = range.getBoundingClientRect();
      const editorEl = document.querySelector('[contenteditable="true"]');
      if (editorEl) {
        const editorRect = editorEl.getBoundingClientRect();
        setMenuPos({
          top: rect.bottom - editorRect.top + 4,
          left: rect.left - editorRect.left,
        });
        return;
      }
    }
    // Fallback
    setMenuPos({ top: 60, left: 60 });
  }, []);

  // ── Detect typing and trigger autocomplete ──
  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }: any) => {
      editorState.read(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          setAcState((s) => (s.visible ? { ...s, visible: false } : s));
          return;
        }

        const anchor = selection.anchor;
        const node = anchor.getNode();
        const text = node.getTextContent();

        // Extract current word token from cursor position
        const offset = anchor.offset;
        const beforeCursor = text.slice(0, offset);
        const tokenMatch = beforeCursor.match(/(\S+)$/);
        const token = tokenMatch ? tokenMatch[1] : "";

        // Determine context
        const parent = node.getParent();
        const context: AutocompleteContext = {
          parentKind: getParentContextKind(parent),
          precedingCardKind: getPrecedingCardKind(node, offset),
        };

        const items = getAutocompleteItems(token, context);
        if (items.length > 0) {
          setAcState({ visible: true, items, index: 0 });
          typingTokenRef.current = token;
          // Schedule cursor-relative menu position update
          requestAnimationFrame(updateMenuPosition);
        } else {
          setAcState((s) => (s.visible ? { ...s, visible: false } : s));
        }
      });
    });
  }, [editor]);

  // ── Backslash → insert input slot ──
  useEffect(() => {
    const removeRootListener = editor.registerRootListener(
      (rootElement: HTMLElement | null) => {
        if (!rootElement) return;
        const handler = (e: KeyboardEvent) => {
          if (e.key === "\\") {
            e.preventDefault();
            editor.update(() => {
              const selection = $getSelection();
              if (!$isRangeSelection(selection)) return;
              const node = selection.anchor.getNode();
              const parent = node.getParent();

              const parentKind = getParentContextKind(parent);
              const slotKind = inferSlotKind(parentKind);
              if (slotKind) {
                const slotNode = $createInputSlotNode("", slotKind as SlotKind);
                node.getParent()?.append(slotNode);
                slotNode.select();
                onCreateSlot?.();
              }
            });
          }
        };
        rootElement.addEventListener("keydown", handler);
        return () => rootElement.removeEventListener("keydown", handler);
      },
    );
    return () => removeRootListener?.();
  }, [editor, onCreateSlot]);

  const executeAutocomplete = useCallback(
    (item: AutocompleteItem) => {
      setAcState((s) => ({ ...s, visible: false }));

      editor.update(() => {
        const token = typingTokenRef.current;
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return;

        const anchorNode = selection.anchor.getNode();

        // Remove the typed token text
        if ($isTextNode(anchorNode) && anchorNode.getTextContentSize() > 0) {
          const text = anchorNode.getTextContent();
          const offset = selection.anchor.offset;
          const beforeToken = text.slice(0, offset - token.length);
          const afterToken = text.slice(offset);
          anchorNode.setTextContent(beforeToken + afterToken);
        }

        const { nodeType, kind } = detectCardKindFromItem(item);
        const pid = uid("cc");

        if (nodeType === "inline") {
          const cardNode = $createCardNode(kind as any, item.label, pid);
          anchorNode.getParent()?.append(cardNode);
          // Insert text node after for continued typing
          const nextText = $createTextNode(" ");
          cardNode.insertAfter(nextText);
          nextText.select();
        } else if (nodeType === "container") {
          const containerNode = $createContainerElementNode(kind as any, pid);
          anchorNode.getParent()?.append(containerNode);
          containerNode.selectStart();
        } else if (nodeType === "block") {
          const blockNode = $createBlockElementNode(kind as any, pid);
          const bodyLine = $createBodyLineNode(uid("bl"));
          blockNode.append(bodyLine);
          anchorNode.getParent()?.append(blockNode);
          bodyLine.selectStart();
        }
      });
    },
    [editor],
  );

  // ── Enter confirms autocomplete selection ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent) => {
        if (!acState.visible) return false;
        event.preventDefault();
        executeAutocomplete(acState.items[acState.index]);
        return true;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, acState, executeAutocomplete]);

  // ── Escape closes autocomplete ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      () => {
        if (acState.visible) {
          setAcState((s) => ({ ...s, visible: false }));
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, acState.visible]);

  // ── Arrow down navigates autocomplete list ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ARROW_DOWN_COMMAND,
      (event: KeyboardEvent) => {
        if (!acState.visible) return false;
        event.preventDefault();
        setAcState((s) => ({
          ...s,
          index: Math.min(s.index + 1, s.items.length - 1),
        }));
        return true;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, acState.visible, acState.items.length]);

  // ── Arrow up navigates autocomplete list ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ARROW_UP_COMMAND,
      (event: KeyboardEvent) => {
        if (!acState.visible) return false;
        event.preventDefault();
        setAcState((s) => ({
          ...s,
          index: Math.max(s.index - 1, 0),
        }));
        return true;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, acState.visible]);

  if (!acState.visible) return null;

  return (
    <AutocompleteMenu
      items={acState.items}
      selectedIndex={acState.index}
      onSelect={executeAutocomplete}
      onClose={() => setAcState((s) => ({ ...s, visible: false }))}
      position={menuPos}
    />
  );
}

// ── Context helpers ──

function getParentContextKind(parent: any): string | null {
  if (!parent || parent === $getRoot()) return null;
  if ($isContainerElementNode(parent)) return parent.getContainerKind();
  if ($isBlockElementNode(parent)) return parent.getBlockKind();
  const grandparent = parent.getParent();
  if ($isContainerElementNode(grandparent)) return grandparent.getContainerKind();
  if ($isBlockElementNode(grandparent)) return grandparent.getBlockKind();
  return null;
}

function getPrecedingCardKind(node: any, offset: number): string | null {
  const text = node.getTextContent();
  if (offset > 0 && text[offset - 1] === ".") return "dot";
  return null;
}

function inferSlotKind(parentKind: string | null): string | null {
  if (!parentKind) return null;
  switch (parentKind) {
    case "let":
    case "set":
      return "expression";
    case "if":
    case "while":
      return "condition";
    case "for":
      return "name";
    case "call":
      return "fn_name";
    case "return":
    case "await":
      return "expression";
    default:
      return null;
  }
}
