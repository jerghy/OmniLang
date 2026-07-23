// poiesis/src/components/canvas/code/lexical/nodes/BodyLineNode.ts
import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type Spread,
} from "lexical";

export interface SerializedBodyLineNode extends Spread<
  { bodyLinePid: string; type: "body-line" },
  SerializedElementNode
> {}

export class BodyLineNode extends ElementNode {
  __bodyLinePid: string;

  static getType(): string {
    return "body-line";
  }

  static clone(node: BodyLineNode): BodyLineNode {
    return new BodyLineNode(node.__bodyLinePid, node.__key);
  }

  constructor(pid: string, key?: string) {
    super(key);
    this.__bodyLinePid = pid;
  }

  getBodyLinePid(): string {
    return this.getLatest().__bodyLinePid;
  }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = "code-body-line";
    el.setAttribute("data-body-line-pid", this.__bodyLinePid);
    return el;
  }

  updateDOM(): boolean {
    return true;
  }

  exportJSON(): SerializedBodyLineNode {
    return {
      ...super.exportJSON(),
      type: "body-line",
      bodyLinePid: this.__bodyLinePid,
    };
  }

  static importJSON(serialized: SerializedBodyLineNode): BodyLineNode {
    const node = $createBodyLineNode(serialized.bodyLinePid);
    node.setFormat(serialized.format);
    node.setIndent(serialized.indent);
    node.setDirection(serialized.direction);
    return node;
  }

  canBeEmpty(): boolean {
    return true;
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

export function $isBodyLineNode(
  node: LexicalNode | null | undefined,
): node is BodyLineNode {
  return node instanceof BodyLineNode;
}

export function $createBodyLineNode(pid: string): BodyLineNode {
  return new BodyLineNode(pid);
}
