# CodeCard Editor v2: 光标交互修复 + 代码清理 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 CardNode 使用 DecoratorNode 导致的"光标只能出现在输入框"问题，实现设计文档 §3 定义的光标三态交互，清理死代码。

**Architecture:** CardNode 从 DecoratorNode 改为 ElementNode + `contenteditable="false"`，利用浏览器原生原子卡片行为实现光标三态。KeyboardPlugin 负责浏览器不懂的 Delete/Enter 特殊场景。`slot-operations.ts` 抽取 `\` 输入框推断逻辑。

**Tech Stack:** Lexical 0.30+, React 18, TypeScript 5, Vitest, Playwright (Storybook)

## Global Constraints

- 0 个 TS 错误（`tsc --noEmit` 必须全部通过）
- 所有现有 vitest 测试必须通过
- Storybook 10 个 story 全部正确渲染
- 不创建新的缺失插件（CardFlow/Clipboard/SyntaxLint），只修已有代码
- 公共 API（`getCardKind`, `getCardLabel`, `getCardPid`, `$createCardNode`, `$isCardNode`）保持不变

---

### Task 1: 删除死代码文件

**Files:**
- Delete: `poiesis/src/components/canvas/code/lexical/nodes/CardElementNode.ts`
- Delete: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardElementNode.test.ts`
- Delete: `poiesis/src/components/canvas/code/lexical/nodes/SlotTextNode.ts`
- Delete: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/SlotTextNode.test.ts`

**Interfaces:**
- Consumes: none
- Produces: none (pure deletion)

- [ ] **Step 1: 删除 4 个死文件**

```bash
rm poiesis/src/components/canvas/code/lexical/nodes/CardElementNode.ts
rm poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardElementNode.test.ts
rm poiesis/src/components/canvas/code/lexical/nodes/SlotTextNode.ts
rm poiesis/src/components/canvas/code/lexical/nodes/__tests__/SlotTextNode.test.ts
```

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
cd poiesis && npx tsc --noEmit --pretty 2>&1 | head -20
```

预期：0 错误。

- [ ] **Step 3: 运行现有测试确认无回归**

```bash
cd poiesis && npx vitest run --reporter=verbose 2>&1 | tail -20
```

预期：所有测试通过，无 FAIL。

- [ ] **Step 4: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/
git commit -m "chore(codecard-v2): delete v1 dead code (CardElementNode, SlotTextNode)"
```

---

### Task 2: CSS 清理

**Files:**
- Modify: `poiesis/src/components/canvas/code/CodeCardEditor.module.css:112-121`

**Interfaces:**
- Consumes: none
- Produces: `.code-card` 不再有 `user-select: none` / `cursor: default`

- [ ] **Step 1: 删除 `user-select: none` 和 `cursor: default`**

文件 `poiesis/src/components/canvas/code/CodeCardEditor.module.css`，找到 `.code-card` 块（约 line 112-121），删除 `user-select: none;` 和 `cursor: default;`：

```css
/* 当前（lines 112-121）： */
.contentEditable :global(.code-card) {
  display: inline;
  font-family: inherit;
  font-size: inherit;
  padding: 0 3px;
  border-radius: 2px;
  cursor: default;        /* ← 删这行 */
  user-select: none;       /* ← 删这行 */
  white-space: nowrap;
}

/* 改为： */
.contentEditable :global(.code-card) {
  display: inline;
  font-family: inherit;
  font-size: inherit;
  padding: 0 3px;
  border-radius: 2px;
  white-space: nowrap;
}
```

- [ ] **Step 2: 验证 Storybook 渲染**

```bash
# 确认 Storybook 仍在运行（端口 6006）
# 在浏览器中确认卡片仍然正常显示
```

- [ ] **Step 3: 提交**

```bash
git add poiesis/src/components/canvas/code/CodeCardEditor.module.css
git commit -m "fix(codecard-v2): remove user-select:none and cursor:default from .code-card"
```

---

### Task 3: CardNode 架构修复 — 核心重写

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts` (完整重写)

**Interfaces:**
- Consumes: `InlineKind` from `core/types/editor-node`
- Produces: `CardNode` class (now extends ElementNode), `$createCardNode(kind, label, pid)`, `$isCardNode(node)`, `SerializedCardNode`

- [ ] **Step 1: 重写 CardNode.ts**

将 `poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts` 替换为以下内容：

```typescript
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
```

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
cd poiesis && npx tsc --noEmit --pretty 2>&1 | head -30
```

预期：0 错误（CardNode 相关的改动）。

- [ ] **Step 3: 检查 Storybook 渲染**

在浏览器中打开 http://localhost:6006，确认 "Full example" story 卡片正确渲染。卡片的 `contenteditable="false"` 应该让它们在 DOM 中显示为不可编辑的 span。

- [ ] **Step 4: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts
git commit -m "refactor(codecard-v2): rewrite CardNode from DecoratorNode to ElementNode with contenteditable=false"
```

---

### Task 4: 更新 CardNode.test.ts

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts`

**Interfaces:**
- Consumes: `CardNode` (Task 3), `$createCardNode`, `$isCardNode`
- Produces: updated tests

- [ ] **Step 1: 重写测试文件**

将 `poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts` 替换为：

```typescript
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode } from "lexical";
import { CardNode, $createCardNode, $isCardNode } from "../CardNode";

describe("CardNode (ElementNode)", () => {
  it("has correct type", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("operator", "=", "c1");
      expect(CardNode.getType()).toBe("inline-card");
      expect(node.getCardKind()).toBe("operator");
      expect(node.getCardLabel()).toBe("=");
      expect(node.getCardPid()).toBe("c1");
    });
  });

  it("is detected by $isCardNode", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("type", "i32", "c2");
      expect($isCardNode(node)).toBe(true);
      expect($isCardNode(null)).toBe(false);
      expect($isCardNode(undefined)).toBe(false);
    });
  });

  it("serializes to JSON with all fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("literal", "42", "c3");
      const json = node.exportJSON();
      expect(json.type).toBe("inline-card");
      expect(json.cardKind).toBe("literal");
      expect(json.cardLabel).toBe("42");
      expect(json.cardPid).toBe("c3");
      // ElementNode serialization fields
      expect(json).toHaveProperty("children");
      expect(json).toHaveProperty("format");
    });
  });

  it("deserializes from JSON", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = CardNode.importJSON({
        type: "inline-card",
        cardKind: "operator",
        cardLabel: "+=",
        cardPid: "c4",
        version: 1,
        children: [],
        format: 0,
        indent: 0,
        direction: null,
      } as any);
      expect($isCardNode(node)).toBe(true);
      expect(node.getCardKind()).toBe("operator");
      expect(node.getCardLabel()).toBe("+=");
      expect(node.getCardPid()).toBe("c4");
    });
  });

  it("is inline", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("mut", "mut", "c5");
      expect(node.isInline()).toBe(true);
    });
  });

  it("can be empty (no Lexical-managed text children)", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("var_ref", "count", "c6");
      expect(node.canBeEmpty()).toBe(true);
    });
  });

  it("allows cursor before and after", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("separator", ":", "c7");
      expect(node.canInsertTextBefore()).toBe(true);
      expect(node.canInsertTextAfter()).toBe(true);
    });
  });

  it("clones preserving all fields", () => {
    const editor = createEditor();
    editor.update(() => {
      const node = $createCardNode("var_ref", "count", "c8");
      const clone = CardNode.clone(node);
      expect(clone.getCardKind()).toBe("var_ref");
      expect(clone.getCardLabel()).toBe("count");
      expect(clone.getCardPid()).toBe("c8");
    });
  });

  it("DOM has contenteditable=false", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const p = $createParagraphNode();
      const card = $createCardNode("operator", "=", "c9");
      p.append(card);
      root.append(p);
    });

    // Check DOM attribute after rendering
    editor.getEditorState().read(() => {
      const root = $getRoot();
      const card = root.getFirstChild()?.getFirstChild();
      expect($isCardNode(card)).toBe(true);
    });
  });
});
```

- [ ] **Step 2: 运行测试确认通过**

```bash
cd poiesis && npx vitest run src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts --reporter=verbose
```

预期：8 个测试全部 PASS。

- [ ] **Step 3: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts
git commit -m "test(codecard-v2): update CardNode tests for ElementNode migration"
```

---

### Task 5: 适配 CodeOpSyncPlugin

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts:125-145, 260-278`

**Interfaces:**
- Consumes: `CardNode` (Task 3), `$createCardNode`, `$isCardNode`, `$createInputSlotNode`
- Produces: unchanged (same function signatures, same return types)

- [ ] **Step 1: 检查并适配卡片创建（`editorNodeToLexical`）**

在 `poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts` 中，`editorNodeToLexical` 函数的 inline node 处理（约 line 126-145）。当前代码：

```typescript
case "mut":
case "break":
case "continue":
case "operator":
case "type":
case "var_ref":
case "literal":
case "dot":
case "separator":
case "comment": {
  const inNode = node as InlineNode;
  return $createCardNode(inNode.kind, inNode.cardLabel, inNode.pid);
}
```

这段代码**不需要改**。`$createCardNode(kind, label, pid)` 签名不变，只是内部实现从 DecoratorNode 变成了 ElementNode。

- [ ] **Step 2: 检查并适配卡片提取（`lexicalToEditorNode`）**

在同一文件中，`lexicalToEditorNode` 函数的 card 提取（约 line 263-269）。当前代码：

```typescript
if ($isCardNode(lexicalNode)) {
  return {
    kind: lexicalNode.getCardKind(),
    cardLabel: lexicalNode.getCardLabel(),
    pid: lexicalNode.getCardPid(),
  } as InlineNode;
}
```

这段代码**不需要改**。`getCardKind()`, `getCardLabel()`, `getCardPid()` API 保持不变。

- [ ] **Step 3: 检查卡片遍历（`extractFromNode`）**

`extractFromNode` 函数（约 line 167-188）遍历 ElementNode children。CardNode 现在是 ElementNode，`getChildren()` 返回空数组（因为 `canBeEmpty() === true`，没有子节点）。`lexicalToEditorNode` 在 CardNode 上直接返回 InlineNode，不会进入 children 遍历。**不需要改。**

- [ ] **Step 4: 运行 roundtrip 测试确认无回归**

```bash
cd poiesis && npx vitest run src/components/canvas/code/lexical/__tests__/integration-v2.test.ts --reporter=verbose
```

预期：所有 roundtrip 测试通过。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
git commit -m "fix(codecard-v2): verify CodeOpSyncPlugin compatible with ElementNode CardNode"
```

---

### Task 6: 适配 KeyboardPlugin — CardNode 检测

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts`

**Interfaces:**
- Consumes: `CardNode` (Task 3), `ContainerElementNode`, `BlockElementNode`, `BodyLineNode`
- Produces: 适配后的 Delete/Backspace/Enter/Tab 处理

- [ ] **Step 1: 重写 KeyboardPlugin.ts**

将 `poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts` 替换为：

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
// v2 cursor keyboard: browser handles ←→↑↓ natively via contenteditable="false" cards.
// We handle: Delete/Backspace at container right-edge, Enter for new body lines, Tab for operator cycling.
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
  type RangeSelection,
  type LexicalNode,
} from "lexical";
import { $isCardNode, $createCardNode } from "../nodes/CardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBlockElementNode } from "../nodes/BlockNode";
import { $isBodyLineNode, $createBodyLineNode } from "../nodes/BodyLineNode";

export function KeyboardPlugin(): null {
  const [editor] = useLexicalComposerContext();

  // ── Delete ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_DELETE_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        const rightEdgeNode = detectRightEdge(selection);
        if (rightEdgeNode !== null) {
          event.preventDefault();
          if ($isContainerElementNode(rightEdgeNode)) {
            // Container right-edge block cursor → delete whole container
            rightEdgeNode.remove();
            return true;
          }
          if ($isBodyLineNode(rightEdgeNode)) {
            // Body line right-edge → delete this line (keep at least one child)
            const parent = rightEdgeNode.getParent();
            if (parent !== null) {
              const siblingCount = parent.getChildren().filter(
                (c) => $isBodyLineNode(c),
              ).length;
              if (siblingCount > 1) {
                rightEdgeNode.remove();
              }
            }
            return true;
          }
        }

        // If a card is selected (browser's atomic selection via contenteditable="false"),
        // the browser will handle deletion natively. Return false to let it through.
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Backspace ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_BACKSPACE_COMMAND,
      (event: KeyboardEvent) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Backspace at very start of a container → no-op (cannot delete container prefix)
        const anchor = selection.anchor;
        const anchorNode = anchor.getNode();
        if (anchor.offset === 0 && $isTextNode(anchorNode)) {
          const parent = anchorNode.getParent();
          if (parent !== null && $isContainerElementNode(parent)) {
            const firstChild = parent.getFirstChild();
            if (firstChild === anchorNode) {
              event.preventDefault();
              return true; // swallow backspace at container start
            }
          }
        }

        // Otherwise let browser handle (card deletion, text deletion, etc.)
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor]);

  // ── Enter: new body line ──
  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent | null) => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) return false;

        // Enter at body line right-edge or inside body line → insert new line below
        const withinBodyLine = findAncestorBodyLine(selection);
        if (withinBodyLine !== null) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          withinBodyLine.insertAfter(newLine);
          newLine.selectStart();
          return true;
        }

        // Enter after a container (right-edge) → insert paragraph after
        const rightEdge = detectRightEdge(selection);
        if (rightEdge !== null && $isContainerElementNode(rightEdge)) {
          event?.preventDefault();
          const newLine = $createBodyLineNode("bl-" + Date.now());
          rightEdge.insertAfter(newLine);
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

        // Check if the node at cursor or its neighboring sibling is an operator card
        const cardNode = findCardAtCursor(anchorNode, selection);
        if (cardNode !== null && cardNode.getCardKind() === "operator") {
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

/**
 * Detect if the collapsed cursor is at the rightmost edge of a container or body line.
 * Returns the container/body-line node if so, or null.
 */
function detectRightEdge(
  selection: RangeSelection,
): LexicalNode | null {
  if (!selection.isCollapsed()) return null;

  const anchor = selection.anchor;
  const anchorNode = anchor.getNode();

  // Cursor must be at the very end of the anchor node
  if (anchor.offset !== anchorNode.getTextContentSize()) return null;

  // Walk up: is this node the last child of a container or body line?
  let node: LexicalNode | null = anchorNode;
  while (node !== null) {
    const parent = node.getParent();
    if (parent === null) return null;

    const lastChild = parent.getLastChild();
    if (lastChild !== node) return null; // not the last child → not at right edge

    if ($isContainerElementNode(parent)) return parent;
    if ($isBodyLineNode(parent)) return parent;
    if ($isBlockElementNode(parent)) return null; // don't delete block from inside

    node = parent;
  }
  return null;
}

/**
 * Find the closest ancestor BodyLineNode that contains the selection.
 */
function findAncestorBodyLine(selection: RangeSelection): LexicalNode | null {
  const anchorNode = selection.anchor.getNode();
  let node: LexicalNode | null = anchorNode;
  while (node !== null) {
    if ($isBodyLineNode(node)) return node;
    node = node.getParent();
  }
  return null;
}

/**
 * Find a CardNode at or near the cursor position.
 * Since cards are contenteditable="false", the cursor may be at a text node
 * adjacent to the card rather than inside it.
 */
function findCardAtCursor(
  anchorNode: LexicalNode,
  selection: RangeSelection,
): LexicalNode | null {
  // Direct hit: cursor is somehow on a card node
  if ($isCardNode(anchorNode)) return anchorNode;

  const parent = anchorNode.getParent();
  if (parent === null) return null;

  const offset = selection.anchor.offset;

  // Cursor at start of text node → check previous sibling
  if (offset === 0) {
    const prev = anchorNode.getPreviousSibling();
    if ($isCardNode(prev)) return prev;
  }

  // Cursor at end of text node → check next sibling
  if (offset === anchorNode.getTextContentSize()) {
    const next = anchorNode.getNextSibling();
    if ($isCardNode(next)) return next;
  }

  // Check if parent itself is a card node
  if ($isCardNode(parent)) return parent;

  return null;
}
```

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
cd poiesis && npx tsc --noEmit --pretty 2>&1 | head -30
```

预期：0 错误。

- [ ] **Step 3: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/KeyboardPlugin.ts
git commit -m "refactor(codecard-v2): rewrite KeyboardPlugin with right-edge detection and cursor navigation"
```

---

### Task 7: 更新 KeyboardPlugin.test.ts

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts`

**Interfaces:**
- Consumes: `KeyboardPlugin` (Task 6 重写), `$createCardNode`, `$createContainerElementNode`, etc.
- Produces: updated tests

- [ ] **Step 1: 重写测试文件**

将 `poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts` 替换为：

```typescript
// poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
import { describe, it, expect } from "vitest";
import { createEditor, $getRoot, $createParagraphNode, $createTextNode } from "lexical";
import { $isCardNode, $createCardNode } from "../../nodes/CardNode";
import { $isContainerElementNode, $createContainerElementNode } from "../../nodes/ContainerNode";
import { $isBodyLineNode, $createBodyLineNode } from "../../nodes/BodyLineNode";
import { $createInputSlotNode, $isInputSlotNode } from "../../nodes/InputSlotNode";

describe("KeyboardPlugin v2 — node operations", () => {
  // ── CardNode as ElementNode ──
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

  // ── Delete: container at right edge ──
  it("Delete at right edge of container removes entire container with children", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const container = $createContainerElementNode("let", "let1");
      const nameSlot = $createInputSlotNode("count", "name");
      container.append(nameSlot);
      root.append(container);

      // Verify container exists and has children
      expect($isContainerElementNode(container)).toBe(true);
      expect(container.getChildren().length).toBe(1);

      // Simulate Delete at right edge: remove container
      container.remove();

      // Container removed
      expect(root.getChildren().length).toBe(0);
    });
  });

  // ── Delete: empty body line ──
  it("Delete on empty body line removes it when siblings exist", () => {
    const editor = createEditor();
    editor.update(() => {
      const root = $getRoot();
      const line1 = $createBodyLineNode("bl-1");
      const line2 = $createBodyLineNode("bl-2");
      root.append(line1, line2);

      expect(root.getChildren().length).toBe(2);

      // Simulate Delete on line2: remove it
      line2.remove();

      expect(root.getChildren().length).toBe(1);
      expect(root.getFirstChild()).toBe(line1);
    });
  });

  // ── Enter: body line insertion ──
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
      const valSlot = $createInputSlotNode("x", "expression");
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

  // ── Delete: card (browser native) ──
  it("Card can be removed via remove()", () => {
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
});
```

- [ ] **Step 2: 运行测试**

```bash
cd poiesis && npx vitest run src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts --reporter=verbose
```

预期：6 个测试全部 PASS。

- [ ] **Step 3: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/__tests__/KeyboardPlugin.test.ts
git commit -m "test(codecard-v2): update KeyboardPlugin tests for v2 cursor model"
```

---

### Task 8: 创建 slot-operations.ts

**Files:**
- Create: `poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts`

**Interfaces:**
- Consumes: `SlotKind` from `core/types/editor-node`, Lexical `$isRangeSelection`, `$isCardNode`, `$isContainerElementNode`, `$isBlockElementNode`, `$isInputSlotNode`, `$createInputSlotNode`
- Produces: `CursorContext`, `buildCursorContext(selection)`, `canCreateSlot(ctx)`, `inferSlotKind(ctx)`, `createSlotAtCursor(ctx, parentNode)`

- [ ] **Step 1: 创建文件**

创建 `poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts`：

```typescript
// poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts
// \ (backslash) input slot inference and creation.
// Extracted from AutocompletePlugin for testability and separation of concerns.
import {
  $getSelection,
  $isRangeSelection,
  $isRootNode,
  type RangeSelection,
  type LexicalNode,
} from "lexical";
import type { SlotKind } from "../../../../../core/types/editor-node";
import { $isCardNode } from "../nodes/CardNode";
import { $isContainerElementNode } from "../nodes/ContainerNode";
import { $isBlockElementNode } from "../nodes/BlockNode";
import { $isInputSlotNode, $createInputSlotNode, type InputSlotNode } from "../nodes/InputSlotNode";

export interface CursorContext {
  parentKind: string | null;
  precedingCardKind: string | null;
  precedingCardLabel: string | null;
  grandparentKind: string | null;
  isTopLevel: boolean;
}

/**
 * Build cursor context from the current range selection.
 * Must be called inside editor.update() or editorState.read().
 */
export function buildCursorContext(selection: RangeSelection): CursorContext {
  const anchor = selection.anchor;
  const anchorNode = anchor.getNode();
  const offset = anchor.offset;

  const parent = anchorNode.getParent();
  const grandparent = parent?.getParent() ?? null;

  const parentKind = getKindName(parent);
  const grandparentKind = getKindName(grandparent);

  // Determine if we're at the top level (root or paragraph under root)
  const isTopLevel =
    parent === null ||
    $isRootNode(parent) ||
    ($isRootNode(grandparent) && parent?.getType() === "paragraph");

  // Find the preceding card (card before the cursor)
  let precedingCardKind: string | null = null;
  let precedingCardLabel: string | null = null;

  if (offset === 0 && $isCardNode(anchorNode.getPreviousSibling())) {
    const prev = anchorNode.getPreviousSibling()!;
    precedingCardKind = prev.getType() === "inline-card"
      ? (prev as any).getCardKind() : null;
    precedingCardLabel = prev.getType() === "inline-card"
      ? (prev as any).getCardLabel() : null;
  } else {
    // Walk backwards through siblings to find the nearest card
    let sibling = anchorNode.getPreviousSibling();
    while (sibling !== null) {
      if ($isCardNode(sibling)) {
        precedingCardKind = sibling.getCardKind();
        precedingCardLabel = sibling.getCardLabel();
        break;
      }
      sibling = sibling.getPreviousSibling();
    }
  }

  return {
    parentKind,
    precedingCardKind,
    precedingCardLabel,
    grandparentKind,
    isTopLevel,
  };
}

/**
 * Determine the slot kind based on cursor context.
 * Returns null if no slot should be created at this position.
 */
export function inferSlotKind(ctx: CursorContext): SlotKind | null {
  // let / set at start → variable name
  if ((ctx.parentKind === "let" || ctx.parentKind === "set") &&
      ctx.precedingCardKind === null) {
    return "name";
  }

  // for at start → item variable name
  if (ctx.parentKind === "for" && ctx.precedingCardKind === null) {
    return "name";
  }

  // After ':' → type annotation
  if (ctx.precedingCardLabel === ":") {
    return "type_value";
  }

  // After 'in' → collection expression
  if (ctx.precedingCardLabel === "in") {
    return "expression";
  }

  // After assignment operator → value expression
  if (ctx.precedingCardLabel === "=" ||
      /^[+\-*/%]=$/.test(ctx.precedingCardLabel ?? "")) {
    return "expression";
  }

  // if / while head → condition
  if (ctx.parentKind === "if" || ctx.parentKind === "while") {
    return "condition";
  }

  // call at start → function name
  if (ctx.parentKind === "call" && ctx.precedingCardKind === null) {
    return "fn_name";
  }

  // return / await → expression
  if (ctx.parentKind === "return" || ctx.parentKind === "await") {
    return "expression";
  }

  // Inside a body line (grandparent is a block) → expression
  if (ctx.grandparentKind !== null && ctx.parentKind === null) {
    return "expression";
  }

  return null;
}

/**
 * Check if a backslash slot can be created at the current cursor position.
 */
export function canCreateSlot(ctx: CursorContext): boolean {
  if (ctx.isTopLevel) return false;
  return inferSlotKind(ctx) !== null;
}

/**
 * Create an InputSlotNode at the cursor position.
 * Returns the created node or null if slot cannot be created.
 * Must be called inside editor.update().
 */
export function createSlotAtCursor(
  ctx: CursorContext,
  parentNode: LexicalNode | null,
): InputSlotNode | null {
  const slotKind = inferSlotKind(ctx);
  if (!slotKind || parentNode === null) return null;

  const slot = $createInputSlotNode("", slotKind);
  parentNode.append(slot);
  return slot;
}

// ── Private helpers ──

function getKindName(node: LexicalNode | null): string | null {
  if (node === null) return null;
  if ($isContainerElementNode(node)) return node.getContainerKind();
  if ($isBlockElementNode(node)) return node.getBlockKind();
  return null;
}
```

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
cd poiesis && npx tsc --noEmit --pretty 2>&1 | head -30
```

预期：0 错误。

- [ ] **Step 3: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/transforms/slot-operations.ts
git commit -m "feat(codecard-v2): add slot-operations.ts for backslash input slot inference"
```

---

### Task 9: 重构 AutocompletePlugin — `\` 委托给 slot-operations

**Files:**
- Modify: `poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx`

**Interfaces:**
- Consumes: `slot-operations.ts` (Task 8), `CardNode` (Task 3)
- Produces: unchanged autocomplete functionality, `\` handler now delegates

- [ ] **Step 1: 更新导入和重写 `\` 处理**

`AutocompletePlugin.tsx` 的改动分两部分：

**A. 添加 import（在文件顶部现有 imports 之后）：**

```typescript
import {
  buildCursorContext,
  canCreateSlot,
  createSlotAtCursor,
} from "../transforms/slot-operations";
```

**B. 替换 `\` handler（约 line 128-167）：**

将现有的 `registerRootListener` 中的 `\` keydown handler 替换为调用 slot-operations：

```typescript
// 替换 AutocompletePlugin.tsx 中约 line 127-167 的整个 useEffect
// ── Backslash → insert input slot (delegated to slot-operations) ──
useEffect(() => {
  const removeRootListener = editor.registerRootListener(
    (rootElement: HTMLElement | null) => {
      if (!rootElement) return;
      const handler = (e: KeyboardEvent) => {
        if (e.key !== "\\") return;
        e.preventDefault();
        editor.update(() => {
          const selection = $getSelection();
          if (!$isRangeSelection(selection)) return;

          const ctx = buildCursorContext(selection);
          if (!canCreateSlot(ctx)) return;

          const parent = selection.anchor.getNode().getParent();
          const slot = createSlotAtCursor(ctx, parent);
          if (slot) {
            slot.select();
            onCreateSlot?.();
          }
        });
      };
      rootElement.addEventListener("keydown", handler);
      return () => rootElement.removeEventListener("keydown", handler);
    },
  );
  return () => removeRootListener?.();
}, [editor, onCreateSlot]);
```

**C. 删除旧的 `inferSlotKind` 和 `hasSlotKind` 函数（约 line 314-354）：**

这两个函数现在在 `slot-operations.ts` 中，从 AutocompletePlugin.tsx 删除。

- [ ] **Step 2: 验证 TypeScript 编译**

```bash
cd poiesis && npx tsc --noEmit --pretty 2>&1 | head -30
```

预期：0 错误。

- [ ] **Step 3: 运行现有测试**

```bash
cd poiesis && npx vitest run --reporter=verbose 2>&1 | tail -20
```

预期：所有测试通过。

- [ ] **Step 4: 提交**

```bash
git add poiesis/src/components/canvas/code/lexical/plugins/AutocompletePlugin.tsx
git commit -m "refactor(codecard-v2): delegate backslash input slot logic to slot-operations.ts"
```

---

### Task 10: CSS 打磨 — 全量

**Files:**
- Modify: `poiesis/src/components/canvas/code/CodeCardEditor.module.css`

**Interfaces:**
- Consumes: CardNode (`.code-card`), ContainerElementNode (`.code-container`), BodyLineNode (`.code-body-line`)
- Produces: cursor visual styles

- [ ] **Step 1: 新增 CSS 规则**

在 `poiesis/src/components/canvas/code/CodeCardEditor.module.css` 中：

**替换 `.code-card` 基础样式块（约 line 112-121）：**

```css
/* ══════════════════════════════════════════
   Cards — inline tokens within blocks
   ══════════════════════════════════════════ */
.contentEditable :global(.code-card) {
  display: inline;
  font-family: inherit;
  font-size: inherit;
  padding: 0 3px;
  border-radius: 2px;
  white-space: nowrap;
  transition: outline-color 120ms ease, box-shadow 120ms ease, filter 120ms ease;
  cursor: default;
}

.contentEditable :global(.code-card:hover) {
  filter: brightness(0.95);
}
```

**在 "Card focus" 块后（约 line 139 之后）新增：**

```css
/* Card highlight — when card is selected as atomic unit (browser selection) */
.contentEditable :global(.code-card.highlighted) {
  outline: 2px solid #4c4cff;
  outline-offset: 0;
  box-shadow: 0 0 0 1.5px rgba(76, 76, 255, 0.25);
}

/* Container right-edge block cursor */
.contentEditable :global(.code-container.at-right-edge) {
  border-right: 2px solid #4c4cff;
  padding-right: 6px;
}

.contentEditable :global(.code-body-line.at-right-edge) {
  border-right: 2px solid #4c4cff;
  padding-right: 6px;
}

/* Right-edge block cursor blink animation */
@keyframes block-blink {
  0%, 100% { border-right-color: #4c4cff; }
  50% { border-right-color: rgba(76, 76, 255, 0.15); }
}

.contentEditable :global(.code-container.at-right-edge) {
  animation: block-blink 1.2s ease-in-out infinite;
}
```

**在 input-slot 块后（约 line 218 之后）新增：**

```css
/* Input slot focus transition */
.contentEditable :global(.input-slot) {
  transition: border-color 150ms ease, box-shadow 150ms ease;
}
```

- [ ] **Step 2: 验证 Storybook 渲染和样式**

在浏览器中打开 http://localhost:6006，确认：
- 卡片 hover 有亮度变化
- 卡片选中（点击）有蓝色 outline

- [ ] **Step 3: 提交**

```bash
git add poiesis/src/components/canvas/code/CodeCardEditor.module.css
git commit -m "style(codecard-v2): add cursor states, hover transitions, block-cursor blink"
```

---

### Task 11: 全量验证

**Files:**
- None (验证 only)

- [ ] **Step 1: 运行全部 TypeScript 编译**

```bash
cd poiesis && npx tsc --noEmit --pretty 2>&1
```

预期：0 错误。

- [ ] **Step 2: 运行全部 vitest 测试**

```bash
cd poiesis && npx vitest run --reporter=verbose 2>&1
```

预期：所有测试 PASS，无 FAIL，无 skip。

- [ ] **Step 3: Storybook 验证**

在 Playwright 浏览器中依次打开 10 个 story，确认：
- 所有卡片正确渲染
- 卡片可以被点击选中（蓝色 outline）
- 输入框可以正常编辑
- 无控制台错误

- [ ] **Step 4: 光标交互 Playwright 验证**

在 Storybook 的 "Full example" story 中：
1. 点击 `let` 容器内的 `=` 卡片 → 应该显示蓝色 outline 高亮
2. 按 Delete → 卡片被删除
3. 在卡片间点击 → 竖线光标出现
4. 在卡片间输入 `set` → 补全菜单弹出
5. 光标移到 `let` 容器末尾 → 出现右边界方块闪烁
6. 按 `\` → 输入框出现在正确位置

- [ ] **Step 5: 提交最终验证**

```bash
git add -A
git commit -m "chore(codecard-v2): final verification — all tests pass, 0 TS errors, 10 stories OK"
```

---

## 任务依赖图

```
Task 1 (删除死代码) ──────────────────────────────────────────────┐
Task 2 (CSS 清理) ──────────────────────────────────────────────┐ │
                                                                 │ │
Task 3 (CardNode 重写) ◄── 依赖 Task 1,2 ─────────────────────┐ │ │
Task 4 (CardNode 测试) ◄── 依赖 Task 3 ───────────────────┐   │ │ │
Task 5 (CodeOpSync 适配) ◄── 依赖 Task 3 ─────────────────┐│   │ │ │
Task 6 (KeyboardPlugin 重写) ◄── 依赖 Task 3 ───────────┐ ││   │ │ │
Task 7 (KeyboardPlugin 测试) ◄── 依赖 Task 6 ───────────┐│ ││   │ │ │
                                                         ││ ││   │ │ │
Task 8 (slot-operations.ts) ───────────────────────────┐ ││ ││   │ │ │
Task 9 (AutocompletePlugin 重构) ◄── 依赖 Task 8 ────┐│ ││ ││   │ │ │
                                                      ││ ││ ││   │ │ │
Task 10 (CSS 打磨) ───────────────────────────────────┼┼─┼┼─┼┼───┼─┼─┘
                                                      ││ ││ ││   │ │
Task 11 (全量验证) ◄── 依赖所有 Task ─────────────────┘│ ││ ││   │ │
                                                       │ ││ ││   │ │
                                                       └─┴─┴─┴─┴───┴─┘
```

建议执行顺序：1 → 2 → 3 → 4,5,6 → 7 → 8 → 9 → 10 → 11
