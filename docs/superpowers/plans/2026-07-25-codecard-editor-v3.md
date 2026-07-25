# CodeCard Editor v3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace InputSlotNode (always-editable TextNode) with SlotCardNode (ElementNode with display/edit dual-mode), simplify cursor model from 3 states to 2, and remove container-level delete.

**Architecture:** New `SlotCardNode extends ElementNode` with `contenteditable="false"` in display mode. When highlighted + Enter, DOM switches to `contenteditable="true"` inline editing. KeyboardPlugin simplified: no more `detectRightEdge` or container-level delete. ← → navigates between pipe cursor and card highlight.

**Tech Stack:** TypeScript, React, Lexical, Vitest

## Global Constraints

- All existing `vitest run` tests must pass after each task
- All Storybook 10 stories must render correctly after each task
- `tsc --noEmit` must produce 0 errors after each task
- `.omni` source emit/parse roundtrip must be preserved
- No changes to `core/types/code-op.ts`
- No changes to `services/codegen/` directory

---

## File Structure Map

```
lexical/
├── nodes/
│   ├── CardNode.ts              [MODIFY] — flat card visual via CSS
│   ├── ContainerNode.ts         [MODIFY] — remove right-edge visual support
│   ├── BlockNode.ts             [NO CHANGE]
│   ├── BodyLineNode.ts          [NO CHANGE]
│   ├── InputSlotNode.ts         [DELETE] — replaced by SlotCardNode.ts
│   ├── SlotCardNode.ts          [CREATE] — new ElementNode with dual-mode
│   ├── index.ts                 [MODIFY] — export SlotCardNode, remove InputSlotNode
│   └── __tests__/
│       ├── CardNode.test.ts     [MODIFY] — minor updates
│       ├── InputSlotNode.test.ts [DELETE]
│       ├── SlotCardNode.test.ts [CREATE] — new tests
│       └── ContainerNode.test.ts [MODIFY] — minor updates if needed
├── plugins/
│   ├── KeyboardPlugin.ts        [REWRITE] — simplified, no right-edge
│   ├── AutocompletePlugin.tsx   [MODIFY] — create SlotCardNode
│   ├── CodeOpSyncPlugin.ts      [MODIFY] — SlotCardNode create/extract
│   └── __tests__/
│       └── KeyboardPlugin.test.ts [REWRITE] — new test scenarios
├── transforms/
│   ├── convertToNodes.ts        [MODIFY] — slot → SlotCardNode
│   ├── convertToOps.ts          [MODIFY] — SlotCardNode → slot
│   └── slot-operations.ts       [MODIFY] — create SlotCardNode
├── autocomplete/
│   ├── autocomplete-rules.ts    [NO CHANGE]
│   └── AutocompleteMenu.tsx     [NO CHANGE]
├── CodeCardEditor.module.css    [REWRITE] — flat cards, slot dual-mode
└── __tests__/
    └── integration-v2.test.ts   [MODIFY] — update references
```

---

### Task 1: Create SlotCardNode (ElementNode)

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/nodes/SlotCardNode.ts`
- Create: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/SlotCardNode.test.ts`
- Modify: `poiesis/src/components/canvas/code/lexical/nodes/index.ts`

**Interfaces:**
- Produces: `SlotCardNode` class, `$createSlotCardNode(value, slotKind, pid)`, `$isSlotCardNode(node)`, `SerializedSlotCardNode` interface
- Produces: `SlotCardNode.getSlotKind()`, `SlotCardNode.getValue()`, `SlotCardNode.setValue(v)`, `SlotCardNode.activateEditing(editor)`, `SlotCardNode.deactivateEditing()`

- [ ] **Step 1: Write SlotCardNode tests**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/__tests__/SlotCardNode.test.ts
import { describe, it, expect } from "vitest";
import { createEditor } from "lexical";
import { SlotCardNode, $createSlotCardNode, $isSlotCardNode } from "../SlotCardNode";

describe("SlotCardNode (ElementNode, dual-mode)", () => {
  it("has correct type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("hello", "name", "s1");
      expect(SlotCardNode.getType()).toBe("slot-card");
      expect(node.getSlotKind()).toBe("name");
      expect(node.getValue()).toBe("hello");
      expect(node.getPid()).toBe("s1");
    });
  });

  it("is detected by $isSlotCardNode", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("x", "expression", "s2");
      expect($isSlotCardNode(node)).toBe(true);
      expect($isSlotCardNode(null)).toBe(false);
    });
  });

  it("is inline ElementNode with cursor access", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("count", "name", "s3");
      expect(node.isInline()).toBe(true);
      expect(node.canBeEmpty()).toBe(true);
      expect(node.canInsertTextBefore()).toBe(true);
      expect(node.canInsertTextAfter()).toBe(true);
    });
  });

  it("serializes to JSON with slot fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("counter", "name", "s4");
      const json = node.exportJSON();
      expect(json.type).toBe("slot-card");
      expect(json.slotKind).toBe("name");
      expect(json.value).toBe("counter");
      expect(json.pid).toBe("s4");
    });
  });

  it("deserializes from JSON", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = SlotCardNode.importJSON({
        type: "slot-card",
        slotKind: "expression",
        value: "42",
        pid: "s5",
        version: 1,
        children: [],
        format: 0,
        indent: 0,
        direction: null,
      } as any);
      expect($isSlotCardNode(node)).toBe(true);
      expect(node.getSlotKind()).toBe("expression");
      expect(node.getValue()).toBe("42");
    });
  });

  it("clones preserving all fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("result", "expression", "s6");
      const clone = SlotCardNode.clone(node);
      expect(clone.getSlotKind()).toBe("expression");
      expect(clone.getValue()).toBe("result");
      expect(clone.getPid()).toBe("s6");
    });
  });

  it("setValue updates the value", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createSlotCardNode("old", "expression", "s7");
      node.setValue("new");
      expect(node.getValue()).toBe("new");
    });
  });

  it("creates DOM with contenteditable=false in display mode", () => {
    const editor = createEditor({ nodes: [SlotCardNode] });
    editor.update(() => {
      const node = $createSlotCardNode("count", "name", "s8");
      const dom = node.createDOM({
        namespace: "test",
        theme: {} as any,
      } as any);
      expect(dom.getAttribute("contenteditable")).toBe("false");
      expect(dom.tagName).toBe("SPAN");
      expect(dom.getAttribute("data-slot-kind")).toBe("name");
      expect(dom.className).toContain("slot-card");
      expect(dom.className).toContain("slot-card--name");
      expect(dom.textContent).toBe("count");
    });
  });

  it("shows placeholder when value is empty", () => {
    const editor = createEditor({ nodes: [SlotCardNode] });
    editor.update(() => {
      const node = $createSlotCardNode("", "name", "s9");
      const dom = node.createDOM({
        namespace: "test",
        theme: {} as any,
      } as any);
      // Empty slot shows placeholder via CSS ::before, DOM may show empty
      expect(dom.textContent).toBe("");
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd poiesis && npx vitest run src/components/canvas/code/lexical/nodes/__tests__/SlotCardNode.test.ts`
Expected: FAIL — `SlotCardNode` not found

- [ ] **Step 3: Implement SlotCardNode**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/SlotCardNode.ts
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd poiesis && npx vitest run src/components/canvas/code/lexical/nodes/__tests__/SlotCardNode.test.ts`
Expected: 9 tests PASS

- [ ] **Step 5: Update nodes/index.ts — export SlotCardNode, remove InputSlotNode**

```typescript
// poiesis/src/components/canvas/code/lexical/nodes/index.ts
export { CardNode, $createCardNode, $isCardNode } from "./CardNode";
export type { SerializedCardNode } from "./CardNode";

export { ContainerElementNode, $createContainerElementNode, $isContainerElementNode } from "./ContainerNode";
export type { SerializedContainerElementNode } from "./ContainerNode";

export { BlockElementNode, $createBlockElementNode, $isBlockElementNode } from "./BlockNode";
export type { SerializedBlockElementNode } from "./BlockNode";

export { BodyLineNode, $createBodyLineNode, $isBodyLineNode } from "./BodyLineNode";
export type { SerializedBodyLineNode } from "./BodyLineNode";

export { SlotCardNode, $createSlotCardNode, $isSlotCardNode } from "./SlotCardNode";
export type { SerializedSlotCardNode } from "./SlotCardNode";
```

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/SlotCardNode.ts \
        poiesis/src/components/canvas/code/lexical/nodes/__tests__/SlotCardNode.test.ts \
        poiesis/src/components/canvas/code/lexical/nodes/index.ts
git commit -m "feat: add SlotCardNode ElementNode with display/edit dual-mode"
```

---

### Task 2: Update slot-operations.ts to use SlotCardNode

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts`

**Interfaces:**
- Consumes: `$createSlotCardNode`, `$isSlotCardNode` from `../nodes/SlotCardNode`
- Produces: Same public API `buildCursorContext`, `inferSlotKind`, `canCreateSlot`, `createSlotAtCursor` (but returns `SlotCardNode`)

- [ ] **Step 1: Update imports and createSlotAtCursor**

Replace all imports and usages of `InputSlotNode` with `SlotCardNode`:

```typescript
// poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts
// ... (lines 1-13 unchanged, but replace line 14)
import { $createSlotCardNode, type SlotCardNode } from "../nodes/SlotCardNode";
// ... (lines 14-128 unchanged)
// Update createSlotAtCursor (line 143-153):
export function createSlotAtCursor(
  ctx: CursorContext,
  parentNode: LexicalNode | null,
): SlotCardNode | null {
  const slotKind = inferSlotKind(ctx);
  if (!slotKind || parentNode === null || !$isElementNode(parentNode)) return null;

  const slot = $createSlotCardNode("", slotKind, `slot-${Date.now()}`);
  parentNode.append(slot);
  return slot;
}
```

- [ ] **Step 2: Verify build**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: 0 errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts
git commit -m "feat: update slot-operations to use SlotCardNode instead of InputSlotNode"
```

---

### Task 3: Update CodeOpSyncPlugin for SlotCardNode

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts`

**Interfaces:**
- Consumes: `$createSlotCardNode`, `$isSlotCardNode` from SlotCardNode
- Produces: Same `CodeOpSyncPlugin` component, `editorNodeToLexical`, `lexicalToEditorNode`

- [ ] **Step 1: Replace InputSlotNode references with SlotCardNode**

In `CodeOpSyncPlugin.ts`:

Line 16: Replace `$createInputSlotNode, $isInputSlotNode` import with:
```typescript
import { $createSlotCardNode, $isSlotCardNode } from "../nodes/SlotCardNode";
```

Lines 140-142 (in `editorNodeToLexical`): Replace InputSlot case:
```typescript
case "slot": {
  const sl = node as InputSlot;
  return $createSlotCardNode(sl.value, sl.slotKind, sl.pid);
}
```

Lines 270-277 (in `lexicalToEditorNode`): Replace InputSlot detection:
```typescript
if ($isSlotCardNode(lexicalNode)) {
  return {
    kind: "slot",
    slotKind: lexicalNode.getSlotKind(),
    value: lexicalNode.getValue(),
    pid: lexicalNode.getPid(),
  } as InputSlot;
}
```

- [ ] **Step 2: Verify roundtrip tests still pass**

Run: `cd poiesis && npx vitest run src/components/canvas/code/lexical/__tests__/integration-v2.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
git commit -m "feat: update CodeOpSyncPlugin to use SlotCardNode"
```

---

### Task 4: Update AutocompletePlugin for SlotCardNode

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx`

**Interfaces:**
- Consumes: `$createSlotCardNode` from SlotCardNode
- Produces: Same AutocompletePlugin component

- [ ] **Step 1: Replace InputSlotNode import and usage**

Line 27: Replace import:
```typescript
import { $createSlotCardNode } from "../nodes/SlotCardNode";
```

Lines 336-358 (in `createDefaultSlot`): Replace all three `$createInputSlotNode` calls with `$createSlotCardNode`:
```typescript
function createDefaultSlot(
  containerNode: ReturnType<typeof $createContainerElementNode>,
  kind: string,
): void {
  switch (kind) {
    case "let":
    case "set": {
      const nameSlot = $createSlotCardNode("", "name", uid("sc"));
      containerNode.append(nameSlot);
      nameSlot.selectStart();
      break;
    }
    case "return":
    case "await": {
      const exprSlot = $createSlotCardNode("", "expression", uid("sc"));
      containerNode.append(exprSlot);
      exprSlot.selectStart();
      break;
    }
    case "call": {
      const fnSlot = $createSlotCardNode("", "fn_name", uid("sc"));
      containerNode.append(fnSlot);
      fnSlot.selectStart();
      break;
    }
    default:
      containerNode.selectStart();
  }
}
```

- [ ] **Step 2: Verify TypeScript**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: 0 errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx
git commit -m "feat: update AutocompletePlugin to create SlotCardNode"
```

---

### Task 5: Rewrite KeyboardPlugin (simplified v3)

**Files:**
- Rewrite: `poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts`
- Rewrite: `poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts`

**Interfaces:**
- Consumes: `$isCardNode`, `$isSlotCardNode`, `SlotCardNode`, `$isBodyLineNode`, `$createBodyLineNode`
- Produces: Same `KeyboardPlugin` component (no props, returns null)

- [ ] **Step 1: Write KeyboardPlugin v3 tests**

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot } from "lexical";
import { $isCardNode, $createCardNode, CardNode } from "../../nodes/CardNode";
import { $isSlotCardNode, $createSlotCardNode, SlotCardNode } from "../../nodes/SlotCardNode";
import { $isBodyLineNode, $createBodyLineNode, BodyLineNode } from "../../nodes/BodyLineNode";
import { ContainerElementNode, $createContainerElementNode } from "../../nodes/ContainerNode";
import { BlockElementNode } from "../../nodes/BlockNode";

describe("KeyboardPlugin v3 — simplified 2-state cursor", () => {
  // ── CardNode as inline ElementNode ──
  it("CardNode is inline ElementNode with cursor access", () => {
    const editor = createEditor();
    editor.update(() => {
      const card = $createCardNode("operator", "=", "c1");
      expect(card.isInline()).toBe(true);
      expect(card.canBeEmpty()).toBe(true);
      expect(card.canInsertTextBefore()).toBe(true);
      expect(card.canInsertTextAfter()).toBe(true);
    });
  });

  // ── SlotCardNode is inline ElementNode ──
  it("SlotCardNode is inline ElementNode with cursor access", () => {
    const editor = createEditor();
    editor.update(() => {
      const slot = $createSlotCardNode("count", "name", "s1");
      expect(slot.isInline()).toBe(true);
      expect(slot.canBeEmpty()).toBe(true);
      expect(slot.canInsertTextBefore()).toBe(true);
      expect(slot.canInsertTextAfter()).toBe(true);
    });
  });

  // ── Delete: remove a highlighted card ──
  it("Delete removes a card via remove()", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const card = $createCardNode("type", "i32", "c2");
      root.append(card);
      expect($isCardNode(card)).toBe(true);
      card.remove();
      expect(root.getChildren().length).toBe(0);
    });
  });

  // ── Delete: remove a slot card ──
  it("Delete removes a slot card via remove()", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const slot = $createSlotCardNode("x", "name", "s2");
      root.append(slot);
      expect($isSlotCardNode(slot)).toBe(true);
      slot.remove();
      expect(root.getChildren().length).toBe(0);
    });
  });

  // ── Enter: body line → new line ──
  it("Enter at end of body line creates new line", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const bodyLine = $createBodyLineNode("bl-1");
      root.append(bodyLine);
      const newLine = $createBodyLineNode("bl-2");
      bodyLine.insertAfter(newLine);
      expect(root.getChildren().length).toBe(2);
      expect($isBodyLineNode(root.getChildren()[1])).toBe(true);
    });
  });

  // ── Tab: operator cycling in set container ──
  it("Tab in set container cycles operator", () => {
    const editor = createEditor();
    editor.update(() => {
      const setContainer = $createContainerElementNode("set", "set1");
      const opCard = $createCardNode("operator", "=", "op1");
      const valSlot = $createSlotCardNode("x", "expression", "s3");
      setContainer.append(opCard, valSlot);

      const operators = ["=", "+=", "-=", "*=", "/=", "%="];
      const current = operators.indexOf(opCard.getCardLabel());
      const next = (current + 1) % operators.length;

      const newNode = $createCardNode("operator", operators[next], opCard.getCardPid());
      opCard.replace(newNode);

      expect(newNode.getCardLabel()).toBe("+=");
      expect(newNode.getCardKind()).toBe("operator");
    });
  });

  // ── SlotCardNode: activateEditing sets contenteditable ──
  it("SlotCardNode.activateEditing makes DOM editable", () => {
    const editor = createEditor({ nodes: [SlotCardNode] });
    editor.update(() => {
      const slot = $createSlotCardNode("hello", "name", "s4");
      const dom = slot.createDOM({ namespace: "test", theme: {} as any } as any);
      expect(dom.getAttribute("contenteditable")).toBe("false");

      const cleanup = slot.activateEditing(editor, dom);
      expect(dom.getAttribute("contenteditable")).toBe("true");
      expect(dom.classList.contains("slot-card--editing")).toBe(true);

      cleanup();
      expect(dom.getAttribute("contenteditable")).toBe("false");
      expect(dom.classList.contains("slot-card--editing")).toBe(false);
    });
  });

  // ── No more container right-edge or container-level delete ──
  it("Container removal is not special — just remove() like any node", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const container = $createContainerElementNode("let", "let1");
      const slot = $createSlotCardNode("x", "name", "s5");
      container.append(slot);
      root.append(container);

      // No special right-edge logic. Just remove the container
      container.remove();
      expect(root.getChildren().length).toBe(0);
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd poiesis && npx vitest run src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts`
Expected: FAIL (old test references InputSlotNode, new tests not yet passing)

- [ ] **Step 3: Implement KeyboardPlugin v3**

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
// v3: Simplified 2-state cursor (pipe | and card highlight).
// No more container right-edge detection or container-level delete.
// Enter activates SlotCardNode editing, or creates new body line.
import { useEffect } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  KEY_DELETE_COMMAND,
  KEY_BACKSPACE_COMMAND,
  KEY_ENTER_COMMAND,
  KEY_TAB_COMMAND,
  COMMAND_PRIORITY_LOW,
  $getSelection,
  $isRangeSelection,
  $isTextNode,
} from "lexical";
import { $isCardNode, $createCardNode } from "../nodes/CardNode";
import { $isSlotCardNode, type SlotCardNode } from "../nodes/SlotCardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBodyLineNode, $createBodyLineNode } from "../nodes/BodyLineNode";

export function KeyboardPlugin(): null {
  const [editor] = useLexicalComposerContext();

  // ── Delete ──
  // v3: No container right-edge logic. Browser handles card deletion natively.
  // We only intercept to prevent Lexical from doing its own paragraph deletion.
  useEffect(() => {
    return editor.registerCommand(
      KEY_DELETE_COMMAND,
      (_event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // If the selection has a highlighted card (contenteditable="false" span),
        // the browser will handle deletion natively. Let it through.
        // Lexical's default Delete behavior would try to delete a character,
        // which fails on contenteditable="false" elements anyway.
        return false; // let browser handle
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Backspace ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_BACKSPACE_COMMAND,
      (_event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Backspace at very start of a container → no-op
        const anchor = selection.anchor;
        const anchorNode = anchor.getNode();
        if (anchor.offset === 0 && $isTextNode(anchorNode)) {
          const parent = anchorNode.getParent();
          if (parent !== null && $isContainerElementNode(parent)) {
            const firstChild = parent.getFirstChild();
            if (firstChild === anchorNode) {
              return true; // swallow
            }
          }
        }

        return false; // let browser handle
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Enter: activate SlotCardNode editing, or create new body line ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent | null) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Check if a SlotCardNode is highlighted (selected)
        const anchorNode = selection.anchor.getNode();
        const slotCard = findSlotCardAtCursor(anchorNode);
        if (slotCard !== null) {
          event?.preventDefault();
          // Activate editing on the slot card's DOM element
          const editorEl = document.querySelector('[contenteditable="true"]');
          const slotDom = editorEl?.querySelector(
            `[data-slot-pid="${slotCard.getPid()}"]`
          ) as HTMLElement | null;
          if (slotDom) {
            slotCard.activateEditing(editor, slotDom);
          }
          return true;
        }

        // Enter inside a body line or at the right edge of one → create new line below
        const bodyLine = findAncestorBodyLine(selection);
        if (bodyLine !== null) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          bodyLine.insertAfter(newLine);
          newLine.selectStart();
          return true;
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Tab: cycle operator in set container ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_TAB_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;
        const anchorNode = selection.anchor.getNode();

        const cardNode = findCardAtCursor(anchorNode, selection);
        if (cardNode !== null && $isCardNode(cardNode) && cardNode.getCardKind() === "operator") {
          const parent = cardNode.getParent();
          if (
            parent !== null &&
            $isContainerElementNode(parent) &&
            parent.getContainerKind() === "set"
          ) {
            event.preventDefault();
            const operators = ["=", "+=", "-=", "*=", "/=", "%="];
            const current = operators.indexOf(cardNode.getCardLabel());
            const next = (current + 1) % operators.length;
            const newNode = $createCardNode(
              "operator",
              operators[next],
              cardNode.getCardPid(),
            );
            cardNode.replace(newNode);
            newNode.selectNext();
            return true;
          }
        }

        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  return null;
}

// ── Helpers ──

function findAncestorBodyLine(selection: ReturnType<typeof $getSelection>): ReturnType<typeof $getSelection> extends infer S ? any : never {
  if (!$isRangeSelection(selection)) return null;
  const anchorNode = selection.anchor.getNode();
  let node: any = anchorNode;
  while (node !== null) {
    if ($isBodyLineNode(node)) return node;
    node = node.getParent();
  }
  return null;
}

/**
 * Find a SlotCardNode at or near the cursor position.
 */
function findSlotCardAtCursor(anchorNode: any): SlotCardNode | null {
  if ($isSlotCardNode(anchorNode)) return anchorNode;

  const parent = anchorNode.getParent();
  if (parent === null) return null;

  // Check if we're inside a SlotCardNode (it traps cursor in display mode)
  if ($isSlotCardNode(parent)) return parent;

  return null;
}

/**
 * Find a CardNode at or near the cursor position.
 */
function findCardAtCursor(
  anchorNode: any,
  selection: any,
): any | null {
  if ($isCardNode(anchorNode)) return anchorNode;

  const parent = anchorNode.getParent();
  if (parent === null) return null;

  const offset = selection.anchor.offset;

  if (offset === 0) {
    const prev = anchorNode.getPreviousSibling();
    if ($isCardNode(prev)) return prev;
  }

  if (offset === anchorNode.getTextContentSize()) {
    const next = anchorNode.getNextSibling();
    if ($isCardNode(next)) return next;
  }

  if ($isCardNode(parent)) return parent;

  return null;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd poiesis && npx vitest run src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts`
Expected: 7 tests PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts \
        poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
git commit -m "feat: rewrite KeyboardPlugin v3 — simplified 2-state cursor, Enter activates SlotCardNode"
```

---

### Task 6: Update CSS — flat cards + slot dual-mode styles

**Files:**
- Modify: `poiesis/src/components/canvas/code/CodeCardEditor.module.css`

- [ ] **Step 1: Add slot-card styles and remove block cursor styles**

Remove these CSS blocks:
- `.contentEditable :global(.code-container.at-right-edge)` and its animation (lines 174-193)
- `.contentEditable :global(.code-body-line.at-right-edge)` (lines 180-183)
- `@keyframes block-blink` (lines 186-189)
- `.contentEditable :global(.code-container)::after` (lines 213-222)

Add these new styles:

```css
/* ── Slot card display mode (flat, semi-transparent) ── */
.contentEditable :global(.slot-card) {
  display: inline-block;
  padding: 1px 8px;
  margin: 1px 2px;
  border-radius: 5px;
  font-size: 13px;
  font-family: "JetBrains Mono", "Fira Code", monospace;
  line-height: 1.6;
  white-space: nowrap;
  cursor: default;
  transition: box-shadow 0.1s;
}

/* Slot card color variants — lighter than keyword cards, same color family */
.contentEditable :global(.slot-card--name)       { background: rgba(167, 139, 250, 0.15); color: #7c3aed; }
.contentEditable :global(.slot-card--type_value) { background: rgba(34, 211, 238, 0.12); color: #0e7490; }
.contentEditable :global(.slot-card--expression) { background: rgba(96, 165, 250, 0.12); color: #1d4ed8; }
.contentEditable :global(.slot-card--condition)  { background: rgba(251, 191, 36, 0.15); color: #b45309; }
.contentEditable :global(.slot-card--fn_name)    { background: rgba(34, 211, 238, 0.12); color: #0e7490; }

/* Empty slot placeholder */
.contentEditable :global(.slot-card--name[data-empty="true"])::before       { content: "变量名"; color: #cbd5e1; }
.contentEditable :global(.slot-card--type_value[data-empty="true"])::before { content: "类型"; color: #cbd5e1; }
.contentEditable :global(.slot-card--expression[data-empty="true"])::before { content: "值"; color: #cbd5e1; }
.contentEditable :global(.slot-card--condition[data-empty="true"])::before  { content: "条件"; color: #cbd5e1; }
.contentEditable :global(.slot-card--fn_name[data-empty="true"])::before    { content: "函数名"; color: #cbd5e1; }

/* ── Slot card editing mode ── */
.contentEditable :global(.slot-card--editing) {
  background: var(--ol-surface, #ffffff) !important;
  color: var(--ol-text, #1a1a1a) !important;
  border: 1.5px solid var(--ol-color-primary, #c96442);
  box-shadow: 0 0 0 2px rgba(201, 100, 66, 0.15);
  outline: none;
  min-width: 60px;
  cursor: text;
}

/* ── Slot card highlight (same as card highlight) ── */
.contentEditable :global(.slot-card.highlighted) {
  outline: 2px solid var(--ol-color-primary, #c96442);
  outline-offset: 1px;
}

/* ── Remove old input-slot styles (from InputSlotNode TextNode era) ── */
/* Delete lines 246-269: .input-slot, .input-slot:focus, .input-slot--*::before */
/* These are replaced by .slot-card styles above */
```

- [ ] **Step 2: Remove old InputSlotNode CSS blocks**

Delete from CodeCardEditor.module.css:
- Lines 115-135: `.contentEditable :global(.slot-text--name)`, `.slot-text--type`, `.slot-text--value` (v1 slot remnants)
- Lines 245-270: `.contentEditable :global(.input-slot)` through empty slot placeholders

- [ ] **Step 3: Flat card visual — remove border-radius extremes, keep cards clean**

Change line 89 `.code-card` border-radius from `2px` to `4px`:
```css
.contentEditable :global(.code-card) {
  border-radius: 4px;  /* was 2px — slightly softer but still flat */
}
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/CodeCardEditor.module.css
git commit -m "feat: flat card + slot dual-mode CSS for v3"
```

---

### Task 7: Update convertToOps.ts for SlotCardNode value extraction

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/transforms/convertToOps.ts`

- [ ] **Step 1: Update findSlot to read from slot value**

The `findSlot` function at line 35-37 reads `(c as InputSlot).value`. Since SlotCardNode stores value internally and `convertToOps` operates on `EditorNode[]` (not Lexical nodes), the `InputSlot.value` field still works. But we need to ensure `CodeOpSyncPlugin.lexicalToEditorNode` extracts `value` from SlotCardNode correctly. That's already done in Task 3.

No code changes needed in `convertToOps.ts` — the `EditorNode` interface still uses `InputSlot.value`.

- [ ] **Step 1 (alternative): Verify all tests pass**

Run: `cd poiesis && npx vitest run src/components/canvas/code/lexical/__tests__/integration-v2.test.ts`
Expected: All tests PASS

- [ ] **Step 2: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/transforms/convertToOps.ts
git commit -m "chore: verify convertToOps works with SlotCardNode values"
```

---

### Task 8: Add SlotCardNode to LexicalComposer config + update CodeCardEditor.tsx

**Files:**
- Modify: `poiesis/src/components/canvas/code/CodeCardEditor.tsx`

- [ ] **Step 1: Register SlotCardNode in lexicalConfig**

Line 11-14: Add SlotCardNode to nodes array:
```typescript
import { SlotCardNode } from "./lexical/nodes/SlotCardNode";

const lexicalConfig = {
  namespace: "code-card-editor-v3",
  nodes: [CardNode, ContainerElementNode, BlockElementNode, BodyLineNode, SlotCardNode],
  onError: (error: Error, _editor: LexicalEditor) => {
    console.error("Lexical error:", error);
  },
};
```

Update import (line 16): remove `InputSlotNode` import, add `SlotCardNode`:
```typescript
import { SlotCardNode } from "./lexical/nodes/SlotCardNode";
```

- [ ] **Step 2: Verify TypeScript**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: 0 errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/CodeCardEditor.tsx
git commit -m "feat: register SlotCardNode in LexicalComposer config"
```

---

### Task 9: Delete InputSlotNode and old tests

**Files:**
- Delete: `poiesis/src/components/canvas/code/lexical/nodes/InputSlotNode.ts`
- Delete: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/InputSlotNode.test.ts`

- [ ] **Step 1: Verify no remaining references**

Run: `cd poiesis && grep -r "InputSlotNode" poiesis/src/ --include="*.ts" --include="*.tsx" | grep -v "node_modules" | grep -v ".test.ts" | grep -v "SlotCardNode"`
Expected: No output (all references already migrated)

- [ ] **Step 2: Delete files**

```bash
rm poiesis/src/components/canvas/code/lexical/nodes/InputSlotNode.ts
rm poiesis/src/components/canvas/code/lexical/nodes/__tests__/InputSlotNode.test.ts
```

- [ ] **Step 3: Verify build and tests**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -10`
Expected: 0 errors

Run: `cd poiesis && npx vitest run`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add -u poiesis/src/components/canvas/code/lexical/nodes/
git commit -m "chore: delete InputSlotNode — replaced by SlotCardNode"
```

---

### Task 10: Update integration tests + full validation

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/__tests__/integration-v2.test.ts`

- [ ] **Step 1: Verify integration tests still pass**

Run: `cd poiesis && npx vitest run src/components/canvas/code/lexical/__tests__/integration-v2.test.ts`
Expected: All tests PASS

- [ ] **Step 2: Run full test suite**

Run: `cd poiesis && npx vitest run`
Expected: All tests PASS

- [ ] **Step 3: Full TypeScript check**

Run: `cd poiesis && npx tsc --noEmit`
Expected: 0 errors

- [ ] **Step 4: Storybook smoke test**

Run: `bash /d/space/OmniLang/.claude/skills/start-storybook/start-storybook.sh`
Navigate to http://localhost:6006
Verify all 10 CodeCardEditor stories render correctly

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/lexical/__tests__/
git commit -m "test: verify integration tests pass with v3 SlotCardNode"
```

---

## Verification Checklist

- [ ] Task 1: SlotCardNode 9 tests pass
- [ ] Task 2: slot-operations builds clean
- [ ] Task 3: CodeOpSyncPlugin integration tests pass
- [ ] Task 4: AutocompletePlugin builds clean
- [ ] Task 5: KeyboardPlugin v3 7 tests pass
- [ ] Task 6: CSS clean, no old InputSlotNode styles
- [ ] Task 7: convertToOps integration tests pass
- [ ] Task 8: CodeCardEditor.tsx registers SlotCardNode
- [ ] Task 9: InputSlotNode fully deleted, all references migrated
- [ ] Task 10: Full vitest run passes, tsc 0 errors, Storybook 10 stories render
