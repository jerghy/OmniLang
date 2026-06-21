# OmniGen — 编辑器前端 .omni 代码生成器设计

> 2026-06-21 | 状态: 设计完成 | 下一步: writing-plans

## 1. 背景与目标

### 1.1 当前状态

OmniLang 编辑器维护两套数据：
- **画布状态**（`ElementData[] + WireData[] + CodeTab[]`）— 用户可视化操作的结果
- **`.omni` 源码**（`activeFileContent`）— Monaco 编辑器中的文本

当前数据流是**单向**的：`.omni 源码 → 解析 → 画布状态`（exporter.py），反向不存在。

### 1.2 目标

实现**双向同步**：画布状态 ↔ `.omni` 源码，采用"编辑时标记 dirty，保存时同步，编译/预览时也触发同步"的混合模式。

### 1.3 方案选择

纯前端 TypeScript 管道（方案 B），10 个独立模块，纯函数 + 值传递，零副作用。

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
│  ⑦ SourceEmitter       结构化文档 → .omni 源码字符串      │
│  ⑧ TokenEmitter         DesignTokens → theme 块           │
│  ⑨ CustomComponentEmitter  自定义组件 → component 块     │
│  ⑩ AnnotationInjector  注入 @editor:managed 标记         │
│  ⑪ Pipeline            编排 ①-⑩ → 完整 .omni 源码        │
└──────────────────────┬───────────────────────────────────┘
                       ↓
                 .omni 源码字符串
```

### 2.1 文件结构

```
editor/client/src/omni-gen/
├── index.ts                  # 入口: generateOmniSource(state) → string
├── types.ts                  # 共享类型定义
├── view-tree-builder.ts      # ①
├── wire-analyzer.ts          # ②
├── store-extractor.ts        # ③
├── expression-lowering.ts    # ④
├── style-resolver.ts         # ⑤
├── component-assembler.ts    # ⑥
├── source-emitter.ts         # ⑦ SourceEmitter (CodeWriter + emitDocument/emitStore/emitComponent/emitViewNode)
├── token-emitter.ts          # ⑧ (独立文件，被 SourceEmitter 调用)
├── custom-component-emitter.ts # ⑨
├── annotation-injector.ts    # ⑩
├── pipeline.ts               # ⑩ 编排层
└── __tests__/
    ├── integration.test.ts
    ├── view-tree-builder.test.ts
    ├── wire-analyzer.test.ts
    ├── store-extractor.test.ts
    ├── expression-lowering.test.ts
    ├── style-resolver.test.ts
    ├── source-emitter.test.ts
    └── fixtures/             # 测试用画布 snapshot
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

**复杂度**: O(n) 时间，O(n) 空间
**代码量**: ~40 行
**边界处理**:
- 自闭合标签（Image, Text, Spinner, Progress 等 children===false）不处理 children
- 保留 children 数组顺序
- 忽略 nodeType !== "view" 的元素
- Web 安全：递归深度限制 5000 层（远低于浏览器栈限制 ~10k）

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

第二阶段：检测 unwired props — 遍历 view 元素，找出未连线且非事件的 props
```

**复杂度**: O(W + V) 时间（W=wires, V=view elements），O(1) 额外空间
**代码量**: ~50 行
**关键决策**:
- 不处理多跳（signal → computed → view），该能力由 ExpressionLowering 负责
- 事件绑定从 `element.config._eventBindings` 获取，不分析 wires

### 3.3 StoreExtractor — Store 提取器

**方案**: 两阶段声明式提取（Two-Phase Declarative）

**Phase 1 — Store Registry 构建**:
```
遍历 elements:
  tag==="store"        → 创建 StoreDefinition skeleton
  props.parentStore≠空 → 按 name 匹配 store，归入该组
  parentId 链有 store  → 归入祖先 store

遍历 wires，构建邻接表供 Phase 2
```

**Phase 2 — 内容填充**:
```
对每个 store 组:
  signal    → type 从 props.typeAnnotation 或端口类型推导
  computed  → returnType 从输出端口推导; body 委托 ExpressionLowering
  action    → 参数从入站 wire 推导; body 委托 ExpressionLowering
  
无归属的 signal/computed → StandaloneSignal[]
```

**复杂度**: O(N + W)，~150 行
**类型推导优先级**:
1. `props.typeAnnotation` 直接使用
2. 端口上的类型标注
3. 默认 "unknown"

### 3.4 ExpressionLowering — 表达式降级器

**方案**: 递归下降构建器（Recursive Descent Builder）— 短期 MVP

**核心思想**: 以**消费端**为入口，沿 wire 反向递归构建表达式。

```
从目标端口（setVariable.value, return.value, branch.condition 等）出发：
  locateWire(targetId, targetPort) → fromId
  对 fromId 节点，递归构建各输入端口的表达式
  终端节点（signal/literal/getVariable）→ 直接返回变量名/字面量
  拼接为最终表达式字符串

共享子表达式：Map<nodeId, string> 缓存，fan-out 时复用
循环检测：Set<nodeId> 递归路径，发现重复则报错
```

**节点 → .omni 映射**:
| 代码块 | 输出 |
|--------|------|
| literal(value="42", type="i32") | `42` |
| literal(value="hello", type="String") | `"hello"` |
| signal(name="counter") | `counter` |
| getVariable(name="x") | `x` |
| add(a, b) | `a + b` |
| compare(a, b, op=">=") | `a >= b` |
| and(a, b) / or(a, b) / not(a) | `a && b` / `a \|\| b` / `!a` |
| computed | (引用名) |
| store items | (引用名) |

**函数体构建**:
```
从 customFn 出发，BFS 遍历可达节点：
  return ← valueExpr  → "return <expr>;"
  setVariable ← expr  → "signalName = <expr>;"
  branch(condition) → "if (<cond>) { <trueBody> } else { <falseBody> }"
  forEach(collection) → "@for item in <coll> { <body> }"

按拓扑序排为线性语句序列
```

**悬挂节点处理**: 无下游消费者的纯计算节点 → 忽略（自然处理）
**fn 节点 source**: 若有 `element.source`（Monaco 手写代码）→ 直接使用，不进行图分析

**复杂度**: O(N + W)，~350 行
**代码量**: ~350 行

**进阶路线**:
- MVP: 方案 A（递归下降）
- 正式版: 轻量拓扑排序预处理（循环检测 + 悬挂 warning）
- 长期: 区域分解编译器（Region-Decomposition）— 支持深层嵌套 branch/forEach

### 3.5 StyleResolver — 样式解析器

**方案**: 统一内联 Style（Simplest）

**输入**: `tag: string, props: Record<string, unknown>`
**输出**: `ResolvedStyle`

```typescript
interface ResolvedStyle {
  componentProps: Record<string, unknown>;
  inlineStyles: Record<string, string>;
  classNames: string[];
  bindings: Record<string, string>;
}

interface PropValue {
  kind: "string" | "number" | "boolean" | "expr" | "bind";
  value: string;
}
```

**核心逻辑**:
```
根据 COMPONENT_PROTOCOL[tag].styleProperties 分离 style vs component props
根据 COMPONENT_PROTOCOL[tag].props 获取 prop 元数据
所有 style 属性 → 内联 style={{ ... }}
所有 component 属性 → props
```

**代码量**: ~30 行纯函数

### 3.6 ComponentAssembler — 组件组装器

**角色**: 将 ViewTreeBuilder + WireAnalyzer + StoreExtractor + ExpressionLowering 的结果聚合成 `ComponentDefinition`。

```typescript
interface ComponentDefinition {
  name: string;
  props: Record<string, PropSchema>;
  signals: Record<string, { type: string; default?: unknown }>;
  computeds: Record<string, { type: string; deps?: string[] }>;
  events: string[];
  storeRefs: string[];
  fns: Array<{ name: string; params: Array<{name:string; type:string}>; returnType: string; body: string }>;
  view: ViewNode | null;
  style: Record<string, Record<string, string>>;
  lifecycles: Record<string, string[]>;
}
```

**代码量**: ~80 行编排逻辑

### 3.7 SourceEmitter — 源码生成器

**方案**: Visitor + Builder（递归下降 + CodeWriter）

**核心组件**:
```typescript
class CodeWriter {
  private lines: string[] = [];
  private _indent = 0;
  
  emit(line: string): void;     // 在当前缩进追加行
  indent(): void;               // 缩进+1
  dedent(): void;               // 缩进-1
  blank(): void;                // 空行
  toString(): string;           // join("\n") + 末尾换行
}

class SourceEmitter {
  constructor(private styleMode: "inline" | "auto" = "inline");
  
  emitDocument(doc: OmniDocument): string;
  emitImport(imp: string): void;
  emitStore(s: StoreDefinition): void;
  emitComponent(c: ComponentDefinition): void;
  emitViewNode(node: ViewNode): void;
  emitTheme(t: ThemeDefinition): void;
}
```

**Props 格式化**:
```
PropValue.kind === "string"   → prop="value"     (转义引号)
PropValue.kind === "number"   → prop={42}
PropValue.kind === "boolean"  → prop={true}
PropValue.kind === "expr"     → prop={expression}
PropValue.kind === "bind"     → bind:prop={signal}
```

**Style 处理**: 默认内联 `style={{ ... }}`，保留 `styleMode: "auto"` 扩展点（>3 props 切 class）

**自闭合**: `children.length === 0 && !textContent` → `<Tag />`

**缩进**: 2 空格，CodeWriter 统一管理

**转义**:
- 属性值中: `"` → `\"`
- 文本内容中: `<` → `&lt;`, `{` → `&#123;`
- 插值表达式: 不转义

**代码量**: ~300 行

### 3.8 TokenEmitter — 设计令牌生成器

**方案**: 始终完整生成

**输入**: `DesignTokens`（来自 `useTokenStore`）
**输出**: `.omni theme { }` 块字符串

**输出格式**:
```omni
theme myTheme {
  colors {
    primary: "#4F46E5";
    secondary: "#7C3AED";
    ...
  }
  spacing {
    md: "16px";
    ...
  }
}
```

**代码量**: ~30 行

### 3.9 CustomComponentEmitter — 自定义组件生成器

**方案**: 简化子管道（C2 — 仅 View + Style）

**逻辑**:
```
对 componentRegistry.definitions 中 source==="embedded" 的组件:
  复用 ViewTreeBuilder.build(component.elements)
  复用 SourceEmitter 生成 view { ... } 块
  不支持内部逻辑（wires 不处理）
  
  source !== "embedded" → import 语句，不内联
```

**代码量**: ~50 行
**扩展点**: `ComponentEmitter` 接口，未来可切换为全管道实现（C1）

### 3.10 AnnotationInjector — 注解注入器

**方案**: 最小标记 — 仅文件头

**逻辑**:
```
在生成的 .omni 源码前插入 "// @editor:managed\n"
如果源码已以此标记开头，不重复插入
```

**反向同步**: 文件以该标记开头 → 全量覆盖模式（编辑器管理）
**代码量**: ~10 行

### 3.11 Pipeline — 编排层

**入口函数**:
```typescript
function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentRegistry: Record<string, ComponentDefinition>;
  designTokens: DesignTokens;
  activeFilePath: string;
}): string;
```

**执行顺序**:
```
1. ViewTreeBuilder: designElements → viewTree
2. WireAnalyzer: designElements + designWires → bindingResult
3. StoreExtractor: codeBlocks + codeWires → stores + standaloneSignals
4. ExpressionLowering: codeBlocks + codeWires → functions + actions + getters
5. StyleResolver: 各 view element → resolvedStyles
6. ComponentAssembler: 以上结果 → ComponentDefinition
7. SourceEmitter.emitDocument({
     imports, stores, components, themes
   }) → .omni 源码字符串
8. TokenEmitter → theme 块（如有自定义令牌）
9. CustomComponentEmitter → 自定义组件块
10. AnnotationInjector → 注入标记
11. 返回完整 .omni 源码
```

**代码量**: ~80 行

---

## 4. 类型系统

### 4.1 核心输出类型

```typescript
// 完整文档
interface OmniDocument {
  imports: string[];
  stores: StoreDefinition[];
  components: ComponentDefinition[];
  themes: ThemeDefinition[];
}

// Store
interface StoreDefinition {
  name: string;
  signals: Record<string, { type: string; default?: unknown }>;
  getters: Record<string, { returnType: string; body: string }>;
  actions: Array<{ name: string; params: Array<{name:string; type:string}>; body: string }>;
}

// Component
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

// View Node（递归）
interface ViewNode {
  id: string;
  tag: string;
  props: Record<string, unknown>;
  children: ViewNode[];
  kind: "element" | "text" | "interpolation" | "if" | "for" | "slot";
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

// Theme
interface ThemeDefinition {
  name: string;
  categories: Record<string, Record<string, string>>;
}

// Prop 值（类型区分联合）
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
```

---

## 5. 代码量估算

| 模块 | 估算行数 |
|------|---------|
| types.ts | 120 |
| view-tree-builder.ts | 40 |
| wire-analyzer.ts | 50 |
| store-extractor.ts | 150 |
| expression-lowering.ts | 350 |
| style-resolver.ts | 30 |
| component-assembler.ts | 80 |
| source-emitter.ts | 300 |
| token-emitter.ts | 30 |
| custom-component-emitter.ts | 50 |
| annotation-injector.ts | 10 |
| pipeline.ts (index.ts) | 80 |
| **总计** | **~1300** |

---

## 6. 与现有系统的集成

### 6.1 保存时触发（saveProject）

在 `App.tsx` 的 `saveProject()` 中：
```typescript
// 现有: 保存 .omni.json
await api.saveOmniJson(projectPath, doc);

// 新增: 生成 .omni 源码并写入
const omniSource = generateOmniSource({
  designElements: store.elements,
  designWires: store.wires,
  codeTabs: useCodeGraphStore.getState().toSerializable(),
  componentRegistry: useComponentRegistry.getState().definitions,
  designTokens: useTokenStore.getState().tokens,
  activeFilePath: store.activeFilePath ?? "App.omni",
});
await api.writeFile(store.activeFilePath ?? "App.omni", omniSource);
store.setSourceCode(omniSource);
```

### 6.2 编译时触发（handleCompile）

在 `handleCompile()` 中，编译前先同步源码：
```typescript
const omniSource = generateOmniSource({ ... });
store.setSourceCode(omniSource);  // 更新 activeFileContent
const result = await api.compile(omniSource, target);
```

### 6.3 不与现有系统冲突

- `.omni.json` 保存逻辑**不变**（编辑器快速加载用）
- `.omni` 源码文件是**附加产出**
- Monaco 编辑器的 `activeFileContent` 在生成后更新
- importer.py（`.omni.json → .omni 源码`）继续可用，作为服务端替代方案

---

## 7. 测试策略

### 7.1 单元测试

每个模块纯函数，输入画布状态 snapshot → 断言输出字符串或结构化数据。

```typescript
// 示例: view-tree-builder.test.ts
test("flat elements → nested view tree", () => {
  const elements: ElementData[] = [
    { id: "root", nodeType: "view", tag: "Container", props: {}, children: ["child1"], parentId: null },
    { id: "child1", nodeType: "view", tag: "Button", props: { label: "Click" }, children: [], parentId: "root" },
  ];
  const tree = buildViewTree(elements);
  expect(tree).toHaveLength(1);
  expect(tree[0].children).toHaveLength(1);
  expect(tree[0].children[0].tag).toBe("Button");
});
```

### 7.2 集成测试

使用现有 E2E 项目的画布 snapshot，走完整管道，验证输出 .omni 源码可被编译器成功解析。

### 7.3 回归测试

用 learn-omniview 等现有项目作为 golden test：
1. 将项目 `.omni` 源码编译 → 画布状态
2. 画布状态 → generateOmniSource → 新 .omni 源码
3. 新源码编译 → 应无错误
4. 新源码 vs 原源码 → diff review（人工确认）

---

## 8. 风险与缓解

| 风险 | 缓解 |
|------|------|
| ExpressionLowering 复杂度过高 | MVP 先处理核心 80% 场景（linear data flow），branch/forEach 作为 Phase 2 |
| 生成代码格式不完美 | 通过 importer.py 的 round-trip 测试验证正确性；格式化由编译器承担 |
| 大型项目性能 | 生成在保存/编译时触发（非实时），1-2 秒可接受；后续可加 Web Worker |
| 与手写代码冲突 | `@editor:managed` 标记明确分界线；用户手写文件不会被覆盖 |

---

## 9. 实施阶段

### Phase 1: 核心管道（MVP）
1. types.ts — 类型定义
2. view-tree-builder.ts
3. wire-analyzer.ts
4. style-resolver.ts
5. source-emitter.ts (+ token-emitter 内嵌)
6. pipeline.ts — 编排以上模块
7. annotation-injector.ts
8. **目标**: 能生成包含 view tree 和基本 props 的 .omni 源码

### Phase 2: 逻辑层
9. store-extractor.ts
10. expression-lowering.ts
11. component-assembler.ts
12. **目标**: 能生成完整的 component + store 定义

### Phase 3: 扩展
13. custom-component-emitter.ts
14. @if/@for 指令支持
15. 集成到 App.tsx
16. **目标**: 完整双向同步上线
