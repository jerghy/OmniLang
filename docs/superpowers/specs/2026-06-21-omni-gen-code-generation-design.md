# OmniGen — 编辑器前端 .omni 代码生成器设计

> 2026-06-21 | 状态: 修订完成（整合 Editor .omni 保存格式规格） | 下一步: writing-plans

## 1. 背景与目标

### 1.1 当前状态

编辑器维护两套数据：
- **画布状态**（`ElementData[] + WireData[] + CodeTab[]`）— 用户可视化操作
- **`.omni` 源码**（`activeFileContent`）— Monaco 编辑器中的文本

当前数据流**单向**：`.omni 源码 → 解析 → 画布状态`（exporter.py），反向不存在。

### 1.2 目标

实现**双向同步**：画布状态 ↔ `.omni` 源码。采用"编辑时标记 dirty，保存时同步，编译/预览时也触发同步"的混合模式。

### 1.3 核心原则（来自 Editor .omni 保存格式规格）

1. **`.omni` 源码 = 唯一真相源** — 不再以 `.omni.json` 为主存储格式
2. **编译器可推导的数据不存注释** — tag、props、children、事件绑定由编译器解析
3. **编辑器元数据嵌入严格的注释格式** — `@code-tab`、`@edit`、`@chain` 块
4. **格式不对 → 解析失败，直接报错** — 严格解析规则
5. **多文件，每个文件 = 一个 Code Tab**

### 1.4 方案选择

纯前端 TypeScript 管道（方案 B），12 个独立模块，纯函数 + 值传递，零副作用。

---

## 2. 架构总览

```
┌──────────────────────────────────────────────────────────┐
│                    Editor State                           │
│  useEditorStore (design) + useCodeGraphStore (code)       │
│  elements[] + wires[] + codeTabs[] + componentRegistry    │
└──────────────────────┬───────────────────────────────────┘
                       ↓
┌──────────────────────────────────────────────────────────┐
│              omni-gen/ 代码生成管道                        │
│                                                          │
│  ① ViewTreeBuilder      扁 elements → 嵌套 view tree     │
│  ② WireAnalyzer         连线 → 数据绑定 + 依赖分析        │
│  ③ StoreExtractor       代码块 → Store 结构化定义         │
│  ④ ExpressionLowering   代码图 → .omni 表达式/语句        │
│  ⑤ StyleResolver        区分 style props / component props│
│  ⑥ ComponentAssembler   聚合 → 完整 Component 定义        │
│  ⑦ SourceEmitter        结构化文档 → .omni 源码           │
│  ⑧ TokenEmitter         DesignTokens → theme 块           │
│  ⑨ CustomComponentEmitter  自定义组件 → component 块      │
│  ⑩ ChainEmitter         代码图 → @chain/@node/@wire 注释  │
│  ⑪ AnnotationInjector   @code-tab + @edit 注入            │
│  ⑫ Pipeline             编排 ①-⑪ → 完整 .omni 源码       │
└──────────────────────┬───────────────────────────────────┘
                       ↓
          .omni 源码（含编辑器注释元数据）
```

### 2.1 文件结构

```
editor/client/src/omni-gen/
├── index.ts                  # 入口: generateOmniSource() / generateOmniFiles()
├── types.ts                  # 共享类型定义
├── view-tree-builder.ts      # ①
├── wire-analyzer.ts          # ②
├── store-extractor.ts        # ③
├── expression-lowering.ts    # ④
├── style-resolver.ts         # ⑤
├── component-assembler.ts    # ⑥
├── source-emitter.ts         # ⑦ (CodeWriter + emitDocument/emitStore/emitComponent/emitViewNode)
├── token-emitter.ts          # ⑧
├── custom-component-emitter.ts # ⑨
├── chain-emitter.ts          # ⑩ 代码图 → @chain 注释块
├── annotation-injector.ts    # ⑪ @code-tab + @edit 注入
├── pipeline.ts               # ⑫ 编排层
└── __tests__/
    ├── integration.test.ts
    ├── view-tree-builder.test.ts
    ├── wire-analyzer.test.ts
    ├── store-extractor.test.ts
    ├── expression-lowering.test.ts
    ├── style-resolver.test.ts
    ├── source-emitter.test.ts
    ├── chain-emitter.test.ts
    ├── annotation-injector.test.ts
    └── fixtures/
```

### 2.2 多文件生成

每个 code tab 产出**一个 `.omni` 文件**，按项目结构分布：

```
my-project/
├── src/
│   ├── main.omni           ← 主画布 + Main logic tab
│   ├── lib/
│   │   └── validation.omni  ← logic tab → @code-tab type=logic
│   ├── stores/
│   │   └── TodoStore.omni   ← store 定义 + action chain
│   └── components/
│       └── MyCard.omni      ← 自定义 component + @edit
```

---

## 3. 模块详细设计

### 3.1 ViewTreeBuilder — 视图树构建

**方案**: Recursive Top-down（自顶向下递归构建）

**输入**: `ElementData[]`（已过滤 nodeType === "view"）
**输出**: `ViewNode[]` 嵌套树

**核心算法**:
```
1. 构建 Map<id, ElementData> 哈希表
2. 找到 parentId === null 的根节点
3. 递归 convert(node):
   - 创建 ViewNode{ id, tag, props, kind:"element", children:[] }
   - 若 COMPONENT_PROTOCOL[tag].children !== false:
       遍历 node.children[]，递归 convert 每个子节点
   - 返回 ViewNode
```

**新增**: 每个 ViewNode 必须保留原始 element.id，供后续 `@edit` 注解和 `id="xxx"` 属性生成使用。

**复杂度**: O(n) 时间，O(n) 空间
**代码量**: ~40 行
**边界处理**:
- 自闭合标签（Image, Text, Spinner, Progress 等 children===false）不处理 children
- 保留 children 数组顺序
- 忽略 nodeType !== "view" 的元素
- Web 安全：递归深度限制 5000 层

### 3.2 WireAnalyzer — 连线分析器

**方案**: 线性端口分类器（Linear Wire Classifier）

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

**核心算法**:
```
单次遍历 wires，对每条 wire 根据 (src.tag, fromPort, tgt.tag, toPort) 分类:
  (signal, "value", view, prop)   → signalBindings[srcName] += {tgtId, prop}
  (store, "items", view, *)       → storeRefs[srcName] += tgtId
  (signal|computed|literal, "value", computed, "deps") → computedDeps[tgtName] += srcName
  (computed, "value", view, prop) → signalBindings[srcName] += {tgtId, prop}

第二阶段：检测 unwired props — 遍历 view 元素，找出既未连线又非事件的 props
```

**复杂度**: O(W + V)，~50 行

### 3.3 StoreExtractor — Store 提取器

**方案**: 两阶段声明式提取

**同前版设计，无变化。**

**复杂度**: O(N + W)，~150 行

### 3.4 ExpressionLowering — 表达式降级器

**方案**: 递归下降构建器（MVP）

**同前版设计，无变化。** 节点 → .omni 映射表不变。

**新增约束**: 降级产出的函数体需要保留 source location 信息（供 ChainEmitter 定位 `@chain` 块插入位置）。

**复杂度**: O(N + W)，~350 行

### 3.5 StyleResolver — 样式解析器

**方案**: 统一内联 Style

**同前版设计，无变化。** ~30 行。

### 3.6 ComponentAssembler — 组件组装器

**同前版设计，无变化。** ~80 行。

### 3.7 SourceEmitter — 源码生成器

**方案**: Visitor + Builder（递归下降 + CodeWriter）

**同前版核心设计**，新增以下约束：

#### 3.7.1 View 元素 `id` 属性生成

每个 view 元素必须生成 `id="xxx"` 属性（规则 I1：同文件内唯一）：

```typescript
// 使用 element.id 作为 id 属性值
// 若 element.id 包含特殊字符，sanitize 为合法标识符
function emitViewNode(node: ViewNode): void {
  const sanitizedId = sanitizeOmniId(node.id);
  // <Container id="el-abc123" padding="16px">
  writer.emit(`<${node.tag} id="${sanitizedId}"${propsStr}>`);
  ...
}
```

**ID sanitize 规则**: 非 `[a-zA-Z0-9_-]` 字符替换为 `-`，以数字开头加前缀 `el-`。

#### 3.7.2 fn 节点的 wire 连接

`fn` 节点的 wire 不写端口名（规格要求）：
```
@wire w5  n4.false -> n6       // n6 为 fn 节点，不写端口名
@wire w6  n6 -> n7.value       // fn 输出到 return，也不写端口名
```

SourceEmitter 需要标记 fn 节点，供 ChainEmitter 正确格式化 wire。

**代码量**: ~320 行

### 3.8 TokenEmitter — 设计令牌生成器

**同前版设计，无变化。** ~30 行。

### 3.9 CustomComponentEmitter — 自定义组件生成器

**方案**: 简化子管道（C2 — 仅 View + Style）

**同前版设计**，新增约束：
- 生成的组件文件第一行写入 `// @code-tab type=logic`
- 组件内部 view 元素需要 `// @edit` 注解

**代码量**: ~60 行

### 3.10 ChainEmitter — 链注释生成器（新增）

**将代码图 elements + wires 转为 `@chain` / `@node` / `@wire` 注释块。**

#### 3.10.1 @chain 块格式

```
// ═══════════════════════════════════════════════════════
// @chain <functionName>
//   @node <id> <tag> <attrs...>  pos=<x>,<y>
//   @wire <id> <from_id>.<from_port> -> <to_id>.<to_port> [collapsed] [group=<id>]
// ═══════════════════════════════════════════════════════
```

#### 3.10.2 @node 属性速查

**变量类:**

| tag | 必填属性 | 可选属性 |
|-----|---------|---------|
| `signal` | name, type | init |
| `store` | name | — |
| `computed` | name, deps | — |

**数据访问类:**

| tag | 必填属性 |
|-----|---------|
| `getVariable` | name |
| `setVariable` | name |

**算术类:**

| tag | 属性 |
|-----|------|
| `add` | — |
| `sub` | — |
| `mul` | — |
| `div` | — |

**逻辑类:**

| tag | 必填属性 |
|-----|---------|
| `compare` | op（==, !=, >, <, >=, <=） |
| `and` | — |
| `or` | — |
| `not` | — |
| `concat` | — |

**字面量:**

| tag | 必填属性 |
|-----|---------|
| `literal` | value, type |

**流程控制:**

| tag | 必填属性 |
|-----|---------|
| `branch` | — |
| `sequence` | outputs |
| `forEach` | — |

**UI 操作:**

| tag | 必填属性 |
|-----|---------|
| `updateUIProperty` | target, property |

**函数/代码:**

| tag | 必填属性 | 可选属性 |
|-----|---------|---------|
| `fn` | source | — |
| `customFn` | name, params, return | desc |
| `return` | — | value |

**通用规则:**
- 必填属性必须齐全（规则 C1）
- `pos=x,y` 必须存在，x/y 为整数（规则 C2）
- tag 只允许上表中列出的值（规则 S4）
- 属性值含空格时必须用引号包裹（规则 F3）

#### 3.10.3 @wire 格式

```
@wire <id> <from_id>.<from_port> -> <to_id>.<to_port> [collapsed] [group=<id>]
```

**特殊规则:**
- `fn` 节点的 wire 不写端口名：`@wire w5  n4.false -> n6`（n6 是 fn 节点）
- `branch` 有 `true` / `false` 输出端口 + `condition` 输入端口
- 箭头必须是 `->`（规则 F2）
- `from_id` / `to_id` 必须在同一 `@chain` 的 `@node` 中存在（规则 S1）
- 端口名必须是该节点的合法端口（规则 S2）

#### 3.10.4 核心算法

```
function emitChain(tab: CodeTab, loweredFunctions: LoweredFunction[]): string {
  // 每个 code tab 对应一个文件，其 elements+wires 生成该文件的 @chain 块
  // 但 main tab 的 customFn 可能散布在设计画布中
  // 查找规则: 每个 code tab 的 graph 生成一个或多个 @chain 块

  const chains: ChainBlock[] = [];

  // 按 customFn 分组
  for (const fn of loweredFunctions) {
    const fnNode = tab.graph.elements.find(e => e.tag === "customFn" && e.config?.fnName === fn.name);
    if (!fnNode) continue;

    // BFS 从 customFn 出发，收集所有可达节点
    const reachableNodes = bfsCollect(fnNode.id, tab.graph);
    // 拓扑排序
    const sortedNodes = topologicalSort(reachableNodes, tab.graph.wires);
    // @node 全部在 @wire 之前（规则 P6）
    const nodeLines = sortedNodes.map(n => formatNode(n));
    const wireLines = tab.graph.wires
      .filter(w => reachableNodes.has(w.fromId) && reachableNodes.has(w.toId))
      .map(w => formatWire(w, tab.graph.elements));

    chains.push({
      functionName: fn.name,
      nodeLines,
      wireLines,
    });
  }

  return formatChainBlocks(chains);
}
```

**formatNode 核心逻辑**:
```
根据 node.tag 查找必填属性表：
  signal → name=<props.name>, type=<props.typeAnnotation>
           可选: init=<默认值>
  literal → value=<config.value>, type=<config.type>
  compare → op=<config.op>
  getVariable → name=<config.variableName>
  setVariable → name=<config.targetSignal>
  updateUIProperty → target=<config.targetId>, property=<config.property>
  customFn → name=<config.fnName>, params=<...>, return=<config.fnReturnType>
  fn → source=<element.source 或 "">
  branch/forEach/add/sub/... → 无必填属性

pos=<element.position.x>,<element.position.y>  ← 从 element.position 获取
```

**formatWire 核心逻辑**:
```
若 targetElement.tag === "fn":
  → "@wire <id> <from>.<port> -> <to>"     (不写端口名)
若 sourceElement.tag === "fn":
  → "@wire <id> <from> -> <to>.<port>"     (不写端口名)
否则:
  → "@wire <id> <from>.<port> -> <to>.<port>"

[collapsed] 和 [group=<id>] 来自 wireData
```

**代码量**: ~200 行

### 3.11 AnnotationInjector — 注解注入器（重写）

**从"最小标记"升级为"完整注释注入"。**

#### 3.11.1 文件头：@code-tab 声明

```typescript
function injectCodeTab(source: string, tabType: "logic" | "event"): string {
  const directive = `// @code-tab type=${tabType}`;
  // 必须是文件第一行（规则 P1）
  // 若 source 已以 @code-tab 开头，替换
  if (source.startsWith("// @code-tab")) {
    return source.replace(/^\/\/ @code-tab .*/, directive);
  }
  return `${directive}\n${source}`;
}
```

#### 3.11.2 View 元素：@edit 注解

对 SourceEmitter 生成的 view 部分，每个 `<` 元素前插入 `// @edit` 行：

```typescript
function injectEditAnnotations(viewSource: string, viewNodes: ViewNode[]): string {
  // 遍历 view tree，在每个 <element> 行前插入 // @edit 注释
  // 规则 P2: // @edit 下一行必须是 < 开头的元素，中间不许有空行
  // 规则 P3: // @edit 和它的 < 元素的缩进必须一致
  // 规则 C4: View block 内每个 < 元素必须有唯一对应的 // @edit

  const lines = viewSource.split("\n");
  const result: string[] = [];

  for (const line of lines) {
    const trimmed = line.trimStart();
    if (trimmed.startsWith("<") && !trimmed.startsWith("</")) {
      const indent = line.length - trimmed.length;
      const indentStr = " ".repeat(indent);

      // 查找对应 ViewNode 获取折叠/渲染模式信息
      const nodeId = extractIdFromLine(trimmed);
      const node = findNodeById(nodeId, viewNodes);
      const flags = buildEditFlags(node);  // "folded" / "render=canvas" / ""

      result.push(`${indentStr}// @edit${flags ? " " + flags : ""}`);
    }
    result.push(line);
  }

  return result.join("\n");
}

function buildEditFlags(node?: ViewNode): string {
  const flags: string[] = [];
  if (node?.isFolded) flags.push("folded");
  if (node?.renderMode === "canvas") flags.push("render=canvas");
  return flags.join(" ");
}
```

**@edit 标志规则** (F1): 只允许 `folded`、`render=dom`、`render=canvas`。

#### 3.11.3 组合注入

```
// 最终产出顺序：
//   1. @code-tab 声明（文件头）
//   2. import 语句
//   3. store 定义
//   4. component 定义（内含 @edit 注解的 view tree + @chain 块）
//   5. theme 块
```

**代码量**: ~150 行

### 3.12 Pipeline — 编排层

**入口函数**:
```typescript
// 单文件生成
function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  activeCodeTab: CodeTab;          // 当前文件的 code tab
  componentRegistry: Record<string, ComponentDefinition>;
  designTokens: DesignTokens;
}): string;

// 多文件生成（保存整个项目）
function generateOmniFiles(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentRegistry: Record<string, ComponentDefinition>;
  designTokens: DesignTokens;
}): Record<string, string>;  // filePath → .omni source
```

**单文件生成执行顺序**:
```
1. ViewTreeBuilder: designElements → viewTree
2. WireAnalyzer: designElements + designWires → bindingResult
3. StoreExtractor: codeBlocks + codeWires → stores + standaloneSignals
4. ExpressionLowering: codeBlocks + codeWires → functions + actions + getters
5. StyleResolver: 各 view element → resolvedStyles
6. ComponentAssembler: 以上结果 → ComponentDefinition
7. SourceEmitter: ComponentDefinition → .omni 源码（不含注释）
8. TokenEmitter: DesignTokens → theme 块
9. CustomComponentEmitter: 自定义组件 → component 块
10. ChainEmitter: activeCodeTab.graph → @chain 块（插入函数前）
11. AnnotationInjector.injectCodeTab: → 文件头 @code-tab 声明
12. AnnotationInjector.injectEditAnnotations: → view 元素 @edit 注解
13. 返回完整 .omni 源码
```

**多文件生成逻辑**:
```
每个 code tab → 一个文件:
  - "main" tab       → src/main.omni (包含主画布 view tree)
  - "event" tabs     → src/main.omni (内嵌，或独立文件)
  - "logic" tabs     → src/lib/<tabLabel>.omni
  - custom component → src/components/<Name>.omni
```

**代码量**: ~120 行

---

## 4. 类型系统

### 4.1 核心输出类型

```typescript
interface OmniDocument {
  codeTabType: "logic" | "event";
  imports: string[];
  stores: StoreDefinition[];
  components: ComponentDefinition[];
  themes: ThemeDefinition[];
}

interface StoreDefinition {
  name: string;
  signals: Record<string, { type: string; default?: unknown }>;
  getters: Record<string, { returnType: string; body: string }>;
  actions: Array<{ name: string; params: Array<{name:string; type:string}>; body: string }>;
}

interface ComponentDefinition {
  name: string;
  props: Record<string, { type: string; default?: unknown }>;
  signals: Record<string, { type: string; default?: unknown }>;
  computeds: Record<string, { type: string; deps?: string[] }>;
  events: string[];
  storeRefs: string[];
  fns: Array<{ name: string; params: Array<{name:string; type:string}>; returnType: string; body: string }>;
  view: ViewNode | null;
  style: Record<string, Record<string, string>>;
  lifecycles: Record<string, string[]>;
}

interface ViewNode {
  id: string;           // 原始 element.id，用于 id="xxx" 和 @edit 映射
  tag: string;
  props: Record<string, unknown>;
  children: ViewNode[];
  kind: "element" | "text" | "interpolation" | "if" | "for" | "slot";
  isFolded?: boolean;
  renderMode?: "dom" | "canvas";
  // @if
  condition?: string;
  branches?: { condition: string; body: ViewNode[] }[];
  fallback?: ViewNode[] | null;
  // @for
  iterable?: string;
  itemBinding?: string;
  keyExpr?: string;
  emptyBody?: ViewNode[] | null;
  // text/interpolation
  text?: string;
  expression?: string;
  // slot
  slotName?: string;
}

interface ThemeDefinition {
  name: string;
  categories: Record<string, Record<string, string>>;
}

type PropValue =
  | { kind: "string"; value: string }
  | { kind: "number"; value: number }
  | { kind: "boolean"; value: boolean }
  | { kind: "expr"; code: string }
  | { kind: "bind"; signalPath: string };
```

### 4.2 中间类型

```typescript
// WireAnalyzer 产出
interface BindingResult {
  signalBindings: Map<string, Array<{ elementId: string; propName: string }>>;
  storeRefs: Map<string, string[]>;
  computedDeps: Map<string, string[]>;
  unwiredProps: Array<{ elementId: string; tag: string; propName: string }>;
}

// StyleResolver 产出
interface ResolvedStyle {
  componentProps: Record<string, PropValue>;
  inlineStyles: Record<string, string>;
  classNames: string[];
  bindings: Record<string, string>;
}

// ExpressionLowering 产出
interface LoweredCode {
  expressions: Map<string, string>;
  functions: Array<{
    name: string;
    params: Array<{ name: string; type: string }>;
    returnType: string;
    body: string;
  }>;
  actions: Array<{
    storeName: string;
    name: string;
    params: Array<{ name: string; type: string }>;
    body: string;
  }>;
  getters: Array<{
    name: string;
    returnType: string;
    body: string;
    deps: string[];
  }>;
}

// ChainEmitter 产出
interface ChainBlock {
  functionName: string;
  nodeLines: string[];    // "//   @node n1  signal  name=counter  pos=50,50"
  wireLines: string[];    // "//   @wire w1  n1.value -> n2.a"
}
```

---

## 5. 代码量估算

| 模块 | 估算行数 | 变化 |
|------|---------|------|
| types.ts | 150 | +30 |
| view-tree-builder.ts | 40 | — |
| wire-analyzer.ts | 50 | — |
| store-extractor.ts | 150 | — |
| expression-lowering.ts | 350 | — |
| style-resolver.ts | 30 | — |
| component-assembler.ts | 80 | — |
| source-emitter.ts | 320 | +20 (id 生成) |
| token-emitter.ts | 30 | — |
| custom-component-emitter.ts | 60 | +10 |
| chain-emitter.ts | 200 | **新增** |
| annotation-injector.ts | 150 | +140 (从最小标记升级) |
| pipeline.ts (index.ts) | 120 | +40 (多文件支持) |
| **总计** | **~1730** | |

---

## 6. 与现有系统的集成

### 6.1 `.omni` 为唯一真相源

`.omni` 源码文件包含完整信息：代码 + 编辑器注释元数据。不再需要 `.omni.json` 作为主要存储格式。

```
.omni 文件内容 = OmniLang 合法代码 + 编辑器注释
  ├── // @code-tab type=logic          ← 编辑器文件类型
  ├── import omniview;                 ← 常规代码
  ├── store TodoStore { ... }          ← 常规代码
  ├── component App {
  │     // @edit                       ← 编辑器元素标记
  │     <Container id="el-root">
  │       // @edit folded              ← 编辑器折叠状态
  │       <Button id="el-btn" onPress={handleClick}>
  │     </Container>
  │
  │     // ════════════════════        ← 编辑器链标记
  │     // @chain handleClick
  │     //   @node n1  ...
  │     //   @wire w1  ...
  │     // ════════════════════
  │     fn handleClick() -> void { ... }
  │   }
  └── theme myTheme { ... }
```

### 6.2 保存时触发（saveProject）

```typescript
const saveProject = useCallback(async (customPath?: string) => {
  // 1. 生成所有 .omni 文件
  const files = generateOmniFiles({
    designElements: store.elements,
    designWires: store.wires,
    codeTabs: useCodeGraphStore.getState().toSerializable().tabs,
    componentRegistry: useComponentRegistry.getState().definitions,
    designTokens: useTokenStore.getState().tokens,
  });

  // 2. 写入每个文件
  for (const [filePath, source] of Object.entries(files)) {
    await api.writeFile(filePath, source);
  }

  // 3. 更新编辑器 UI 状态文件
  await api.saveEditorToml({
    active_file: store.activeFilePath || "src/main.omni",
    active_mode: store.activeMode,
    open_files: Object.keys(files),
    left_open: true,
    right_width: 320,
    viewports: { /* ... */ },
  });

  // 4. 更新编辑器内 activeFileContent
  const mainSource = files["src/main.omni"] || "";
  store.setSourceCode(mainSource);

  store.markClean();
});
```

### 6.3 编译时触发（handleCompile）

```typescript
const handleCompile = useCallback(async () => {
  // 编译前先同步：生成当前文件源码
  const source = generateOmniSource({
    designElements: store.elements,
    designWires: store.wires,
    activeCodeTab: currentCodeTab,
    componentRegistry: useComponentRegistry.getState().definitions,
    designTokens: useTokenStore.getState().tokens,
  });
  store.setSourceCode(source);
  const result = await api.compile(source, target);
  // ...
});
```

### 6.4 与旧 `.omni.json` 的共存

- **过渡期**: `.omni.json` 继续保存（编辑器快速加载缓存），但 `.omni` 是权威源
- **打开项目**: 优先读 `.omni` 源码 → 解析画布；若不存在则 fallback 到 `.omni.json`
- **迁移**: 使用 `omnic migrate` CLI 命令一次性迁移旧项目

---

## 7. 测试策略

### 7.1 单元测试

每个模块纯函数，输入画布状态 → 断言输出。

```typescript
// chain-emitter.test.ts
test("customFn → @chain block with @node and @wire", () => {
  const tab: CodeTab = {
    id: "tab-1", type: "event", label: "handleClick", icon: "⚡",
    graph: {
      elements: [
        { id: "n1", tag: "customFn", config: { fnName: "handleClick", fnReturnType: "void" },
          position: { x: 50, y: 50 }, /* ... */ },
        { id: "n2", tag: "literal", config: { value: "0", type: "i32" },
          position: { x: 50, y: 150 }, /* ... */ },
      ],
      wires: [
        { id: "w1", fromId: "n2", fromPort: "value", toId: "n1", toPort: "param" },
      ],
    },
    variables: [],
  };

  const result = emitChain(tab, [{ name: "handleClick", params: [], returnType: "void", body: "..." }]);

  expect(result).toContain("// @chain handleClick");
  expect(result).toContain("@node n1  customFn  name=handleClick  return=void  pos=50,50");
  expect(result).toContain("@node n2  literal  value=0  type=i32  pos=50,150");
  expect(result).toContain("@wire w1  n2.value -> n1.param");
});
```

### 7.2 集成测试

使用现有 E2E 项目的画布 snapshot，走完整管道，验证：
1. 输出 .omni 源码符合所有解析规则（P1-P6, I1-I4, C1-C4, S1-S5, F1-F3）
2. 编译器可成功解析输出 .omni 源码
3. 解析后的画布状态与输入画布状态等价（round-trip）

### 7.3 回归测试

用 learn-omniview 等现有项目作为 golden test。

---

## 8. 风险与缓解

| 风险 | 缓解 |
|------|------|
| ExpressionLowering 复杂度过高 | MVP 先处理核心 80% 场景（linear data flow），branch/forEach 作为 Phase 2 |
| ChainEmitter @node/@wire 格式与解析器不一致 | 共享测试 fixture，两端用同一套 node tag 属性表 |
| 生成代码格式不完美 | round-trip 测试验证正确性；格式细节由编译器承担 |
| 大型项目性能 | 生成在保存/编译时触发（非实时），1-2 秒可接受；后续可加 Web Worker |
| 与手写代码冲突 | `@code-tab` / `@edit` / `@chain` 标记明确分界线；无标记文件不覆盖 |
| 多文件项目保存一致性 | `generateOmniFiles()` 原子化写入，或先写临时文件再 rename |

---

## 9. 实施阶段

### Phase 1: 核心管道（无注释，验证 round-trip）
1. types.ts
2. view-tree-builder.ts
3. wire-analyzer.ts
4. style-resolver.ts
5. source-emitter.ts（含 id 生成）
6. pipeline.ts — 单文件生成
7. **目标**: 能生成合法 .omni 源码，编译器解析无错误

### Phase 2: 编辑器注释
8. chain-emitter.ts — @chain/@node/@wire 块
9. annotation-injector.ts — @code-tab + @edit 注入
10. **目标**: 生成含完整编辑器元数据的 .omni 源码

### Phase 3: 逻辑层
11. store-extractor.ts
12. expression-lowering.ts
13. component-assembler.ts
14. **目标**: component + store + fn 定义完整生成

### Phase 4: 扩展 + 集成
15. token-emitter.ts
16. custom-component-emitter.ts
17. pipeline.ts — 多文件生成
18. 集成到 App.tsx（saveProject / handleCompile）
19. **目标**: 完整双向同步上线
