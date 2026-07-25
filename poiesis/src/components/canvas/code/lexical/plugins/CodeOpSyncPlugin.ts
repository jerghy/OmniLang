// poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
// Renamed internally to NodeSyncPlugin — file name kept for git history.
// Now syncs EditorNode[] instead of directly converting to CodeOp[].

import { useEffect, useRef } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import { $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine, Branch } from "../../../../../core/types/editor-node";
import { convertToNodes } from "../transforms/convertToNodes";
import { convertToOps } from "../transforms/convertToOps";
import { $createCardNode, $isCardNode } from "../nodes/CardNode";
import { $createContainerElementNode, $isContainerElementNode } from "../nodes/ContainerNode";
import { $createBlockElementNode, $isBlockElementNode } from "../nodes/BlockNode";
import { $createBodyLineNode, $isBodyLineNode } from "../nodes/BodyLineNode";
import { $createSlotCardNode, $isSlotCardNode } from "../nodes/SlotCardNode";

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
      const bNode = node as BlockNode;

      // Build branch metadata (serializable head content)
      let branchMeta: import("../nodes/BlockNode").BranchMeta[] | undefined;
      if (bNode.branches && bNode.branches.length > 0) {
        branchMeta = bNode.branches.map(br => ({
          kind: br.kind,
          pid: br.pid,
          head: br.head.map(h => ({
            kind: h.kind,
            cardLabel: (h as any).cardLabel,
            slotKind: (h as any).slotKind,
            value: (h as any).value,
            pid: h.pid,
          })),
        }));
      }

      const block = $createBlockElementNode(node.kind, node.pid, branchMeta);
      // Head line
      for (const headItem of bNode.head) {
        block.append(editorNodeToLexical(headItem));
      }
      // Main body lines
      for (const line of bNode.body) {
        const bl = $createBodyLineNode(line.pid);
        for (const child of line.children) {
          bl.append(editorNodeToLexical(child));
        }
        block.append(bl);
      }
      // Branch body lines (marked with branchIndex)
      if (bNode.branches) {
        for (let bi = 0; bi < bNode.branches.length; bi++) {
          const br = bNode.branches[bi];
          for (const line of br.body) {
            const bl = $createBodyLineNode(line.pid, bi);
            for (const child of line.children) {
              bl.append(editorNodeToLexical(child));
            }
            block.append(bl);
          }
        }
      }
      return block;
    }
    case "keyword":
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
      return $createSlotCardNode(sl.value, sl.slotKind, sl.pid);
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
    const extracted = extractFromNode(child);
    if (extracted) {
      if (Array.isArray(extracted)) {
        nodes.push(...extracted);
      } else {
        nodes.push(extracted);
      }
    }
  }
  return nodes;
}

/** Recursively extract EditorNodes from any Lexical node, traversing into paragraphs. */
function extractFromNode(lexicalNode: any): EditorNode | EditorNode[] | null {
  const direct = lexicalToEditorNode(lexicalNode);
  if (direct) return direct;

  // Traverse into ElementNodes (e.g. ParagraphNode) that may wrap our custom nodes
  if (lexicalNode.getChildren && typeof lexicalNode.getChildren === "function") {
    const result: EditorNode[] = [];
    for (const child of lexicalNode.getChildren()) {
      const extracted = extractFromNode(child);
      if (extracted) {
        if (Array.isArray(extracted)) {
          result.push(...extracted);
        } else {
          result.push(extracted);
        }
      }
    }
    if (result.length > 0) return result;
  }

  return null;
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
    const mainBody: BodyLine[] = [];
    const branchBodies: Map<number, BodyLine[]> = new Map();

    // Read branch metadata
    const branchMeta = lexicalNode.getBranches();

    for (const child of lexicalNode.getChildren()) {
      if ($isBodyLineNode(child)) {
        const bi = child.getBranchIndex();
        const lineChildren: EditorNode[] = [];
        for (const lc of child.getChildren()) {
          const c = lexicalToEditorNode(lc);
          if (c) lineChildren.push(c);
        }
        const line: BodyLine = { pid: child.getBodyLinePid(), children: lineChildren };

        if (bi !== undefined) {
          // Branch body line
          const existing = branchBodies.get(bi) || [];
          existing.push(line);
          branchBodies.set(bi, existing);
        } else {
          // Main body line
          mainBody.push(line);
        }
      } else {
        const c = lexicalToEditorNode(child);
        if (c) head.push(c);
      }
    }

    // Reconstruct branches from metadata + body lines
    let branches: import("../../../../../core/types/editor-node").Branch[] | undefined;
    if (branchMeta && branchMeta.length > 0) {
      branches = branchMeta.map((bm, bi) => ({
        kind: bm.kind,
        pid: bm.pid,
        head: bm.head.map(h => {
          if (h.kind === "slot") {
            return { kind: "slot" as const, slotKind: h.slotKind || "condition" as const, value: h.value || "", pid: h.pid };
          }
          return { kind: h.kind as any, cardLabel: h.cardLabel || "", pid: h.pid };
        }),
        body: branchBodies.get(bi) || [],
      })) as Branch[];
    }

    return {
      kind: lexicalNode.getBlockKind(),
      pid: lexicalNode.getBlockPid(),
      head,
      body: mainBody,
      tail: { pid: "tail-" + lexicalNode.getBlockPid() },
      branches,
    } as BlockNode;
  }

  if ($isCardNode(lexicalNode)) {
    return {
      kind: lexicalNode.getCardKind(),
      cardLabel: lexicalNode.getCardLabel(),
      pid: lexicalNode.getCardPid(),
    } as InlineNode;
  }

  if ($isSlotCardNode(lexicalNode)) {
    return {
      kind: "slot",
      slotKind: lexicalNode.getSlotKind(),
      value: lexicalNode.getValue(),
      pid: lexicalNode.getPid(),
    } as InputSlot;
  }

  return null;
}
