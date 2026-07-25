// poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
// v3 flat: containers and blocks are unfolded to flat inline card sequences.
// Only BodyLineNode (for block body indentation) remains as a structural node.
// Keyword cards act as delimiters for reconstruction during extraction.

import { useEffect, useRef } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import { $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import type { CodeOp } from "../../../../../core/types/code-op";
import type { EditorNode, ContainerNode, BlockNode, InlineNode, InputSlot, BodyLine, Branch } from "../../../../../core/types/editor-node";
import { convertToNodes } from "../transforms/convertToNodes";
import { convertToOps } from "../transforms/convertToOps";
import { $createCardNode, $isCardNode } from "../nodes/CardNode";
import { $createBodyLineNode, $isBodyLineNode } from "../nodes/BodyLineNode";
import { $createSlotCardNode, $isSlotCardNode } from "../nodes/SlotCardNode";

const CONTAINER_KEYWORDS = new Set(["let", "set", "return", "await", "call", "array", "map"]);
const BLOCK_KEYWORDS = new Set(["if", "while", "for"]);

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
        const lexicalNodes = editorNodeToLexical(node);
        if (Array.isArray(lexicalNodes)) {
          // Group inline cards in a paragraph with cursor text anchors
          const para = $createParagraphNode();
          for (let i = 0; i < lexicalNodes.length; i++) {
            const ln = lexicalNodes[i];
            if (ln.isInline?.()) {
              // Add text anchor before inline card if first or after body-line
              if (i === 0 || !lexicalNodes[i - 1]?.isInline?.()) {
                para.append($createTextNode("​"));
              }
              para.append(ln);
              para.append($createTextNode("​"));
            } else {
              // BodyLineNode or other block — append to root directly
              if (para.getChildrenSize() > 0) {
                root.append(para);
              }
              root.append(ln);
            }
          }
          if (para.getChildrenSize() > 0) {
            root.append(para);
          }
        } else {
          if (lexicalNodes.isInline?.()) {
            const para = $createParagraphNode();
            para.append($createTextNode("​"));
            para.append(lexicalNodes);
            para.append($createTextNode("​"));
            root.append(para);
          } else {
            root.append(lexicalNodes);
          }
        }
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

// ── EditorNode → Lexical node(s) ──
// Containers and blocks are FLATTENED: no wrapping ElementNode.

/** Convert a single EditorNode to zero or more LexicalNodes (may be nested). */
function editorNodeToLexical(node: EditorNode): any | any[] {
  return _editorNodeToLexical(node);
}

/** Flatten to a flat array of LexicalNodes. */
function editorNodeToLexicalFlat(node: EditorNode): any[] {
  const r = _editorNodeToLexical(node);
  return Array.isArray(r) ? r.flat(Infinity) : [r];
}

function _editorNodeToLexical(node: EditorNode): any | any[] {
  switch (node.kind) {
    case "let":
    case "set":
    case "return":
    case "await":
    case "call":
    case "array":
    case "map": {
      // Flat: just return children as inline cards, no container wrapper
      const children = (node as ContainerNode).children;
      return children.map(c => editorNodeToLexical(c));
    }
    case "if":
    case "while":
    case "for": {
      const bNode = node as BlockNode;
      const result: any[] = [];

      // Head as flat inline cards
      for (const headItem of bNode.head) {
        result.push(...editorNodeToLexicalFlat(headItem));
      }

      // Branch metadata stored on first body line for reconstruction
      let branchMeta: any = undefined;
      if (bNode.branches && bNode.branches.length > 0) {
        branchMeta = bNode.branches.map(br => ({
          kind: br.kind,
          pid: br.pid,
          head: br.head.map((h: any) => ({
            kind: h.kind,
            cardLabel: h.cardLabel,
            slotKind: h.slotKind,
            value: h.value,
            pid: h.pid,
          })),
        }));
      }

      // Main body lines
      for (const line of bNode.body) {
        const bl = $createBodyLineNode(line.pid);
        // Attach branch meta to first body line for reconstruction
        if (branchMeta && !(bl as any).__branchMeta) {
          (bl as any).__branchMeta = branchMeta;
        }
        for (const child of line.children) {
          for (const ln of editorNodeToLexicalFlat(child)) {
            bl.append(ln);
          }
        }
        result.push(bl);
      }

      // Tail line (closing boundary marker)
      const tail = $createBodyLineNode(bNode.tail.pid);
      tail.setTailMarker(true);
      result.push(tail);

      // Branch heads as flat inline cards + branch body lines
      if (bNode.branches) {
        for (let bi = 0; bi < bNode.branches.length; bi++) {
          const br = bNode.branches[bi];
          // Branch head keywords + slots as flat inline cards
          for (const headItem of br.head) {
            result.push(...editorNodeToLexicalFlat(headItem));
          }
          // Branch body lines
          for (const line of br.body) {
            const bl = $createBodyLineNode(line.pid, bi);
            for (const child of line.children) {
              for (const ln of editorNodeToLexicalFlat(child)) {
                bl.append(ln);
              }
            }
            result.push(bl);
          }
        }
      }

      return result;
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
// Reconstruct containers/blocks from flat keyword-delimited card sequences.

function extractEditorNodes(root: ReturnType<typeof $getRoot>): EditorNode[] {
  // First, collect all flat EditorNodes from the Lexical tree
  const flatNodes: EditorNode[] = [];
  const children = root.getChildren();
  for (const child of children) {
    const extracted = extractFromNode(child);
    if (extracted) {
      if (Array.isArray(extracted)) {
        flatNodes.push(...extracted);
      } else {
        flatNodes.push(extracted);
      }
    }
  }

  // Second, reconstruct containers and blocks from flat keyword-delimited nodes
  const topLevel = reconstructContainers(flatNodes);

  // Third, recursively reconstruct containers inside body lines and blocks
  return topLevel.map(node => {
    if (node.kind === "body_line" as any) {
      const bl = node as any as BodyLineEx;
      return {
        pid: bl.pid,
        children: reconstructContainers(bl.children),
      } as BodyLine;
    }
    if (node.kind === "if" || node.kind === "while" || node.kind === "for") {
      const b = node as BlockNode;
      return {
        ...b,
        body: b.body.map(line => ({
          pid: line.pid,
          children: reconstructContainers(line.children),
        })),
        branches: b.branches?.map(br => ({
          ...br,
          body: br.body.map(line => ({
            pid: line.pid,
            children: reconstructContainers(line.children),
          })),
        })),
      };
    }
    return node;
  }) as EditorNode[];
}

function extractFromNode(lexicalNode: any): EditorNode | EditorNode[] | null {
  const direct = lexicalToEditorNode(lexicalNode);
  if (direct) return direct;

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

  if ($isBodyLineNode(lexicalNode)) {
    // Skip tail markers during extraction (visual only)
    if ((lexicalNode as any).isTailMarker?.()) return null;
    const lineChildren: EditorNode[] = [];
    for (const lc of lexicalNode.getChildren()) {
      const c = lexicalToEditorNode(lc);
      if (c) lineChildren.push(c);
    }
    const bl: any = {
      kind: "body_line" as any,
      pid: lexicalNode.getBodyLinePid(),
      children: lineChildren,
    };
    // Carry branch metadata for reconstruction
    const bi = lexicalNode.getBranchIndex();
    if (bi !== undefined) bl.branchIndex = bi;
    if ((lexicalNode as any).__branchMeta) bl.__branchMeta = (lexicalNode as any).__branchMeta;
    return bl;
  }

  return null;
}

// ── Reconstruction: flat nodes → containers + blocks ──

function isInline(n: EditorNode): n is InlineNode {
  return "cardLabel" in n;
}

interface BodyLineEx extends BodyLine {
  branchIndex?: number;
  __branchMeta?: any;
}

function reconstructContainers(flat: EditorNode[]): EditorNode[] {
  const result: EditorNode[] = [];
  let i = 0;

  while (i < flat.length) {
    const node = flat[i];

    // Check if this is the start of a container or block (keyword card)
    if (isInline(node) && node.kind === "keyword") {
      const kw = node.cardLabel;

      if (CONTAINER_KEYWORDS.has(kw)) {
        // Container: consume children until next keyword or end of current context
        const children: EditorNode[] = [node];
        i++;
        while (i < flat.length) {
          const next = flat[i];
          if (isInline(next) && next.kind === "keyword") break;
          if ((next as any).kind === "body_line") break;
          children.push(next);
          i++;
        }
        result.push({
          kind: kw as ContainerNode["kind"],
          pid: node.pid,
          children,
        } as ContainerNode);
      } else if (BLOCK_KEYWORDS.has(kw)) {
        // Block: consume head, then body lines
        const head: EditorNode[] = [node];
        const body: BodyLine[] = [];
        const branches: Branch[] = [];
        i++;

        // Consume head cards until first BodyLine
        while (i < flat.length && (flat[i] as any).kind !== "body_line") {
          head.push(flat[i]);
          i++;
        }

        // Collect branch metadata from first body line
        let branchMeta: any = undefined;
        const firstBodyLine = flat[i] as BodyLineEx | undefined;
        if (firstBodyLine?.__branchMeta) {
          branchMeta = firstBodyLine.__branchMeta;
        }

        // Consume main body lines
        while (i < flat.length && (flat[i] as any).kind === "body_line") {
          const bl = flat[i] as BodyLineEx;
          if (bl.branchIndex !== undefined) break; // branch body lines handled below
          body.push({ pid: bl.pid, children: bl.children });
          i++;
        }

        // If there are branches, consume them
        if (branchMeta && branchMeta.length > 0) {
          for (let bi = 0; bi < branchMeta.length; bi++) {
            const bm = branchMeta[bi];
            const branchBody: BodyLine[] = [];

            // Skip branch head inline nodes (already captured in branchMeta)
            while (i < flat.length && (flat[i] as any).kind !== "body_line") {
              i++;
            }
            // Consume branch body lines with matching branchIndex
            while (i < flat.length && (flat[i] as any).kind === "body_line") {
              const bl = flat[i] as BodyLineEx;
              if (bl.branchIndex === bi) {
                branchBody.push({ pid: bl.pid, children: bl.children });
                i++;
              } else {
                break;
              }
            }

            branches.push({
              kind: bm.kind,
              pid: bm.pid,
              head: bm.head.map((h: any) => {
                if (h.kind === "slot") {
                  return { kind: "slot" as const, slotKind: h.slotKind || "condition", value: h.value || "", pid: h.pid };
                }
                return { kind: h.kind as any, cardLabel: h.cardLabel || "", pid: h.pid };
              }),
              body: branchBody,
            });
          }
        }

        result.push({
          kind: kw as BlockNode["kind"],
          pid: node.pid,
          head,
          body,
          tail: { pid: "tail-" + node.pid },
          branches: branches.length > 0 ? branches : undefined,
        } as BlockNode);
      } else {
        // Standalone keyword (e.g. break, continue — not containers/blocks)
        result.push(node);
        i++;
      }
    } else {
      // Non-keyword node (e.g. break, continue inline, or standalone slots)
      result.push(node);
      i++;
    }
  }

  return result;
}
