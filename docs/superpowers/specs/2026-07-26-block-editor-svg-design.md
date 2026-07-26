# Block Editor v4.1 — 移除 Blockly，用纯 React + SVG 渲染

> 版本：draft | 2026-07-26
> 前置文档：[Block Editor v4 Design Spec](./2026-07-25-block-editor-design.md)

## 1. 动机

Blockly v13 引入 breaking change（`moduleType` 字段），导致当前 storybook 渲染报 500 错误。

审视 Blockly 在本项目中的实际角色后发现：

- 它在 500KB 的依赖中只做了一件事——**把 13 种积木画成 SVG**
- 所有交互逻辑（选择框导航、gap 管理、键盘拦截、模糊搜索）全在自建代码中
- BlockAdapter 的 400 行代码本质是"把我们的 CodeOp 翻译成 Blockly JSON → 再从 Blockly block 反读回来"，两层转换带来大量阻抗匹配

**决策：移除 Blockly，用纯 React + SVG 组件渲染积木。** CodeOp[] 保持为 SSOT，不再经过任何中间格式。

## 2. 架构

### 2.1 数据流

```
CodeOp[] (React state, SSOT)
    │
    ▼
BlockList (React 组件)
    │
    ├── 13 种 BlockSvg 组件（从 CodeOp 直接渲染）
    ├── GapIndicator（行间空白区域）
    └── SelectionOverlay（手柄选择框）
    │
    ├── KeyboardHandler（容器 onKeyDown）
    ├── FuzzySearchPopup（不变）
    └── RawInputModal（不变）
```

不再需要 BlockAdapter 的 CodeOp→Blockly→CodeOp 双向转换。CodeOp[] 是唯一数据源，React state 变化直接驱动 SVG 重渲染。

### 2.2 与 v4（Blockly）方案的差异

| 层级 | v4 (Blockly) | v4.1 (Pure SVG) |
|------|-------------|-----------------|
| 渲染 | Blockly Zelos renderer | 13 个 React SVG 组件 |
| 选择框导航 | SelectionManager 基于 `workspace.getTopBlocks()` | 基于 BlockNode 链表 + DOM ref |
| Gap 管理 | GapManager Set | BlockModel.gaps Set（逻辑不变） |
| 键盘 | KeyboardHandler 挂 Blockly SVG | KeyboardHandler 挂容器 div |
| 双向转换 | BlockAdapter (~400 行) | 不需要——CodeOp 直接渲染 |
| 积木连接 | Blockly connection API | BlockNode 链表 prevId/nextId |
| Clipboard | Blockly.clipboard | 自建 ClipboardManager |
| Zoom/Scroll | Blockly workspace | 不需要——div overflow-y 滚动 |
| Bundle | Blockly ~500KB | 0 额外依赖 |

## 3. 数据模型（不变）

SlotValue、InlineOp（7 种）、CodeOp（12 种）、bodyId——**完全不动**。

```typescript
// 来自原设计文档 §2，零改动
type SlotValue =
  | { kind: "text"; value: string }
  | { kind: "expr"; children: InlineOp[] }
  | { kind: "empty" };

type CodeOp =
  | DeclareVarOp | SetVarOp | IfBlockOp | WhileLoopOp | ForLoopOp
  | ReturnExprOp | CallFnOp | AwaitExprOp
  | BreakStmtOp | ContinueStmtOp | CommentOp | RawCodeOp;
```

## 4. BlockModel —— 吸收 SelectionManager + GapManager + BlockAdapter

BlockModel 是编辑器核心状态机，管理块列表、选择框位置、间隙操作。替代原来分散在四个模块中的状态。

### 4.1 状态

```typescript
interface BlockModelState {
  ops: CodeOp[];              // SSOT，不变
  gaps: Set<number>;          // gap 位置集合（吸收 GapManager）
  selectionIndex: number;     // 当前选择框在一维列表中的位置
}
```

### 4.2 一维导航列表

一维导航列表是选择框移动的轨道。列表每一项是一个可选中目标：block、gap、或 slot。gap 是积木之间的空白行——不是在列表里有独立索引，而是标记"积木之间可以插入新积木的位置"。

```
一个 CodeOp[] = [letOp, setOp, ifOp]  的导航列表展开后：

   gap₀     = 第一个积木之前
   block₀   = letBlock
     slot₀₀  = NAME
     slot₀₁  = VARTYPE
     slot₀₂  = VALUE
   gap₀.₅   = letBlock 和 setBlock 之间
   block₁   = setBlock
     slot₁₀  = NAME
     slot₁₁  = VALUE
   gap₁.₅   = setBlock 和 ifBlock 之间
   block₂   = ifBlock
     slot₂₀  = CONDITION
   gap₂.₅   = ifBlock 之后
```

导航列表由 `ops` 数组派生。每个元素的 DOM rect 通过该积木或 slot 的 React ref 获取。

- **← →**：在平直列表上逐元素移动（下一个/上一个），经过所有 block、slot、gap
- **↑ ↓**：D-pad 空间导航——找上方/下方屏幕空间最近的元素，不跳过任何类型

- **← →**：在列表上逐元素移动（prevIndex / nextIndex）
- **↑ ↓**：依据屏幕空间坐标找上方/下方最近的元素

### 4.3 操作

```typescript
interface BlockModel {
  // 查询
  getCurrentTarget(): SelectionTarget;           // { type, blockIndex?, slotName?, gapPosition? }
  getTargetRect(target: SelectionTarget): DOMRect | null;

  // 导航
  moveLeft(): void;
  moveRight(): void;
  moveUp(): void;
  moveDown(): void;

  // 编辑
  insertBlock(type: BlockType, atGapIndex: number): void;
  deleteBlock(blockIndex: number): void;
  deleteSelected(): void;                        // 根据 target type 分发
  insertGap(atPosition: number): void;
  deleteGap(atPosition: number): void;
  setSlotValue(blockIndex: number, slotName: string, value: SlotValue): void;
  clearSlot(blockIndex: number, slotName: string): void;
}
```

`BlockModel.toCodeOps()` 等同于原 `BlockAdapter.fromBlocks()`——但它不需要"反向解析"，因为它直接持有 `ops` 数组，只需要处理嵌套 body 结构。

### 4.4 积木连接

积木之间的垂直连接由它们在 `ops` 数组中的顺序决定——`ops[i]` 的积木在 `ops[i+1]` 的积木之上，两者之间渲染拼图咬合连接器。不需要额外的链表数据结构。

## 5. 积木 SVG 组件

### 5.1 13 种积木

| 积木类型 | 视觉结构 | 组件 |
|---------|---------|------|
| declare_var | `[let/mut] NAME∶ : TYPE∶ = VALUE∶` | `<LetVarBlock>` |
| set_var | `[=] NAME∶ VALUE∶` | `<SetVarBlock>` |
| return_stmt | `return VALUE∶?` | `<ReturnBlock>` |
| call_fn | `call FN∶ ( ARGS∶ )` | `<CallBlock>` |
| await_expr | `await VALUE∶` | `<AwaitBlock>` |
| break_stmt | `break` | `<BreakBlock>` |
| continue_stmt | `continue` | `<ContinueBlock>` |
| comment_stmt | `// text` | `<CommentBlock>` |
| raw_stmt | `raw SOURCE` | `<RawBlock>` ⇄ edit mode |
| if_block | `if ◇ CONDITION◇` C形 + elif/else 分支 | `<IfBlock>` |
| while_loop | `while ◇ CONDITION◇` C形 | `<WhileBlock>` |
| for_in | `for ITEM∶ in COLLECTION∶` C形 | `<ForInBlock>` |

### 5.2 积木外观

所有积木共享相同的视觉风格（复用 Zelos/Scratch 3.0 风格）：

- **形状**：圆角矩形，左侧拼图凸起（previousConnection），右侧拼图凹槽（nextConnection）
- **C 形积木**：底部开口，body 区域缩进，右侧拼图凹槽在底部下方
- **颜色**：复用原 `definitions.ts` 中的色值常量（橙 control、红 variable、紫 function、蓝 async、黄 break、绿 comment、灰 raw）
- **槽位**：文本槽 `∶` 显示 tinted 背景，表达式槽 `◇` 显示不同形状

### 5.3 slot 值和 body 内嵌渲染

文本槽值直接渲染为 SVG `<text>`。表达式槽值渲染为内联的表达式积木（小号文本带边框）。空槽位显示灰色 placeholder 文字。

C 形积木的 body 内嵌积木递归渲染——`<IfBlock>` 接收 `branches[].body: CodeOp[]`，为每个分支渲染对应的 `<BlockList>`（缩进）。

### 5.4 RawCodeOp 双模式

- **Display 模式**（常态）：渲染为 `[raw 文本内容]` 积木，和其他积木一样参与导航
- **Edit 模式**（选择框在上面按 Enter）：SVG text 替换为 `<foreignObject><input/></foreignObject>`，焦点进入 input
- 按 **Enter → 提交**（更新 CodeOp.source，回到 display 模式）
- 按 **Esc → 取消**（保持原内容，回到 display 模式）

### 5.5 if_block 动态分支

`<IfBlock>` 的 `branches[]` 通过 props 传入，组件内部提供 +/- 按钮为每个分支添加/移除 elif/else。不需要 Blockly mutator/extension 机制——纯 React state 管理。

## 6. 交互模型

### 6.1 积木网格（常态）

| 按键 | 上下文 | 行为 |
|------|--------|------|
| ↑↓←→ | 任意 | 空间方向导航（找最近 rect 的元素） |
| 字符键 | slot | 弹出 FuzzySearchPopup，字符作为首字符 |
| 字符键 | gap | 弹出 FuzzySearchPopup，字符作为首字符 |
| Enter | gap | 插入 gap + 弹出 FuzzySearchPopup |
| Enter | slot | 弹出 FuzzySearchPopup（无初始字符） |
| Enter | raw block (display) | 切换为 edit 模式 |
| Enter | raw block (edit) | 提交编辑，切换回 display |
| Enter | 其他 block | 无操作 |
| Delete | block | 删除积木 |
| Delete | slot | 清空槽位值为 `{kind: "empty"}` |
| Delete | gap | 删除 gap |
| Backspace | block | 删除积木 |
| Backspace | slot | 清空槽位值为 `{kind: "empty"}` |
| Backspace | gap | 删除 gap，光标移到上一位置 |
| `\` | 任意 | 弹出 raw input modal（新建 RawCodeOp） |
| Ctrl+C | 任意 | 复制选中 block 的 CodeOp 到剪贴板 |
| Ctrl+V | 任意 | 从剪贴板粘贴 CodeOp 插入当前 gap |
| Ctrl+X | 任意 | 剪切选中 block |

### 6.2 FuzzySearchPopup（不变）

纯 DOM 浮层，零改动。键盘 ↑↓ 选择，Enter 确认，Esc 取消，继续输入过滤。

### 6.3 Raw Input Modal（不变）

`\` 触发的 modal textarea。Enter 提交 → 创建 RawCodeOp，Esc 取消。

### 6.4 选择框（SelectionOverlay）

视觉上是一个浮在积木上的高亮层：

| target type | 视觉效果 |
|-------------|---------|
| block | 实线轮廓（`outline: 2px solid`） |
| slot | 虚线边框（`border: 2px dashed`） |
| gap | 水平高亮行 + 闪烁动画 |
| raw block (edit) | input 原生焦点样式 |

通过查询目标元素的 DOM rect 定位（React ref → `getBoundingClientRect()`）。

## 7. Clipboard

```typescript
interface ClipboardManager {
  copy(ops: CodeOp[]): void;    // JSON 序列化到 navigator.clipboard
  paste(): CodeOp[] | null;     // 从剪贴板反序列化
}
```

## 8. 文件变更

### 8.1 删除

| 文件 | 原因 |
|------|------|
| `blocks/definitions.ts` | Blockly JSON 定义，被 SVG 组件替代 |
| `blocks/mutations.ts` | Blockly mutator 扩展，被 IfBlock React state 替代 |
| `blocks/BlockAdapter.ts` | Blockly 双向转换，被 BlockModel 吸收 |
| `blocks/SelectionManager.ts` | 被 BlockModel + SelectionOverlay 吸收 |
| `blocks/GapManager.ts` | 被 BlockModel.gaps Set 吸收 |
| `blockly` npm 依赖 | 移除 |

### 8.2 重写

| 文件 | 变更 |
|------|------|
| `BlockEditor.tsx` | 移除 Blockly，用 BlockModel + BlockList |
| `blocks/KeyboardHandler.ts` | 移除 Blockly workspace 引用，事件挂容器 div |
| `BlockEditor.module.css` | 移除 Blockly 相关样式 |

### 8.3 新建

| 文件 | 职责 |
|------|------|
| `blocks/BlockModel.ts` | 状态机：op 列表、gap 集合、选择框位置、CRUD |
| `blocks/ClipboardManager.ts` | 剪贴板序列化/反序列化 |
| `blocks/SelectionOverlay.tsx` | 浮层选择框（outline/border/blink） |
| `blocks/BlockSvg.tsx` | 积木外壳（拼图形状、颜色、slot 背景） |
| `blocks/BlockSvg/LetVarBlock.tsx` | declare_var 积木 |
| `blocks/BlockSvg/SetVarBlock.tsx` | set_var 积木 |
| `blocks/BlockSvg/ReturnBlock.tsx` | return 积木 |
| `blocks/BlockSvg/CallBlock.tsx` | call_fn 积木 |
| `blocks/BlockSvg/AwaitBlock.tsx` | await 积木 |
| `blocks/BlockSvg/BreakBlock.tsx` | break 积木 |
| `blocks/BlockSvg/ContinueBlock.tsx` | continue 积木 |
| `blocks/BlockSvg/CommentBlock.tsx` | comment 积木 |
| `blocks/BlockSvg/RawBlock.tsx` | raw 积木（含 display/edit 双模式） |
| `blocks/BlockSvg/IfBlock.tsx` | if/elif/else 积木（含动态分支） |
| `blocks/BlockSvg/WhileBlock.tsx` | while 积木 |
| `blocks/BlockSvg/ForInBlock.tsx` | for_in 积木 |

### 8.4 不变

| 文件 | 说明 |
|------|------|
| `blocks/fields/FuzzySearchPopup.ts` | 纯 DOM，零依赖 Blockly |
| `core/types/code-op.ts` | CodeOp 类型定义 |
| `core/types/slot-value.ts` | SlotValue 类型定义 |
| `core/types/inline-op.ts` | InlineOp 类型定义 |
| `services/codegen/code-op-emitter.ts` | CodeOp[] → OmniLang 源文本 |
| `FlowCanvas.tsx` | 集成接口不变（还是传 initialOps + onSave） |

## 9. 测试策略

| 层 | 方法 | 说明 |
|---|------|------|
| BlockModel roundtrip | 单元测试 | `createBlock(ops) → toCodeOps()` ≈ ops |
| 向后兼容 | 单元测试 | 旧 CodeOp 数据无损加载 |
| FuzzySearchPopup | 单元测试 | 已有 36 个测试，不动 |
| BlockSvg 渲染 | 单元测试 + Storybook | 每种积木的 render 快照 |
| 键盘导航 | 单元测试 | BlockModel 导航逻辑 |
| 集成 | Playwright | ↑↓←→ 导航、插入积木、删除、slot 编辑 |
| 已有测试 | 保留/更新 | 去掉 Blockly 依赖的 mock |

## 10. 不在本期范围

- 拖拽交互
- 多选积木
- Undo/Redo
- 表达式积木嵌套渲染（InlineOp → 内联 SVG）
- 自定义积木颜色（先用 Zelos 配色）

## 11. 风险

| 风险 | 缓解 |
|------|------|
| SVG 拼图形状（C 形 + 咬合）手写复杂 | 13 种积木形状有限，可以先做 declare_var 验证可行性 |
| ↑↓ 空间导航在 slot 密集时可能跳不准 | 退化为"跳到同一列最近元素"，比 D-pad 更耐受 |
| 现有 174 个测试依赖 Blockly mock | 逐个更新测试，BlockModel 测试纯数据驱动无 mock 依赖 |
| RawCodeOp inline edit 的 `foreignObject` 兼容性 | Chrome/Edge/Firefox 均支持，Storybook + Playwright 验证 |

## 12. 实施顺序

1. BlockModel（纯数据，可独立测试）
2. BlockSvg 组件 + SelectionOverlay
3. KeyboardHandler（挂容器 div）
4. ClipboardManager
5. BlockEditor.tsx 重写 + 删除 Blockly
6. 更新测试
7. 更新 FlowCanvas 集成（接口不变，仅改 import）
8. Storybook 验证
9. Poiesis 集成验证
