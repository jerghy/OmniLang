// poiesis/src/components/canvas/code/lexical/nodes/BlockNode.ts
import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type Spread,
} from "lexical";
import type { BlockKind } from "../../../../../core/types/editor-node";

export interface SerializedBlockElementNode extends Spread<
  { blockKind: BlockKind; blockPid: string; type: "block" },
  SerializedElementNode
> {}

export class BlockElementNode extends ElementNode {
  __blockKind: BlockKind;
  __blockPid: string;

  static getType(): string {
    return "block";
  }

  static clone(node: BlockElementNode): BlockElementNode {
    return new BlockElementNode(node.__blockKind, node.__blockPid, node.__key);
  }

  constructor(kind: BlockKind, pid: string, key?: string) {
    super(key);
    this.__blockKind = kind;
    this.__blockPid = pid;
  }

  getBlockKind(): BlockKind {
    return this.getLatest().__blockKind;
  }

  getBlockPid(): string {
    return this.getLatest().__blockPid;
  }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = `code-block-card code-block-card--${this.__blockKind}`;
    el.setAttribute("data-block-kind", this.__blockKind);
    el.setAttribute("data-block-pid", this.__blockPid);
    return el;
  }

  updateDOM(): boolean {
    return true;
  }

  exportJSON(): SerializedBlockElementNode {
    return {
      ...super.exportJSON(),
      type: "block",
      blockKind: this.__blockKind,
      blockPid: this.__blockPid,
    };
  }

  static importJSON(serialized: SerializedBlockElementNode): BlockElementNode {
    const node = $createBlockElementNode(serialized.blockKind, serialized.blockPid);
    node.setFormat(serialized.format);
    node.setIndent(serialized.indent);
    node.setDirection(serialized.direction);
    return node;
  }

  canBeEmpty(): boolean {
    return false;
  }

  canInsertTextBefore(): boolean {
    return true;
  }

  canInsertTextAfter(): boolean {
    return true;
  }

  isInline(): boolean {
    return false;
  }
}

export function $isBlockElementNode(
  node: LexicalNode | null | undefined,
): node is BlockElementNode {
  return node instanceof BlockElementNode;
}

export function $createBlockElementNode(kind: BlockKind, pid: string): BlockElementNode {
  return new BlockElementNode(kind, pid);
}
