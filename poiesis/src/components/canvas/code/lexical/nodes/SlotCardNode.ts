import {
  ElementNode,
  type SerializedElementNode,
  type LexicalNode,
  type EditorConfig,
  type NodeKey,
  type Spread,
  type LexicalEditor,
} from "lexical";
import type { SlotKind } from "../../../../../core/types/editor-node";

export interface SerializedSlotCardNode extends Spread<
  { slotKind: SlotKind; value: string; pid: string; type: "slot-card" },
  SerializedElementNode
> {}

const SLOT_PLACEHOLDERS: Record<SlotKind, string> = {
  name: "变量名",       // 变量名
  type_value: "类型",        // 类型
  expression: "值",              // 值
  condition: "条件",         // 条件
  fn_name: "函数名",     // 函数名
};
// Placeholder map reserved for future CSS ::before content usage.
void SLOT_PLACEHOLDERS;

export class SlotCardNode extends ElementNode {
  __slotKind: SlotKind;
  __value: string;
  __pid: string;

  static getType(): string {
    return "slot-card";
  }

  static clone(node: SlotCardNode): SlotCardNode {
    return new SlotCardNode(node.__value, node.__slotKind, node.__pid, node.__key);
  }

  constructor(value: string, slotKind: SlotKind, pid: string, key?: NodeKey) {
    super(key);
    this.__value = value;
    this.__slotKind = slotKind;
    this.__pid = pid;
  }

  // ── Inline atomic card in display mode ──
  isInline(): boolean {
    return true;
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

  // ── DOM: contenteditable="false" in display mode ──
  createDOM(_config: EditorConfig): HTMLElement {
    const el = document.createElement("span");
    el.className = `slot-card slot-card--${this.__slotKind}`;
    el.setAttribute("data-slot-kind", this.__slotKind);
    el.setAttribute("data-slot-pid", this.__pid);
    el.setAttribute("contenteditable", "false");
    el.tabIndex = -1;
    el.textContent = this.__value || "";
    // Empty state: CSS ::before handles placeholder via .slot-card--{kind}:empty::before
    if (!this.__value) {
      el.dataset.empty = "true";
    }
    return el;
  }

  updateDOM(
    prevNode: SlotCardNode,
    dom: HTMLElement,
    _config: EditorConfig,
  ): boolean {
    if (prevNode.__slotKind !== this.__slotKind) {
      dom.className = `slot-card slot-card--${this.__slotKind}`;
      dom.setAttribute("data-slot-kind", this.__slotKind);
    }
    if (prevNode.__value !== this.__value) {
      dom.textContent = this.__value || "";
      if (this.__value) {
        delete dom.dataset.empty;
      } else {
        dom.dataset.empty = "true";
      }
    }
    return false;
  }

  // ── Public API ──
  getSlotKind(): SlotKind {
    return this.getLatest().__slotKind;
  }

  getValue(): string {
    return this.getLatest().__value;
  }

  setValue(v: string): void {
    const self = this.getWritable();
    (self as SlotCardNode).__value = v;
  }

  getPid(): string {
    return this.getLatest().__pid;
  }

  /**
   * Activate editing mode: make DOM contenteditable and focus.
   * Called by KeyboardPlugin when this node is highlighted and Enter is pressed.
   * Returns a cleanup function to restore display mode.
   */
  activateEditing(editor: LexicalEditor, dom: HTMLElement): () => void {
    dom.setAttribute("contenteditable", "true");
    dom.classList.add("slot-card--editing");
    dom.focus();

    // Place cursor at end of text
    const range = document.createRange();
    const sel = window.getSelection();
    if (dom.firstChild) {
      range.setStart(dom.firstChild, dom.textContent?.length ?? 0);
    } else {
      range.setStart(dom, 0);
    }
    range.collapse(true);
    sel?.removeAllRanges();
    sel?.addRange(range);

    const handleKeydown = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        e.stopPropagation();
        this.deactivateEditing(dom);
        // Commit value back to Lexical
        editor.update(() => {
          this.setValue(dom.textContent || "");
        });
      } else if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        // Restore original value
        dom.textContent = this.__value || "";
        this.deactivateEditing(dom);
      } else if (e.key === "ArrowLeft" || e.key === "ArrowRight" ||
                 e.key === "ArrowUp" || e.key === "ArrowDown") {
        // Allow ← → for in-slot character movement, but don't let
        // the event bubble to Lexical which would move cursor out.
        // Actually, we want ← → to work within the slot; we need to
        // NOT prevent default for left/right (they work within contenteditable).
        // Only block up/down (they could jump to other lines).
        if (e.key === "ArrowUp" || e.key === "ArrowDown") {
          e.preventDefault();
          e.stopPropagation();
        }
      }
    };

    dom.addEventListener("keydown", handleKeydown);

    return () => {
      this.deactivateEditing(dom);
      dom.removeEventListener("keydown", handleKeydown);
    };
  }

  deactivateEditing(dom: HTMLElement): void {
    dom.setAttribute("contenteditable", "false");
    dom.classList.remove("slot-card--editing");
    dom.blur();
  }

  // ── Serialization ──
  exportJSON(): SerializedSlotCardNode {
    return {
      ...super.exportJSON(),
      type: "slot-card",
      slotKind: this.__slotKind,
      value: this.__value,
      pid: this.__pid,
    };
  }

  static importJSON(serialized: SerializedSlotCardNode): SlotCardNode {
    const node = $createSlotCardNode(
      serialized.value,
      serialized.slotKind,
      serialized.pid,
    );
    node.setFormat(serialized.format);
    node.setIndent(serialized.indent);
    node.setDirection(serialized.direction);
    return node;
  }
}

export function $isSlotCardNode(node: LexicalNode | null | undefined): node is SlotCardNode {
  return node instanceof SlotCardNode;
}

export function $createSlotCardNode(value: string, slotKind: SlotKind, pid: string): SlotCardNode {
  return new SlotCardNode(value, slotKind, pid);
}
