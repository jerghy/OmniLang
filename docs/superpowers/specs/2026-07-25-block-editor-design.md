# Block Editor v4 Design Spec

> 从 CodeCard Editor v3（Lexical 扁平卡片）迁移到 Scratch 式积木编辑器
> 底层引擎：Google Blockly（Apache 2.0）
> 状态：design approved | 2026-07-25

## 1. 架构总览

### 数据流

```
CodeOp[] ←→ BlockAdapter ←→ Blockly Workspace
 (SSOT)     (双向转换层)      (视图 + 键盘交互)
```

- **CodeOp[]** 是唯一数据源（SSOT）
- **BlockAdapter** 负责 `CodeOp[] → Blockly blocks`（构建）和 `Blockly blocks → CodeOp[]`（序列化回写）
- **Blockly Workspace** 只负责渲染 + 键盘交互 + 积木操作，不持有业务数据

### 生命周期

1. `CodeOp[]` 传入 → BlockAdapter.toBlocks() 生成 Blockly 积木 JSON
2. workspace.clear() + 批量创建积木
3. 每次用户操作引发 workspace change 事件 → BlockAdapter.fromBlocks() 回写 CodeOp[]
4. CodeOp[] 变更通知父组件

### 迁移策略

- **直接替换**：删除旧 CodeCardEditor 的所有 Lexical 相关代码
- 删除的文件：CardNode, SlotCardNode, BodyLineNode, ContainerElementNode, BlockElementNode, KeyboardPlugin, AutocompletePlugin, CodeOpSyncPlugin, convertToNodes, convertToOps, CodeCardEditor.module.css（旧）
- 保留的文件：code-op.ts（扩展 SlotValue + InlineOp + bodyId 字段）
- 重写的文件：CodeCardEditor.tsx → 改名/重构为 BlockEditor.tsx（新组件入口）
- 删除的文件：editor-node.ts（中间层不再需要，CodeOp ↔ Blockly 直接转换）

## 2. 数据模型优化

### SlotValue —— 区分槽位类型

```typescript
// 槽位值：区分纯文本、表达式、空状态
type SlotValue =
  | { kind: "text"; value: string }          // 纯文本槽（变量名、函数名）
  | { kind: "expr"; children: InlineOp[] }   // 表达式槽（条件、值）
  | { kind: "empty" };                       // 空槽位（积木新建未填）
```

### InlineOp —— 内联表达式积木

仅用于表达式槽内嵌套的积木类型：

```typescript
type InlineOp =
  | { kind: "var_ref"; name: string }
  | { kind: "literal"; value: string; literalType: "number" | "string" | "bool" }
  | { kind: "binary"; op: string; left: SlotValue; right: SlotValue }
  | { kind: "unary"; op: string; operand: SlotValue }
  | { kind: "call_fn"; fnName: string; args: string[] }
  | { kind: "member_access"; target: string; member: string }
  | { kind: "parens"; inner: InlineOp[] };
```

### CodeOp 字段更新

将 string 字段升级为 SlotValue：

| CodeOp 类型 | 字段变更 |
|------------|---------|
| DeclareVarOp | `name: string` → `name: SlotValue`（文本槽）; `initValue: string` → `initValue: SlotValue`（表达式槽） |
| SetVarOp | `name: string` → `name: SlotValue`（文本槽）; `value: string` → `value: SlotValue`（表达式槽） |
| IfBlockOp | `branches[i].condition: string` → `condition: SlotValue`（表达式槽） |
| WhileLoopOp | `condition: string` → `condition: SlotValue`（表达式槽） |
| ForLoopOp | `itemName: string` → `itemName: SlotValue`（文本槽）; `collection: string` → `collection: SlotValue`（表达式槽） |
| ReturnExprOp | `value: string` → `value: SlotValue`（表达式槽） |
| CallFnOp | `fnName: string` → `fnName: SlotValue`（文本槽）; `args: string[]` → `args: SlotValue[]`（表达式槽） |
| AwaitExprOp | `value: string` → `value: SlotValue`（表达式槽） |

### body 显式化

给有 body 的 CodeOp 类型（IfBlockOp, WhileLoopOp, ForLoopOp）加 `bodyId: string`：

```typescript
interface IfBlockOp extends CodeOpBase {
  kind: "if_block";
  bodyId: string;  // 新增
  branches: { condition: SlotValue; body: CodeOp[] }[];
}

interface WhileLoopOp extends CodeOpBase {
  kind: "while";
  bodyId: string;  // 新增
  condition: SlotValue;
  body: CodeOp[];
}

interface ForLoopOp extends CodeOpBase {
  kind: "for_in";
  bodyId: string;  // 新增
  itemName: SlotValue;
  collection: SlotValue;
  body: CodeOp[];
}
```

bodyId 是稳定的字符串 ID（创建时生成），选择框通过 `bodyId + position` 定位 body 内的 gap 行位置。

### 向后兼容

- 旧 CodeOp 的 string 字段在加载时自动转换为 `{ kind: "text", value: oldString }`
- Roundtrip 测试确保旧数据无损转换

## 3. 积木类型体系

### 3.1 简单语句积木（单行，无 body，垂直堆叠）

| Blockly 类型 | 视觉结构 | 映射 CodeOp |
|-------------|---------|------------|
| `declare_var` | `[let/mut] [name∶] [∶] [type∶] [=] [value∶]` | DeclareVarOp |
| `set_var` | `[set] [name∶] [operator] [value∶]` | SetVarOp |
| `return_stmt` | `[return] [value∶?]` | ReturnExprOp |
| `call_fn` | `[call] [fn_name∶] [(] [args...∶] [)]` | CallFnOp |
| `await_expr` | `[await] [value∶]` | AwaitExprOp |
| `break_stmt` | `[break]` | BreakStmtOp |
| `continue_stmt` | `[continue]` | ContinueStmtOp |
| `comment_stmt` | `[// text∶]` | CommentOp |
| `raw_stmt` | `[raw source∶]` | RawCodeOp |

### 3.2 C 形积木（包裹 body，可嵌套）

| Blockly 类型 | 视觉结构 | 映射 CodeOp |
|-------------|---------|------------|
| `if_block` | `[if] ◇condition◇` + C形开口包裹 body + 可变数量的 elif/else 分支 | IfBlockOp |
| `while_loop` | `[while] ◇condition◇` + C形开口包裹 body | WhileLoopOp |
| `for_in` | `[for] [item∶] [in] [collection∶]` + C形开口包裹 body | ForLoopOp |

### 3.3 if_block 的 Mutation（动态分支）

Blockly mutation 机制管理 if/elif/else 的可变分支：

```
创建时:  [if] ◇condition◇ + body区
+ 按钮:  [if] ◇condition◇ + body区 + [else] + body区
++ 按钮: [if] ◇condition◇ + body区 + [elif] ◇condition◇ + body区 + [else] + body区
```

每个 elif/else 分支在同一个积木实例内部动态增减。

### 3.4 槽位符号约定

- `∶` = 文本槽（`SlotValue { kind: "text" }`）—— 键盘输入直接显示字符
- `◇` = 表达式槽（`SlotValue { kind: "expr" }`）—— 键盘输入弹出 fuzzy search

### 3.5 颜色方案

初期使用 **Blockly Zelos（Scratch 3.0）默认配色**，后续按需调整。

## 4. 交互模型

### 4.1 三面操作

**面 1：积木网格（常态，≈ 导航 + 基础操作）**

- `← → ↑ ↓` = 空间导航。选择框按空间顺序跳转（行内左右、跨行上下）
- 字符键 = 在 slot/gap 上：立即弹出 fuzzy search 浮层，该字符作为过滤输入的第一个字符。继续输入则继续过滤。选择框在积木上时字符键无反应
- `Enter` = 在 slot/gap 上：弹出 fuzzy search 浮层（输入为空→展示全部选项）
  - 在 gap 上额外效果：**增加一个 gap 行**
- `Delete` = 删积木 / 删 gap 行
- `Backspace` = 删积木 / 合并到上一行
- `\` = 弹出原始文本输入框（存储为 RawCodeOp）
- `Ctrl+C/V/X` = 复制/粘贴/剪切积木

**面 2：Fuzzy search 浮层（在 slot/gap 触发）**

- `↑ ↓` = 在提示列表中上下选
- `Enter` = 选中当前高亮项 → 插入积木或值 → 回到网格
- `Esc` = 取消 → 回到网格
- 继续输入 = 模糊过滤（VS Code Command Palette 式）

**面 3：文本输入框（按 `\` 触发）**

- `Enter` = 提交文本 → 存为 RawCodeOp（**不解析**）
- `Esc` = 取消 → 回到网格

### 4.2 选择框（手柄式光标）

视觉形态是一个矩形选择框（类似游戏手柄选中），在积木网格中移动：

```
积木    [let] [name∶ count] [=] [value∶ 0]     ← ← → → 在积木间跳转
                                                ↑ ↓ 跨行跳转
插入点  ───────────────────────────────         ← 闪烁的水平线
slot    [  name∶ █  ]                            ← 槽位选中时高亮
```

与 Blockly `@blockly/keyboard-navigation` 插件集成，在其标记系统之上包装选择框样式。

### 4.3 初始状态

打开空编辑器时，workspace 显示一个 gap 行，等待用户输入第一个积木。

### 4.4 槽位编辑细节

- **文本槽** `[name∶]`：fuzzy search 结果范围为可输入的文本值类型（var_ref、literal、type_name）。选中后值填入槽位。
- **表达式槽** `◇condition◇`：fuzzy search 结果返回全部 InlineOp。选中后积木插入槽内，可继续在槽内嵌套。Backspace 在槽内逐字/逐积木删除。
- **空 slot** `{ kind: "empty" }`：选择框落上时显示 placeholder 占位文字（如"条件"、"变量名"）

## 5. 技术栈

| 层 | 方案 |
|---|------|
| 积木引擎 | Blockly v12+（npm `blockly`） |
| React 集成 | `react-blockly` 或自写 thin wrapper |
| 外观 | Zelos renderer（Scratch 3.0 风格） |
| 键盘导航 | `@blockly/keyboard-navigation` 插件 |
| Fuzzy search | 自定义 Blockly Field（替代默认 dropdown/text） |
| 文本输入 | 自定义 modal textarea（`\` 触发） |
| 不可拖拽 | 关闭 Blockly drag 策略 |

## 6. BlockAdapter 设计

### toBlocks(ops: CodeOp[]): Blockly Block[]

```
CodeOp[] 遍历
├── DeclareVarOp → declare_var 积木
├── SetVarOp     → set_var 积木
├── IfBlockOp    → if_block 积木（含 mutation 配置 elif/else 分支）
├── WhileLoopOp  → while_loop 积木（C 形 + body）
├── ForLoopOp    → for_in 积木（C 形 + body）
├── CallFnOp     → call_fn 积木
├── AwaitExprOp  → await_expr 积木
├── ReturnExprOp → return_stmt 积木
├── BreakStmtOp  → break_stmt 积木
├── ContinueStmtOp → continue_stmt 积木
├── CommentOp    → comment_stmt 积木
└── RawCodeOp    → raw_stmt 积木
```

嵌套 body（body: CodeOp[]）递归调用 toBlocks 填入积木的 body 连接区。

### fromBlocks(workspace): CodeOp[]

```
workspace.getTopBlocks() 遍历
├── 从积木类型 + 字段值反向构造 CodeOp
├── 递归遍历 C 形积木的 body 连接区获取嵌套 body
├── 从 if_block 的 mutation 获取分支结构
└── SlotValue 按字段类型还原（text/expr/empty）
```

### Roundtrip 保证

- `fromBlocks(toBlocks(ops))` 必须 ≈ `ops`（语义等价，不要求严格字面等值）
- Pid 保持不变

## 7. 待删除的旧代码

| 文件/目录 | 原因 |
|----------|------|
| `lexical/nodes/CardNode.ts` | Lexical ElementNode，被 Blockly 积木替代 |
| `lexical/nodes/SlotCardNode.ts` | Lexical ElementNode，被 Blockly field 替代 |
| `lexical/nodes/BodyLineNode.ts` | 被 Blockly C 形积木 body 替代 |
| `lexical/nodes/ContainerElementNode.ts` | 已未注册，直接删除 |
| `lexical/nodes/BlockElementNode.ts` | 已未注册，直接删除 |
| `lexical/plugins/KeyboardPlugin.ts` | 被 Blockly 键盘导航替代 |
| `lexical/plugins/AutocompletePlugin.ts` | 被 FuzzySearchField 替代 |
| `lexical/plugins/CodeOpSyncPlugin.ts` | 被 BlockAdapter 替代 |
| `lexical/utils/convertToNodes.ts` | 转换逻辑被 BlockAdapter 替代 |
| `lexical/utils/convertToOps.ts` | 转换逻辑被 BlockAdapter 替代 |
| `lexical/` 目录（如果已空） | Lexical 完全移除 |
| `CodeCardEditor.module.css`（旧样式） | 被 Blockly workspace 样式替代 |
| `editor-node.ts`（EditorNode 类型） | 中间层不需要，CodeOp ↔ Blockly 直接转换 |

## 8. 待创建的新文件

| 文件 | 职责 |
|------|------|
| `BlockEditor.tsx` | 新的积木编辑器 React 组件（替代 CodeCardEditor） |
| `BlockEditor.module.css` | Blockly workspace 容器样式 |
| `BlockAdapter.ts` | CodeOp ↔ Blockly 双向转换 |
| `blocks/definitions.ts` | Blockly 积木 JSON 定义（13 种积木） |
| `blocks/fields/FuzzySearchField.ts` | 模糊搜索 Field（Blockly Field 子类） |
| `blocks/renderer/ScratchRenderer.ts` | Zelos 子类化 Renderer（C 形 + 拼图咬合） |
| `blocks/keyboard.ts` | 键盘导航配置（方向键映射 + 自定义快捷键） |
| `blocks/mutations.ts` | if_block mutation helper |

## 9. 测试策略

| 层 | 测试方法 |
|---|---------|
| BlockAdapter roundtrip | CodeOp[] → blocks → CodeOp[]，验证语义等价 |
| BlockAdapter 向后兼容 | 旧 CodeOp 加载（string 字段）→ 验证无损转换 |
| FuzzySearchField | 单元测试：模糊匹配精度、排序、选中回调 |
| BlockEditor 集成 | Playwright 测试：键盘导航、插入积木、删除、Ctrl+C/V |
| Blockly block definitions | 每类积木的创建/连接/mutation 测试 |
| 旧 Lexical 测试 | 随旧代码一起删除 |

## 10. 不在本期范围内

- 自定义积木颜色（先用 Zelos 默认）
- 拖拽交互（纯键盘操作）
- 多选积木
- Undo/Redo（留给后续集成 Blockly 内置 undo）
- 代码编辑器中的文本模式（`\` 输入框暂代）
- SlotValue 中的数据流映射（向后兼容，旧 CodeOp string 自动转 SlotValue）

## 11. 风险与缓解

| 风险 | 缓解 |
|------|------|
| Blockly v12 keyboard-navigation 插件尚在 beta | 优先评估，若有硬件伤则用 v11 + 自建键盘导航 |
| react-blockly 维护状态不明 | 评估后决定是否自写 thin wrapper |
| CodeOp ↔ Blockly 转换遗漏 edge case | Roundtrip 测试覆盖所有 13 种积木类型 |
| Blockly bundle 体积（~500KB gzipped） | 仅在工作区打开时动态加载（lazy import） |
