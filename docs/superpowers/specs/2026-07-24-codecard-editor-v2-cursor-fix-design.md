# CodeCard Editor v2：光标交互修复 + 代码清理

> 状态：设计完成 | 2026-07-24
>
> 依赖：[2026-07-23-codecard-editor-v2-design.md](./2026-07-23-codecard-editor-v2-design.md)
>
> 目标：修复 CardNode DecoratorNode 架构缺陷，实现设计文档 §3 定义的光标三态交互，清理死代码。

## 核心问题

CardNode 使用 `DecoratorNode`（Lexical 只读节点），导致：
- 光标无法落在卡片上 → 卡片高亮态不存在
- 光标无法在卡片间定位 → 竖线 `│` 态不存在
- KeyboardPlugin 无方向键处理 → 无法导航
- 容器右边界无光标概念 → 方块 `▌` 态不存在

**根本修复**：CardNode 从 `DecoratorNode` 改为 `ElementNode`，利用浏览器 `contenteditable="false"` 实现"原子卡片、不可编辑、可选中、可导航"。

## 实施总览

```
Layer 1: 清理 + 地基
  ├─ 删死代码：CardElementNode.ts, SlotTextNode.ts 及对应测试
  ├─ CSS 清理：删 user-select: none
  └─ 验证：Storybook 渲染正确

Layer 2: CardNode 架构修复
  ├─ CardNode: DecoratorNode → ElementNode (contenteditable="false")
  ├─ 适配 CodeOpSyncPlugin 创建/提取
  ├─ 更新 CardNode.test.ts + 序列化兼容
  └─ 验证：10 story 渲染 + roundtrip

Layer 3: 光标三态 + 方向键导航
  ├─ KeyboardPlugin：容器右边界检测 + 双层 Delete/Enter
  ├─ ← → ↑ ↓ 浏览器原生处理（返回 false 放行）
  ├─ CSS：卡片高亮 .highlighted + 右边界 .at-right-edge
  └─ 验证：Playwright 键盘操作流程

Layer 4: \ 输入框完善 + 交互打磨
  ├─ 新建 slot-operations.ts：CursorContext + 推断规则表
  ├─ AutocompletePlugin 重构：\ 逻辑委托给 slot-operations
  ├─ KeyboardPlugin 边缘场景：空 body line、最后一个子节点
  ├─ CSS 打磨：过渡动画、hover、方块光标闪烁
  └─ 验证：所有边缘场景 + Storybook
```

---

## Layer 1：清理 + 地基

### 删除文件（4 个）

| 文件 | 原因 | 引用检查 |
|------|------|---------|
| `nodes/CardElementNode.ts` | v1 ElementNode，被 v2 的 CardNode + ContainerNode + BlockNode 替代 | ✅ 无人引用 |
| `nodes/__tests__/CardElementNode.test.ts` | 对应测试 | ✅ |
| `nodes/SlotTextNode.ts` | v1 slot，被 InputSlotNode 替代 | ✅ 无人引用 |
| `nodes/__tests__/SlotTextNode.test.ts` | 对应测试 | ✅ |

**保留**：`integration.test.ts` — 测试 codegen 服务层（emitOps/parseOps），非 v1 编辑器代码。

### CSS 变更

| 改什么 | 哪里 | 为什么 |
|--------|------|--------|
| 删 `user-select: none` | `.code-card` (line 119) | 卡片必须能被浏览器选中 |
| 删 `cursor: default` | `.code-card` (line 118) | 默认文本光标即可 |

### 验证

- `vitest run` 现有测试全部通过
- Storybook 10 个 story 全部正常渲染
- 0 个新 TS 错误

---

## Layer 2：CardNode 架构修复

### CardNode (ElementNode) 新设计

```typescript
export class CardNode extends ElementNode {
  __cardKind: InlineKind;
  __cardLabel: string;
  __cardPid: string;

  static getType(): string { return "inline-card"; }

  // 行内流式排列
  isInline(): boolean { return true; }
  canBeEmpty(): boolean { return true; }
  canInsertTextBefore(): boolean { return true; }
  canInsertTextAfter(): boolean { return true; }

  // DOM：contenteditable="false" 实现原子卡片
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

  updateDOM(prevNode: CardNode, dom: HTMLElement, _config: EditorConfig): boolean {
    if (prevNode.__cardKind !== this.__cardKind) {
      dom.className = `code-card code-card--${this.__cardKind}`;
      dom.setAttribute("data-card-kind", this.__cardKind);
    }
    if (prevNode.__cardLabel !== this.__cardLabel) {
      dom.textContent = this.__cardLabel;
    }
    return false;
  }

  // 公共 API 不变
  getCardKind(): InlineKind { return this.getLatest().__cardKind; }
  getCardLabel(): string { return this.getLatest().__cardLabel; }
  getCardPid(): string { return this.getLatest().__cardPid; }
}
```

### 核心洞察：`contenteditable="false"` 天然实现交互

| 用户操作 | 浏览器行为 | 映射到设计文档 |
|---------|-----------|---------------|
| 点击卡片 | 整个 span 被整体选中 | 卡片高亮 |
| ← → 在卡间 | 光标在卡片间跳跃 | 竖线 `│` 态 |
| Delete 选中卡片 | 删除整个 span | 删卡 |
| 在卡片间打字 | 文本在间隙出现 | 触发补全 |

### 需适配的文件

| 文件 | 变更 |
|------|------|
| **CardNode.ts** | 完全重写：DecoratorNode → ElementNode |
| **CardNode.test.ts** | 更新 7 个测试：序列化格式、clone 行为 |
| **CodeOpSyncPlugin.ts** | 适配 `lexicalToEditorNode` 中 `getCardLabel` 不变；`extractFromNode` 遍历逻辑不变 |
| **AutocompletePlugin.tsx** | 轻微调整：`executeAutocomplete` 中卡片插入后光标位置 |
| **KeyboardPlugin.ts** | Delete 检测改为"光标在卡边界"，不再依赖 `$isCardNode(anchorNode)` |
| **CSS** | 删 `user-select: none` |
| **nodes/index.ts** | 不需改 |

### 序列化兼容

```typescript
exportJSON(): SerializedCardNode {
  return {
    ...super.exportJSON(),
    type: "inline-card",
    cardKind: this.__cardKind,
    cardLabel: this.__cardLabel,
    cardPid: this.__cardPid,
  };
}

static importJSON(serialized: any): CardNode {
  // 兼容旧 DecoratorNode 格式（无 children 字段）
  const node = $createCardNode(
    serialized.cardKind,
    serialized.cardLabel,
    serialized.cardPid,
  );
  if (serialized.format !== undefined) node.setFormat(serialized.format);
  if (serialized.indent !== undefined) node.setIndent(serialized.indent);
  if (serialized.direction !== undefined) node.setDirection(serialized.direction);
  return node;
}
```

### 验证

- `CardNode.test.ts` 全部通过
- `KeyboardPlugin.test.ts` 适配后通过
- `integration-v2.test.ts` roundtrip 通过
- Storybook 10 个 story 渲染正确
- 0 个 TS 错误
- 0 个控制台新错误

---

## Layer 3：光标三态 + 方向键导航

### KeyboardPlugin 职责划分

```
浏览器原生（返回 false，放行）：
  ← → ↑ ↓ — 光标移动
  选中卡片后 Delete — 删 contenteditable="false" span
  卡片后 Backspace — 删卡

KeyboardPlugin 拦截（返回 true）：
  Delete / Backspace 特殊场景：
    ├─ 容器右边界 ▌ → Delete 删整容器（含子卡+输入框）
    ├─ 空 body line 上 Delete → 删该行（保留至少一个）
    └─ 最后一个 body line → Delete 无操作

  Enter 特殊场景：
    ├─ body line 末尾 → 下方新增空行
    ├─ 容器右边界 ▌ → 容器后插入新行
    └─ 顶层 → 新增空行

  Tab — 不变（set 内循环运算符）
```

### 容器右边界检测

```typescript
function detectContainerRightEdge(
  selection: RangeSelection,
): ContainerElementNode | BodyLineNode | null {
  if (!selection.isCollapsed()) return null;

  const anchor = selection.anchor;
  const anchorNode = anchor.getNode();

  // 条件 1：光标在节点末尾
  if (anchor.offset !== anchorNode.getTextContentSize()) return null;

  // 条件 2：向上搜索，看是否是某个容器的最后一个子节点
  let node: LexicalNode | null = anchorNode;
  while (node !== null) {
    const parent = node.getParent();
    if (parent === null) return null;

    const lastChild = parent.getLastChild();
    if (lastChild !== node) return null; // 不是最后一个

    if ($isContainerElementNode(parent)) return parent;
    if ($isBodyLineNode(parent)) return parent;

    node = parent;
  }
  return null;
}
```

### CSS 新增

```css
/* 卡片选中高亮 */
.contentEditable :global(.code-card.highlighted) {
  outline: 2px solid #4c4cff;
  outline-offset: 0;
  box-shadow: 0 0 0 1.5px rgba(76, 76, 255, 0.25);
}

/* 容器右边界方块光标 */
.contentEditable :global(.code-container.at-right-edge),
.contentEditable :global(.code-body-line.at-right-edge) {
  border-right: 2px solid #4c4cff;
  padding-right: 6px;
}
```

### 文件变更

| 文件 | 变更 |
|------|------|
| `KeyboardPlugin.ts` | 重写 Delete/Backspace/Enter 逻辑；新增右边界检测 |
| `KeyboardPlugin.test.ts` | 覆盖所有新场景 |
| `CodeCardEditor.module.css` | 新增 `.highlighted`、`.at-right-edge` |

### 验证

- 点击卡片 → 高亮
- ← → 在卡片间移动
- 卡片间打字 → 补全弹出
- Delete 高亮卡片 → 删卡
- 光标在容器右边界 → 视觉方块 → Delete 删容器
- ↑ ↓ 在 body lines 间移动
- Enter body line 末尾 → 新行

---

## Layer 4：`\` 输入框完善 + 交互打磨

### slot-operations.ts（新文件）

```typescript
// transforms/slot-operations.ts

interface CursorContext {
  parentKind: string | null;
  precedingCardKind: string | null;
  precedingCardLabel: string | null;
  grandparentKind: string | null;
  isTopLevel: boolean;
}

const SLOT_INFERENCE: Array<{
  condition: (ctx: CursorContext, parent: LexicalNode | null) => boolean;
  slotKind: SlotKind;
}> = [
  // let / set 开头 — 变量名
  { condition: c => (c.parentKind === 'let' || c.parentKind === 'set')
    && !c.precedingCardKind, slotKind: 'name' },
  // for 开头 — 变量名
  { condition: c => c.parentKind === 'for'
    && !c.precedingCardKind, slotKind: 'name' },
  // : 后面 — 类型
  { condition: c => c.precedingCardLabel === ':', slotKind: 'type_value' },
  // in 后面 — 集合表达式
  { condition: c => c.precedingCardLabel === 'in', slotKind: 'expression' },
  // 运算符后面 — 值表达式
  { condition: c => c.precedingCardLabel === '='
    || /^[+\-*/%]=$/.test(c.precedingCardLabel ?? ''), slotKind: 'expression' },
  // 块条件头行
  { condition: c => c.parentKind === 'if'
    || c.parentKind === 'while', slotKind: 'condition' },
  // call 开头 — 函数名
  { condition: c => c.parentKind === 'call'
    && !c.precedingCardKind, slotKind: 'fn_name' },
  // return / await — 表达式
  { condition: c => c.parentKind === 'return'
    || c.parentKind === 'await', slotKind: 'expression' },
  // body line 内 — 表达式
  { condition: c => !!c.grandparentKind && !c.parentKind, slotKind: 'expression' },
];

export function canCreateSlot(ctx: CursorContext): boolean {
  if (ctx.isTopLevel) return false;
  return inferSlotKind(ctx) !== null;
}

export function createSlotAtCursor(
  ctx: CursorContext,
  parentNode: LexicalNode,
): InputSlotNode | null {
  const slotKind = inferSlotKind(ctx);
  if (!slotKind) return null;
  const slot = $createInputSlotNode('', slotKind);
  parentNode.append(slot);
  return slot;
}
```

### AutocompletePlugin 重构

`\` 处理从插件中 delegate 到 `slot-operations`：

```typescript
// AutocompletePlugin.tsx — 重构后
const handler = (e: KeyboardEvent) => {
  if (e.key !== '\\') return;
  e.preventDefault();
  editor.update(() => {
    const selection = $getSelection();
    if (!$isRangeSelection(selection)) return;

    const ctx = buildCursorContext(selection);
    if (!canCreateSlot(ctx)) return;

    const slot = createSlotAtCursor(ctx, selection.anchor.getNode().getParent()!);
    if (slot) slot.select();
  });
};
```

### Delete/Backspace 边缘场景

| 场景 | 处理 |
|------|------|
| Backspace 在容器最开头 | no-op |
| Delete 在空 body line | 删该行（至少保留一个） |
| Backspace 在 block 内删到只剩空行 | 不塌缩 |
| 卡片间 Backspace/Delete | 浏览器原生删除 ✅ |

### CSS 打磨

```css
/* 卡片交互过渡 */
.contentEditable :global(.code-card) {
  transition: outline-color 120ms ease, box-shadow 120ms ease, filter 120ms ease;
  cursor: default;
}
.contentEditable :global(.code-card:hover) {
  filter: brightness(0.95);
}

/* 右边界方块闪烁 */
@keyframes block-blink {
  0%, 100% { border-right-color: #4c4cff; }
  50% { border-right-color: rgba(76, 76, 255, 0.15); }
}
.contentEditable :global(.code-container.at-right-edge) {
  animation: block-blink 1.2s ease-in-out infinite;
}

/* 输入框过渡 */
.contentEditable :global(.input-slot) {
  transition: border-color 150ms ease, box-shadow 150ms ease;
}
```

---

## 文件变更总览

| 文件 | Layer | 操作 |
|------|-------|------|
| `nodes/CardElementNode.ts` | L1 | **删除** |
| `nodes/__tests__/CardElementNode.test.ts` | L1 | **删除** |
| `nodes/SlotTextNode.ts` | L1 | **删除** |
| `nodes/__tests__/SlotTextNode.test.ts` | L1 | **删除** |
| `nodes/CardNode.ts` | L2 | **重写** |
| `nodes/__tests__/CardNode.test.ts` | L2 | **更新** |
| `plugins/CodeOpSyncPlugin.ts` | L2 | **微调** |
| `plugins/AutocompletePlugin.tsx` | L2 + L4 | **微调 + 重构** |
| `plugins/KeyboardPlugin.ts` | L3 + L4 | **重写** |
| `plugins/__tests__/KeyboardPlugin.test.ts` | L3 + L4 | **更新** |
| `transforms/slot-operations.ts` | L4 | **新建** |
| `CodeCardEditor.module.css` | L1-L4 | **修改** |

## 验证清单

- [ ] Layer 1：4 个死文件删除，0 TS 错误
- [ ] Layer 2：`CardNode.test.ts` 通过，Storybook 10 story 渲染正确
- [ ] Layer 2：`integration-v2.test.ts` roundtrip 通过
- [ ] Layer 3：Playwright 光标三态全流程
  - [ ] 点击卡片 → 高亮
  - [ ] ← → 卡片间导航
  - [ ] 卡片间打字 → 补全菜单
  - [ ] Delete 删卡
  - [ ] 右边界 → 方块 → Delete 删容器
  - [ ] ↑ ↓ body line 导航
  - [ ] Enter 新建行
- [ ] Layer 4： `\` 输入框全场景
  - [ ] let 内按 `\` → name slot
  - [ ] `:` 后按 `\` → type_value slot
  - [ ] `=` 后按 `\` → expression slot
  - [ ] if 头行按 `\` → condition slot
  - [ ] 顶层按 `\` → no-op
- [ ] 全层：`vitest run` 所有测试通过
- [ ] 全层：`tsc --noEmit` 0 错误
- [ ] 全层：Storybook 10 个 story 交互正常

## 不做的东西

以下属于设计文档 §8-9 的后续 Phase，本次不碰：
- CardFlowPlugin / ClipboardPlugin / SyntaxLintPlugin
- `.` dot 方法补全
- 语法波浪线提示
- 拖拽移动卡片
- 多光标编辑
