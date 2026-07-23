// poiesis/src/components/canvas/code/lexical/nodes/CardElementNode.ts
import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type Spread,
} from "lexical";
import type { CodeOp } from "../../../../../core/types/code-op";

export type CardKind = CodeOp["kind"];

export interface SerializedCardElementNode extends Spread<
  { cardKind: CardKind; cardPid: string; type: "card" },
  SerializedElementNode
> {}

export class CardElementNode extends ElementNode {
  __cardKind: CardKind;
  __cardPid: string;

  static getType(): string {
    return "card";
  }

  static clone(node: CardElementNode): CardElementNode {
    return new CardElementNode(node.__cardKind, node.__cardPid, node.__key);
  }

  constructor(kind: CardKind, pid: string, key?: string) {
    super(key);
    this.__cardKind = kind;
    this.__cardPid = pid;
  }

  getCardKind(): CardKind {
    const self = this.getLatest();
    return (self as CardElementNode).__cardKind;
  }

  getCardPid(): string {
    const self = this.getLatest();
    return (self as CardElementNode).__cardPid;
  }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = `code-card code-card--${this.__cardKind}`;
    el.setAttribute("data-card-pid", this.__cardPid);
    return el;
  }

  updateDOM(_prevNode: CardElementNode, _dom: HTMLElement): boolean {
    // Always re-render for simplicity; className changes on kind mutation
    return true;
  }

  exportJSON(): SerializedCardElementNode {
    return {
      ...super.exportJSON(),
      type: "card",
      cardKind: this.__cardKind,
      cardPid: this.__cardPid,
    };
  }

  static importJSON(serialized: SerializedCardElementNode): CardElementNode {
    const node = $createCardElementNode(serialized.cardKind, serialized.cardPid);
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

export function $isCardElementNode(
  node: LexicalNode | null | undefined,
): node is CardElementNode {
  return node instanceof CardElementNode;
}

export function $createCardElementNode(kind: CardKind, pid: string): CardElementNode {
  return new CardElementNode(kind, pid);
}
