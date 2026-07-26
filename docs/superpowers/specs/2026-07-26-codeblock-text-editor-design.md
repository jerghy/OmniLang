# CodeBlock 文本编辑器 + 数据依赖打破黑盒

> 版本：draft | 2026-07-26
> 前置文档：[Block Editor v4.1 SVG Design](./2026-07-26-block-editor-svg-design.md)
> 关联记忆：[[poiesis-block-editor-v4]] [[poiesis-structural-projection-editor]]

## 1. 动机

Block Editor v4.1（Scratch 式 SVG 积木编辑器）经过两轮迭代暴露出根本性问题：

- **积木编辑器为初学者设计，不适合专业开发者**——打字永远比导航+选择快
- **积木太复杂，效果不好**——C 形臂、拼图连接器、3D 阴影等视觉元素经过多次调整仍不理想
- **不适合写大段代码**——每个语句都要通过 FuzzySearch 选择类型再填 slot，效率极低
- **"代码黑盒"未解决**——编辑器仍然不知道 code-block 里的代码读了哪些变量、写了哪些 Signal、调了哪些 Store

### 核心洞察

Poiesis 的 CodeFlow 是高层控制流图（fn → code-block → return），code-block 是有边界的抽象单元——**不应被拆开**。CodeFlow 的设计哲学是"让代码结构变清晰"，不是"把所有细节展开"。

真正需要的是：**code-block 里的代码不再是不透明文本**，编辑器通过解析它来提取数据依赖信息，喂给 ArchitecturePage 泡泡图，形成完整的数据流可视化。

### 决策

- 移除 Block Editor（Blockly → 纯 SVG 两代方案），用 Monaco 文本编辑回归本质
- 利用 WASM semantic 分析（已有 SymbolTable + dependencies）提取数据依赖
- 渐进增强：先跑通文本编辑 + 依赖追踪，再叠加结构投影（方案 3，长期愿景）

## 2. 架构

### 2.1 数据流

```
FlowCanvas 双击 code-block 节点
    │
    ▼
CodeBlockEditor (新包装组件，~300行)
    │
    ├── Monaco Editor (文本编辑 —— 复用 CodeEditorModal 核心)
    │   ├── OmniLang 语法高亮 (已有)
    │   ├── WASM diagnostics → Monaco markers (已有, debounce 500ms)
    │   └── OmniLang 代码片段 + 类型注入 (已有)
    │
    ├── 增量解析管线 (debounce 300ms)
    │   ├── 1. pipeline.ts → 生成当前组件完整 .omni 源码
    │   │      (含 signal/store_ref/fn params/parent scope 上下文)
    │   ├── 2. WASM OmniCompiler.upsert_source() → compile_to_ir()
    │   │      或新增 compile_with_deps() 返回 SemanticResult
    │   └── 3. 解析结果: { diagnostics, symbols, dependencies }
    │
    └── DependencyPanel (新组件, ~150行)
        ├── 实时显示: [读取] / [写入] / [调用]
        ├── 每项标注来源 (param/signal/store/local/unknown)
        └── 写入 store.nodeMeta → ArchitecturePage 泡泡图
```

### 2.2 与旧方案的差异

| 层级 | v4.1 (Pure SVG) | v5 (Monaco + WASM) |
|------|-----------------|---------------------|
| 编辑方式 | 键盘导航 + FuzzySearch 选择积木 | Monaco 文本编辑，直接打字 |
| 解析 | BlockModel → CodeOp[] （UI 直接构造） | WASM parser → semantic → CodeOp[] |
| 数据依赖 | 无 | WASM SemanticResult.symbols + .dependencies |
| 选择框导航 | SelectionOverlay + KeyboardHandler (~800行) | Monaco 原生光标/选择 |
| 剪贴板 | ClipboardManager JSON | Monaco 原生 + 系统剪贴板 |
| Bundle | 12 个 SVG 组件 + BlockModel + 导航系统 | Monaco + 复用现有 WASM |
| 代码黑盒 | 完全黑盒 | **打破**——依赖实时可见 |
| CodeFlow | 不变 | 不变 |
| CodeOp 类型体系 | 不变 | 不变 |
| emitOps() | 不变 | 不变 |

## 3. 核心：WASM 依赖分析（替代手写 DependencyExtractor）

### 3.1 为什么用 WASM 而不是手写 TS 解析器

`omnilang/src/semantic/` 已经实现了完整的语义分析：

```rust
// omnilang/src/semantic/mod.rs — 已有，不需要修改核心逻辑
pub struct SemanticResult {
    pub symbols: SymbolTable,                          // 层级作用域 + 所有已解析符号
    pub dependencies: FxHashMap<String, Vec<String>>,  // 依赖关系图
    pub diagnostics: Vec<Diagnostic>,                  // 错误/警告
}

// omnilang/src/semantic/symbols.rs — 已有
pub enum SymbolKind {
    Variable, Signal, Computed, Prop, Event, Ref,
    Function, Action, Store, Component, Getter, Param, Unknown,
}
```

WASM 的 semantic 分析已经处理了：作用域嵌套、变量遮蔽、类型推导、跨文件引用。手写 TS 解析器无法可靠地复现这些。

### 3.2 WASM 变更（轻量）

`omnilang-wasm/src/lib.rs` 中 `OmniCompiler` 新增一个方法，或在现有 `diagnostics()` 基础上扩展返回值：

```rust
// 新增专用方法——不改动现有 diagnostics() 的返回格式
pub fn analyse_dependencies(&mut self) -> JsValue {
    // 对所有已注册 sources 运行 semantic 分析
    // 需要先在 omnilang/src/lib.rs 的 PipelineOutput 中
    // 增加 symbols 和 dependencies 字段（semantic 已产出，只需透传）
    // 返回 { symbols: SymbolInfo[], dependencies: Record<string, string[]>, diagnostics }
}
```

**关键**：semantic 分析需要完整上下文（component 定义、signal 声明、store 引用）。因此调用前需要把通过 `pipeline.ts` 生成的当前组件 `.omni` 源码注册到 OmniCompiler 中。

### 3.3 TS 侧调用

```typescript
// CodeBlockEditor 中的增量分析流程
async function analyseDependencies() {
  // 1. 生成当前组件的 .omni 源码（含编辑中的 code-block 内容）
  const fullSource = generateComponentOmniSource(componentName, currentCodeBlockSource);

  // 2. 注册到 WASM
  compiler.upsert_source(`${componentName}.omni`, fullSource);

  // 3. 运行分析
  const result = compiler.analyse_dependencies(); // 或 compile_to_ir()
  // result: { symbols: [...], dependencies: {...}, diagnostics: [...] }

  // 4. 过滤出当前 code-block 相关的依赖
  return filterRelevantDeps(result, codeBlockPid);
}
```

## 4. CodeBlockEditor 组件

### 4.1 接口

```typescript
interface CodeBlockEditorProps {
  elementId: string;
  initialSource: string;
  // 从 store 读取的上下文（传给 WASM 做 semantic 分析）
  context: {
    fnParams: string[];           // fn 节点的参数名列表
    componentName: string;        // 当前组件名
    signals: string[];            // 组件/Store 中声明的 signal
    stores: string[];             // 组件引用的 store 名
    allSources: Record<string, string>; // 全项目 .omni 源码（已有）
  };
  onSave: (source: string, deps: DependencyInfo) => void;
  onClose: () => void;
}
```

### 4.2 DependencyInfo 类型

```typescript
interface DependencyInfo {
  codeBlockPid: string;
  reads:  { name: string; kind: SymbolKind; sourceLine: number }[];
  writes: { name: string; kind: SymbolKind; sourceLine: number }[];
  calls:  { storeName: string; actionName?: string; sourceLine: number }[];
}

type SymbolKind = "param" | "signal" | "computed" | "prop" | "store"
                | "local" | "function" | "unknown";
```

### 4.3 UI 布局

```
┌────────────────────────────────────────────────────┐
│  CodeBlockEditor                         [保存] [✕] │
├────────────────────────────┬───────────────────────┤
│                            │   📊 数据依赖          │
│                            │                       │
│   Monaco Editor            │   📖 读取              │
│   (OmniLang 语法高亮)       │   count → signal     │
│                            │   items → param       │
│                            │   filterFn → local    │
│                            │                       │
│                            │   ✏️ 写入              │
│                            │   filteredList → signal│
│                            │                       │
│                            │   📞 调用              │
│                            │   TodoStore.toggle    │
│                            │                       │
├────────────────────────────┴───────────────────────┤
│  功能说明: [添加待办项并刷新列表_______________]     │
└────────────────────────────────────────────────────┘
```

- 左侧：Monaco 编辑器（占比 ~70%）
- 右侧：DependencyPanel（占比 ~30%），可折叠

## 5. 集成点

### 5.1 FlowCanvas 变更

```tsx
// 替换: <BlockEditor ... /> 
// 为:   <CodeBlockEditor ... />

{editingElement && (
  <CodeBlockEditor
    elementId={editingElement.id}
    initialSource={(editingElement as CodeblockElementData).source || ""}
    context={{
      fnParams: getFnParams(editingElement),
      componentName: currentComponentName,
      signals: getVisibleSignals(currentComponentName),
      stores: getVisibleStores(currentComponentName),
      allSources: getAllOmniSources(),
    }}
    onSave={(source, deps) => {
      // 保存时: 文本 → source + deps 写入 store
      setElementSource(editingNodeId, source);
      // deps 写入 nodeMeta 供 ArchitecturePage 使用
      setNodeMeta(editingNodeId, { dependencies: deps });
      setEditingNodeId(null);
    }}
    onClose={() => setEditingNodeId(null)}
  />
)}
```

### 5.2 Store 扩展

```typescript
// nodeMeta 已有字段（元素位置等），扩展依赖信息
interface NodeMeta {
  position?: { x: number; y: number };
  dependencies?: DependencyInfo;  // ★ 新增
}
```

### 5.3 ArchitecturePage 消费

ArchitecturePage 泡泡图读取 `nodeMeta[codeBlockId].dependencies`：
- `reads signal` → 从 Signal 泡泡连线到 code-block
- `writes signal` → 从 code-block 连线到 Signal 泡泡
- `calls store.action` → 从 Store 泡泡连线到 code-block

## 6. 文件变更

### 6.1 删除

| 文件 | 原因 |
|------|------|
| `blocks/BlockEditor.tsx` | 被 CodeBlockEditor 替代 |
| `blocks/BlockModel.ts` | 不再需要积木状态机 |
| `blocks/KeyboardHandler.ts` | Monaco 原生键盘处理 |
| `blocks/SelectionOverlay.tsx` | Monaco 原生选择 |
| `blocks/ClipboardManager.ts` | Monaco 原生剪贴板 |
| `blocks/BlockSvg.tsx` | 不再渲染 SVG 积木 |
| `blocks/BlockSvg/*.tsx` (12 个文件) | 同上 |
| `blocks/BlockEditor.module.css` | 不再需要 |
| `blocks/BlockEditor.stories.tsx` | 更新或删除 |
| `blocks/fields/FuzzySearchPopup.ts` | FuzzySearch 在文本编辑器中不需要 |
| `blocks/__tests__/BlockSvg*.test.tsx` | 更新或删除 |

### 6.2 重写

| 文件 | 变更 |
|------|------|
| `FlowCanvas.tsx` | BlockEditor → CodeBlockEditor，传递 context |
| `CodeEditorModal.tsx` | 提取 Monaco 核心逻辑为可复用 hook，或直接作为 CodeBlockEditor 的基础 |

### 6.3 新建

| 文件 | 职责 |
|------|------|
| `CodeBlockEditor.tsx` | 主组件：Monaco + 依赖分析 + DependencyPanel 编排 |
| `DependencyPanel.tsx` | 侧面板：分类显示 reads/writes/calls |
| `DependencyPanel.module.css` | 面板样式 |

### 6.4 WASM 变更

| 文件 | 变更 |
|------|------|
| `omnilang-wasm/src/lib.rs` | OmniCompiler 新增 `analyse_dependencies()` 方法，返回 symbols + dependencies |

### 6.5 不变

| 文件 | 说明 |
|------|------|
| `core/types/code-op.ts` | CodeOp 类型定义不变 |
| `core/types/slot-value.ts` | SlotValue 类型定义不变 |
| `services/codegen/code-op-emitter.ts` | CodeOp[] → 源文本，不变 |
| `services/codegen/code-op-parser.ts` | 源文本 → CodeOp[]（parseOpsWasm 等）保留，供其他模块使用 |
| `services/codegen/graph-parser.ts` | CodeFlow 图解析，不变 |
| `services/monaco/diagnostics.ts` | WASM → Monaco markers，不变 |
| `services/monaco/omnilang-language.ts` | 语法高亮，不变 |
| `nodes/CodeBlockNode.tsx` | React Flow 节点渲染，不变 |
| `FlowInspector.tsx` | 右侧属性面板，不变 |
| `flowConstants.ts` | 标签常量，不变 |

## 7. 实施顺序

1. **WASM 扩展**：`OmniCompiler.analyse_dependencies()` —— 在 diagnostics 基础上同时返回 symbols + dependencies
2. **CodeBlockEditor 核心**：Monaco 编辑 + 增量调用 WASM 分析 + 结果缓存
3. **DependencyPanel**：侧面板 UI 组件
4. **FlowCanvas 集成**：替换 BlockEditor，传递 context
5. **Store 扩展**：nodeMeta 增加 dependencies 字段
6. **ArchitecturePage 集成**：消费依赖数据，画泡泡连线
7. **清理**：删除 blocks/ 下全部文件，移除 BlockEditor 相关 import
8. **Storybook**：CodeBlockEditor + DependencyPanel stories（编辑态/依赖展示/保存流程），Playwright 交互验证

## 8. 不在本期范围

- 结构投影编辑器（方案 3，见 [[poiesis-structural-projection-editor]]）
- Rust AST Stmt 中 else-if 链的完整映射（`stmtsToCodeOps` 已知 bug，但不影响依赖分析——依赖分析走 semantic 不走过时的 TS 映射）
- CodeEditorModal 重构（保留向后兼容，CodeBlockEditor 复用其核心逻辑，不破坏现有调用方）
- 跨 code-block 的依赖聚合（同一 fn 下多个 code-block 的依赖合并展示）

## 9. 风险

| 风险 | 缓解 |
|------|------|
| WASM semantic 需要完整 .omni 上下文才能正确解析 | pipeline.ts 已能生成完整源码；先用单组件分析，跨文件引用后续迭代 |
| Monaco 实例生命周期管理（CodeEditorModal + CodeBlockEditor 可能同时存在） | 提取 Monaco 初始化逻辑为共享 hook，确保单例 |
| 删除 blocks/ 目录影响现有 50 个测试 | 分步：先建新的 CodeBlockEditor + 测试，跑通后再删旧代码 |
| 依赖分析性能——每次输入触发 WASM 编译 | debounce 500ms + OmniCompiler 结果缓存（已有） |
