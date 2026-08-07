# CodeFlow Canvas 丰富升级设计（2026-08-07）

状态：设计已批准（五节分节评审通过），待转入 writing-plans。

## 1. 背景与目标

CodeFlow Canvas 的定位：**逻辑代码编辑**（语句、控制流、函数、声明），界面生成（view）与样式归 Design Canvas。

目标：能生成 `D:\space\OmniLang\test\projects\learn-omniview\src` 中**除 view 内容与样式外**的全部语法（逻辑代码域）。

宿主架构（三个复用 FlowCanvas 的宿主，均已实证）：
- **组件 tab**：DesignPage Z 轴双层（OmniViewRenderer 顶层 + CodeFlowCanvas 底层，`codeFlowVisible` 时显示）；左侧 ComponentMembersPanel
- **store tab**：StorePage = 左侧 StoreMembersPanel + 中央 FlowCanvas（每个 action 一张图）
- **code tab**（lib 文件）：DesignPage.tsx:349 裸 `<CodeFlowCanvas />`——目前无左面板、无物化、无保存分支

### 设计原则（用户拍板）

1. **节点编辑器承载逻辑架构**——语句、块、分支、调用链；「不要让任何东西阻挡它的优点（逻辑结构清晰）」
2. **细节交给右侧 FlowInspector 属性面板与公式框**——修饰符、依赖、变量、类型等在节点上只显示摘要
3. 复用现有承载机制（SyntaxNode / NodeFormulaField / FnInspector / ConditionNode 分支行模式 / StoreMembersPanel 面板模式），**不造新 UI 组件**
4. 解析-发射 round-trip 以 `// @pid:xxx` 注释为稳定性基础（现有机制）

## 2. 现状缺口（全部实证）

| # | 缺口 | 位置 |
|---|------|------|
| G1 | 组件 `action` 静默丢失——parseComponentKeyword 无 `action` case，token 逐个被 skip | parser.ts:387-436 |
| G2 | lib 文件顶层 `pub async fn` 被丢弃——parseRootIdentifier 无 fn/pub/async case | parser.ts:249-318（317 行 skip） |
| G3 | ROOT 层 `struct`/`type` 声明被丢弃（lib/types.omni、FeatureCard.omni:4、TodoStore.omni:4 全丢） | 同上 |
| G4 | code tab 无物化——全 src 无 `tabType === "code"` 处理，画布显示残留/空白 | DesignPage.tsx:171-176（仅 component 触发 materializeFnGraphs） |
| G5 | lib 文件保存走组件格式——useSaveProject 无 code tab 分支，落到 generateOmniFiles | useSaveProject.ts:185 |
| G6 | 语法支持缺失：@mount/@effect/@unmount、try/catch/finally、throw、await 均无节点/解析/发射 | graph-parser / graph-compiler / syntax-nodes / flowConstants |

## 3. 设计 1：生命周期节点（tag: `lifecycle`）

**形态**：块节点（对标 FnNode），底部 `flow-out` source handle → body 链。节点显示摘要（`@effect(searchText, setFilter)` / `@mount` / `@unmount`）。

**props**：`{ hookType: "mount" | "effect" | "unmount", deps?: string }`

**单例约束**：每组件每种 hookType 仅一个（learn-omniview 实证 15 处 / 10 文件，每组件每种一个）。创建时若已存在同 hookType 则禁用。

**解析**（graph-parser.ts）：
- `detectSegmentType` 三分支：`@mount` / `@unmount` / `@effect`（含 `@effect(`）→ `"lifecycle"`（置于现有 fn/condition 判断之后、call 之前；`@` 开头不匹配现有正则，无冲突）
- 新 `parseLifecycleSegment(code)`：首行提取 hookType + deps（`@effect(searchText, setFilter)` → `deps: "searchText, setFilter"`）；兼容单行块 `@mount { initRows(); }`
- 生命周期是**组件级成员**（不在任何 fn 内）：段归属组件级（owner = 组件），与 fn 节点平级独立成块

**发射**（graph-compiler.ts 新 case `"lifecycle"`）：
```
// @pid:xxx
@effect(searchText, setFilter) {
  <body 链缩进>
}
```
`@pid` 注释仅块首行（镜像 fn 模式）。deps 仅 effect 发射括号形式。

**FlowInspector**（FlowInspector.tsx 扩展）：hookType 下拉（mount/effect/unmount）+ deps 公式框（仅 effect 时显示）。

**注册**：flowConstants CODE_TAGS / MENU_SECTIONS / TAG_CN + FlowCanvas nodeTypes。

## 4. 设计 2：try 块节点（tag: `try`）

**形态**：对标 ConditionNode 分支行模式——每分支一行（行首 label + 行内公式框 + source handle id=`br.id`），底部 `flow-out` source handle 用于块后续接。

**props**：`{ branches: Array<{ id: string, type: "try" | "catch" | "finally", catchVar?: string }> }`

**语义约束**：try 必需；catch ≤ 1；finally ≤ 1；至少 catch 或 finally 之一（BNF：`try_catch_stmt ::= 'try' block ('catch' <ident>? block)? ('finally' block)?`，14-grammar-lang.md:486-499）。

**解析**：
- `detectSegmentType`：`/^try\s*\{/` → `"try"`（置于 call 正则之前；try 首 token 不匹配现有正则，实际无冲突但显式前置防回归）
- `assignBranchContext`：`isElifElseCode` 扩展分支延续 regex——`/^\}\s*catch\b/`、`/^\}\s*finally\b/`（镜像现有 `} else {` 模式）
- 新 `parseTryBranches(code)`：逐行提取 try / catch [err] / finally，生成 BranchDef（真实 uid id）

**发射**（graph-compiler.ts 新 case `"try"`，镜像 condition 分支模式）：
- 逐分支：`try {\n <分支链缩进> }` → `catch err {\n ... }` → `finally {\n ... }`
- `@pid` 仅首行（try 行）；branch port key = `br.id`（真实 id，非索引——镜像 condition 的 brPortId 修复经验）
- 分支链走 `branchSuccessors.get(br.id)`；块后继走 `successors`（flow-out）

**TryNode.tsx**：新组件，对标 ConditionNode 分支行渲染 + 行内公式框 + `+ catch` / `+ finally` 按钮（hasCatch / hasFinally guard）；catchVar 行内公式框编辑；HandlesBoundary error-boundary class。

**注册**：flowConstants 三处 + FlowCanvas nodeTypes。

## 5. 设计 3：throw / await 节点 + fn 属性

### 5.1 throw / await —— 通用语法节点（零新 UI）

均为 SyntaxNode 新 case（镜像 return 的轻量形态）：

| case | 发射 | 编辑 |
|------|------|------|
| `throw` | `throw ${value};` | value = 公式框 |
| `await` | `await ${value};` | value = 公式框 |

- `syntax-nodes.ts`：defaultSyntaxProps + formatSyntaxNode 加两 case
- `SyntaxNode.tsx`：加两个渲染 case（一行 label + NodeFormulaField）
- graph-parser：`detectSegmentType` 加 `/^throw\s+/`、`/^await\s+/`（置于 call 正则之前——`await async_sleep(20)` 首 token 是 await）；parseSyntaxProps 对应提取
- graph-compiler：case `"throw"` / `"await"` 走 formatSyntaxNode（合并进 let/assign/store-call/call 组或独立 case）
- flowConstants：菜单两项

**await 出现于 let/return/assign 表达式内部**（如 `let a = async_spawn(task_a());`、`return await async_all([a, b]);`）→ 公式框自由文本天然表达，零改动。

### 5.2 fn 属性（全部走 FnInspector，无节点 UI 变化）

FnInspector 加三字段：`pub` 开关（仅 lib tab 显示）、`async` 开关、`declKind` 下拉（`fn` / `action`）。

**fn 节点 props 扩展**：`{ name, params, returnType, pub?: boolean, isAsync?: boolean, declKind: "fn" | "action" }`

**发射**（graph-compiler.ts case `"fn"` 双格式）：
```
fn:      ${pub ? "pub " : ""}${isAsync ? "async " : ""}fn ${name}(${params})${ret} {
action:  action ${name}(${params})${isAsync ? " async" : ""} {
```

**解析**：
- fn 签名 regex 扩展：`^(pub\s+)?(async\s+)?fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*(\w+))?`（parseFn / parseRootFn 共用）
- `parseComponentKeyword` 加 `case "action"`（**修复 G1**）：action 签名解析复用 store action 的签名收集模式（parser.ts:747-790 的 signature → synthesizeFnEntry），declKind="action"
- `parseRootIdentifier` 加 `fn` / `pub` / `async` case → `parseRootFn()`（**修复 G2**）：消费可选 pub/async 前缀 → 复用 parseFn 的签名收集 + synthesizeFnEntry → codeFlowGraphs + fileDefs.fns

## 6. 设计 4：lib 文件全链路（code tab）

### 6.1 统一模型：顶层声明集合 `fileDefs`

```
fileDefs: Record<filePath, {
  imports: string[];      // 顶层 import 行（原样保留）
  structs: StructDef[];   // { pid, name, pub, nominal, fields: Array<{ name, type }> }
  types: TypeDef[];       // { pid, name, pub, value }  // value = 类型表达式文本（alias/tagged union 共用）
  fns: FnDef[];           // { pid, name, params, returnType, pub, isAsync }
}>
```

ParseResult 扩展 `fileDefs`。一种形态管三种文件的 ROOT 层（lib / 组件文件头部 / store 文件头部）——全部实证声明均出现在 ROOT 层。

### 6.2 解析（修复 G2/G3）

- `parseRootIdentifier` 加 `struct` / `type` case + `pub` 前缀 peek（`pub` 后跟 struct/type/fn 时消费并分派）
- struct：字段对（`name: Type`，逗号分隔、尾逗号容忍）收集到 `}`；`nominal` 在 struct 前（`struct nominal UserId`）
- type：收集到 `;` 或行尾（`type Score = i32;` / `type ValidationResult = ... | ...`——union 整行作为 value 文本）
- 归属：`fileDefs[当前文件路径]`（parser 需感知当前文件路径，从 ctx 或 parse 入参）

### 6.3 物化（修复 G4）

store 新增 `materializeLibGraphs(filePath)`，对标 `materializeFnGraphs`（element-crud.ts:176）：
- 按 lib fn 名从 `codeFlowGraphs` 取图，带同款「fnRegistry.pid 归属校验」防跨文件同名污染
- pid 去重（已物化不覆盖）+ persistedPositions 布局复用（缺 position 时确定性行布局）
- 合并进全局 elements/wires；空合并短路（无新元素不重建数组引用）

DesignPage 加同款 effect：`activeTabType === "code" && activeTab` → `materializeLibGraphs(activeTab.filePath)`（对标 167-176 的 component effect）。

### 6.4 发射（修复 G5 配套）

新 `emitLibFile(def)`：**imports → structs → types → fns**（fn 逐个 `compileCodeFlowGraph`，pub/async 由设计 3 支持）。**不发射 `import omniview;`**（async_demo.omni:5 实证纯逻辑文件不写）。文件头装饰注释不保留（与现有生成式写盘模型一致）。fn 间空行分隔。

组件/store 文件发射器合并 ROOT 层：pipeline 的组件源发射把 `fileDefs[组件文件路径]` 的 structs/types 发射在 `component X {` 前（FeatureCard 头部 struct 保存后不丢）；`emitStoreDefinition` 同理（TodoStore 头部 struct）。

### 6.5 保存（修复 G5）

useSaveProject 加 code tab 分支（对标 store 分支结构）：
1. 当前画布 elements/wires 写回 libDef（fn 图更新）
2. `emitLibFile(def)` 生成源
3. WASM 验证（collectProjectSources 含其他项目源）
4. 写盘 + saveProject + markClean

## 7. 设计 5：LibMembersPanel（code tab 文件级成员面板）

**位置**：StorePage 同款布局——左侧固定 280px 常驻列（StorePage.tsx:184-200 模式），进入 code tab 自动显示，**无按钮开关**（用户已确认）。右侧 FlowInspector 不变。

**三段**：

| 段 | 条目形态 | 编辑表单 |
|----|---------|---------|
| Imports | 每条一行 import 语句 | 自由文本行编辑 + 添加/删除（import 形态多样：`import omniview;` / `import { a, b } from "std/async"` / `import { X } from "./comp"` → 不做结构化表单） |
| Structs | 名称 + pub/nominal 徽标 + 字段数 | pub 开关 + nominal 开关 + name + fields 行列表（`name: Type` 文本对，增删行） |
| Types | 名称 + pub 徽标 | pub 开关 + name + 类型表达式文本（alias 与 tagged union 共用一个文本字段） |

**发射格式**：
```
struct:  ${pub ? "pub " : ""}struct${nominal ? " nominal" : ""} ${name} {\n  ${field}: ${type},\n...\n}
type:    ${pub ? "pub " : ""}type ${name} = ${value};\n
```
（多行 + 尾逗号，对齐 types.omni 风格）

## 8. 范围外（明确排除）

- struct/type 声明出现在组件体/store 体内——实证全部在 ROOT 层，不做成员内声明
- tagged union 结构化表单——类型表达式文本承载（YAGNI）
- `class`（Design Canvas 域，design.omni）
- 组件内 import 编辑——pipeline 自动收集（import-collector），不动
- getter/event/computed 等成员——ComponentMembersPanel / StoreMembersPanel 已覆盖
- 跨文件同名 fn 冲突根治——沿用现有 pid 归属校验防护（保存从画布走天然无冲突）

## 9. 数据流

```
磁盘 .omni → parser（parseRootIdentifier 扩展 + parseFn/parseRootFn + struct/type case）
           → ParseResult { componentDefs, storeDefs, codeFlowGraphs, fileDefs }
           → store（codeFlowGraphs 全局按 fn 名；fileDefs 按路径）
           → materializeLibGraphs / materializeFnGraphs → 画布 elements/wires
编辑 → store.elements/wires（节点增删改 + FlowInspector 表单）
保存 → emitLibFile / generateOmniFiles / emitStoreDefinition（画布 → 源文本）
     → WASM validateSources（最终门禁）→ 写盘
```

## 10. 错误处理

- **WASM 门禁为最终裁决**：保存前 validateSources，critical errors 阻止写盘（现有机制，不动）
- 解析失败段 → fallback `code-block`（现有兜底，手写源码保真）
- 分支语义约束（try/catch/finally 合法性、生命周期单例）在 UI 层 guard（创建禁用/按钮禁用），不阻塞保存（WASM 兜底）

## 11. 测试计划

**Round-trip 回归**（加载 → 保存 → 重解析 → 文本不变）：
- `lib/types.omni`（3 pub struct + nominal + tagged union + 2 alias）
- `lib/async_demo.omni`（8 pub async fn + import + await 独立语句）
- `lib/error_handling.omni`（7 pub fn + try/catch/finally + throw）
- `components/FeatureCard.omni`（头部 struct + struct 构造）
- `stores/TodoStore.omni`（头部 struct）
- `pages/DataFetching.omni`（2 个 async action + try/catch 内 await）——G1 修复回归

**新增单测**：
- graph-parser-syntax.test.ts 模式：lifecycle / try / throw / await 解析 → compile → 再解析闭环
- parseRootIdentifier：pub/async fn、struct、type 顶层解析（含 nominal、尾逗号、union 文本）
- materializeLibGraphs：物化合并、pid 归属校验、持久化位置复用
- emitLibFile：imports → structs → types → fns 顺序、无 import omniview;
- useSaveProject code 分支：写回 + 发射 + 门禁（mock validateSources）
- 生命周期单例守卫、try 分支约束、FnInspector declKind/pub/async 表单

**验证**：tsc + 现有 poiesis 测试套件全绿；learn-omniview 现有文件 WASM 门禁通过为基线。
