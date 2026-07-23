// poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts
import React from "react";
import {
  DecoratorNode,
  type SerializedLexicalNode,
  type LexicalNode,
  type EditorConfig,
  type NodeKey,
} from "lexical";
import type { InlineKind } from "../../../../../core/types/editor-node";

// Using DecoratorNode: inline cards are read-only blocks, users cannot
// place cursor inside them. Typing replaces or inserts adjacent cards.

export interface SerializedCardNode extends SerializedLexicalNode {
  cardKind: InlineKind;
  cardLabel: string;
  cardPid: string;
  type: "inline-card";
}

export class CardNode extends DecoratorNode<React.ReactNode> {
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

  getCardKind(): InlineKind {
    return this.__cardKind;
  }

  getCardLabel(): string {
    return this.__cardLabel;
  }

  getCardPid(): string {
    return this.__cardPid;
  }

  createDOM(_config: EditorConfig): HTMLElement {
    const el = document.createElement("span");
    el.className = `code-card code-card--${this.__cardKind}`;
    el.setAttribute("data-card-kind", this.__cardKind);
    el.setAttribute("data-card-pid", this.__cardPid);
    el.textContent = this.__cardLabel;
    return el;
  }

  updateDOM(): boolean {
    return false; // no dynamic updates needed
  }

  decorate(): React.ReactNode {
    return React.createElement(
      "span",
      {
        className: `code-card code-card--${this.__cardKind}`,
        "data-card-kind": this.__cardKind,
        "data-card-pid": this.__cardPid,
      },
      this.__cardLabel,
    );
  }

  exportJSON(): SerializedCardNode {
    return {
      ...super.exportJSON(),
      type: "inline-card",
      cardKind: this.__cardKind,
      cardLabel: this.__cardLabel,
      cardPid: this.__cardPid,
    };
  }

  static importJSON(_serializedNode: SerializedLexicalNode & Record<string, unknown>): CardNode {
    const serialized = _serializedNode as unknown as SerializedCardNode;
    return $createCardNode(serialized.cardKind, serialized.cardLabel, serialized.cardPid);
  }

  isInline(): boolean {
    return true;
  }
}

export function $isCardNode(node: LexicalNode | null | undefined): node is CardNode {
  return node instanceof CardNode;
}

export function $createCardNode(kind: InlineKind, label: string, pid: string): CardNode {
  return new CardNode(kind, label, pid);
}
