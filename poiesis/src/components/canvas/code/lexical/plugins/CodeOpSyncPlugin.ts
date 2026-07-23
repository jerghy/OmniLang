// poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
// Renamed internally to NodeSyncPlugin — file name kept for git history.
// Now syncs EditorNode[] instead of directly converting to CodeOp[].

import { useEffect, useRef } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import { $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine } from "../../../../../core/types/editor-node";
import { convertToNodes } from "../transforms/convertToNodes";
import { convertToOps } from "../transforms/convertToOps";
import { $createCardNode, $isCardNode } from "../nodes/CardNode";
import { $createContainerElementNode, $isContainerElementNode } from "../nodes/ContainerNode";
import { $createBlockElementNode, $isBlockElementNode } from "../nodes/BlockNode";
import { $createBodyLineNode, $isBodyLineNode } from "../nodes/BodyLineNode";
import { $createInputSlotNode, $isInputSlotNode } from "../nodes/InputSlotNode";

interface Props {
  initialOps: CodeOp[];
  onSync: (ops: CodeOp[]) => void;
}

export function CodeOpSyncPlugin({ initialOps, onSync }: Props): null {
  const [editor] = useLexicalComposerContext();
  const initializedRef = useRef(false);

  // ── Load initial ops into editor (once) ──
  useEffect(() => {
    if (initializedRef.current) return;
    initializedRef.current = true;

    editor.update(() => {
      const root = $getRoot();
      root.clear();

      const nodes = convertToNodes(initialOps);
      for (const node of nodes) {
        const lexicalNode = editorNodeToLexical(node);
        root.append(lexicalNode);
      }
      root.append($createParagraphNode());
    });
  }, [editor, initialOps]);

  // ── Extract EditorNode[] → CodeOp[] on change and report ──
  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const root = $getRoot();
        const nodes = extractEditorNodes(root);
        const ops = convertToOps(nodes);
        onSync(ops);
      });
    });
  }, [editor, onSync]);

  return null;
}

// ── EditorNode → Lexical node ──

function editorNodeToLexical(node: EditorNode) {
  switch (node.kind) {
    case "let":
    case "set":
    case "return":
    case "await":
    case "call":
    case "array":
    case "map": {
      const container = $createContainerElementNode(node.kind, node.pid);
      for (const child of (node as ContainerNode).children) {
        container.append(editorNodeToLexical(child));
      }
      return container;
    }
    case "if":
    case "while":
    case "for": {
      const block = $createBlockElementNode(node.kind, node.pid);
      const bNode = node as BlockNode;
      // Head line
      for (const headItem of bNode.head) {
        block.append(editorNodeToLexical(headItem));
      }
      // Body lines
      for (const line of bNode.body) {
        const bl = $createBodyLineNode(line.pid);
        for (const child of line.children) {
          bl.append(editorNodeToLexical(child));
        }
        block.append(bl);
      }
      // Branches (if)
      if (bNode.branches) {
        for (const br of bNode.branches) {
          // Represent branch as container with head + body
          const brContainer = $createContainerElementNode("call" as any, br.pid); // temporary
          for (const headItem of br.head) {
            brContainer.append(editorNodeToLexical(headItem));
          }
          for (const line of br.body) {
            const bl = $createBodyLineNode(line.pid);
            for (const child of line.children) {
              bl.append(editorNodeToLexical(child));
            }
            brContainer.append(bl);
          }
          block.append(brContainer);
        }
      }
      return block;
    }
    case "mut":
    case "break":
    case "continue":
    case "operator":
    case "type":
    case "var_ref":
    case "literal":
    case "dot":
    case "separator":
    case "comment": {
      const inNode = node as InlineNode;
      return $createCardNode(inNode.kind, inNode.cardLabel, inNode.pid);
    }
    case "slot": {
      const sl = node as InputSlot;
      return $createInputSlotNode(sl.value, sl.slotKind);
    }
    default:
      return $createTextNode("");
  }
}

// ── Lexical nodes → EditorNode[] ──

function extractEditorNodes(root: ReturnType<typeof $getRoot>): EditorNode[] {
  const nodes: EditorNode[] = [];
  const children = root.getChildren();
  for (const child of children) {
    const node = lexicalToEditorNode(child);
    if (node) nodes.push(node);
  }
  return nodes;
}

function lexicalToEditorNode(lexicalNode: any): EditorNode | null {
  if ($isContainerElementNode(lexicalNode)) {
    const children: EditorNode[] = [];
    for (const child of lexicalNode.getChildren()) {
      const c = lexicalToEditorNode(child);
      if (c) children.push(c);
    }
    return {
      kind: lexicalNode.getContainerKind(),
      pid: lexicalNode.getContainerPid(),
      children,
    } as ContainerNode;
  }

  if ($isBlockElementNode(lexicalNode)) {
    const head: EditorNode[] = [];
    const body: BodyLine[] = [];
    for (const child of lexicalNode.getChildren()) {
      if ($isBodyLineNode(child)) {
        const lineChildren: EditorNode[] = [];
        for (const lc of child.getChildren()) {
          const c = lexicalToEditorNode(lc);
          if (c) lineChildren.push(c);
        }
        body.push({ pid: child.getBodyLinePid(), children: lineChildren });
      } else {
        const c = lexicalToEditorNode(child);
        if (c) head.push(c);
      }
    }
    return {
      kind: lexicalNode.getBlockKind(),
      pid: lexicalNode.getBlockPid(),
      head,
      body,
      tail: { pid: "tail-" + lexicalNode.getBlockPid() },
    } as BlockNode;
  }

  if ($isCardNode(lexicalNode)) {
    return {
      kind: lexicalNode.getCardKind(),
      cardLabel: lexicalNode.getCardLabel(),
      pid: lexicalNode.getCardPid(),
    } as InlineNode;
  }

  if ($isInputSlotNode(lexicalNode)) {
    return {
      kind: "slot",
      slotKind: lexicalNode.getSlotKind(),
      value: lexicalNode.getTextContent(),
      pid: lexicalNode.getKey(),
    } as InputSlot;
  }

  return null;
}
