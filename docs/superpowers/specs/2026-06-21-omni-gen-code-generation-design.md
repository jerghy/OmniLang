# OmniGen — 编辑器前端 .omni 代码生成器设计

> 2026-06-21 | 状态: 架构终版 | 下一步: writing-plans

## 1. 背景与目标

### 1.1 当前状态

编辑器维护两套数据：
- **画布状态**（`ElementData[] + WireData[] + CodeTab[]`）— 用户可视化操作
- **`.omni` 源码**（`activeFileContent`）— Monaco 编辑器中的文本

当前数据流**单向**：`.omni 源码 → 解析 → 画布状态`（exporter.py），反向不存在。

### 1.2 目标

实现**双向同步**：画布状态 ↔ `.omni` 源码。采用"编辑时标记 dirty，保存时同步，编译/预览时也触发同步"的混合模式。

### 1.3 核心原则（来自 Editor .omni 保存格式规格）

1. **`.omni` 源码 = 唯一真相源**
2. **编译器可推导的数据不存注释** — tag、props、children、事件绑定由编译器解析
3. **编辑器元数据嵌入严格的注释格式** — `@code-tab`、`@edit`、`@chain`
4. **格式不对 → 解析失败，直接报错**
5. **多文件，每个文件 = 一个 Code Tab**（event tab 除外，见 §2.2）

### 1.4 架构决策

| 决策 | 选择 |
|------|------|
| 实现位置 | 纯前端 TypeScript |
| 模块风格 | 纯函数 + 值传递，零副作用 |
| 错误传播 | 三阶段 `StageResult<T>`，阶段内全收集，阶段间短路 |
| 数据聚合 | 取消 ComponentAssembler，Pipeline 直连 SourceEmitter |
| 注解生成 | 结构化 `AnnotationData` + `AnnotationStrategy` 回调，不后期注入 |
| Tab 映射 | Event inline + Logic external |

---

## 2. 架构总览

### 2.1 三阶段管道

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
          errors? → 返回错误, 终止
                       ↓
┌─ Phase B: 语义降级 ──────────────────────────────────────┐
│  ④ StoreExtractor     代码块 → Store 结构化定义           │
│  ⑤ ExpressionLowering 代码图 → fns/actions/getters       │
│  ⑥ StyleResolver      tag映射 + style分离 + class生成    │
│  ⑦ ChainEmitter       代码图 → @chain/@node/@wire 注释   │
│  产出: StageResult<{stores, loweredCode, chains, styledView}>│
└──────────────────────┬───────────────────────────────────┘
          errors? → 返回错误, 终止
                       ↓
┌─ Phase C: 代码生成 ──────────────────────────────────────┐
│  ⑧ buildAnnotationData  → codeTab + editAnnotation map  │
│  ⑨ SourceEmitter        → .omni 源码（内嵌 @edit + @chain）│
│  ⑩ TokenEmitter         → theme 块                       │
│  ⑪ CustomComponentEmitter → 自定义组件文件               │
│  产出: StageResult<string>                                │
└──────────────────────────────────────────────────────────┘
```

### 2.2 Tab → File 映射

| Tab 类型 | 映射目标 | 说明 |
|---------|---------|------|
| **main tab** (id="main") | `src/main.omni` | component 主体（view + signals + fns） |
| **event tabs** | **内联到 main.omni** | handler 需访问 component scope，不独立文件 |
| **logic tabs** (非 main) | `src/lib/<label>.omni` | 独立工具函数 |
| **store 定义** | `src/stores/<Name>.omni` | 独立 store 文件 |
| **自定义组件** | `src/components/<Name>.omni` | 独立 component 文件 |

**Event tab round-trip**: 内联时在每个 handler 前注入 `// @code-tab type=event label=<tabLabel>`，解析器据此重建 event tab 边界。

### 2.3 文件结构

```
editor/client/src/omni-gen/
├── index.ts                  # 入口: generateOmniSource() / generateOmniFiles()
├── types.ts                  # 共享类型 + StageResult<T> + PipelineError
├── view-tree-builder.ts      # ①
├── wire-analyzer.ts          # ②
├── import-collector.ts       # ③ 新增
├── store-extractor.ts        # ④
├── expression-lowering.ts    # ⑤
├── style-resolver.ts         # ⑥
├── chain-emitter.ts          # ⑦
├── annotation-data.ts        # ⑧ buildAnnotationData() + AnnotationStrategy
├── source-emitter.ts         # ⑨ (接收 EmitInput + AnnotationStrategy + chains)
├── token-emitter.ts          # ⑩
├── custom-component-emitter.ts # ⑪
├── pipeline.ts               # 编排层（三阶段 StageResult）
└── __tests__/
    ├── integration.test.ts
    ├── import-collector.test.ts
    ├── annotation-data.test.ts
    ├── ... (各模块测试 + fixtures/)
```

---

## 3. 模块详细设计

### 3.1 ViewTreeBuilder — 视图树构建

**方案**: Recursive Top-down

**输入**: `ElementData[]`（已过滤 nodeType === "view"）
**输出**: `ViewNode[]` 嵌套树

**核心算法**:
```
1. 构建 Map<id, ElementData>
2. 找到 parentId === null 的根节点
3. 递归 convert(node):
   - 创建 ViewNode{ id, tag, props, kind:"element", children:[] }
   - 若 COMPONENT_PROTOCOL[tag].children !== false → 递归子节点
   - 返回 ViewNode
```

**新增**: 收集所有 PascalCase 标签（不在 TAG_HTML_MAP 中），供 ImportCollector 使用。

**复杂度**: O(n)，~50 行

---

### 3.2 WireAnalyzer — 连线分析器

**方案**: 线性端口分类器

**输入**: `ElementData[] + WireData[]`
**输出**: `BindingResult`

```typescript
interface BindingResult {
  signalBindings: Map<string, Array<{ elementId: string; propName: string }>>;
  storeRefs: Map<string, string[]>;
  computedDeps: Map<string, string[]>;
  unwiredProps: Array<{ elementId: string; tag: string; propName: string }>;
}
```

**复杂度**: O(W + V)，~50 行

---

### 3.3 ImportCollector — Import 收集器（新增）

**职责**: 扫描所有引用，生成 import 语句列表。

**输入**: `{ viewTree, storeRefs, allCodeTabs, componentRegistry }`
**输出**: `string[]` — import 语句列表

**收集规则**:

```
1. "import omniview;"  ← 始终第一

2. 扫描 view tree → PascalCase 标签（不在 TAG_HTML_MAP 中）
   → import { Tag } from "./components/Tag";

3. storeRefs → import { StoreName } from "./stores/StoreName";

4. 跨 tab 函数引用:
   a. 扫描所有 logic tab → Map<fnName, tabLabel>
   b. 扫描所有 event handler body 中的函数调用
   c. 若调用的函数在 logic tab map 中 → import { fn } from "./lib/<tabLabel>";
```

**复杂度**: O(N + W)，~60 行

---

### 3.4 StoreExtractor — Store 提取器

两阶段声明式提取。无变化。~150 行。

---

### 3.5 ExpressionLowering — 表达式降级器

递归下降构建器（MVP）。无变化。~350 行。

---

### 3.6 StyleResolver — 样式/属性分离 + 标签映射

**TAG_HTML_MAP**（19 个设计系统→HTML 标签映射）+ **STYLE_PROP_CSS_MAP**（snake_case→kebab-case）。

**输出**: `ResolvedStyle`（`htmlTag`, `cssClassName`, `contentText`, `htmlAttrs`, `eventBindings`, `bindings`, `cssRules`）

~80 行。

---

### 3.7 ChainEmitter — 链注释生成器

**输入**: `CodeTab.graph + LoweredCode.functions`
**输出**: `Map<string, ChainBlock>` — fnName → { functionName, nodeLines[], wireLines[] }

**@node 必填属性表** + **@wire 格式规则**（fn 节点不写端口名、箭头 `->`、branch true/false 端口）+ **S5 变量引用验证**。

~210 行。

---

### 3.8 buildAnnotationData — 注解数据生成器（重构）

**原 AnnotationInjector 拆分后的数据层**。

```typescript
interface AnnotationData {
  codeTabDirective: string;                             // "// @code-tab type=logic"
  editAnnotations: Map<string, EditAnnotation>;         // nodeId → annotation
}

interface EditAnnotation {
  line: string;         // "// @edit" | "// @edit folded" | "// @edit render=canvas"
  indent: number;       // view tree 深度（SourceEmitter 控制实际缩进空格数）
}

function buildAnnotationData(
  viewNodes: ViewNode[],
  tabType: "logic" | "event",
): AnnotationData;
```

**纯数据转换**: 遍历 view tree → 为每个 ViewNode 生成 `EditAnnotation`（含折叠/渲染标志）。不涉及字符串拼接，不依赖 SourceEmitter。

~60 行。

---

### 3.9 SourceEmitter — 源码生成器（重构）

#### 3.9.1 输入接口

```typescript
interface EmitInput {
  // 文件级
  codeTabType: "logic" | "event";
  imports: string[];

  // Store（可有多个）
  stores: Array<{
    name: string;
    signals: Record<string, { type: string; default?: unknown }>;
    getters: Record<string, { returnType: string; body: string }>;
    actions: Array<{ name: string; params: Array<{name:string; type:string}>; body: string }>;
  }>;

  // Component（主组件）
  component: {
    name: string;
    signals: Record<string, { type: string; default?: unknown }>;
    computeds: Record<string, { type: string; body: string }>;
    storeRefs: string[];
    fns: Array<{ name: string; params: Array<{name:string; type:string}>; returnType: string; body: string }>;
    view: ViewNode | null;
    style: Record<string, Record<string, string>>;
  };

  // 可选
  themes?: ThemeDefinition[];
  customComponents?: Array<{ name: string; source: string }>;
}

interface AnnotationStrategy {
  /** 在每个 view 元素前调用，返回注解行或 null */
  beforeViewNode(node: ViewNode, depth: number): string | null;
  /** 在整个文件生成前调用，返回文件头指令（@code-tab） */
  beforeFile(): string;
}

class SourceEmitter {
  constructor(
    private annotationStrategy?: AnnotationStrategy,
  ) {}
  emitDocument(input: EmitInput, chains: Map<string, ChainBlock>): string;
}
```

#### 3.9.2 关键变化

| 变化 | 旧设计 | 新设计 |
|------|--------|--------|
| **输入** | ComponentDefinition | `EmitInput`（展开的字段） |
| **主题/自定义组件** | 后期字符串拼接 | 作为 EmitInput 字段，emitDocument 内部输出 |
| **注解** | 后期行解析注入 | `AnnotationStrategy.beforeViewNode` 回调，生成时内嵌 |
| **Chain** | 构造函数传入 chains Map | 同，emitComponent 内 emit fn 时先 emit chain |
| **@code-tab** | AnnotationInjector 后期注入 | `AnnotationStrategy.beforeFile()` 在 emitDocument 开头调用 |

#### 3.9.3 View 元素生成

```
<{htmlTag} id="{id}" class: {className}[ @onEvent={handler}]*[ attr="value"]*[ bind:prop={signal}]*
```

- **标签**: 始终成对 `<tag></tag>`
- **CSS class**: `class: className`
- **事件**: `@onEvent={handler}`
- **缩进**: 4 空格

#### 3.9.4 Store 生成

使用 `action` 关键字（非 `fn`）：`action addTodo(text: String) { ... }`，`getter name: type = expr;`。

#### 3.9.5 已知限制（Phase 3）

- `@if` / `@for` 指令的 view 节点生成
- 动态 CSS class: `class: { cls1: true, cls2: condition }`

~370 行。

---

### 3.10 TokenEmitter

始终完整生成。~30 行。

### 3.11 CustomComponentEmitter

简化子管道（C2）。~60 行。

---

### 3.12 错误处理系统

#### 3.12.1 类型定义

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
  value: T | null;               // null = fatal 导致阶段失败
  errors: PipelineError[];       // 当前阶段收集的错误
}
```

#### 3.12.2 三阶段执行规则

```
Phase A: 所有模块执行 → 全收集错误
  ├─ 有 fatal → 返回 StageResult{ value: null, errors }
  └─ 无 fatal → 继续 Phase B

Phase B: 所有模块执行 → 全收集错误
  ├─ 有 fatal → 返回 StageResult{ value: null, errors }
  └─ 无 fatal → 继续 Phase C

Phase C: 所有模块执行 → 全收集错误
  返回 StageResult{ value: source, errors }
```

**关键**: 阶段内不短路——即使模块 2 报 fatal，仍执行模块 3（收集更多错误）。阶段间短路——Phase A 失败则跳过 B/C。

~50 行（Pipeline 编排逻辑）。

---

### 3.13 Pipeline — 编排层

```typescript
// 单文件生成
function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];             // 所有 tab（main + event + logic）
  componentRegistry: Record<string, ComponentDefinition>;
  designTokens: DesignTokens;
}): StageResult<string>;

// 多文件生成
function generateOmniFiles(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentRegistry: Record<string, ComponentDefinition>;
  designTokens: DesignTokens;
}): StageResult<Record<string, string>>;  // filePath → source
```

**单文件生成执行顺序**:
```
Phase A:
  1. ViewTreeBuilder: designElements → viewTree + pascalCaseTags[]
  2. WireAnalyzer: elements + wires → BindingResult
  3. ImportCollector: tags + storeRefs + codeTabs → importLines[]
  检查 fatal → 终止或继续

Phase B:
  4. StoreExtractor: codeBlocks + codeWires → stores + standaloneSignals
  5. ExpressionLowering: codeBlocks + codeWires → LoweredCode
  6. StyleResolver: viewTree 各节点 → ViewNode.resolvedStyle (in-place)
  7. ChainEmitter: codeTabs + loweredCode → Map<fnName, ChainBlock>
  检查 fatal → 终止或继续

Phase C:
  8. buildAnnotationData: viewTree → AnnotationData
  9. SourceEmitter.emitDocument(EmitInput{...}, chains)
     - 构造 EmitInput（imports + stores + component + themes + customComponents）
     - 构造 AnnotationStrategy（from AnnotationData）
     - 生成 .omni 源码
  10. 返回 StageResult{ value: source, errors }
```

**多文件生成**: main.omni（含内联 event）+ logic tab 文件 + store 文件 + 自定义组件文件。

~130 行。

---

## 4. 类型系统

### 4.1 核心类型

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

// ── EmitInput（SourceEmitter 输入）──
interface EmitInput {
  codeTabType: "logic" | "event";
  imports: string[];
  stores: StoreDef[];
  component: ComponentPart;
  themes?: ThemeDef[];
  customComponents?: Array<{ name: string; source: string }>;
}

interface StoreDef {
  name: string;
  signals: Record<string, { type: string; default?: unknown }>;
  getters: Record<string, { returnType: string; body: string }>;
  actions: Array<{ name: string; params: Array<{name:string; type:string}>; body: string }>;
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

interface EditAnnotation {
  line: string;
  indent: number;
}

interface AnnotationStrategy {
  beforeViewNode(node: ViewNode, depth: number): string | null;
  beforeFile(): string;
}

// ── ViewNode ──
interface ViewNode {
  id: string;
  tag: string;          // 设计系统标签（Button, Container...），StyleResolver 填充 htmlTag
  props: Record<string, unknown>;
  children: ViewNode[];
  kind: "element" | "text" | "interpolation" | "if" | "for" | "slot";
  resolvedStyle?: ResolvedStyle;
  isFolded?: boolean;
  renderMode?: "dom" | "canvas";
  condition?: string;
  branches?: { condition: string; body: ViewNode[] }[];
  fallback?: ViewNode[] | null;
  iterable?: string;
  itemBinding?: string;
  keyExpr?: string;
  emptyBody?: ViewNode[] | null;
  text?: string;
  expression?: string;
  slotName?: string;
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

// ── ChainEmitter 产出 ──
interface ChainBlock {
  functionName: string;
  nodeLines: string[];
  wireLines: string[];
}

// ── ExpressionLowering 产出 ──
interface LoweredCode {
  expressions: Map<string, string>;
  functions: Array<{ name: string; params: Array<{name:string; type:string}>; returnType: string; body: string }>;
  actions: Array<{ storeName: string; name: string; params: Array<{name:string; type:string}>; body: string }>;
  getters: Array<{ name: string; returnType: string; body: string; deps: string[] }>;
}
```

---

## 5. 代码量估算

| 模块 | 行数 | 变化 |
|------|------|------|
| types.ts | 180 | +20 (StageResult + PipelineError) |
| view-tree-builder.ts | 50 | +10 (收集 PascalCase 标签) |
| wire-analyzer.ts | 50 | — |
| import-collector.ts | 60 | **新增** |
| store-extractor.ts | 150 | — |
| expression-lowering.ts | 350 | — |
| style-resolver.ts | 80 | — |
| chain-emitter.ts | 210 | — |
| annotation-data.ts | 60 | **拆分自旧 AnnotationInjector** |
| source-emitter.ts | 370 | +20 (EmitInput + AnnotationStrategy) |
| token-emitter.ts | 30 | — |
| custom-component-emitter.ts | 60 | — |
| pipeline.ts | 130 | +10 (三阶段 StageResult 编排) |
| **总计** | **~1780** | |

> 旧设计 ~1830 行，移除 ComponentAssembler (-80)、拆分 AnnotationInjector、新增 ImportCollector (+60)，净减少 ~50 行且架构更清晰。

---

## 6. 与现有系统的集成

### 6.1 `.omni` 为唯一真相源

```
.omni 文件内容 = OmniLang 合法代码 + 编辑器注释
  ├── // @code-tab type=logic
  ├── import omniview;
  ├── import { TodoStore } from "./stores/TodoStore";
  ├── store TodoStore { ... }
  ├── component App {
  │     @store TodoStore;
  │     signal counter: i32 = 0;
  │
  │     // ════════════════════
  │     // @chain handleClick
  │     //   @node n1  signal  name=counter  pos=50,50
  │     //   @wire w1  n1.value -> n2.a
  │     // ════════════════════
  │     fn handleClick() -> void { counter = counter + 1; }
  │
  │     // @code-tab type=event label=onReset    ← event tab 边界
  │     fn onReset() -> void { counter = 0; }
  │
  │     view {
  │         // @edit
  │         <div id="el-root" class: app>
  │             // @edit
  │             <button id="el-btn" class: btn @onPress={handleClick}>+</button>
  │         </div>
  │     }
  │     style {
  │         .app { max-width: 700px; margin: 0 auto; }
  │         .btn { width: 40px; border-radius: 8px; }
  │     }
  │   }
  └── theme myTheme { ... }
```

### 6.2 保存时触发

```typescript
const saveProject = useCallback(async () => {
  const result = generateOmniFiles({
    designElements: store.elements,
    designWires: store.wires,
    codeTabs: useCodeGraphStore.getState().toSerializable().tabs,
    componentRegistry: useComponentRegistry.getState().definitions,
    designTokens: useTokenStore.getState().tokens,
  });

  if (!result.value) {
    // 有 fatal 错误 → 显示错误面板，不写入
    setCompileErrors(result.errors);
    return;
  }

  // 写入所有文件
  for (const [filePath, source] of Object.entries(result.value)) {
    await api.writeFile(filePath, source);
  }

  // 更新 .omni-editor.toml
  await api.saveEditorToml({ active_file, active_mode, open_files, ... });

  store.setSourceCode(result.value["src/main.omni"] || "");
  store.markClean();

  // 显示 warnings（如有）
  if (result.errors.length > 0) {
    showWarnings(result.errors);
  }
});
```

### 6.3 编译时触发

```typescript
const handleCompile = useCallback(async () => {
  const result = generateOmniSource({
    designElements: store.elements,
    designWires: store.wires,
    codeTabs: useCodeGraphStore.getState().toSerializable().tabs,
    componentRegistry: useComponentRegistry.getState().definitions,
    designTokens: useTokenStore.getState().tokens,
  });

  if (!result.value) {
    setCompileErrors(result.errors);
    return;
  }

  store.setSourceCode(result.value);
  const compileResult = await api.compile(result.value, target);
  setCompileErrors(compileResult.errors || []);
});
```

---

## 7. 测试策略

### 7.1 单元测试

每个模块纯函数测试。各阶段可独立构造输入数据。

### 7.2 集成测试

- 用现有 E2E 项目 snapshot 走完整管道
- 验证输出 .omni 符合所有解析规则（P1-P6, I1-I4, C1-C4, S1-S5, F1-F3）
- 验证 round-trip：编译器解析生成的 .omni → 无错误

### 7.3 错误路径测试

- 构造有循环依赖的 ExpressionLowering 输入 → 验证 Phase B 返回 fatal
- 构造有非法 port 的 wire → 验证 Phase A 收集 error 但继续执行其余模块

---

## 8. 风险与缓解

| 风险 | 缓解 |
|------|------|
| ExpressionLowering 复杂度过高 | MVP 先处理 linear data flow，branch/forEach 作为 Phase 2 |
| Event tab round-trip 边界识别 | `// @code-tab type=event label=xxx` 作为明确的切分标记 |
| 生成代码格式不完美 | round-trip 测试验证正确性 |
| 大型项目性能 | 生成在保存/编译时触发（非实时），1-2 秒可接受 |
| 与手写代码冲突 | `@code-tab` / `@edit` / `@chain` 标记明确分界线 |
| StageResult 样板代码 | 类型定义集中在 types.ts，Pipeline 编排在一个函数内 |

---

## 9. 实施阶段

### Phase 1: Phase A（结构分析 → round-trip）
1. types.ts（含 StageResult + PipelineError）
2. view-tree-builder.ts
3. wire-analyzer.ts
4. import-collector.ts
5. style-resolver.ts
6. source-emitter.ts（基础版，不含 AnnotationStrategy）
7. pipeline.ts（Phase A + C 骨架）
8. **目标**: 能生成合法 .omni 源码（view tree + props + style）

### Phase 2: Phase B（语义降级）
9. store-extractor.ts
10. expression-lowering.ts
11. chain-emitter.ts
12. **目标**: 完整的 component + store + fn + @chain 生成

### Phase 3: 注解 + 错误处理
13. annotation-data.ts
14. SourceEmitter 集成 AnnotationStrategy
15. Pipeline 三阶段 StageResult 编排
16. **目标**: 含完整编辑器元数据 + 错误报告的 .omni 源码

### Phase 4: 扩展 + 集成
17. token-emitter.ts
18. custom-component-emitter.ts
19. 多文件生成（generateOmniFiles）
20. 集成到 App.tsx
21. @if / @for 指令支持
22. **目标**: 完整双向同步上线
