// poiesis/src/components/canvas/code/lexical/nodes/BodyLineNode.ts
import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type Spread,
} from "lexical";

export interface SerializedBodyLineNode extends Spread<
  { bodyLinePid: string; branchIndex?: number; type: "body-line" },
  SerializedElementNode
> {}

export class BodyLineNode extends ElementNode {
  __bodyLinePid: string;
  __branchIndex: number | undefined;
  __isTail: boolean;

  static getType(): string {
    return "body-line";
  }

  static clone(node: BodyLineNode): BodyLineNode {
    const clone = new BodyLineNode(node.__bodyLinePid, node.__branchIndex, node.__key);
    clone.__isTail = node.__isTail;
    return clone;
  }

  constructor(pid: string, branchIndex?: number, key?: string) {
    super(key);
    this.__bodyLinePid = pid;
    this.__branchIndex = branchIndex;
    this.__isTail = false;
  }

  getBodyLinePid(): string {
    return this.getLatest().__bodyLinePid;
  }

  getBranchIndex(): number | undefined {
    return (this.getLatest() as BodyLineNode).__branchIndex;
  }

  setTailMarker(v: boolean): void {
    const self = this.getWritable();
    (self as BodyLineNode).__isTail = v;
  }

  isTailMarker(): boolean {
    return (this.getLatest() as BodyLineNode).__isTail;
  }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = "code-body-line" + (this.__isTail ? " code-body-line--tail" : "");
    el.setAttribute("data-body-line-pid", this.__bodyLinePid);
    if (this.__branchIndex !== undefined) {
      el.setAttribute("data-branch-index", String(this.__branchIndex));
    }
    if (this.__isTail) {
      el.setAttribute("data-tail", "true");
    }
    return el;
  }

  updateDOM(_prev: BodyLineNode, dom: HTMLElement): boolean {
    if (this.__isTail) {
      dom.classList.add("code-body-line--tail");
      dom.setAttribute("data-tail", "true");
    }
    return false;
  }

  exportJSON(): SerializedBodyLineNode {
    return {
      ...super.exportJSON(),
      type: "body-line",
      bodyLinePid: this.__bodyLinePid,
      branchIndex: this.__branchIndex,
    };
  }

  static importJSON(serialized: SerializedBodyLineNode): BodyLineNode {
    const node = $createBodyLineNode(serialized.bodyLinePid, serialized.branchIndex);
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

export function $createBodyLineNode(pid: string, branchIndex?: number): BodyLineNode {
  return new BodyLineNode(pid, branchIndex);
}
