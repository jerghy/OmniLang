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
  | CommentOp
  | RawCodeOp;
```

### 操作卡片类型定义

```typescript
// ── 变量声明 ──
interface DeclareVarOp {
  kind: "declare_var";
  pid: string;                 // 唯一标识，对应 .omni 中的 // @pid:xxx
  mutable: boolean;            // false=let, true=let mut
  name: string;
  varType?: string;            // 可选，省略则类型推断
  initValue?: string;          // 可选初始值表达式
  scope: "fn" | "container";   // 作用域选择
}

// ── 变量赋值 ──
interface SetVarOp {
  kind: "set_var";
  pid: string;
  name: string;                // 目标变量名
  operator: "=" | "+=" | "-=" | "*=" | "/=" | "%=";
  value: string;               // 赋值表达式
}

// ── if / else if / else ──
interface IfBlockOp {
  kind: "if_block";
  pid: string;                 // PID 只属于 if 头，elif/else 不发独立 PID
  branches: {
    condition: string;         // if/elif 条件表达式，else 为 ""
    body: CodeOp[];            // 该分支内的子操作列表
  }[];
}

// ── while 循环 ──
interface WhileLoopOp {
  kind: "while";
  pid: string;
  condition: string;
  body: CodeOp[];
}

// ── for..in 循环 ──
interface ForLoopOp {
  kind: "for_in";
  pid: string;
  itemName: string;            // 循环变量名
  collection: string;          // 被遍历的表达式
  body: CodeOp[];
}

// ── 其他 ──
interface ReturnExprOp { kind: "return"; pid: string; value?: string; }
interface CallFnOp { kind: "call_fn"; pid: string; fnName: string; args: string[]; }
interface AwaitExprOp { kind: "await"; pid: string; value: string; }
interface BreakStmtOp { kind: "break"; pid: string; }
interface ContinueStmtOp { kind: "continue"; pid: string; }
interface CommentOp { kind: "comment"; pid: string; text: string; }

// ── 解析回退 ──
interface RawCodeOp {
  kind: "raw";
  pid: string;
  source: string;              // 无法结构化的原始源码，编辑器只读展示
}
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

## 8. 往返机制总览

**核心原则**：code-block 自身已有顶层 PID（来自 `graph-parser.ts` 的 segment 分割），内部操作不需要在 `.omni` 源码中额外标注 PID。

```
保存方向：CodeOp[] → emitOps() → 干净 Omni 源码（无 // @pid 噪声）
解析方向：干净 Omni 源码 → parseOps() → CodeOp[]
          (Phase 1: JS 正则关键词检测 + 括号深度追踪)
          (Phase 2: 待 WASM 暴露 AST API 后切换)
```

CodeOp.pid 是**编辑器内部标识**——只在内存中使用（追踪卡片、撤销/重做），加载时自动生成，不持久化到 `.omni`。

### 8.1 数据结构

`pid` 字段标注为编辑器内部：

```typescript
interface CodeOpBase {
  pid: string;  // 编辑器内部 ID，由 uid() 生成，不写入 .omni
}
// ... 各类型同前，pid 保留但仅用于编辑器内部
```

---

## 9. 编译器变更（graph-compiler.ts）

新增 `emitOp()` / `emitOps()` 函数。**不生成任何 PID 注释**，只输出干净的 Omni 语法。

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
    case "raw":      return op.source;  // 原始文本原样输出
  }
}

function emitOps(ops: CodeOp[]): string {
  return ops.map(op => emitOp(op, 0)).join("");
}
```

**emit 示例**——干净的 Omni 源码，无 PID 注释：

```omni
let sum = 0.0;
let mut i = 0;
while i < values.len() {
    sum = sum + values.get(i);
    i = i + 1;
}
if count != 0 {
    return sum / count;
}
return 0.0;
```

**顶层 PID 不变**：code-block 节点的 `// @pid:xxx` 由 `emitNode()` 照常生成。内部操作由 `emitOps()` 追加在 PID 行之后，干净无标注。

---

## 10. 解析器变更（graph-parser.ts）

新增 `parseOps()` 函数。**不依赖 PID 标注**——纯靠 Omni 语法关键词检测 + 括号深度追踪解析为 `CodeOp[]`。

### 10.0 WASM 能力评估与扩展方案

omnilang Rust crate 已有完整的递归下降解析器：
- `parser::stmt::Parser::parse_stmt()`：解析单条语句 → `Stmt` AST 节点
- `parser::stmt::Parser::parse_block()`：解析 `{ stmt* }` → `Block { stmts, trailing_expr }`
- `Stmt` 枚举已覆盖所有 code-block 操作：`VarDecl`、`Assign`、`If`、`While`、`ForIn`、`Return`、`Break`、`Continue`、`ExprStmt`、`Throw`、`TryCatch`、`Print`

**缺的是**：AST 类型目前没有 `Serialize`，无法通过 serde_wasm_bindgen 返回；没有独立的"解析语句序列" WASM 函数。

**扩展计划（3 步）**：

| 步骤 | 位置 | 内容 |
|------|------|------|
| 1 | `omnilang/src/parser/ast/mod.rs` | AST 类型添加 `#[derive(Serialize)]`（`Stmt`, `Block`, `Expr`, `TypeAnnotation`, `Literal`, 等） |
| 2 | `omnilang/src/lib.rs` | 新增 `pub fn parse_stmts(source: &str) -> ParseStmtsResult`，调用 `lexer::tokenize` → `parser::parse_stmts_until_eof` → 返回 `Vec<Stmt>` + diagnostics |
| 3 | `omnilang-wasm/src/lib.rs` | 新增 `#[wasm_bindgen] pub fn parse_code_block(source: &str) -> JsValue`，返回 `ParseStmtsResult` |

完成后，`parseOps()` 的实现简化为：

```
parseCodeBlock(omni源码)
    → WASM 返回 Vec<Stmt> JSON
    → TS 侧遍历 Stmt[] 映射到 CodeOp[]：
        Stmt::VarDecl   → DeclareVarOp
        Stmt::Assign    → SetVarOp
        Stmt::If        → IfBlockOp (递归处理 then_body/else_ifs/else_body)
        Stmt::While     → WhileLoopOp
        Stmt::ForIn     → ForLoopOp
        Stmt::Return    → ReturnExprOp
        Stmt::Break     → BreakStmtOp
        Stmt::Continue  → ContinueStmtOp
        Stmt::ExprStmt  → CallFnOp / AwaitExprOp (按 Expr 类型判断)
        Stmt::Print     → CallFnOp (print/println)
        Stmt::Error     → RawCodeOp
```

**比 JS 正则方案的优势**：
- 精确：编译器级解析，不靠启发式规则
- 一致：与 omnic build 用同一个 parser，语义统一
- 自带诊断：`parse_code_block` 可返回语法错误（line/col/message），直接显示在卡片编辑器槽位上
- 成本低：parser 已存在，只需加 `Serialize` + 暴露函数

### 10.1 解析流程（WASM parser 优先，JS 正则降级）

```
输入: code-block segment 的 Omni 源码文本（无 PID 标注）
输出: CodeOp[]（每个 op 的 pid 由 uid() 自动生成）

Step 1 — 逐行扫描
  遍历源码的所有行，跳过空行

Step 2 — 类型检测（关键词正则）
  匹配每行的第一个非空白 token：
    "let" / "let mut"              → declare_var
    "if"                           → if_block
    "else if" / "else"             → if_block 分支延续（合并到上一个 if_block）
    "while"                        → while
    "for"                          → for_in
    "return"                       → return
    "break"                        → break
    "continue"                     → continue
    "await"                        → await
    标识符 + "("                    → call_fn
    标识符 + 赋值运算符（=, += 等）  → set_var
    "//"                           → comment
    其他                            → raw（回退文本）

Step 3 — 括号深度追踪
  跟踪每行的 { } 深度
  当遇到 if_block / while / for_in 时，记录其起始深度
  后续行深度 > 起始深度 → 属于该块的 body
  elif/else 出现在同深度 → 同一 if_block 的新分支

Step 4 — 嵌套组装
  简单操作（declare_var, set_var, return, call_fn, await, break, continue）→ 单个 CodeOp
  块操作（if_block, while, for_in）→ 递归调用 parseOps() 解析 body
```

### 10.2 类型检测

```typescript
function detectOpKind(firstLine: string): CodeOp["kind"] | "raw" {
  const t = firstLine.trim();

  if (/^let\s+mut\s/.test(t) || /^let\s+\w/.test(t)) return "declare_var";
  if (/^if\s+/.test(t)) return "if_block";
  if (/^else\s+if\s+/.test(t) || /^else\s*\{/.test(t)) return "if_block"; // elif/else
  if (/^while\s+/.test(t)) return "while";
  if (/^for\s+\w+\s+in\s+/.test(t)) return "for_in";
  if (/^return/.test(t)) return "return";
  if (/^break\s*;?\s*$/.test(t)) return "break";
  if (/^continue\s*;?\s*$/.test(t)) return "continue";
  if (/^await\s+/.test(t)) return "await";
  if (/^\w+\s*\(/.test(t)) return "call_fn";
  if (/^\w+\s*(=|\+=|-=|\*=|\/=|%=)/.test(t)) return "set_var";
  if (/^\/\//.test(t)) return "comment";

  return "raw";
}
```

### 10.3 核心解析逻辑（伪代码）

```typescript
function parseOps(source: string): CodeOp[] {
  const lines = source.split("\n");
  const ops: CodeOp[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // 跳过空行和纯注释行（注释单独处理为 CommentOp）
    if (line.trim() === "") { i++; continue; }

    const kind = detectOpKind(line);
    const depth = braceDepth(line);  // 本行的 { } 净变化

    switch (kind) {
      case "declare_var":
      case "set_var":
      case "return":
      case "call_fn":
      case "await":
      case "break":
      case "continue":
      case "comment":
        ops.push(buildOp(kind, line, uid()));
        i++;
        break;

      case "if_block": {
        const { branches, nextI } = parseIfBlock(lines, i);
        ops.push({
          kind: "if_block", pid: uid(),
          branches: branches.map(br => ({
            condition: br.condition,
            body: parseOps(br.bodyLines.join("\n")),
          })),
        });
        i = nextI;
        break;
      }

      case "while": {
        const { bodyLines, nextI } = extractBlockBody(lines, i);
        ops.push({
          kind: "while", pid: uid(),
          condition: extractWhileCondition(line),
          body: parseOps(bodyLines.join("\n")),
        });
        i = nextI;
        break;
      }

      case "for_in": {
        const { bodyLines, nextI } = extractBlockBody(lines, i);
        ops.push({
          kind: "for_in", pid: uid(),
          itemName: extractForItem(line),
          collection: extractForCollection(line),
          body: parseOps(bodyLines.join("\n")),
        });
        i = nextI;
        break;
      }

      case "raw":
        ops.push({ kind: "raw", pid: uid(), source: line });
        i++;
        break;
    }
  }

  return ops;
}
```

### 10.4 向后兼容与回退

| 场景 | 处理方式 |
|------|---------|
| 卡片编辑器产生的新源码 | `parseOps()` 语法检测 → 完美还原 CodeOp[] |
| 旧项目 code-block 源码（任意 Omni 文本） | 同样的语法检测，成功则还原，失败处标记为 `raw` |
| 手写 .omni 中无法结构化的行 | 标记为 `RawCodeOp`，卡片编辑器中只读文本框展示 |
| 旧 source 字段（无 operations） | 加载时尝试 `parseOps(source)`，成功则自动填充 `operations`

---

## 11. 编辑器组件架构

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

## 12. 错误处理

- **语法错误**：Expression 槽位实时 WASM 诊断，错误时红色下划线
- **必填槽位未填**：`set █ = expr` 目标变量为空 → 浅红占位提示，阻止保存
- **变量作用域检查**：引用不存在的变量名 → 黄色警告下划线
- **解析回退**：无法结构化的 code-block 源码 → 保留原始文本，编辑器只读展示

---

## 13. 向后兼容

1. 现有 `.omni` 源码中已存在的 `// @pid:xxx` + code-block 文本 → 解析时尝试 AST 解析，成功则生成 operations，失败则回退到 source
2. 保存时始终从 operations 重新生成 source（若 operations 存在），确保 source 与 operations 同步
3. 旧 project.json 中 `codeFlowGraphs` 无 operations 字段 → 加载时从 source 反向解析生成
4. 新的卡片编辑产生的 `.omni` 源码与手写源码完全一致，不影响 omnic 编译器

---

## 14. 测试策略

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

## 15. 实现阶段

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
