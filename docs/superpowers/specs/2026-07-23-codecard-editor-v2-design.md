# CodeCard Editor v2：细粒度结构化卡片编辑器

> 状态：设计完成 | 2026-07-23
>
> 替代 [codecard-editor-design.md](./2026-07-23-codecard-editor-design.md) 和 [codecard-editor-lexical-arch.md](./2026-07-23-codecard-editor-lexical-arch.md)。

## 核心原则

**纯卡片组合式编程**：每个语义 token 都是一张独立卡片。结构由打字匹配插入卡片，只有命名和字面值用 `\` 唤起内联输入框。

- **打字匹配 → 插入卡片**（let, mut, set, if, while, for, return, await, break, continue, call, 运算符, 类型, 变量引用...）
- **`\` → 内联输入框**（变量名、值表达式、条件、函数名）
- **纯键盘操作 + Ctrl+C/V**

---

## 1. 卡片类型体系

### 1.1 语句级卡片（可在空白处直接触发）

| 卡片 | 类型 | 颜色 | 说明 |
|------|------|------|------|
| `let` | 容器 | 紫色系 | 变量声明容器，包裹内部子卡 |
| `set` | 轻容器 | 蓝色系 | 赋值容器，常驻运算符提示（Tab 切换 `=` / `+=` / `-=` / `*=` / `/=` / `%=`） |
| `if` | 块结构 | 黄色系 | 条件分支，三行封闭色条 |
| `while` | 块结构 | 橙色系 | while 循环，三行封闭色条 |
| `for` | 块结构 | 绿色系 | for..in 循环，选中自动附带 `in` 只读卡，三行封闭色条 |
| `return` | 轻容器 | 红色系 | 返回值容器 |
| `await` | 轻容器 | 青色系 | 异步等待容器 |
| `break` | 单卡 | 灰色系 | 跳出循环 |
| `continue` | 单卡 | 灰色系 | 跳过本次 |
| `call` | 轻容器 | 青色系 | 函数调用，`(` `)` 是容器边界视觉元素 |

### 1.2 容器内触发的修饰/运算符卡（打字匹配）

| 卡片 | 触发范围 | 说明 |
|------|---------|------|
| `mut` | 仅 `let` 容器 | 可变修饰 |
| `:` | 仅 `let` 容器 | 类型标注分隔 |
| `=` | `let` / `set` | 赋值/初始化 |
| `+=` `-=` `*=` `/=` `%=` | `set` | 复合赋值（Tab 循环切换） |
| `>` `<` `>=` `<=` `==` `!=` | 条件位置（if/elif/while 头行） | 比较运算符 |
| `+` `-` `*` `/` `%` | 表达式位置 | 算术运算符 |
| `&&` `\|\|` `!` | 表达式位置 | 逻辑运算符 |
| `.` | 引用卡后 | dot 访问，触发方法补全 |
| `,` | call / array / map | 参数/元素分隔 |

### 1.3 值卡

| 卡片 | 说明 |
|------|------|
| 类型卡 | `i32` `i64` `f32` `f64` `String` `bool` `Array<T>` |
| 字面量卡 | `true` `false` `null` |
| 变量引用卡 | 已有变量名、store 名、signal 名 |

### 1.4 `\` 输入框（上下文自动判定类型）

| 上下文位置 | 输入框类型 |
|-----------|-----------|
| `let` / `set` / `for` 名字位 | 变量名输入 |
| `:` 后面 | 类型输入 |
| `=` / 运算符后面 | 值表达式输入 |
| `if` / `elif` / `while` 头行 | 条件表达式输入 |
| `call` 内部 | 函数名输入 |

### 1.5 特殊卡

| 卡片 | 类型 | 说明 |
|------|------|------|
| `//` | text block | 纯注释文本，灰色半透明（无 `/` 冲突，因为输入框改用 `\`） |
| `[` / Array | 轻容器 | 数组字面量，`[` `]` 是边界视觉元素，`,` 分隔元素 |
| `{` / Map | 轻容器 | 对象字面量，`{` `}` 是边界视觉元素，`,` 分隔键值 |

### 1.6 块结构内部卡

| 卡片 | 触发位置 | 说明 |
|------|---------|------|
| `elif` | 仅 `if` 块内部 | 同色条连接，条件在头行 |
| `else` | 仅 `if` 块内部 | 同色条连接，无头行条件 |

---

## 2. 数据模型

### 2.1 EditorNode（编辑器内部格式）

细粒度嵌套树，取代当前的扁平行模型。

```typescript
// 顶层：一个 code-block 就是 EditorNode[] 的序列
type CodeBlock = EditorNode[];

type EditorNode =
  | ContainerNode    // let, set, return, await, call, array, map
  | BlockNode        // if, while, for
  | InlineNode       // 非容器的独立卡
  | InputSlot;       // \ 唤起的内联输入框

interface ContainerNode {
  kind: "let" | "set" | "return" | "await" | "call" | "array" | "map";
  pid: string;
  children: EditorNode[];  // 内部流式排列的子卡/输入框
}

interface BlockNode {
  kind: "if" | "while" | "for";
  pid: string;
  head: EditorNode[];       // 头行：条件/循环表达式（流式卡 + 输入框）
  body: BodyLine[];         // 中间代码行（可无限扩展）
  tail: TailLine;           // 收口行（细条）
  // if 专属
  branches?: Branch[];      // elif / else 分支
}

interface Branch {
  kind: "elif" | "else";
  pid: string;
  head: EditorNode[];
  body: BodyLine[];
}

interface BodyLine {
  pid: string;
  children: EditorNode[];
}

interface TailLine {
  pid: string;
}

interface InlineNode {
  kind: "mut" | "break" | "continue" | "operator" | "type" | "var_ref" | "literal" | "dot" | "separator" | "comment";
  cardLabel: string;        // 显示文字，如 ">" / "+=" / "i32" / "sum"
  pid: string;
}

interface InputSlot {
  kind: "slot";
  slotKind: "name" | "type_value" | "expression" | "condition" | "fn_name";
  value: string;
  pid: string;
}
```

### 2.2 为什么是嵌套树而非扁平 pid 引用？

1. **视图直接映射**：`let` 容器渲染子卡 → `children: []`。块结构三行 → `head / body / tail`
2. **操作简单**：Delete `let` → 整棵子树干掉。Ctrl+Z → 恢复子树
3. **与现有 CodeOp 衔接**：`IfBlockOp.branches[].body` 等已嵌套，细粒度化把嵌套延伸到 let/set/return 容器层

### 2.3 数据流

```
CodeCardEditor (EditorNode[] 嵌套树, 内存中)
  ↕ 实时双向
Lexical 节点树 (React 视图层)

── 打开时 ──
CodeOp[] → convertToNodes() → EditorNode[]

── 保存时 ──
EditorNode[] → convertToOps() → CodeOp[]
  → emitOps() → source 字符串
  → 更新 CodeblockElementData.source + operations
```

| 层 | 格式 | 用途 |
|----|------|------|
| `.omni` 源码 | 文本字符串 | 持久化、编译器消费 |
| `CodeOp[]` | 粗粒度结构化操作 | 画布侧分析/检查/诊断 |
| `EditorNode[]` | 细粒度嵌套树 | 编辑器内部状态 |

### 2.4 转换关键示例

```
CodeOp → EditorNode:
  DeclareVarOp { mutable:true, name:"count", varType:"i32", initValue:"0" }
    → ContainerNode { kind:"let", children: [
        InlineNode(mut), InputSlot(name:"count"), InlineNode(:),
        InlineNode(type:"i32"), InlineNode(=), InputSlot(expression:"0")
      ]}
  SetVarOp { name:"sum", operator:"+=", value:"sum + 1" }
    → ContainerNode { kind:"set", children: [
        InputSlot(name:"sum"), InlineNode(operator:"+="), InputSlot(expression:"sum + 1")
      ]}
  IfBlockOp { branches: [{condition:"x>0", body:[...]}] }
    → BlockNode { kind:"if", head: [/* tokenized condition */], body: [...], tail }

EditorNode → CodeOp:
  ContainerNode(let, children) → DeclareVarOp (从 children 提取 mut/name/type/init)
  ContainerNode(set, children) → SetVarOp (提取 name/operator/value)
  BlockNode(if, head, body, branches) → IfBlockOp (条件从 head 反解, body 递归)
```

---

## 3. 交互模型

### 3.1 光标三态

| 光标形态 | 位置 | 行为 |
|---------|------|------|
| **卡片高亮** | 光标落在一张卡上 | ← → 移到邻卡，Delete 删此卡 |
| **竖线 `│`** | 两张卡之间 / 空行 | 开打字触发补全，`\` 唤起输入框 |
| **方块 `▌`** | 容器右侧边界外 | Delete → 整容器含子卡删除，Enter → 下方另起一行 |

### 3.2 卡片插入

```
在任意"竖线│"位置直接打字
  ↓
补全菜单弹出（按字母前缀过滤）
  ↓ Enter 选中 → 卡片插入光标位

触发范围：
  空白处 → let / set / if / while / for / return / await / break / continue / call
  let 容器内 → mut / : / = / 类型卡 / //
  set 容器内 → = / += / -= 等 / 变量引用卡 / \
  if / while 头行 → 变量引用卡 / 比较运算符 / 逻辑运算符 / \
  for 头行 → 变量引用卡 / \
  return 容器内 → 变量引用卡 / 运算符 / \
  任何引用卡后 → .
```

### 3.3 `\` 内联输入框

- 只在合适位置按 `\` 才唤起，上下文自动判定类型
- 输入框不折叠，始终可见，内容可随时编辑
- Tab / → 离开输入框

### 3.4 块结构操作

| 按键 | 效果 |
|------|------|
| Enter（光标在 body line 最右侧） | 新增一行 body line |
| ← → | 在头行/body/输入框之间移动 |
| ↑ ↓ | 在 body lines 之间上下移动 |
| `elif` / `else` 卡 | 仅能在 if 块内部触发，同色条同级连接 |

### 3.5 删除层级

| 光标位置 | Delete 效果 |
|---------|------------|
| 内联卡片/输入框上（高亮） | 删除此卡/槽 |
| 容器右侧边界（▌方块） | 删除整容器 + 所有子卡 + 子输入框 |
| body line 上（▌） | 删除此行及其内容 |
| if 块的 elif/else 头上（▌） | 删除此分支（含 head + body） |

### 3.6 特殊交互

| 操作 | 效果 |
|------|------|
| **Tab**（在 `set` 内运算符位） | 循环选择 `=` / `+=` / `-=` / `*=` / `/=` / `%=` |
| **. （dot）** | 光标在引用卡后输入 `.` → 方法补全（`.len()` `.push()` `.get()` ...） |
| **Ctrl+Z / Ctrl+Shift+Z** | 撤销/重做（卡片级操作） |
| **Ctrl+C / Ctrl+V** | 复制/粘贴选中卡片或整容器 |

### 3.7 容器内语法提示

- 子卡顺序自由，但不符合语法规则时 → 卡片下方淡黄色波浪线（lint 级别）
- 不阻止编辑，不影响保存
- 引用不存在变量 → 红色下划线 + hover 提示

### 3.8 `for` 特殊行为

- 选中 `for` 卡后自动附带 `in` 只读分隔卡
- 仅 `in` 左右两槽位需用户填入（itemName 用 `\` 输入框 / collection 用引用或 `\`）

---

## 4. 视觉模型

### 4.1 卡片原子

所有卡片统一外形：**小圆角色块** + 语义色。流式行内排列，超出自动换行。

| 颜色 | 语义 |
|------|------|
| 紫色 | 变量声明 (let) |
| 蓝色 | 赋值 (set) |
| 橙色 | 修饰符 (mut) |
| 灰色 | 运算符/分隔符 (=, +=, :, ,, in, .) |
| 青色 | 类型/函数调用 (i32, String, bool, call) |
| 绿色 | 字面量/变量引用 (true, null, sum) |
| 红色 | return |
| 黄色 | if 块 |
| 橙色 | while 块 |
| 绿色 | for 块 |

### 4.2 容器卡

容器卡 = 关键词卡 + 浅色半透明背景包裹子卡（宽度=内容宽），右侧空白区为方块光标区：

```
  ╭ let ─────────────────────────╮
  │ ┌─────┐ ┌───────────┐ ┌───┐ ┌──────┐ ┌───┐ ┌───────────┐ │  ▌
  │ │ mut │ │ count \框 │ │ : │ │ i32  │ │ = │ │ 0 \框     │ │
  │ └─────┘ └───────────┘ └───┘ └──────┘ └───┘ └───────────┘ │
  ╰──────────────────────────────────────────────────────────╯
```

### 4.3 `set` 轻容器

和 `let` 同样视觉，常驻运算符半透明水印提示（Tab 切换）：

```
  ╭ set ──────────────────────╮
  │ ┌───────────┐ ┌───┐ ┌───┐ │  ▌
  │ │ count \框 │ │ = │ │ 1 │ │
  │ └───────────┘ └───┘ └───┘ │
  ╰───────────────────────────╯
```

### 4.4 `call` 轻容器

`(` `)` 是容器边界视觉元素，不是独立卡片：

```
  ╭ call ───────────────────────────╮
  │ ┌───────────┐ ( ┌──────┐ , ┌───┐)│  ▌
  │ │ print \框 │   │ msg  │   │ \ │  │
  │ └───────────┘   └──────┘   └───┘  │
  ╰───────────────────────────────────╯
```

### 4.5 `return` / `await` 轻容器

```
  ╭ return ────────────╮
  │ ┌───┐ ┌───┐ ┌───┐ │  ▌
  │ │ x │ │ + │ │ 1 │ │
  │ └───┘ └───┘ └───┘ │
  ╰────────────────────╯
```

### 4.6 块结构（三行封闭色条）

头行 `┏`、代码行 `┃`（Enter 可无限扩展）、收口行 `┗`（细条）。色条颜色 = 块类型。

```
┏ if [x] > [0]                    ← 头行
┃   ╭ set ──────────────╮        ← 代码行：左侧色条 + 缩进
┃   │ [result] = [1]    │
┃   ╰───────────────────╯
┃   ╭ return ──────╮
┃   │ [result]     │
┃   ╰──────────────╯
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  ← 收口行（细条）
```

### 4.7 `elif` / `else`（在 if 块内部，同色条同级）

```
┏ if [x] > [0]
┃     ...
┏ elif [x] < [0]        ← 同色条，同级
┃     ...
┏ else                   ← 同色条，同级
┃     ...
┗━━━━━━━━━━━━━━━━━━━━━━
```

### 4.8 `\` 内联输入框

```
┌───────────┐
│ count     │    ← 白色底，细边框，光标在内部闪烁
└───────────┘
```

- 始终可见，不折叠
- 背景白色/浅色，与卡片区分
- 宽度自适应内容，有最小宽度

### 4.9 Array / Map 容器

```
  ╭ [ ───────────────────────╮
  │ ┌───┐ , ┌───┐ , ┌───────┐│  ▌
  │ │ 1 │   │ 2 │   │ 3 \框 ││
  │ └───┘   └───┘   └───────┘│
  ╰──────────────────────────╯
```

### 4.10 语法错误提示

```
┌──────┐
│ i32  │     ← 正常
└──────┘
┌──────┐
│ i32  │     ← 顺序不对时淡黄波浪下划线
└─﹏﹏──┘
```

---

## 5. 组件架构

```
CodeCardEditor/
├── CodeCardEditor.tsx              ← 编辑器主入口
│   ├── LexicalComposer (单一实例)
│   │   ├── RichTextPlugin
│   │   │   └── ContentEditable    ← 唯一编辑区
│   │   ├── HistoryPlugin          ← Ctrl+Z/Y
│   │   ├── CardFlowPlugin         ← 卡片流式排版 + 行内换行
│   │   ├── AutocompletePlugin     ← 打字匹配 + 补全菜单
│   │   ├── KeyboardPlugin         ← ← → ↑ ↓ Tab Enter Delete \
│   │   ├── ClipboardPlugin        ← Ctrl+C/V 序列化/反序列化
│   │   ├── NodeSyncPlugin         ← EditorNode 树 ↔ Lexical 节点树 同步
│   │   └── SyntaxLintPlugin       ← 语序错误淡黄波浪线
│   ├── AutocompleteMenu.tsx        ← 浮动补全菜单
│   └── styles.module.css

├── nodes/                          ← Lexical 自定义节点
│   ├── CardNode.ts                 ← InlineNode（所有非容器卡片）
│   ├── ContainerNode.ts            ← ContainerNode（let/set/return/await/call/array/map）
│   ├── BlockNode.ts                ← BlockNode（if/while/for）
│   ├── BodyLineNode.ts             ← 块内代码行
│   ├── InputSlotNode.ts            ← \ 输入框（contentEditable span）
│   └── index.ts

├── plugins/
│   ├── CardFlowPlugin.ts           ← 流式排版 + 自动换行
│   ├── AutocompletePlugin.ts       ← 打字检测 → 上下文感知补全
│   ├── KeyboardPlugin.ts           ← 光标导航 + 特殊键位
│   ├── ClipboardPlugin.ts          ← 序列化/粘贴
│   ├── NodeSyncPlugin.ts           ← EditorNode ↔ Lexical 双向转换
│   └── SyntaxLintPlugin.ts         ← 语序错误的淡黄波浪线

├── autocomplete/
│   ├── autocomplete-rules.ts       ← 补全规则（按上下文过滤）
│   └── AutocompleteMenu.tsx        ← 浮动菜单 UI

├── transforms/
│   ├── convertToNodes.ts            ← CodeOp[] → EditorNode[]（加载）
│   ├── convertToOps.ts              ← EditorNode[] → CodeOp[]（保存）
│   ├── text-to-card.ts              ← 文本 → 卡片（打字检测）
│   ├── card-to-text.ts              ← 卡片退化（空卡 → 文本）
│   └── slot-operations.ts           ← 输入框创建/删除

└── __tests__/
```

### 现有代码处理

| 文件 | 处理 |
|------|------|
| `card-editor/` 整目录 | **删除** |
| `lexical/nodes/CardElementNode.ts` | **重写** → 拆为 CardNode + ContainerNode + BlockNode |
| `lexical/nodes/SlotTextNode.ts` | **重写** → InputSlotNode |
| `lexical/autocomplete/autocomplete-rules.ts` | **重写** — 细粒度卡片上下文 |
| `lexical/plugins/*` | **大部分重写** |
| `lexical/transforms/text-to-card.ts` | **重写** — 新卡片类型 |
| `CodeCardEditor.tsx`（顶层） | **重写** — 新节点 + 新插件 |
| `core/types/code-op.ts` | **保留不变** — EditorNode 承载细粒度 |
| `core/types/editor-node.ts` | **新增** |
| `services/codegen/` 下 emit/parse | **扩展** — 细粒度往返 |

---

## 6. 错误处理

| 场景 | 处理方式 |
|------|---------|
| 子卡语序不对 | 淡黄波浪线，不阻止编辑 |
| 必填槽位为空（`let █ = 1` 缺名字） | `\` 框显示浅红占位文字"变量名"，保存用空串 |
| 块结构不完整（if 有头行无 body） | 允许保存空 body |
| 引用不存在变量 | 引用卡红色下划线 + hover "未定义变量" |
| `\` 在不合法位置唤起 | 不响应 |
| 容器内打了不合法卡（如 let 内打 set） | 补全菜单不返回不合法的项 |
| 粘贴卡片无法解析 | 粘贴为 `//` 注释文本块包裹原始内容 |
| Lexical 初始化崩溃 | Fallback 到 Monaco 模态窗（保留现有机制） |
| 解析旧 source 失败 | 整段标记为 RawCodeOp + 编辑器只读注释块展示 |

---

## 7. 向后兼容

| 层面 | 策略 |
|------|------|
| 旧 source 无 operations | 加载时从 source 调用 `parseOpsWasm` 生成 CodeOp[]，再转 EditorNode[] |
| 旧 source 字符串 | 保留，双向转换保证 emit → parse 往返一致 |
| `.omni` 文件 | 不添加任何新标注或 PID，输出干净 Omni 源码 |
| Monaco fallback | 保留 CodeEditorModal + 文本模式按钮 |
| 画布 API | `onSave(operations: CodeOp[])` 签名不变 |

---

## 8. 实现阶段

### Phase 1：数据层 + 转换器
1. 定义 `EditorNode` 类型
2. 实现 `convertToNodes()` 和 `convertToOps()`
3. 扩展 emit/parse 细粒度支持
4. 单元测试：所有 CodeOp 类型往返转换

### Phase 2：Lexical 节点层
1. 创建 CardNode、ContainerNode、BlockNode、BodyLineNode、InputSlotNode
2. 注册到 Lexical
3. createDOM + CSS 样式实现
4. 节点单元测试

### Phase 3：插件 + 交互
1. CardFlowPlugin — 流式排版
2. AutocompletePlugin — 上下文感知补全 + `\` 输入框
3. KeyboardPlugin — 光标三态、导航、Delete 层级
4. ClipboardPlugin — Ctrl+C/V
5. SyntaxLintPlugin — 语序提示
6. NodeSyncPlugin — 树同步

### Phase 4：集成 + UI
1. 重写 CodeCardEditor 容器组件
2. 连接 FlowCanvas 双击
3. Monaco fallback 保留
4. 清理旧 card-editor/ 代码

### Phase 5：测试 + 打磨
1. Playwright E2E：打字匹配 → 补全 → 插卡 → 填槽 → 保存 → 编译
2. 键盘导航流程
3. 回归测试 learn-omniview
4. 性能优化

---

## 9. 不做的东西

- @store / @for / @mount / @effect — 归 Poiesis 可视化界面编辑器
- 拖拽移动卡片 — 纯键盘操作
- 多光标编辑
- Emmet 类快捷方式
