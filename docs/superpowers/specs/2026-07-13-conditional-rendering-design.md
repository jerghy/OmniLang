# Poiesis @if / @else if / @else 条件渲染设计

> 日期: 2026-07-13 · 状态: 设计完成

## 1. 背景

OmniView 支持 `@if` / `@else if` / `@else` 条件渲染语法（详见 [03-模板语法](../../omniview/03-templates.md)）：

```omni
@if status == "loading" {
    <div class: spinner>Loading...</div>
} @else if status == "error" {
    <div class: error>Error: {errorMsg}</div>
} @else {
    <div class: content>{data}</div>
}
```

**已有基础设施**：
- `IfViewNode` 类型（`services/codegen/types.ts`）—— 定义完整
- `source-emitter.ts` —— 已支持 `IfViewNode` → `@if/@else` 源码输出
- Rust 编译器 —— `ViewNode::IfDirective` + `lower_if_directive` 完整实现

**缺失部分**：
- Parser 不解析 view body 中的 `@if` 指令
- Element 模型无 `ConditionalBlockData` 类型
- 元素树无条件块展示/编辑 UI
- Store 无 CRUD 操作

## 2. 设计目标

1. Parser 解析 `@if`/`@else if`/`@else` → `ConditionalBlockData`
2. 元素树可视化展示和编辑条件块
3. Canvas 通过调试面板改 signal 观察分支切换
4. 完整 round-trip：解析 → 编辑 → 源码输出无损

## 3. Element Model

### 3.1 新增 ConditionalBlockData

```typescript
// core/types/element.ts

interface ConditionalBlockData extends ElementDataBase {
  nodeType: "conditional";
  /** 条件分支列表，按 @if → @else if → @else 顺序 */
  branches: Array<{
    type: "if" | "elif" | "else";
    condition: string;       // @else 时为空字符串 ""
    children: string[];      // 该分支内的子元素 ID
  }>;
  /** 所有分支子元素的扁平列表（兼容现有 children 遍历模式） */
  children: string[];
  isFolded?: boolean;
  position?: { x: number; y: number };
}

// 扩展联合类型
type ElementData = ViewElementData | CodeblockElementData | GroupElementData | ConditionalBlockData;
```

### 3.2 不变式

- 至少 2 个 branch（`@if` + 至少一个 `@else` 或 `@else if`）
- `branches[0].type === "if"`（有且仅有一个）
- `branches[last].type === "else"`（有且仅有一个），condition 为空
- `@else if` 分支位于 `@if` 和 `@else` 之间
- `children` = 所有 `branches[*].children` 的扁平拼接

### 3.3 与 IfViewNode 的映射

`IfViewNode`（types.ts 中已有，codegen 输入端）保持不变。Pipeline 负责双向转换：

| ConditionalBlockData | IfViewNode |
|---|---|
| `branches[0]` | `condition` + `children`（@if body） |
| `branches[1..n-1]` | `branches[]`（@else if） |
| `branches[last]` (type="else") | `fallback`（@else body） |

## 4. Parser 变更

### 4.1 parseViewBody 扩展

`parser-view.ts` 的 `parseViewBody` 中新增 `AT` token 处理：

```typescript
// 在现有分支之后添加
if (tok.kind === "AT") {
  parseViewDirective(internals);
  internals.stream.skipNewlines();
  continue;
}
```

### 4.2 parseViewDirective

```
1. consume "@"
2. expect IDENTIFIER — 断言为 "if" / "for" / "slot"（本次仅处理 "if"）
3. 收集条件表达式: consume tokens 直到 BRACE_OPEN → condition 字符串
4. 解析分支 body: brace-depth 追踪，遇到匹配 BRACE_CLOSE 停止
   内部递归调用 parseViewElement / parseViewTextContent
5. 保存 branch { type:"if", condition, children: [...] }
6. 检测后续 @else / @else if:
   - peek: AT + IDENTIFIER("else") + IDENTIFIER("if") → @else if，goto 3
   - peek: AT + IDENTIFIER("else") + BRACE_OPEN → @else，condition=""，goto 4
   - 否则: 结束
7. 构建 ConditionalBlockData, push 到 elements
```

**关键细节**：
- 条件表达式仅收集原文，不做语法解析（编译器负责）
- 分支 body 解析复用现有 `parseViewElement` / `parseViewTextContent`
- 解析分支内部时，条件块 ID 在 viewParentStack 中作为父节点
- ID 生成为 `"{compName}.@if.{elements.length}"`

## 5. Pipeline 转换

### 5.1 正向: ConditionalBlockData → IfViewNode

在 `graph-compiler.ts` 或 `view-tree-builder.ts` 中添加转换函数。

### 5.2 元素删除/移动适配

`element-crud.ts` 需要处理 `ConditionalBlockData`：
- 删除条件块 → 递归删除所有 branch children（通过现有 `children` 扁平列表）
- 移动子元素进出条件块 → 更新对应 branch 的 `children` + 扁平 `children`
- 移入分支 = 从当前 parent.children 删除, 添加到 branch.children, 扁平 children 重算

## 6. Store 与 CRUD

### 6.1 新增 Actions（element-crud.ts）

| Action | 说明 |
|---|---|
| `addConditionalBlock(parentId)` | 在父节点下添加条件块（默认含 @if + @else 两个空分支） |
| `addConditionalBranch(blockId, type)` | 添加 @else if 或 @else 分支 |
| `removeConditionalBranch(blockId, branchIndex)` | 删除分支（if/else 不可删，至少保留 2 个分支） |
| `updateBranchCondition(blockId, branchIndex, condition)` | 修改分支条件表达式 |
| `addChildToBranch(blockId, branchIndex, element)` | 向分支添加子元素 |
| `removeChildFromBranch(blockId, branchIndex, childId)` | 从分支移除子元素 |

### 6.2 isDirty 标记

所有 mutation action 设置 `isDirty: true`，遵循现有 undo/redo 模式。

## 7. UI 变更

### 7.1 元素树面板

`ElementTree` 中识别 `nodeType === "conditional"`：

```
🔻 条件渲染                    ← 折叠/展开 (isFolded)
  ├─ 🔻 (num > 1)              ← 分支 if，可折叠
  │    ├─ 🧩 Button
  │    └─ 🧩 Text
  ├─ 🔻 (status == "error")    ← 分支 else if
  │    └─ 🧩 Alert
  ├─ 🔻 else                   ← 分支 else
  │    └─ 🧩 Text
  └─ [+ 添加条件]               ← 按钮
```

- 点击分支行 → 属性面板显示条件表达式编辑框
- 点击分支内子元素 → 属性面板显示该子元素（现有逻辑）
- 右键分支 → 「添加子组件」(现有) / 「删除分支」
- 右键条件块整体 → 「删除条件块」/ 「添加分支」

### 7.2 右键菜单

`ElementContextMenu` 扩展：
- 普通元素 → 现有菜单 + 「用条件块包裹」
- 条件块 → 「添加 @else if 分支」/ 「添加 @else 分支」

### 7.3 属性面板

新增 `BranchConditionEditor`（显示在属性面板中）：
- 条件表达式 textarea（`@else` 时 disabled，固定显示 "else"）
- 子元素列表（只读）
- 「删除此分支」按钮（if/else 不可删）

### 7.4 Canvas

保持现有逻辑：canvas 按当前 signal 值渲染对应激活分支。条件块折叠状态不影响 canvas。调试面板改 signal → canvas 自动切换到对应分支。

## 8. 涉及文件

| 文件 | 变更 |
|---|---|
| `core/types/element.ts` | 新增 `ConditionalBlockData`，扩展 `ElementData` |
| `services/codegen/types.ts` | 无变更（`IfViewNode` 已存在） |
| `services/codegen/parser-view.ts` | 新增 `parseViewDirective`，扩展 `parseViewBody` |
| `services/codegen/source-emitter.ts` | 可能需要适配（验证现有逻辑） |
| `services/codegen/view-tree-builder.ts` | 新增条件块构建逻辑 |
| `store/slices/element-crud.ts` | 新增 6 个 CRUD action |
| `store/types.ts` | 如需，扩展 state 类型 |
| `store/undo-slice.ts` | 注册新 action 的 undo 逻辑 |
| `components/panels/ElementTree.tsx` | 条件块节点渲染 |
| `components/panels/ElementContextMenu.tsx` | 右键菜单扩展 |
| `components/panels/inspector/` | 新增 `BranchConditionEditor.tsx` |
| `__tests__/` | parser 测试 + element-crud 测试 |

## 9. 测试策略

| 测试类型 | 覆盖内容 |
|---|---|
| Parser 单元测试 | `@if` / `@if-else` / `@if-elif-else` / 嵌套 `@if` / 空分支 |
| Round-trip 测试 | parse → emit → parse，验证源码一致性 |
| CRUD 单元测试 | 添加/删除/修改分支、子元素进出分支、不变式约束 |
| UI 测试 (Playwright) | 元素树展示、右键菜单、属性面板编辑、canvas 分支切换 |

## 10. 后续迭代

- `@for` / `@empty` 循环渲染（可复用 ConditionalBlockData 模式，用 `ForBlockData`）
- `@slot` 插槽
- `mode="sync"` 动画模式
- 条件表达式可视化编辑器（替代纯文本框）
