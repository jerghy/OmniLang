# CodeCard Editor：Lexical 基座架构设计

> 状态：设计完成 | 2026-07-23
>
> 补充 [codecard-editor-design.md](./2026-07-23-codecard-editor-design.md)，细化编辑器基座实现方案。

## 架构决策

| # | 决策 | 结论 | 理由 |
|---|------|------|------|
| 1 | 编辑器基座 | **Lexical** | DecoratorNode/ElementNode 原生支持嵌入组件；光标管理、IME、选区由框架处理，不重造轮子 |
| 2 | 单/多 Lexical 实例 | **单一实例** | 卡片用自定义 ElementNode 而非 DecoratorNode，光标自然流动，无需手动跳进跳出 |
| 3 | 卡片双态 | **单态** | 卡片始终可编辑，光标在哪就能打字，无"进入编辑"切换 |
| 4 | Monaco fallback | **双模式** | 保留 Monaco 模态窗作为 fallback，提供"切换到文本模式"按钮 |
| 5 | 卡片状态 | **单态始终可编辑** | 简化交互，无模式切换 |

---

## 1. 核心结构：单一 Lexical 实例

### 1.1 节点类型

```
外层 Lexical Editor (contentEditable)
│
├── ParagraphNode                          ← 空行/纯文本行/未补全的文本
│   └── TextNode("") | TextNode("use")     ← 用户正在输入时，文本在此累积
│
├── CardElementNode                        ← 自定义 ElementNode，CSS 画卡片边框
│   ├── TextNode("let")                    ← 只读关键词（color: #7c3aed）
│   ├── TextNode(" ")
│   ├── SlotTextNode("sum")                ← 变量名槽位：可编辑 + 补全
│   ├── TextNode(" : ")
│   ├── SlotTextNode("f64")                ← 类型槽位：类型补全
│   ├── TextNode(" = ")
│   └── SlotTextNode("0.0")               ← 值槽位：表达式补全
│
├── CardElementNode (if_block)             ← 块状卡片（含子卡片）
│   ├── TextNode("if")
│   ├── SlotTextNode("cond")               ← 条件槽位
│   ├── TextNode(" {")
│   ├── LineBreakNode
│   ├── CardElementNode (body child)       ← 子卡片（缩进显示）
│   ├── LineBreakNode
│   └── TextNode("}")
│
└── ParagraphNode                          ← 光标落点，等待下一次输入
```

### 1.2 自定义节点

```typescript
// SlotTextNode — 槽位节点，继承 TextNode
// 标记哪些文本属于"槽位"（变量名/类型/值），用于：
//   1. 槽位级键盘导航（Tab/Shift+Tab 跳槽位）
//   2. 槽位级补全上下文（在类型槽位只提示类型名）
//   3. 槽位级错误标记（红色下划线）
class SlotTextNode extends TextNode {
  __slot: "name" | "type" | "value" | "condition" | "collection" | "item";
  
  static getType(): string { return "slot-text"; }
  static clone(node: SlotTextNode): SlotTextNode;
  createDOM(): HTMLElement;  // 渲染为 <span class="slot-text slot-text--name">
}

// CardElementNode — 卡片容器
// 子节点是 TextNode + SlotTextNode 的有序组合
class CardElementNode extends ElementNode {
  __cardKind: CodeOp["kind"];  // "declare_var" | "set_var" | "if_block" | ...
  __cardPid: string;            // 编辑器内部 pid
  
  static getType(): string { return "card"; }
  createDOM(): HTMLElement;
  // 渲染为 <div class="code-card code-card--declare-var">，内含子节点
}
```

**为什么 CardElementNode 不用 DecoratorNode？**

DecoratorNode 对 Lexical 是黑盒——光标不会进入内部。要编辑槽位就必须手动管理焦点跳入跳出。而 ElementNode 的子节点仍是 Lexical 管理的 TextNode，光标自然流动。

---

## 2. 补全系统

### 2.1 触发逻辑

```
用户打字 → TextNode 文本更新 → 检测光标位置
  ↓
提取光标前的连续文本 token（从上一个空格/卡片边界到光标位置）
  ↓
token → 匹配补全规则 → 弹出 AutocompleteMenu
  ↓ Enter 选中
  ↓
执行 transform：
  - 若光标在 ParagraphNode 中 → 删除匹配 token 的 TextNode 部分
    → 替换为 CardElementNode（关键词语法补全）
  - 若光标在 SlotTextNode 中 → 替换槽位文本（槽位补全）
```

### 2.2 上下文感知补全

| 光标位置 | 匹配上下文 | 补全项 |
|----------|-----------|--------|
| ParagraphNode 中，输入 `l` | 语句起始 | `let`, `let mut`, `while`（过滤后） |
| `let █` 中 SlotTextNode(name) | 变量名槽位 | 已有变量名（避免重名） |
| `let name : █` 中 SlotTextNode(type) | 类型槽位 | `i32`, `String`, `f64`, `bool`, `Array<T>` |
| `let name : T = █` 中 SlotTextNode(value) | 值槽位 | 变量名 + 字面量模板 |
| `set █` 中 SlotTextNode(name) | 赋值目标 | 已有变量名列表 |
| `if █` / `while █` 中 SlotTextNode(condition) | 条件槽位 | 变量名 + 比较运算符模板 |
| `fn.█` | 方法调用 | `.len()`, `.push()`, `.pop()`, `.to_string()` |

### 2.3 前缀过滤

```typescript
function filterAutocomplete(token: string, context: SlotContext): AutocompleteItem[] {
  const all = getItemsForContext(context);
  return all.filter(item => item.label.startsWith(token));
  // "l" → ["let", "let mut"]（while 也被滤掉，不以 l 开头）
}
```

---

## 3. 键盘交互

### 3.1 键位绑定

| 按键 | 行为 |
|------|------|
| `←` `→` | Lexical 默认光标移动（字符级） |
| `↑` `↓` | 上下行移动 |
| `Tab` | 跳到下一个 SlotTextNode；若无，插入缩进 |
| `Shift+Tab` | 跳到上一个 SlotTextNode |
| `Enter` | 光标在 ParagraphNode 中 → 换行；光标在补全菜单中 → 选中补全项执行 transform |
| `Delete` / `Backspace` | Lexical 默认删除；若删除后 CardElementNode 所有槽位为空 → 转换为 ParagraphNode（取消卡片） |
| `Ctrl+Z` | Lexical 内置撤销 |
| `Escape` | 关闭补全菜单；若菜单已关闭 → 关闭编辑器 |

### 3.2 卡片→文本退化

```
┌─────────────────────────┐
│ let sum : f64 = 0.0    ▌│  光标在卡片末
└─────────────────────────┘
       ↓ Backspace 删光所有槽位
┌─────────────────────────┐
│ let █ : █ = █           │  所有槽位为空
└─────────────────────────┘
       ↓ Backspace 再按一次 → 整卡退化为文本
use result;                 ← 恢复为 ParagraphNode + TextNode("let █ : █ = █")
                              （用户可继续编辑或重新补全）
```

---

## 4. 组件架构

```
CodeCardEditor/
├── CodeCardEditor.tsx              ← 编辑器主入口
│   ├── LexicalComposer             ← LexicalProvider + initialConfig
│   │   ├── CodeCardPlugins.tsx     ← 自定义插件集合
│   │   │   ├── CardNodePlugin      ← 注册 CardElementNode + SlotTextNode
│   │   │   ├── AutocompletePlugin  ← 监听文本变化，触发补全检测
│   │   │   ├── KeyboardPlugin      ← Tab/Enter/Delete 自定义行为
│   │   │   └── CodeOpSyncPlugin    ← 节点树 ↔ CodeOp[] 双向同步
│   │   └── LexicalContentEditable  ← contentEditable div，唯一编辑区
│   ├── AutocompleteMenu.tsx        ← 浮动补全菜单（复用现有）
│   ├── CardToolbar.tsx             ← 卡片悬停工具条（删卡、切 mut）
│   ├── EditorToolbar.tsx           ← Save / Cancel / Switch to Monaco
│   └── styles.module.css           ← 卡片边框、槽位着色
│
├── nodes/                          ← Lexical 节点定义
│   ├── SlotTextNode.ts             ← 槽位 TextNode 子类
│   └── CardElementNode.ts          ← 卡片 ElementNode
│
├── plugins/                        ← Lexical 插件
│   ├── CardNodePlugin.ts           ← 注册节点 + 序列化
│   ├── AutocompletePlugin.ts       ← 补全检测与触发
│   ├── KeyboardPlugin.ts           ← 自定义键盘行为
│   └── CodeOpSyncPlugin.ts         ← 节点树 ↔ CodeOp[] 同步
│
├── transforms/                     ← Lexical transform 函数
│   ├── text-to-card.ts             ← 文本 token → 卡片转换
│   ├── card-to-text.ts             ← 卡片退化 → 文本
│   └── slot-operations.ts          ← 槽位插入/删除/类型转换
│
└── cards/                          ← 【保留】现有卡片组件删除
    └── (移除 — 不再需要独立 React 卡片组件，由 CardElementNode 内部子节点渲染)
```

### 4.1 现有代码处理

| 文件 | 处理 |
|------|------|
| `CodeCardEditor.tsx` | 重写为 Lexical 容器 |
| `cards/CardList.tsx` | **删除** — 卡片由 Lexical 渲染 |
| `cards/DeclareVarCard.tsx` 等 | **删除** — 卡片结构由 CardElementNode + 子节点替代 |
| `cards/InlineSlot.tsx` | **删除** — 槽位由 SlotTextNode 替代 |
| `cards/AutocompleteMenu.tsx` | **保留并内联到外层** |
| `cards/card-registry.ts` | **删除** — 改用 transform 函数 |
| `cards/index.ts` | **删除** |
| `CodeEditorModal.tsx` | **保留** — 作为双模式 fallback |
| `core/types/code-op.ts` | **保留不变** — 数据模型不变 |

---

## 5. 数据流

```
用户打字
  ↓
Lexical onChange → 更新内部节点树
  ↓
CodeOpSyncPlugin: 节点树 → CodeOp[]
  ↓ (debounce, 保持内存)
CodeOp[] 暂存（不持久化到磁盘）
  ↓ 用户按 Save
emitOps(operations) → source 字符串
  ↓
更新 CodeblockElementData.source + operations
```

**同步方向：**

```
节点树 → CodeOp[]（编辑时实时）
  - CardElementNode → CodeOp（按 __cardKind 构造）
  - ParagraphNode + TextNode → RawCodeOp（未结构化文本）

CodeOp[] → 节点树（加载时）
  - CodeOp → CardElementNode + 子 TextNode/SlotTextNode
  - RawCodeOp → ParagraphNode + TextNode
```

---

## 6. 双模式

```
┌──────────────────────────────────────┐
│  Code Card Editor        [文本模式]  │  ← 工具条切换按钮
│                                      │
│  ┌──────────────────────────────┐    │
│  │ let sum : f64 = 0.0          │    │  ← Lexical contentEditable
│  │ while i < n {                │    │
│  │   sum = sum + values.get(i)  │    │
│  │   i = i + 1                  │    │
│  │ }                            │    │
│  │ return sum / count           │    │
│  └──────────────────────────────┘    │
│                                      │
│  [Save]  [Cancel]                    │
└──────────────────────────────────────┘

点击"文本模式" → 打开 CodeEditorModal (Monaco)
  - initialSource = emitOps(operations)
  - Monaco 编辑 → 保存 → parseOps(source) 还原 operations
  - 若解析失败 → RawCodeOp 降级
```

---

## 7. 错误处理

| 场景 | 处理 |
|------|------|
| 补全匹配失败（输入不匹配任何关键词） | 保留 TextNode 文本，继续等待输入 |
| CardElementNode 不完整（必填槽位为空） | 保存时不报错，emit 时跳过空槽位 |
| Monaco 文本→CodeOp 解析失败 | 整段标记为 RawCodeOp，保留原始文本 |
| Lexical 初始化失败 | Fallback 到 Monaco 模态窗 |

---

## 8. 测试策略

| 层 | 内容 | 工具 |
|----|------|------|
| 单元 | `text-to-card.ts` transform 函数：输入 "let" → CardElementNode | Vitest |
| 单元 | `card-to-text.ts` 退化：空卡片 → ParagraphNode | Vitest |
| 单元 | `CodeOpSyncPlugin` 双向转换一致性 | Vitest |
| 单元 | `emitOps()` / `parseOps()` 往返 | Vitest |
| 集成 | Lexical 内输入 "l" → 补全菜单出现 → Enter → 卡片渲染 | Playwright |
| 集成 | Tab 键在槽位间跳转 | Playwright |
| 集成 | Delete 删除卡片退化为文本 | Playwright |
| 集成 | 双模式切换：CardEditor ↔ Monaco → 源码一致 | Playwright |
| 回归 | learn-omniview 项目编译后行为不变 | Vitest + omnic build |
