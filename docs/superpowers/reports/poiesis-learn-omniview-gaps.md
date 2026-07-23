# Poiesis → learn-omniview 代码生成能力缺口分析（修正版）

> 分析日期：2026-07-23
> 对比基线：learn-omniview（74+ 源文件）vs Poiesis 当前 parser/codegen/emitter

## 一、总体评估（修正）

Poiesis 的定位是**架构优先的可视化编辑器**，核心能力覆盖范围比初步印象广得多：

| 能力层 | 覆盖度 | 说明 |
|--------|--------|------|
| 组件模板（view） | ~80% | @if 完整，@for/@slot emit 支持但 parse 未完成 |
| 组件成员（signal/computed/prop/event/fn） | ~70% | 基本形式完整，缺 computed 函数形式、const、ref |
| Store | ~90% | signal/getter/action + async + returnType + inline store |
| Design System | ~95% | token/class/variant 完整往返 |
| 多文件项目 | ~60% | generateOmniFiles 已实现，但仅覆盖 main+stores |
| fn/action body（code flow graph） | ~75% | fn/code-block/condition/return 节点，code-block=任意文本 |
| 类型系统 | ~0% | struct/tagged union/type alias 完全不支持 |
| 动画系统 | ~0% | @motion/@timeline 完全不支持 |
| 生命周期 | ~0% | @mount/@unmount/@effect 完全不支持 |

**关键纠正**：
- ❌ 之前说"不支持多文件"——**错误**。`generateOmniFiles()` 已实现 main+stores 多文件输出。
- ❌ 之前说"fn/action body 不能生成"——**错误**。code flow graph 通过 code-block 节点可表示任意逻辑，graph-compiler 支持 fn/code-block/condition/return 四种节点。
- ❌ 之前说"store action 不支持 return type/async"——**错误**。parser 和 emitter 都完整支持。

---

## 二、按"数据流方向"分类的缺口

Poiesis 的双向数据流是：`.omni 源码 → parser → Store → codegen emitter → .omni 源码`

缺口分为三类：
- **Parse gap**：无法从 .omni 源码读到 Store
- **Emit gap**：Store 中有数据但无法生成到 .omni
- **Full gap**：两端都不支持

### 2.1 Full gaps（parse + emit 都不支持）

| # | 特性 | learn-omniview 文件 | 影响 |
|---|------|-------------------|------|
| 1 | **类型系统**：struct / nominal struct / tagged union / type alias | `lib/types.omni`, `lib/validation.omni`, `FeatureCard.omni` | 16 个 lib/ 文件 + type 声明无法生成 |
| 2 | **@motion / @timeline** 动画系统 | 全部 `pages/Motion*.omni` + `Timeline*.omni` 等 12+ 文件 | 动画完全不可用 |
| 3 | **@mount / @unmount / @effect** 生命周期 | `AddTodo.omni`, `TodoPage.omni`, `StatsDashboard.omni` | 组件初始化/清理/副作用逻辑丢失 |
| 4 | **computed 函数形式**：`computed name() -> T { ... }` | docs 中有定义，learn-omniview 多用简单形式 | 复杂派生逻辑需要多行时无法表达 |
| 5 | **ref 系统**：`ref name: Element;` + `ref: name` 属性 | `StatsDashboard.omni` | DOM 元素引用不可用 |
| 6 | **纯逻辑文件**：不带 `import omniview;` 的 .omni | `lib/types.omni`, `lib/validation.omni`, `lib/api.omni` 等 | lib/ 下全部文件不可生成 |
| 7 | **pub / export** 可见性系统 | 全部 lib/ 文件 + `lib/reexports.omni` | 跨文件导出/重导出不可用 |
| 8 | **async fn**（组件级独立 async 函数） | `lib/api.omni`, `async_demo.omni` | 异步工具函数不可用 |

### 2.2 Parse gaps（emitter 支持但 parser 不支持）

这些特性在 emitter 中已实现，可通过可视化编辑器创建并正确生成 .omni，但**无法从现有 .omni 解析**：

| # | 特性 | Emit 状态 | Parse 状态 | 影响 |
|---|------|----------|-----------|------|
| 9 | **@for / @empty** | ✅ `ForViewNode` + emitViewNode 完整 | ❌ 跳过 body（`@if only` 注释） | 现有 @for 源码解析丢失 |
| 10 | **@slot**（含默认回退） | ✅ `SlotViewNode` + emitViewNode 完整 | ❌ 跳过 body | 现有 @slot 源码解析丢失 |
| 11 | **const 声明** | ✅ component-builder 提取 + emitter emit | ❌ parser 不处理 `const` 关键字 | round-trip 断裂 |

### 2.3 Emit gaps（parser 支持但 emitter 不完整）

| # | 特性 | Parse 状态 | Emit 状态 |
|---|------|-----------|----------|
| 12 | **event? 可选事件** | ⚠️ 解析了 event name 但丢失 `?` 标记 | ❌ 不 emit `?` |
| 13 | **action 多参数** | ✅ collectActionParams 完整 | ✅ emitStoreDefinition 完整（此项无 gap，之前误判） |

### 2.4 Import 系统缺口

| # | 特性 | 状态 | 说明 |
|---|------|------|------|
| 14 | `import { X } from "..."` 命名导入 | ✅ 完整 | parser 逐行收集 + import-collector 生成 |
| 15 | `import type { X } from "..."` 类型导入 | ❌ Full gap | parser 存为字符串但无特殊处理 |
| 16 | `import "./file" as Name` 命名空间导入 | ❌ Full gap | parser 不识别 `as` |
| 17 | `import { fn } from "std/array"` 标准库 | ⚠️ 字符串透传 | import 字符串可存可取但不识别语义 |
| 18 | `export { X } from "./..."` 重导出 | ❌ Full gap | 完全不支持 |

### 2.5 View 属性缺口

| # | 特性 | Parse | Emit |
|---|------|-------|------|
| 19 | `ref: name` DOM 引用属性 | ❌ | ❌ |
| 20 | `testId="..."` 测试 ID | ❌ | ❌ |
| 21 | `class: "string literal"` 字符串 class | ❌ | ❌ |
| 22 | `class: { dynamicClass }` 动态 class 对象 | ✅ | ✅ |
| 23 | `style: { key: val }` 内联样式 | ✅ | ✅ |
| 24 | `bind:value` / `bind:checked` 双向绑定 | ✅ | ✅ |
| 25 | `variant: Design.Variant(...)` 变体引用 | ✅ | ✅ |
| 26 | `@onEvent={handler}` 事件绑定 | ✅ | ✅ |
| 27 | `@onEvent={expr}` 内联表达式 | ✅ | ✅ |
| 28 | `@else @if` 紧凑语法 | ⚠️ 未验证 | ✅（emitter 使用标准分行格式） |

### 2.6 fn/action body 内部缺口（code flow graph）

code flow graph 通过 **code-block 节点**（source 字段存原始文本）可以**表示任意 OmniLang 代码**。编辑器可以用可视化节点（condition/return）编排控制流，也可以直接用 code-block 存手写代码。

| 代码构造 | 可视化节点 | code-block 文本 | 说明 |
|----------|-----------|----------------|------|
| `let x = expr;` | ❌ 无专用节点 | ✅ 可存为 code-block source | 需要用户手写 |
| `return expr;` | ✅ return 节点 | ✅ | 有专用节点 |
| `if/else` | ✅ condition 节点 | ✅ | 有专用节点，支持 br0/br1/br2 分支连线 |
| `for..in` | ⚠️ 图解析识别为 code-block | ✅ 可存文本 | 无专用 for 循环节点 |
| `while` | ⚠️ 同上 | ✅ | 无专用 while 节点 |
| `match` | ❌ | ✅ 可存文本 | 完全依赖手写 |
| 闭包 `\|x\| expr` | ❌ | ✅ 可存文本 | 完全依赖手写 |
| 表达式（`??`, `?.`, `?:` 等） | ❌ | ✅ 可存文本 | 完全依赖手写 |
| `println` / `print` | ❌ | ✅ 可存文本 | 完全依赖手写 |
| `break` / `continue` | ❌ | ✅ 可存文本 | 完全依赖手写 |
| `+=` / `-=` 等复合赋值 | ❌ | ✅ 可存文本 | 完全依赖手写 |
| 事件对象 `fn f(e) { e.key }` | ❌ | ✅ 可存文本 | 完全依赖手写 |
| `async` / `await` | ❌ | ✅ 可存文本 | 编译器需识别 async 关键字 |

**结论**：code flow graph 的 code-block 机制提供了**表达能力上的逃生舱**——任何不能通过可视化节点表达的代码都可以作为文本存在 code-block 中。但这意味着这些代码对编辑器是**不透明的**（无法结构化编辑）。

---

## 三、按 learn-omniview 源文件的覆盖矩阵（修正）

### 可完整往返（parse → store → emit）
| 文件 | 说明 |
|------|------|
| `simple.omni` | 最简组件 |
| `components/Counter.omni` | signal + computed + variant + 内联闭包 + style |
| `design.omni` | Token + Class + Variant 完整往返 |
| `stores/TodoStore.omni` | signal + getter + action + store imports |
| `stores/ThemeStore.omni` | getter if-expr + action + std/storage（import 字符串透传） |
| `stores/NotificationStore.omni` | action returnType + async action + multi-param |

### 可 emit 但解析缺失 @for/@slot
| 文件 | 缺失 |
|------|------|
| `components/TodoList.omni` | @for/@empty（emit OK, parse 丢失） |
| `components/Modal.omni` | @slot（emit OK, parse 丢失） |
| `components/SlotDefaultDemo.omni` | @slot default content |
| `components/ForDirectiveKeyDemo.omni` | @for key=expr |
| `components/EmitEventDemo.omni` | @for/@empty |

### 不可生成（关键特性缺失）
| 文件 | 主要缺口 |
|------|---------|
| `main.omni` | 多文件 import 透传 OK，但 `import "./lib/stats" as StatsLib` namespace import 不识别 |
| `lib/types.omni` | struct/type/pure logic file |
| `lib/validation.omni` | pub fn + for..in + match + tagged union |
| `lib/api.omni` | async fn + await + tagged union |
| `lib/*.omni` | pub fn/struct/type |
| `pages/DataFetching.omni` | async action + std/http + @else if（@if 解析 OK）+ bind:value |
| `components/AddTodo.omni` | @mount + @effect（生命周期缺失）+ computed 函数形式 |
| `components/StatsDashboard.omni` | ref + const + @unmount + std/math |
| `components/FeatureCard.omni` | struct 字面量 |
| `components/InlineStoreDemo.omni` | inline store 解析 OK（parser 支持），但 action body 内 Array 方法依赖 code-block 文本 |
| `components/OptionalChainDemo.omni` | `?.` + `??` + `@else @if` |
| `components/TernaryExprDemo.omni` | `?:` 三元表达式 |
| `components/BlockExprDemo.omni` | 块表达式 |
| 所有 `pages/Motion*.omni` | @motion / @timeline 全不支持 |
| 所有 `*_demo.omni`（stdlib） | std 模块（字符串可透传，语义不识别） |

---

## 四、Poiesis 补齐优先级（修正版）

### 阶段 1：补齐 parse 缺口（让现有 .omni 能正确往返）
1. **@for / @slot parser** — emitter 已就绪，只需 parser 补齐（工作量小）
2. **@mount / @unmount / @effect parser + emitter** — 生命周期钩子
3. **const 声明 parser** — emitter 已就绪
4. **event? 可选事件** — parse 保留 `?` + emit 输出 `?`
5. **ref 系统** — ref 声明 + ref: 属性

### 阶段 2：补齐核心语言特性
6. **类型系统** — struct / tagged union / type alias（parser + emitter）
7. **pub / export** — 可见性系统
8. **纯逻辑文件支持** — 不含 `import omniview;` 的文件
9. **import 增强** — namespace import、import type
10. **computed 函数形式** — 多行 computed body

### 阶段 3：补齐高级特性
11. **@motion / @timeline** — 动画系统（工作量大）
12. **async fn**（组件级独立 async 函数）
13. **View 属性补齐** — ref:, testId, class: "string"
14. **@else @if 紧凑语法** parser 支持

### 阶段 4：生态与完善
15. **std/* 语义识别** — import 路径校验 + 自动补全
16. **多文件项目增强** — generateOmniFiles 扩展支持 lib/、shared/、pages/ 目录
17. **code flow graph 节点扩展** — 专用 for/while/let 节点（减少对不透明 code-block 的依赖）
18. **表达式系统可视化** — ?:/??/?. 的结构化编辑

---

## 五、架构洞察

### 5.1 Poiesis 的"逃生舱"设计

code flow graph 的 **code-block 节点**是最关键的设计——它的 `source` 字段可以存储任意 OmniLang 源代码文本。这意味着：

- **任何不能结构化编辑的代码**都可以作为**不透明文本块**存在
- 往返时 code-block.source 原样输出（通过 `// @pid:xxx` 注释定位）
- 编辑器不需要理解所有语言特性——只需要知道"这里有一段代码"

这大大降低了对 parser/emitter 的完备性要求。但代价是这些代码对编辑器是**不可编辑的**（或只能作为纯文本编辑）。

### 5.2 真正的阻塞点

对于 learn-omniview 这样的项目，**真正的阻塞点**不是表达式或控制流（可以通过 code-block 逃生），而是**顶层结构**：

1. **类型系统** — struct/type 是跨文件共享的"合约"，没有它们 lib/ 无法工作
2. **pub/export** — 没有 pub，所有 fn 都是私有的，lib 模块无意义
3. **@motion/@timeline** — 动画系统是完全独立的一等公民，不能降级为 code-block
4. **生命周期** — @mount/@unmount/@effect 是组件初始化/清理的核心机制
5. **纯逻辑文件** — 不带 `import omniview;` 的文件是根本不同的文件类型

### 5.3 Poiesis 定位建议

Poiesis 最自然的定位是**组件模板 + 设计系统编辑器**：
- ✅ View 模板（@if/@for/@slot + HTML + bind + event）—— 可视化编辑
- ✅ Design System（token/class/variant）—— 可视化编辑
- ✅ Store 定义（signal/getter/action 签名）—— 结构化编辑
- ⚠️ fn/action body —— code flow graph 可视化 + code-block 文本逃生
- ❌ 类型系统 / pub / 纯逻辑文件 —— 可能需要纯文本编辑器模式
