// poiesis/src/components/canvas/code/lexical/nodes/ContainerNode.ts
import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type Spread,
} from "lexical";
import type { ContainerKind } from "../../../../../core/types/editor-node";

export interface SerializedContainerElementNode extends Spread<
  { containerKind: ContainerKind; containerPid: string; type: "container" },
  SerializedElementNode
> {}

export class ContainerElementNode extends ElementNode {
  __containerKind: ContainerKind;
  __containerPid: string;

  static getType(): string {
    return "container";
  }

  static clone(node: ContainerElementNode): ContainerElementNode {
    return new ContainerElementNode(node.__containerKind, node.__containerPid, node.__key);
  }

  constructor(kind: ContainerKind, pid: string, key?: string) {
    super(key);
    this.__containerKind = kind;
    this.__containerPid = pid;
  }

  getContainerKind(): ContainerKind {
    return this.getLatest().__containerKind;
  }

  getContainerPid(): string {
    return this.getLatest().__containerPid;
  }

  createDOM(): HTMLElement {
    const el = document.createElement("div");
    el.className = `code-container code-container--${this.__containerKind}`;
    el.setAttribute("data-container-kind", this.__containerKind);
    el.setAttribute("data-container-pid", this.__containerPid);
    return el;
  }

  updateDOM(_prev: ContainerElementNode, _dom: HTMLElement): boolean {
    return true;
  }

  exportJSON(): SerializedContainerElementNode {
    return {
      ...super.exportJSON(),
      type: "container",
      containerKind: this.__containerKind,
      containerPid: this.__containerPid,
    };
  }

  static importJSON(serialized: SerializedContainerElementNode): ContainerElementNode {
    const node = $createContainerElementNode(serialized.containerKind, serialized.containerPid);
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

export function $isContainerElementNode(
  node: LexicalNode | null | undefined,
): node is ContainerElementNode {
  return node instanceof ContainerElementNode;
}

export function $createContainerElementNode(
  kind: ContainerKind,
  pid: string,
): ContainerElementNode {
  return new ContainerElementNode(kind, pid);
}
