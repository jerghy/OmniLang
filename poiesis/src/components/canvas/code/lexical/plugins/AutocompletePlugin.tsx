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
  $isRootNode,
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
import { uid } from "../../../../../core/utils/id";
import { AutocompleteMenu } from "../autocomplete/AutocompleteMenu";
import {
  buildCursorContext,
  canCreateSlot,
  createSlotAtCursor,
} from "../transforms/slot-operations";

interface Props {
  /** Called when backslash key creates a new input slot */
  onCreateSlot?: () => void;
}

interface AcState {
  visible: boolean;
  items: AutocompleteItem[];
  index: number;
}

const EMPTY_AC: AcState = { visible: false, items: [], index: 0 };

export function AutocompletePlugin({ onCreateSlot }: Props): React.ReactNode {
  const [editor] = useLexicalComposerContext();
  const [acState, setAcState] = useState<AcState>(EMPTY_AC);
  const typingTokenRef = useRef("");
  /** Suppress the update listener from re-opening the menu after executeAutocomplete */
  const suppressNextUpdateRef = useRef(false);

  // ── Ref mirror of acState for synchronous reads in keyboard handlers ──
  // React setState is async; keyboard events may fire before re-render, so
  // command handlers MUST read from the ref, never from acState directly.
  const acRef = useRef<AcState>(EMPTY_AC);

  const updateAc = useCallback((next: AcState) => {
    acRef.current = next;
    setAcState(next);
  }, []);

  const patchAc = useCallback((fn: (prev: AcState) => AcState) => {
    const next = fn(acRef.current);
    acRef.current = next;
    setAcState(next);
  }, []);

  // ── Menu position computed from cursor (viewport-relative for position:fixed) ──
  const [menuPos, setMenuPos] = useState({ top: 0, left: 0 });

  // ── Detect typing and trigger autocomplete ──
  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }: any) => {
      editorState.read(() => {
        // Skip if we just executed an autocomplete action (prevent immediate re-open)
        if (suppressNextUpdateRef.current) {
          suppressNextUpdateRef.current = false;
          return;
        }

        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          patchAc((s) => (s.visible ? { ...s, visible: false } : s));
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
        if (token && items.length > 0) {
          // Calculate position BEFORE showing menu (avoid flash at 0,0)
          const sel = window.getSelection();
          if (sel && sel.rangeCount > 0) {
            const rect = sel.getRangeAt(0).getBoundingClientRect();
            setMenuPos({ top: rect.bottom + 4, left: rect.left });
          } else {
            const editorEl = document.querySelector('[contenteditable="true"]');
            if (editorEl) {
              const editorRect = editorEl.getBoundingClientRect();
              setMenuPos({ top: editorRect.top + 40, left: editorRect.left + 24 });
            }
          }
          updateAc({ visible: true, items, index: 0 });
          typingTokenRef.current = token;
        } else {
          patchAc((s) => (s.visible ? { ...s, visible: false } : s));
        }
      });
    });
  }, [editor, updateAc, patchAc]);

  // ── Backslash → insert input slot (delegated to slot-operations) ──
  useEffect(() => {
    const removeRootListener = editor.registerRootListener(
      (rootElement: HTMLElement | null) => {
        if (!rootElement) return;
        const handler = (e: KeyboardEvent) => {
          if (e.key !== "\\") return;
          e.preventDefault();
          editor.update(() => {
            const selection = $getSelection();
            if (!$isRangeSelection(selection)) return;

            const ctx = buildCursorContext(selection);
            if (!canCreateSlot(ctx)) return;

            const parent = selection.anchor.getNode().getParent();
            const slot = createSlotAtCursor(ctx, parent);
            if (slot) {
              slot.select();
              onCreateSlot?.();
            }
          });
        };
        rootElement.addEventListener("keydown", handler);
        return () => rootElement.removeEventListener("keydown", handler);
      },
    );
    return () => removeRootListener?.();
  }, [editor, onCreateSlot]);

  const executeAutocomplete = useCallback(
    (item: AutocompleteItem) => {
      // Suppress the update listener from re-opening the menu after this edit
      suppressNextUpdateRef.current = true;
      updateAc({ ...acRef.current, visible: false });

      editor.update(() => {
        const token = typingTokenRef.current;
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return;

        const anchorNode = selection.anchor.getNode();
        // Save parent BEFORE modifying text (text node may become stale after setTextContent)
        const parentNode = anchorNode.getParent();

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
          parentNode?.append(cardNode);
          const nextText = $createTextNode(" ");
          cardNode.insertAfter(nextText);
          nextText.select();
        } else if (nodeType === "container") {
          const containerNode = $createContainerElementNode(kind as any, pid);
          insertStatementNode(containerNode, parentNode, anchorNode);
          createDefaultSlot(containerNode, kind);
        } else if (nodeType === "block") {
          const blockNode = $createBlockElementNode(kind as any, pid);
          const bodyLine = $createBodyLineNode(uid("bl"));
          blockNode.append(bodyLine);
          insertStatementNode(blockNode, parentNode, anchorNode);
          bodyLine.selectStart();
        }
      });
    },
    [editor, updateAc],
  );

  // ── Enter confirms autocomplete selection ──
  // Reads from acRef (not acState) to avoid stale closure after async setState.
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent) => {
        const cur = acRef.current;
        if (!cur.visible) return false;
        event.preventDefault();
        executeAutocomplete(cur.items[cur.index]);
        return true;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, executeAutocomplete]);

  // ── Escape closes autocomplete ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      () => {
        if (acRef.current.visible) {
          patchAc((s) => ({ ...s, visible: false }));
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, patchAc]);

  // ── Arrow down navigates autocomplete list ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ARROW_DOWN_COMMAND,
      (event: KeyboardEvent) => {
        if (!acRef.current.visible) return false;
        event.preventDefault();
        patchAc((s) => ({
          ...s,
          index: Math.min(s.index + 1, s.items.length - 1),
        }));
        return true;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, patchAc]);

  // ── Arrow up navigates autocomplete list ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ARROW_UP_COMMAND,
      (event: KeyboardEvent) => {
        if (!acRef.current.visible) return false;
        event.preventDefault();
        patchAc((s) => ({
          ...s,
          index: Math.max(s.index - 1, 0),
        }));
        return true;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, patchAc]);

  if (!acState.visible) return null;

  return (
    <AutocompleteMenu
      items={acState.items}
      selectedIndex={acState.index}
      onSelect={executeAutocomplete}
      onClose={() => patchAc((s) => ({ ...s, visible: false }))}
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

/**
 * Insert a statement-level node (container or block) at the root level.
 * If the anchor node is inside an empty paragraph at root, replace the paragraph.
 */
function insertStatementNode(
  node: ReturnType<typeof $createContainerElementNode> | ReturnType<typeof $createBlockElementNode>,
  parentNode: any,
  _anchorNode?: any,
): void {
  const parent = parentNode;
  if (!parent) return;

  const grandParent = parent.getParent();
  if (grandParent && $isRootNode(grandParent)) {
    // Top-level statement — insert at root
    if (parent.getTextContentSize() === 0) {
      parent.replace(node);
    } else {
      parent.insertAfter(node);
    }
  } else {
    // Nested inside a container/block — append as child
    parent.append(node);
  }
}

/**
 * Auto-create the first default slot for a newly created container,
 * so the user can immediately start typing.
 */
function createDefaultSlot(
  containerNode: ReturnType<typeof $createContainerElementNode>,
  kind: string,
): void {
  switch (kind) {
    case "let":
    case "set": {
      const nameSlot = $createInputSlotNode("", "name");
      containerNode.append(nameSlot);
      nameSlot.select();
      break;
    }
    case "return":
    case "await": {
      const exprSlot = $createInputSlotNode("", "expression");
      containerNode.append(exprSlot);
      exprSlot.select();
      break;
    }
    case "call": {
      const fnSlot = $createInputSlotNode("", "fn_name");
      containerNode.append(fnSlot);
      fnSlot.select();
      break;
    }
    default:
      containerNode.selectStart();
  }
}
