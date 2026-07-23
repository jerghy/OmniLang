// poiesis/src/components/canvas/code/lexical/nodes/BlockNode.ts
import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type Spread,
} from "lexical";
import type { BlockKind } from "../../../../../core/types/editor-node";

export interface BranchHeadItem {
  kind: string;
  cardLabel?: string;
  slotKind?: string;
  value?: string;
  pid: string;
}

export interface BranchMeta {
  kind: "elif" | "else";
  pid: string;
  head: BranchHeadItem[];
}

export interface SerializedBlockElementNode extends Spread<
  { blockKind: BlockKind; blockPid: string; branches?: BranchMeta[]; type: "block" },
  SerializedElementNode
> {}

export class BlockElementNode extends ElementNode {
  __blockKind: BlockKind;
  __blockPid: string;
  __branches: BranchMeta[] | undefined;

  static getType(): string {
    return "block";
  }

  static clone(node: BlockElementNode): BlockElementNode {
    return new BlockElementNode(node.__blockKind, node.__blockPid, node.__branches, node.__key);
  }

  constructor(kind: BlockKind, pid: string, branches?: BranchMeta[], key?: string) {
    super(key);
    this.__blockKind = kind;
    this.__blockPid = pid;
    this.__branches = branches;
  }

  getBlockKind(): BlockKind {
    return this.getLatest().__blockKind;
  }

  getBlockPid(): string {
    return this.getLatest().__blockPid;
  }

  getBranches(): BranchMeta[] | undefined {
    return (this.getLatest() as BlockElementNode).__branches;
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
      branches: this.__branches,
    };
  }

  static importJSON(serialized: SerializedBlockElementNode): BlockElementNode {
    const node = $createBlockElementNode(serialized.blockKind, serialized.blockPid, serialized.branches);
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

export function $createBlockElementNode(kind: BlockKind, pid: string, branches?: BranchMeta[]): BlockElementNode {
  return new BlockElementNode(kind, pid, branches);
}
