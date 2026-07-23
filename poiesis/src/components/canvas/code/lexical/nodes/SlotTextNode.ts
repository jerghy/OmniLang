import { TextNode, type SerializedTextNode, type LexicalNode } from "lexical";

export type SlotType =
  | "name"       // variable name
  | "type"       // type annotation (: i32)
  | "value"      // init value / expression
  | "condition"  // if/while condition
  | "collection" // for..in collection
  | "item"       // for..in item variable
  | "fnName"     // function name in call
  | "arg";       // function argument

export interface SerializedSlotTextNode extends SerializedTextNode {
  slot: SlotType;
  type: "slot-text";
}

export class SlotTextNode extends TextNode {
  __slot: SlotType;

  static getType(): string {
    return "slot-text";
  }

  static clone(node: SlotTextNode): SlotTextNode {
    return new SlotTextNode(node.__text, node.__slot, node.__key);
  }

  constructor(text: string, slotType: SlotType, key?: string) {
    super(text, key);
    this.__slot = slotType;
  }

  getSlotType(): SlotType {
    const self = this.getLatest();
    return (self as SlotTextNode).__slot;
  }

  setSlotType(slotType: SlotType): void {
    const self = this.getWritable();
    (self as SlotTextNode).__slot = slotType;
  }

  exportJSON(): SerializedSlotTextNode {
    return {
      ...super.exportJSON(),
      type: "slot-text",
      slot: this.__slot,
    };
  }

  static importJSON(serialized: SerializedSlotTextNode): SlotTextNode {
    const node = $createSlotTextNode(serialized.text, serialized.slot);
    node.setFormat(serialized.format);
    node.setDetail(serialized.detail);
    node.setMode(serialized.mode);
    node.setStyle(serialized.style);
    return node;
  }
}

export function $isSlotTextNode(node: LexicalNode | null | undefined): node is SlotTextNode {
  return node instanceof SlotTextNode;
}

export function $createSlotTextNode(text: string, slotType: SlotType): SlotTextNode {
  return new SlotTextNode(text, slotType);
}
