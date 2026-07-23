# CodeCard Editor：Token 级结构化代码编辑器

> 状态：设计完成 | 2026-07-23

## 1. 问题陈述

### 现状

Poiesis 的 CodeFlow 画布有 4 种节点：`fn`（函数入口）、`condition`（条件分支）、`code-block`（自由代码）、`return`（返回）。其中 `code-block` 节点存储任意 OmniLang 源码为纯文本字符串（`CodeblockElementData.source`），双击后弹出 Monaco Editor 模态窗（`CodeEditorModal`）进行文本编辑。

### 黑箱问题

因为 code-block 内容是纯文本，Poiesis 编辑器对其内部完全无知：

1. **变量不可见**：`let x = 1` 声明的变量在画布上看不到
2. **状态变更不可追踪**：`count = count + 1` 等修改无法被编辑器感知
3. **调试困难**：无法查找"这个变量在哪里被修改"
4. **设计时容易遗忘**：设计师看不到代码块内部有什么

### 设计目标

保留 code-block 作为架构节点的角色，但将其内部编辑方式从**自由文本编辑**改为**结构化卡片编辑**——用键盘打字的速度来搭建结构化的积木块。

---

## 2. 现有方案调研

### 2.1 学术原型

#### Tylr（密歇根大学）— 最接近的交互模型

**[tylr.fun](https://tylr.fun)** | [github.com/hazelgrove/tylr](https://github.com/hazelgrove/tylr)

Tylr 是一个瓦片式结构编辑器（tile-based structure editor）。用键盘打字操作结构化的瓦片（tile），每次编辑保证能重新解析为合法的 AST。

**三层结构**：
| 层级 | 描述 |
|------|------|
| **Term** | 完整 AST 节点，显示为凸六边形轮廓 |
| **Tile** | 成对分隔符组（如 `let`/`≡`/`in`），term 可分解为 tile |
| **Shard** | 单个 token 和分隔符，最细粒度 |

**核心机制**：
- **Grouter**：每次编辑后自动插入/移除 Grout（占位符），维持语法完整性。凸 Grout = 缺操作数的空洞，凹 Grout = 缺运算符的占位
- **Backpack**：结构感知剪贴板。只允许搬运平衡的 tile 组合，防止括号不匹配
- **任意选区**：不同于 Scratch/MPS（只能选完整子树），Tylr 支持跨子树的 token 选区（如 `2 + 3 * 4` 中选 `* 4`）

**与 CodeCard Editor 的异同**：

| 维度 | Tylr | CodeCard Editor |
|------|------|----------------|
| 输入方式 | 键盘打字 | 键盘打字 |
| 结构单位 | Tile（瓦片） | Card（卡片） |
| 占位符 | Grout（自动插入/移除） | `█` 空槽位 |
| 删除粒度 | 瓦片级 | 逐级：槽值 → 子组件 → 整卡 |
| 选区能力 | 跨子树任意选区 | 卡片级 + 槽位级 |
| 实现语言 | Reason/OCaml（不可直接复用） | TypeScript + React |
| 成熟度 | 研究原型 | 生产功能 |

#### Hazel（hazel.org）

同一实验室（Cyrus Omar 组）的实时函数式编程环境。

**可借鉴的交互**：
- **关键词即语法前缀**：输入 `let` 自动被识别为 let 绑定结构，无需快捷键或菜单选择
- **自动空洞插入**：不完整/类型错误的位置自动插入类型化空洞，程序始终保持"可运行"
- **Node Staging Mode**："先调用再配置"。输入 `let … in` 后按 Backspace 进入 staging，可调整分隔符位置

#### Prong（Projectional JSON GUI）

[JSON DSL 投影编辑器](https://ar5iv.labs.arxiv.org/html/2307.11260)

**可借鉴的交互**：
- **结构化自动补全**：打字过滤 → 展示当前光标位置合法的结构类型 → 选中后插入带必填字段的完整对象
- **光标跟随菜单**：补全菜单跟随光标，像 IDE 提示框

### 2.2 工程化 Block Editor 参考

| 项目 | 平台 | 相关能力 |
|------|------|---------|
| **@blokhaus/core** | React + Lexical | `/` 命令菜单、自定义 block node、代码高亮（Shiki）、Markdown 快捷输入 |
| **Cascade Editor** | Kotlin/Compose | 跨平台 block editor、自定义 block、斜杠命令 |
| **Notion** | Web | `/` 命令触发结构化块插入的标杆体验 |

### 2.3 结论：无可直接复用的库

Tylr 和 Hazel 提供了概念验证和交互灵感，但它们都是学术原型（OCaml/Reason），无法直接集成到 Poiesis 的 React + TypeScript 技术栈中。现有的 Notion-like block editor 库（如 @blokhaus/core）解决的是"文档编辑"问题，而非"代码编辑"问题——它们不处理类型系统、变量作用域、表达式语义等代码领域需求。

**推荐实现策略**：基于 Lexical（Meta 的编辑器框架）自研 CodeCard Editor。理由：

1. Lexical 原生支持在文本流中嵌入自定义 React 组件（DecoratorNode）——这就是卡片
2. 成熟的 command 系统和 plugin 架构，适合实现键盘优先的补全
3. @blokhaus/core 已证明在 Lexical 上做 block editor 的可行性
4. 现有的 Monaco 语言服务（语法高亮、类型注入、WASM 诊断）可复用为槽位诊断

---

## 3. 设计范围（改动边界）

### 改动范围

```
FlowCanvas (ReactFlow)         → 不变
├── FnNode                     → 不变
├── ConditionNode              → 不变
├── CodeBlockNode              → 不变
│   └── 双击打开 CodeEditorModal → 改为 CodeCardEditor ✨
├── ReturnNode                 → 不变
└── wires                      → 不变

CodeblockElementData           → 扩展
├── source: string（保留，兼容）  → 由 operations 重新生成
└── operations: CodeOp[]（新增） → 结构化权威数据源

graph-compiler.ts              → 扩展
└── case "code-block" 原样emit  → 遍历 operations 逐条 emit 为 Omni 源码

graph-parser.ts                → 扩展
└── code-block segment → source → 解析内部操作 → operations[]
```

**唯一的大改动**：用 CodeCardEditor 替换 CodeEditorModal。其他均为小扩展。

### 不改的东西

- CodeFlow 画布的节点+连线交互方式
- 组件/Store 的 signal/prop/computed 声明方式
- Design System 编辑方式
- OmniLang 语言语法

---

## 4. 数据模型：CodeOp 操作类型

### 顶层结构

```typescript
// CodeblockElementData 扩展
interface CodeblockElementData {
  // ... 现有字段不变
  source: string;                // 保留兼容，由 operations 生成
  operations?: CodeOp[];         // 新增：结构化操作列表（权威数据源）
}

// 编辑器的读写单位
type CodeOp =
  | DeclareVarOp
  | SetVarOp
  | IfBlockOp
  | WhileLoopOp
  | ForLoopOp
  | ReturnExprOp
  | CallFnOp
  | AwaitExprOp
  | BreakStmtOp
  | ContinueStmtOp
  | CommentOp;
```

### 操作卡片类型定义

```typescript
// ── 变量声明 ──
interface DeclareVarOp {
  kind: "declare_var";
  mutable: boolean;            // false=let, true=let mut
  name: string;
  varType?: string;            // 可选，省略则类型推断
  initValue?: string;          // 可选初始值表达式
  scope: "fn" | "container";   // 作用域选择
}

// ── 变量赋值 ──
interface SetVarOp {
  kind: "set_var";
  name: string;                // 目标变量名
  operator: "=" | "+=" | "-=" | "*=" | "/=" | "%=";
  value: string;               // 赋值表达式
}

// ── if / else if / else ──
interface IfBlockOp {
  kind: "if_block";
  branches: {
    condition: string;         // if/elif 条件表达式，else 为 ""
    body: CodeOp[];            // 该分支内的子操作列表
  }[];
}

// ── while 循环 ──
interface WhileLoopOp {
  kind: "while";
  condition: string;
  body: CodeOp[];
}

// ── for..in 循环 ──
interface ForLoopOp {
  kind: "for_in";
  itemName: string;            // 循环变量名
  collection: string;          // 被遍历的表达式
  body: CodeOp[];
}

// ── 其他 ──
interface ReturnExprOp { kind: "return"; value?: string; }
interface CallFnOp { kind: "call_fn"; fnName: string; args: string[]; }
interface AwaitExprOp { kind: "await"; value: string; }
interface BreakStmtOp { kind: "break"; }
interface ContinueStmtOp { kind: "continue"; }
interface CommentOp { kind: "comment"; text: string; }
```

---

## 5. 编辑器 UX：卡片式代码编辑

### 4.1 核心交互模型

**键盘优先，积木随打字自然生长，尽量少用鼠标点选。**

不弹窗填表——打字触发补全，选中后插入结构化卡片骨架，骨架中的空槽位通过 Tab/Enter 激活为内联输入框。

### 4.2 一次典型的编辑流程

```
步骤                        画面

1. 输入 "l"                 ┌─────────────────────────┐
                            │ l▌                       │
                            │ ───────────────          │ 补全菜单弹出
                            │ let  >  声明不可变变量    │
                            │ let mut > 声明可变变量    │
                            │ while >  循环            │
                            └─────────────────────────┘

2. Enter 选 let             ┌─────────────────────────┐
                            │ let █ : █ = █            │ 卡片骨架，三个空槽
                            └─────────────────────────┘ 光标停在第一个槽

3. 输入 "sum"               ┌─────────────────────────┐
                            │ let sum : █ = █          │
                            └─────────────────────────┘

4. Tab → 类型槽             ┌─────────────────────────┐
  输入 "f"                  │ let sum : f▌ = █         │
                            │         ───               │ 补全 f64 | f32
                            │         f64               │
                            │         f32               │
                            └─────────────────────────┘

5. Enter 选 f64             ┌─────────────────────────┐
                            │ let sum : f64 = █        │
                            └─────────────────────────┘

6. Tab → 初始值槽           ┌─────────────────────────┐
  输入 "0.0" → Esc 完成     │ let sum : f64 = 0.0     │
                            └─────────────────────────┘
```

### 4.3 mut 融入已有卡片

```
已有卡片：
┌─────────────────────────┐
│ let sum : f64 = 0.0     │
│ ▌                        │     光标在卡片头，输入 "m"
└─────────────────────────┘
                                   补全: mut > 改为可变变量

选中后 mut 直接融入：
┌──────────────────────────────┐
│ let mut sum : f64 = 0.0     │
└──────────────────────────────┘
```

### 4.4 光标与导航

**核心原则：上下左右键控制一切光标移动，没有模式切换。**

```
光标形态                          含义
──────────────────────────────────────────────────
│ (竖线)                          在文本槽位内，可输入字符
▌ (方块)                          在卡片/组件边界上，可删除整组件
```

**光标移动规则**：

```
← → 在卡片内移动：
  - 文本槽位内：按字符移动
  - 卡片边界：跳到相邻组件/槽位

↑ ↓ 在卡片列表间移动：
  - 在子卡片列表（如 while body）内：上下跳到相邻子卡片
  - 在嵌套卡片边界：↓ 进入子卡片列表，↑ 退出到父卡片
```

**光标在组件右侧边界时的行为**：

```
┌──────────────────────────────┐
│ let mut sum : f64 = 0.0    ▌│  光标在卡片右边界（方块）
└──────────────────────────────┘
       ↓ Enter                              ↓ Delete
┌──────────────────────────────┐   ┌──────────────────────────────┐
│ let mut sum : f64 = 0.0     │   │                              │
│ ▌                             │   │  （整张卡片被删除）            │
│                              │   │                              │
└──────────────────────────────┘   └──────────────────────────────┘
（在卡片下方插入新空行）
```

### 4.5 删除：光标所在层级即删除粒度

```
光标位置                              按 Delete 删除的粒度
─────────────────────────────────────────────────────────────
文本槽位内（│竖线光标）                 单个字符
槽位内按到底（删光了槽内容）            该槽位复位为 █ 占位
卡片内的 token/子组件上（▌方块光标）    该子组件整体删除
卡片右边界（▌方块光标）                 整张卡片（含所有子卡片）
```

**示例**：

```
┌──────────────────────────────┐
│ let mut| sum : f64 = 0.0    │  光标在 mut 文本中（竖线）
└──────────────────────────────┘
       ↓ Delete 删字符 "t"
┌──────────────────────────────┐
│ let mu| sum : f64 = 0.0     │
└──────────────────────────────┘
       ↓ Delete 继续删...直到删光 "mut"
┌──────────────────────────────┐
│ let █ sum : f64 = 0.0       │  mut 槽位复位为占位符
└──────────────────────────────┘
       ↓ Delete 再按一次
┌─────────────────────────────┐
│ let sum : f64 = 0.0        │  整个 mut 子组件消失，融入 let
└─────────────────────────────┘
```

```
┌──────────────────────────────┐
│ while i < n {               │  光标移到 while 卡片右边界（→ 键）
│   set sum = sum + i         │
│ }                           ▌│
└──────────────────────────────┘
       ↓ Delete
┌──────────────────────────────┐
│                              │  while + 所有子卡片一起删除
│                              │
└──────────────────────────────┘
```

### 4.6 槽位激活

光标进入 `█` 槽位时，自动变为内联输入框：

```
光标在槽位外：                     ← 键移入后：
┌─────────────────────────┐      ┌─────────────────────────────┐
│ let sum : f64 = █       │      │ let sum : f64 = ┌──────────┐│
│                          │      │                  │          ││
│                          │      │                  └──────────┘│
└─────────────────────────┘      └─────────────────────────────┘
                                       直接打字输入值

输入完成按 Enter → 值 token 化，光标跳回卡片右边界：
┌──────────────────────────────┐
│ let sum : f64 = 0.0        ▌│
└──────────────────────────────┘
```

不想填这个槽 → 按 ← 或 ↑ 离开，槽位保持 `█` 占位。

### 4.7 键盘操作

| 按键 | 动作 |
|------|------|
| `←` `→` | 光标在卡片内移动（字符级 / 组件级） |
| `↑` `↓` | 光标在卡片列表间移动，进入/退出嵌套卡片 |
| 打字（字母/数字） | 触发补全菜单 / 在活动槽位中输入文本 |
| `Enter` | 光标在文本槽位中 → 提交输入值；光标在卡片右边界 → 在下方插入新空行 |
| `Delete` | 删除光标所在层级的东西（字符 / token / 子组件 / 整卡） |
| `Ctrl+Z` | 撤销（卡片级操作） |

---

## 6. 上下文感知补全

编辑器根据光标所在位置自动弹出对应类型的补全提示。

| 位置 | 自动弹出的提示 |
|------|---------------|
| 空白行（输任意字符） | `let`, `let mut`, `if`, `while`, `for`, `return`, `set`, `await`, `break`, `continue`, 函数名 |
| `let █` | 已有变量名列表（避免重名） |
| `let name █` | 类型提示（`: i32`, `: String`, `: f64`, `: bool`, `: Array<T>`, ...） |
| `let name : Type = █` | 变量名 + 字面量模板 |
| `set █` | 已有变量名列表 |
| `if █` / `while █` | 变量名 + 比较运算符模板 |
| `return █` | 变量名 + 表达式模板 |
| `for █ in █` | 循环变量名 + 可迭代对象列表 |
| `fn.█` | 方法名列表（`.len()`, `.push()`, `.pop()`, `.get()`, `.to_string()`, ...） |
| `{` | 创建新的嵌套卡片区域 |
| `let █ : █ = █` 之后 | 下一个可能的操作（`set`, `if`, `while`, `for`, `return`, 下一行补全） |

---

## 7. 数据流

```
┌──────────────────────────────────────────────────┐
│                  CodeCardEditor                    │
│                                                    │
│  按键 → Token匹配 → 补全菜单 → 创建/修改卡片        │
│       ↕                                           │
│  CodeOp[] (内存中的操作列表)                        │
└────────────────────┬─────────────────────────────┘
                     │ onSave(operations: CodeOp[])
                     ▼
┌──────────────────────────────────────────────────┐
│            CodeblockElementData                    │
│  source: string      ← 由 operations 重新生成      │
│  operations: CodeOp[] ← 权威数据源（新增）           │
└────────────────────┬─────────────────────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────┐
│          graph-compiler.ts (扩展 emitNode)          │
│                                                    │
│  case "code-block":                                │
│    if (operations) {                               │
│      for (const op of operations) emitOp(op);      │
│    } else {                                        │
│      原有 source 原样 emit (向后兼容)                 │
│    }                                               │
│                                                    │
│  emitOp(DeclareVar) → "let mut sum: f64 = 0.0;"   │
│  emitOp(WhileLoop)  → "while i < n {\n  ...\n}"   │
│  emitOp(IfBlock)    → "if cond {\n...\n} else..."  │
│  emitOp(ReturnExpr) → "return expr;"               │
│  ...                                               │
└────────────────────┬─────────────────────────────┘
                     │
                     ▼
                 .omni 源码 → omnic build
```

---

## 8. 编译器变更（graph-compiler.ts）

新增 `emitOp()` 递归函数，与现有的 `emitNode()` 同层：

```typescript
function emitOp(op: CodeOp, indent: number): string {
  const pad = "  ".repeat(indent);
  switch (op.kind) {
    case "declare_var": {
      const mut = op.mutable ? " mut" : "";
      const type = op.varType ? `: ${op.varType}` : "";
      const init = op.initValue ? ` = ${op.initValue}` : "";
      return `${pad}let${mut} ${op.name}${type}${init};\n`;
    }
    case "set_var": {
      return `${pad}${op.name} ${op.operator} ${op.value};\n`;
    }
    case "if_block": {
      let out = "";
      for (let i = 0; i < op.branches.length; i++) {
        const br = op.branches[i];
        const kw = i === 0 ? "if" : br.condition ? "else if" : "else";
        const cond = br.condition ? ` ${br.condition}` : "";
        out += `${pad}${kw}${cond} {\n`;
        for (const child of br.body) out += emitOp(child, indent + 1);
        out += `${pad}}\n`;
      }
      return out;
    }
    case "while": {
      let out = `${pad}while ${op.condition} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "for_in": {
      let out = `${pad}for ${op.itemName} in ${op.collection} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "return":   return `${pad}return${op.value ? " " + op.value : ""};\n`;
    case "call_fn":  return `${pad}${op.fnName}(${op.args.join(", ")});\n`;
    case "await":    return `${pad}await ${op.value};\n`;
    case "break":    return `${pad}break;\n`;
    case "continue": return `${pad}continue;\n`;
    case "comment":  return `${pad}// ${op.text}\n`;
  }
}
```

---

## 9. 解析器变更（graph-parser.ts）

扩展现有 segment 解析，当 segment type 为 `"code-block"` 时，进一步解析其内部为 `CodeOp[]`：

**解析策略**：优先使用 OmniLang WASM parser 的 AST 能力（若 TS 侧 API 可用），将 code-block 源码解析为 AST 后映射到 `CodeOp`。如果 WASM AST 暂时不可用，Phase 1 先用 JS-side 正则/LR 解析器做降级方案，后续切换到 WASM AST：

> **Phase 1 需验证**：WASM parser 是否暴露 AST 节点给 TypeScript 调用方。

| AST 节点 | 映射为 CodeOp |
|----------|--------------|
| `LetStmt` | `DeclareVarOp` |
| `AssignExpr` / `CompoundAssignExpr` | `SetVarOp` |
| `IfExpr` / `IfStmt` | `IfBlockOp` |
| `WhileStmt` | `WhileLoopOp` |
| `ForStmt` | `ForLoopOp` |
| `ReturnStmt` | `ReturnExprOp` |
| `CallExpr` | `CallFnOp` |
| `AwaitExpr` | `AwaitExprOp` |
| `BreakStmt` | `BreakStmtOp` |
| `ContinueStmt` | `ContinueStmtOp` |

**向后兼容**：如果 code-block segment 无法解析为结构化操作（太复杂或含不支持语法），回退到现有的 `source` 字段原样存储，编辑器只读显示 source 文本。

---

## 10. 编辑器组件架构

```
CodeCardEditor (新组件，替换 CodeEditorModal)
├── CardList                  ← 操作卡片列表容器
│   ├── DeclareVarCard        ← 每种 CodeOp 对应一个卡片组件
│   ├── SetVarCard
│   ├── IfBlockCard
│   │   ├── BranchCard        ← 每个分支的子卡片列表
│   │   │   └── CardList (递归)
│   │   └── ...
│   ├── WhileLoopCard
│   │   └── CardList (递归)
│   ├── ForLoopCard
│   │   └── CardList (递归)
│   ├── ReturnExprCard
│   ├── CallFnCard
│   ├── AwaitExprCard
│   ├── BreakStmtCard
│   ├── ContinueStmtCard
│   └── CommentCard
├── InlineSlot                ← 槽位内联输入框（通用）
├── AutocompleteMenu          ← 上下文感知补全菜单
├── CardToolbar               ← 卡片级操作（删除、折叠、变 mut）
└── ErrorIndicator            ← WASM 诊断错误标记
```

---

## 11. 错误处理

- **语法错误**：Expression 槽位实时 WASM 诊断，错误时红色下划线
- **必填槽位未填**：`set █ = expr` 目标变量为空 → 浅红占位提示，阻止保存
- **变量作用域检查**：引用不存在的变量名 → 黄色警告下划线
- **解析回退**：无法结构化的 code-block 源码 → 保留原始文本，编辑器只读展示

---

## 12. 向后兼容

1. 现有 `.omni` 源码中已存在的 `// @pid:xxx` + code-block 文本 → 解析时尝试 AST 解析，成功则生成 operations，失败则回退到 source
2. 保存时始终从 operations 重新生成 source（若 operations 存在），确保 source 与 operations 同步
3. 旧 project.json 中 `codeFlowGraphs` 无 operations 字段 → 加载时从 source 反向解析生成
4. 新的卡片编辑产生的 `.omni` 源码与手写源码完全一致，不影响 omnic 编译器

---

## 13. 测试策略

| 层 | 测试内容 | 工具 |
|----|---------|------|
| 单元测试 | `emitOp()` 每种 CodeOp 输出正确 Omni 源码 | Vitest |
| 单元测试 | AST → CodeOp 解析每种语法构造 | Vitest |
| 单元测试 | CodeOp 往返：parse(emit(op)) === op | Vitest |
| 集成测试 | CodeCardEditor 正确渲染 | Playwright |
| 集成测试 | 键盘补全流程（输入 "l" → 选 let → 填槽 → 保存） | Playwright |
| 集成测试 | 删除流程（组件粒度删除、嵌套删除） | Playwright |
| 集成测试 | 编辑器输出 → 编译器 → 编译成功 | Vitest |
| 回归测试 | 现有 learn-omniview 项目编译后行为不变 | Playwright + omnic build |

---

## 14. 实现阶段

### Phase 1：数据模型 + 编译器

1. 定义 `CodeOp` 类型（`poiesis/src/core/types/code-op.ts`）
2. 扩展 `CodeblockElementData` 加 `operations` 字段
3. 实现 `emitOp()` 写入 `graph-compiler.ts`
4. 实现 AST → CodeOp 解析器
5. 单元测试覆盖所有操作类型往返

### Phase 2：编辑器 UI

1. 创建 `CodeCardEditor` 组件（替换 `CodeEditorModal`）
2. 实现 `CardList` + 每种卡片组件
3. 实现 `InlineSlot` 内联输入框
4. 实现 `AutocompleteMenu` 补全菜单
5. 实现键盘导航（Tab/Enter/Esc/↑↓/Backspace）

### Phase 3：集成 + 诊断

1. 连接 FlowCanvas 双击事件 → CodeCardEditor
2. 集成 WASM diagnostics 到槽位实时检查
3. 变量作用域检查
4. 向后兼容：旧 source → operations 自动转换

### Phase 4：测试 + 打磨

1. Playwright 自动化测试键盘流程
2. 回归测试 learn-omniview 全量编译
3. 性能优化（大函数 >50 个操作卡片）
4. 撤销/重做
