# OmniGen — 编辑器前端 .omni 代码生成器设计

> 2026-06-21 | 状态: 架构终版 | 下一步: writing-plans

## 1. 背景与目标

### 1.1 当前状态

编辑器维护两套数据：
- **画布状态**（`ElementData[] + WireData[] + CodeTab[]`）— 用户可视化操作
- **`.omni` 源码**（`activeFileContent`）— Monaco 编辑器中的文本

当前数据流**单向**：`.omni 源码 → 解析 → 画布状态`（服务端 exporter.py），反向不存在。

### 1.2 目标

- **双向同步**：画布状态 ↔ `.omni` 源码
- **Parse 方向**: `.omni 源码 → 编辑器画布状态`（替代服务端 exporter）
- **Generate 方向**: `编辑器画布状态 → .omni 源码`
- **omnic analyze 验证**: parse 和 generate 都必须先过编译验证，不过就报错/回退

### 1.3 核心原则

1. **`.omni` 源码 = 唯一真相源**
2. **编译器可推导的数据不存注释** — tag、props、children、事件绑定由编译器解析
3. **编辑器元数据嵌入严格的注释格式** — `@code-tab`、`@edit`、`@chain`
4. **格式不对 → 解析失败，直接报错** — 严格解析规则
5. **注释本身就是解析路标** — 无需 IR 遍历或 AST 解析器
6. **compileToIR 是纯验证闸门** — 只判别通过/失败，不做数据源
7. **多文件，每个文件 = 一个 Code Tab**（event tab 内联到主文件）

### 1.4 架构决策

| 决策 | 选择 |
|------|------|
| 实现位置 | 纯前端 TypeScript |
| 模块风格 | 纯函数 + 值传递，零副作用 |
| Generate 错误传播 | 三阶段 `StageResult<T>`，阶段内全收集，阶段间短路 |
| 数据聚合 | 取消 ComponentAssembler，Pipeline 直连 SourceEmitter |
| 注解生成 | 结构化 `AnnotationData` + `AnnotationStrategy` 回调 |
| Parse 策略 | **注释驱动行级状态机**，compileToIR 仅验证 |
| Tab 映射 | Event inline + Logic external |
| id 策略 | `{compName}.{path}` 确定性生成，保证 round-trip 一致性 |

---

## 2. 架构总览

### 2.1 双向数据流

```
PARSE 方向 (Open File / 外部修改):
──────────────────────────────────────────
  .omni 源码 (from disk)
    │
    ├─→ compileToIR(source) ──→ errors? → 报错, 终止
    │                              ↓ pass
    └─→ OmniParser(source) ──→ Editor State
         (注释驱动, 行级状态机)
              │
         elements[] + wires[] + codeTabs[]


GENERATE 方向 (Save / Compile):
──────────────────────────────────────────
  Editor State
    │
    └─→ Pipeline (3-phase) ──→ .omni 源码
              │
              └─→ compileToIR(source) ──→ errors? → 回退, 报错
                                              ↓ pass
                                         写入磁盘, 更新内容
```

### 2.2 Generate 管道（三阶段）

```
┌──────────────────────────────────────────────────────────┐
│                    Editor State                           │
│  designElements[] + designWires[] + codeTabs[]            │
│  + componentRegistry + designTokens                      │
└──────────────────────┬───────────────────────────────────┘
                       ↓
┌─ Phase A: 结构分析 ──────────────────────────────────────┐
│  ① ViewTreeBuilder    扁 elements → 嵌套 view tree       │
│  ② WireAnalyzer       连线 → signalBindings + storeRefs  │
│  ③ ImportCollector    扫描引用 → import 语句列表          │
│  产出: StageResult<{viewTree, bindings, imports}>        │
└──────────────────────┬───────────────────────────────────┘
          fatal? → 返回错误, 终止
                       ↓
┌─ Phase B: 语义降级 ──────────────────────────────────────┐
│  ④ StoreExtractor     代码块 → Store 结构化定义           │
│  ⑤ ExpressionLowering 代码图 → fns/actions/getters       │
│  ⑥ StyleResolver      标签映射 + style分离 + class生成   │
│  ⑦ ChainEmitter       代码图 → @chain/@node/@wire 注释   │
│  产出: StageResult<{stores, loweredCode, chains, styledView}>│
└──────────────────────┬───────────────────────────────────┘
          fatal? → 返回错误, 终止
                       ↓
┌─ Phase C: 代码生成 ──────────────────────────────────────┐
│  ⑧ buildAnnotationData  → @code-tab + @edit 注解 map    │
│  ⑨ SourceEmitter        → .omni 源码（内嵌注解）          │
│  ⑩ TokenEmitter         → theme 块                       │
│  ⑪ CustomComponentEmitter → 自定义组件文件               │
│  产出: StageResult<string>                                │
└──────────────────────┬───────────────────────────────────┘
                       ↓
              compileToIR 验证闸门
            pass → 写入 | fail → 回退
```

### 2.3 Tab → File 映射

| Tab 类型 | 映射目标 | 说明 |
|---------|---------|------|
| **main tab** (id="main") | `src/main.omni` | component 主体 |
| **event tabs** | **内联到 main.omni** | `// @code-tab type=event label=xxx` 标记边界 |
| **logic tabs** (非 main) | `src/lib/<label>.omni` | 独立工具文件 |
| **store 定义** | `src/stores/<Name>.omni` | 独立 store 文件 |
| **自定义组件** | `src/components/<Name>.omni` | 独立 component 文件 |

### 2.4 文件结构

```
editor/client/src/omni-gen/
├── index.ts                  # 入口: parseOmniSource + generateOmniSource
├── types.ts                  # 共享类型 + StageResult<T> + PipelineError
│
│  ── Parse 方向 ──
├── validator.ts              # validate(source) → { valid, errors }
├── omni-parser.ts            # parse(source) → Editor State (行级状态机)
│
│  ── Generate 方向 ──
├── view-tree-builder.ts      # ①
├── wire-analyzer.ts          # ②
├── import-collector.ts       # ③
├── store-extractor.ts        # ④
├── expression-lowering.ts    # ⑤
├── style-resolver.ts         # ⑥
├── chain-emitter.ts          # ⑦
├── annotation-data.ts        # ⑧
├── source-emitter.ts         # ⑨
├── token-emitter.ts          # ⑩
├── custom-component-emitter.ts # ⑪
├── pipeline.ts               # 编排层（三阶段）
│
└── __tests__/
    ├── omni-parser.test.ts
    ├── validator.test.ts
    ├── integration-parse.test.ts
    ├── integration-generate.test.ts
    └── ... (各模块测试 + fixtures/)
```

---

## 3. Parse 方向模块

### 3.1 Validator — 编译验证闸门

**职责**: 调用 Rust WASM 编译器验证 .omni 源码。**仅做通过/失败判别**，不用 IR 做数据源。

```typescript
interface ValidationResult {
  valid: boolean;
  errors: Diagnostic[];  // Rust 编译器产出的错误列表
}

function validate(source: string): ValidationResult {
  const result = rustCompiler.compileToIR(source);
  return { valid: result.success, errors: result.errors || [] };
}
```

**规则**:
- Parse 方向：先 validate，不 pass → 报错，不进入 Parser
- Generate 方向：生成后 validate，不 pass → 回退（不写入磁盘），报错

~20 行。

### 3.2 OmniParser — 注释驱动行级状态机

**职责**: 利用 .omni 源码中严格的 Editor 注释格式，以行级状态机方式解析为编辑器画布状态。

**核心洞察**: 注释本身就是路标——`// @edit` 告诉你下一行是 view 元素，`// @chain` 告诉你下面要解析 nodes/wires。**不需要 IR 遍历、不需要 AST、不需要完整语法 parser。**

#### 3.2.1 状态机设计

```
状态:
  ROOT         — 文件顶层，等待 @code-tab / import / component / store
  IN_COMPONENT — 在 component { } 内部
  IN_STORE     — 在 store { } 内部
  IN_VIEW      — 在 view { } 内部，等待 @edit 或 <tag
  IN_CHAIN     — 在 @chain 块内，收集 @node / @wire
  IN_FN        — 在 fn/action 体内，跳过代码行

触发:
  读取一行 → 根据当前状态 + 行内容 → 执行动作 + 可能切换状态

动作:
  - 创建 ElementData (view / codeblock)
  - 设置 parentId / children
  - 提取 @edit flags (folded, render)
  - 收集 @node → codeblock ElementData
  - 收集 @wire → WireData
  - 记录 @code-tab type
  - 记录 import 行
```

#### 3.2.2 逐行解析规则

```
行内容                               → 动作

"// @code-tab type=<logic|event>"   → 记录 codeTabType
"import omniview;"                  → 记录到 imports[]
"import { X } from \"...\";"        → 记录到 imports[]

"component <Name> {"                → 创建 component 根 ElementData, 进入 IN_COMPONENT
"store <Name> {"                    → 创建 store 容器, 进入 IN_STORE
"}"                                 → 退出当前块 (IN_COMPONENT/STORE/VIEW)

// IN_COMPONENT 内:
"  @store <Name>;"                  → 记录 storeRef
"  signal <name>: <type> = <val>;" → 创建 codeblock element (tag="signal")
"  computed <name>: <type> = ...;" → 创建 codeblock element (tag="computed")
"  fn <name>(...) -> <T> {"        → 记录 fn 签名, 进入 IN_FN, 跳过体
"  view {"                          → 进入 IN_VIEW
"  style {"                         → 跳过 style 块(匹配大括号)

// IN_VIEW 内:
"    // @edit [flags]"              → 标记 nextLineIsElement, 提取 flags
"    <tag id=... class=... @event=...>" → 创建 view ElementData, 填 id/props/events
"    </tag>"                        → 关闭当前元素 (pop parentId 栈)

// IN_CHAIN 内:
"// @chain <name>"                  → 进入 IN_CHAIN
"//   @node <id> <tag> <attrs> pos=x,y" → 创建 codeblock ElementData + 位置
"//   @wire <id> <from.port> -> <to.port>" → 创建 WireData
"// ═══════..."                     → 退出 IN_CHAIN

// IN_FN / IN_STORE action 内:
任何代码行                            → 跳过, 直到匹配大括号 "  }"
```

#### 3.2.3 View 元素解析

```typescript
// 输入: "    <button id=\"el-btn\" class: btn @onPress={inc}>+</button>"
// 输出: ElementData

function parseViewLine(line: string, parentId: string | null): ElementData {
  // 1. 提取 tag: 匹配 /<(\w+)/
  const tag = extractTag(line);

  // 2. 提取 id: 匹配 /id="([^"]*)"/
  const id = extractAttr(line, "id");

  // 3. 提取 class: 匹配 /class:\s*(\w+)/
  const className = extractClass(line);

  // 4. 提取事件: 匹配 /@(\w+)={(\w+)}/g
  const events = extractEvents(line);  // { onPress: "inc" }

  // 5. 提取其他属性: 匹配 /\w+="[^"]*"/g (排除 id/class/@)
  const attrs = extractAttrs(line);

  // 6. 提取文本内容: 匹配 />([^<]*)<\// 或 >{([^}]*)}<\//
  const contentText = extractTextContent(line);

  // 7. 判断是否自闭合: />$/
  const isSelfClosing = line.trimEnd().endsWith("/>");

  return {
    id, nodeType: "view", tag,
    props: { ...attrs, ...events, contentText, className },
    children: [], parentId,
    isSelfClosing,
  };
}
```

#### 3.2.4 @chain 块解析

```typescript
// @node 行 → ElementData
function parseNodeLine(line: string): ElementData {
  // "//   @node n1  signal  name=counter  type=i32  init=0  pos=50,50"
  //            ^id  ^tag   ^key=value pairs...              ^pos
  const parts = line.trim().split(/\s+/);
  const id = parts[2];
  const tag = parts[3];
  const attrs: Record<string, string> = {};
  for (let i = 4; i < parts.length; i++) {
    if (parts[i] === "pos=") { /* 后面的 x,y */ }
    else if (parts[i].includes("=")) { const [k,v] = parts[i].split("="); attrs[k]=v; }
  }
  return {
    id, nodeType: "codeblock", tag,
    props: attrs,
    position: parsePosition(parts),
    children: [], parentId: null,
  };
}

// @wire 行 → WireData
function parseWireLine(line: string): WireData {
  // "//   @wire w1  n1.value -> n2.a"
  //            ^id  ^from   ^to
  // 或: "//   @wire w5  n4.false -> n6"  (fn 节点不写端口名)
  // 或: "//   @wire w6  n6 -> n7.value"  (fn 输出不写端口名)
  const match = line.match(/@wire\s+(\S+)\s+(\S+)\s*->\s*(\S+)/);
  const [_, id, from, to] = match!;
  const [fromId, fromPort] = from.includes(".") ? from.split(".") : [from, ""];
  const [toId, toPort] = to.includes(".") ? to.split(".") : [to, ""];
  return { id, fromId, fromPort, toId, toPort };
}
```

#### 3.2.5 关键决策

**元素 id 策略**: 使用 `{compName}.{path}`，例如 `App.button.inc`。Generate 方向用相同策略生成 `id="App.button.inc"`，保证 round-trip 稳定。

**自闭合检测**: `/^<tag ... />$/` 正则匹配行末 `/>`。

**代码块跳过**: `fn { }` / `action { }` 体只匹配大括号深度，不解析内容。代码逻辑由 compileToIR 保证正确性。

**缩进感知**: 利用缩进层级推断 parent 关系。`<tag` 的缩进比上一个 `<tag` 深 → 嵌套；相同 → 兄弟；浅 → 父关闭。

**代码量**: ~200 行

---

## 4. Generate 方向模块

### 4.1 ViewTreeBuilder

递归构建 view tree，顺带收集 PascalCase 标签供 ImportCollector。~50 行。

### 4.2 WireAnalyzer

线性端口分类器。~50 行。

### 4.3 ImportCollector（新增）

收集 import 语句。~60 行。

### 4.4 StoreExtractor

两阶段声明式提取。~150 行。

### 4.5 ExpressionLowering

递归下降构建器。~350 行。

### 4.6 StyleResolver

TAG_HTML_MAP（19 个映射）+ STYLE_PROP_CSS_MAP（snake→kebab）+ CSS class 生成。~80 行。

### 4.7 ChainEmitter

代码图 → `@chain` / `@node` / `@wire` 注释块。含 @node 必填属性表 + S5 变量引用验证。~210 行。

### 4.8 buildAnnotationData

**从旧 AnnotationInjector 拆分的数据层**。

```typescript
interface AnnotationData {
  codeTabDirective: string;
  editAnnotations: Map<string, EditAnnotation>;
}

function buildAnnotationData(viewNodes: ViewNode[], tabType: "logic" | "event"): AnnotationData;
```

纯数据转换，~60 行。

### 4.9 SourceEmitter

**接收 `EmitInput` + `AnnotationStrategy` + chains，内嵌生成注解**。

```typescript
interface EmitInput {
  codeTabType: "logic" | "event";
  imports: string[];
  stores: StoreDef[];
  component: ComponentPart;
  themes?: ThemeDef[];
  customComponents?: Array<{ name: string; source: string }>;
}

interface AnnotationStrategy {
  beforeViewNode(node: ViewNode, depth: number): string | null;
  beforeFile(): string;
}

class SourceEmitter {
  constructor(private annotationStrategy?: AnnotationStrategy) {}
  emitDocument(input: EmitInput, chains: Map<string, ChainBlock>): string;
}
```

**注解内嵌**: `emitViewNode` 在每个 `<tag` 前调用 `annotationStrategy.beforeViewNode()` 输出 `// @edit`。不再有后期字符串注入。

**Store 关键字**: `action` 不是 `fn`；`getter name: type = expr;`。

**已知限制（Phase 3）**: `@if` / `@for` 指令、动态 `class: {}` 语法。

~370 行。

### 4.10 TokenEmitter

~30 行。

### 4.11 CustomComponentEmitter

C2 简化子管道。~60 行。

---

## 5. 错误处理系统

### 5.1 Parse 方向

```
validate(source) → errors?
  ├─ 有 error → 返回 { valid: false, errors }, 不进入 Parser
  └─ 无 error → OmniParser.parse(source) → Editor State
```

**Parse 本身的错误**（注释格式违反规则 P1-P6 等）→ 尽量收集所有错误后返回，不加载不完整的画布。

### 5.2 Generate 方向

```typescript
interface PipelineError {
  phase: "A" | "B" | "C";
  module: string;
  severity: "fatal" | "warning";
  elementId?: string;
  code: string;
  message: string;
}

interface StageResult<T> {
  value: T | null;
  errors: PipelineError[];
}
```

**三阶段规则**: 阶段内全部模块执行（全收集错误），阶段间判 fatal 短路。Phase A 有 fatal → 跳过 B/C；Phase B 有 fatal → 跳过 C。

**生成后验证**: SourceEmitter 产出 → `validate(source)` → errors? → 回退（不写入磁盘），保留旧 activeFileContent。

~50 行（Pipeline 编排）。

---

## 6. Pipeline 编排层

```typescript
// ── Parse 方向 ──
function parseOmniSource(source: string): {
  state: EditorState | null;
  errors: Diagnostic[];
};

// ── Generate 方向（单文件）──
function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentRegistry: Record<string, ComponentDefinition>;
  designTokens: DesignTokens;
}): StageResult<string>;

// ── Generate 方向（多文件）──
function generateOmniFiles(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentRegistry: Record<string, ComponentDefinition>;
  designTokens: DesignTokens;
}): StageResult<Record<string, string>>;
```

**generateOmniSource 执行顺序**:
```
Phase A:
  1. ViewTreeBuilder → viewTree + pascalCaseTags[]
  2. WireAnalyzer → BindingResult
  3. ImportCollector → importLines[]
Phase B:
  4. StoreExtractor → stores + standaloneSignals
  5. ExpressionLowering → LoweredCode
  6. StyleResolver → ViewNode.resolvedStyle (in-place)
  7. ChainEmitter → Map<fnName, ChainBlock>
Phase C:
  8. buildAnnotationData → AnnotationData
  9. SourceEmitter.emitDocument(EmitInput, chains)
     - 构造 EmitInput (imports + stores + component + themes + customComponents)
     - 构造 AnnotationStrategy (from AnnotationData)
     - 内嵌生成 .omni 源码（含 @code-tab + @edit + @chain）
```

~130 行。

---

## 7. 关键类型

```typescript
// ── 管道错误 ──
interface PipelineError {
  phase: "A" | "B" | "C";
  module: string;
  severity: "fatal" | "warning";
  elementId?: string;
  code: string;
  message: string;
}

interface StageResult<T> {
  value: T | null;
  errors: PipelineError[];
}

// ── Validator ──
interface ValidationResult {
  valid: boolean;
  errors: Diagnostic[];
}

// ── EmitInput ──
interface EmitInput {
  codeTabType: "logic" | "event";
  imports: string[];
  stores: StoreDef[];
  component: ComponentPart;
  themes?: ThemeDef[];
  customComponents?: Array<{ name: string; source: string }>;
}

interface ComponentPart {
  name: string;
  signals: Record<string, { type: string; default?: unknown }>;
  computeds: Record<string, { type: string; body: string }>;
  storeRefs: string[];
  fns: Array<{ name: string; params: Array<{name:string; type:string}>; returnType: string; body: string }>;
  view: ViewNode | null;
  style: Record<string, Record<string, string>>;
}

// ── Annotation ──
interface AnnotationData {
  codeTabDirective: string;
  editAnnotations: Map<string, EditAnnotation>;
}

interface AnnotationStrategy {
  beforeViewNode(node: ViewNode, depth: number): string | null;
  beforeFile(): string;
}

// ── StyleResolver 产出 ──
interface ResolvedStyle {
  htmlTag: string;
  cssClassName: string;
  contentText?: string;
  htmlAttrs: Record<string, string>;
  eventBindings: Record<string, string>;
  bindings: Record<string, string>;
  cssRules: Record<string, string>;
  usedClassNames: string[];
}
```

---

## 8. 代码量估算

| 模块 | 行数 | 说明 |
|------|------|------|
| **Parse 方向** | | |
| validator.ts | 20 | compileToIR 封装 |
| omni-parser.ts | 200 | 行级状态机 |
| **Generate 方向** | | |
| types.ts | 150 | StageResult + PipelineError + EmitInput |
| view-tree-builder.ts | 50 | |
| wire-analyzer.ts | 50 | |
| import-collector.ts | 60 | 新增 |
| store-extractor.ts | 150 | |
| expression-lowering.ts | 350 | |
| style-resolver.ts | 80 | |
| chain-emitter.ts | 210 | |
| annotation-data.ts | 60 | 拆分自旧 AnnotationInjector |
| source-emitter.ts | 370 | EmitInput + AnnotationStrategy |
| token-emitter.ts | 30 | |
| custom-component-emitter.ts | 60 | |
| pipeline.ts | 130 | 三阶段 + parse 入口 |
| **总计** | **~1970** | |

---

## 9. 集成

### 9.1 Open File

```typescript
async function openFile(filePath: string) {
  const source = await api.readFile(filePath);
  const { state, errors } = parseOmniSource(source);

  if (!state) {
    setCompileErrors(errors);    // 有错误, 不加载
    return;
  }
  setEditorState(state);         // 加载画布
  if (errors.length > 0) showWarnings(errors);
}
```

### 9.2 Save Project

```typescript
async function saveProject() {
  const result = generateOmniFiles({ ... });

  if (!result.value) {
    setCompileErrors(result.errors);  // Generate 管道出错
    return;
  }

  for (const [path, source] of Object.entries(result.value)) {
    // compileToIR 验证闸门
    const { valid, errors } = validate(source);
    if (!valid) {
      // 回退: 不写入, 保留旧源码, 显示错误
      setCompileErrors(errors);
      return;
    }
    await api.writeFile(path, source);
  }

  store.markClean();
}
```

### 9.3 Compile

```typescript
async function handleCompile() {
  const result = generateOmniSource({ ... });
  if (!result.value) { setCompileErrors(result.errors); return; }

  const { valid, errors } = validate(result.value);
  if (!valid) { setCompileErrors(errors); return; }

  store.setSourceCode(result.value);
  const compileResult = await api.compile(result.value, target);
}
```

---

## 10. 风险与缓解

| 风险 | 缓解 |
|------|------|
| Parser 缩进推断 parent 关系不准确 | 优先使用 `id` 的路径结构 `{comp}.{path}` 反推层级；缩进仅作辅助 |
| @chain 正则与格式化变体不匹配 | Generate 方向产出严格格式；外部编辑时宽松匹配 + 交叉校验 @node name |
| compileToIR 性能 | WASM 编译 1-10ms；缓存上次成功 IR |
| round-trip id 漂移 | Generate 和 Parse 共享同一套 `{comp}.{path}` id 生成函数 |

---

## 11. 实施阶段

### Phase 1: Parse 方向
1. validator.ts
2. omni-parser.ts（行级状态机）
3. `parseOmniSource()` 入口
4. **目标**: 能正确加载已有 .omni 文件到画布

### Phase 2: Generate 核心（Phase A + 骨架）
5. types.ts + view-tree-builder + wire-analyzer + import-collector
6. style-resolver + source-emitter（基础版）
7. pipeline.ts（Phase A + C 骨架）
8. **目标**: view tree + style → 合法 .omni 源码

### Phase 3: Generate 逻辑层
9. store-extractor + expression-lowering
10. chain-emitter
11. **目标**: 完整 component + store + fn + @chain

### Phase 4: 注解 + 错误 + 集成
12. annotation-data + SourceEmitter AnnotationStrategy 集成
13. StageResult 三阶段编排
14. token-emitter + custom-component-emitter
15. 集成到 App.tsx
16. **目标**: 完整双向同步上线
