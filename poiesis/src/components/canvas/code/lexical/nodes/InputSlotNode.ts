import { TextNode, type SerializedTextNode, type LexicalNode, type EditorConfig } from "lexical";
import type { SlotKind } from "../../../../../core/types/editor-node";

export interface SerializedInputSlotNode extends SerializedTextNode {
  slotKind: SlotKind;
  type: "input-slot";
}

export class InputSlotNode extends TextNode {
  __slotKind: SlotKind;

  static getType(): string {
    return "input-slot";
  }

  static clone(node: InputSlotNode): InputSlotNode {
    return new InputSlotNode(node.__text, node.__slotKind, node.__key);
  }

  constructor(text: string, slotKind: SlotKind, key?: string) {
    super(text, key);
    this.__slotKind = slotKind;
  }

  getSlotKind(): SlotKind {
    const self = this.getLatest();
    return (self as InputSlotNode).__slotKind;
  }

  setSlotKind(kind: SlotKind): void {
    const self = this.getWritable();
    (self as InputSlotNode).__slotKind = kind;
  }

  createDOM(config: EditorConfig): HTMLElement {
    const dom = super.createDOM(config);
    dom.className = `input-slot input-slot--${this.__slotKind}`;
    dom.setAttribute("data-slot-kind", this.__slotKind);
    return dom;
  }

  exportJSON(): SerializedInputSlotNode {
    return {
      ...super.exportJSON(),
      type: "input-slot",
      slotKind: this.__slotKind,
    };
  }

  static importJSON(serialized: SerializedInputSlotNode): InputSlotNode {
    const node = $createInputSlotNode(serialized.text, serialized.slotKind);
    node.setFormat(serialized.format);
    node.setDetail(serialized.detail);
    node.setMode(serialized.mode);
    node.setStyle(serialized.style);
    return node;
  }
}

export function $isInputSlotNode(node: LexicalNode | null | undefined): node is InputSlotNode {
  return node instanceof InputSlotNode;
}

export function $createInputSlotNode(text: string, slotKind: SlotKind): InputSlotNode {
  return new InputSlotNode(text, slotKind);
}
