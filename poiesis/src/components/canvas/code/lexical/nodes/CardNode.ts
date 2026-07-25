// poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts
// v2 ElementNode — contenteditable="false" inline atomic card.
// Browser handles: click-to-select, arrow-key navigation, Delete on selected card.
// KeyboardPlugin handles: container right-edge Delete, Enter at boundaries.
import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type EditorConfig,
  type NodeKey,
  type Spread,
} from "lexical";
import type { InlineKind } from "../../../../../core/types/editor-node";

export interface SerializedCardNode extends Spread<
  { cardKind: InlineKind; cardLabel: string; cardPid: string; type: "inline-card" },
  SerializedElementNode
> {}

export class CardNode extends ElementNode {
  __cardKind: InlineKind;
  __cardLabel: string;
  __cardPid: string;

  static getType(): string {
    return "inline-card";
  }

  static clone(node: CardNode): CardNode {
    return new CardNode(node.__cardKind, node.__cardLabel, node.__cardPid, node.__key);
  }

  constructor(kind: InlineKind, label: string, pid: string, key?: NodeKey) {
    super(key);
    this.__cardKind = kind;
    this.__cardLabel = label;
    this.__cardPid = pid;
  }

  // ── Inline atomic card ──
  isInline(): boolean {
    return true;
  }

  canBeEmpty(): boolean {
    return true; // no Lexical-managed children; label is direct DOM text
  }

  canInsertTextBefore(): boolean {
    return true; // cursor can appear before this card (pipe state)
  }

  canInsertTextAfter(): boolean {
    return true; // cursor can appear after this card (pipe state)
  }

  // ── DOM: contenteditable="false" makes the browser treat it as an atomic unit ──
  createDOM(_config: EditorConfig): HTMLElement {
    const el = document.createElement("span");
    el.className = `code-card code-card--${this.__cardKind}`;
    el.setAttribute("data-card-kind", this.__cardKind);
    el.setAttribute("data-card-label", this.__cardLabel);
    el.setAttribute("data-card-pid", this.__cardPid);
    el.setAttribute("contenteditable", "false");
    el.tabIndex = -1;
    el.textContent = this.__cardLabel;
    return el;
  }

  updateDOM(
    prevNode: CardNode,
    dom: HTMLElement,
    _config: EditorConfig,
  ): boolean {
    if (prevNode.__cardKind !== this.__cardKind) {
      dom.className = `code-card code-card--${this.__cardKind}`;
      dom.setAttribute("data-card-kind", this.__cardKind);
    }
    if (prevNode.__cardLabel !== this.__cardLabel) {
      dom.textContent = this.__cardLabel;
    }
    return false; // Lexical does not manage child DOM for us
  }

  // ── Public API (unchanged) ──
  getCardKind(): InlineKind {
    const self = this.getLatest();
    return (self as CardNode).__cardKind;
  }

  getCardLabel(): string {
    const self = this.getLatest();
    return (self as CardNode).__cardLabel;
  }

  getCardPid(): string {
    const self = this.getLatest();
    return (self as CardNode).__cardPid;
  }

  // ── Serialization ──
  exportJSON(): SerializedCardNode {
    return {
      ...super.exportJSON(),
      type: "inline-card",
      cardKind: this.__cardKind,
      cardLabel: this.__cardLabel,
      cardPid: this.__cardPid,
    };
  }

  static importJSON(serializedNode: SerializedCardNode): CardNode {
    const node = $createCardNode(
      serializedNode.cardKind,
      serializedNode.cardLabel,
      serializedNode.cardPid,
    );
    node.setFormat(serializedNode.format);
    node.setIndent(serializedNode.indent);
    node.setDirection(serializedNode.direction);
    return node;
  }
}

export function $isCardNode(node: LexicalNode | null | undefined): node is CardNode {
  return node instanceof CardNode;
}

export function $createCardNode(kind: InlineKind, label: string, pid: string): CardNode {
  return new CardNode(kind, label, pid);
}
