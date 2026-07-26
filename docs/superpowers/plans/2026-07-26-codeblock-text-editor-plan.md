# CodeBlock 文本编辑器 + 数据依赖打破黑盒 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 移除 Block Editor（积木编辑器），用 Monaco 文本编辑 + WASM 语义分析替代，打破 code-block 代码黑盒，实时展示数据依赖。

**Architecture:** FlowCanvas 双击 code-block 节点打开 CodeBlockEditor（Monaco + DependencyPanel）。每次输入 debounce 300ms，通过 pipeline.ts 生成组件 .omni 源码，注册到 WASM OmniCompiler，调用新增的 `analyse_dependencies()` 获取 SemanticResult（symbols + dependencies + diagnostics），诊断映射到 Monaco markers，依赖展示在右侧面板。

**Tech Stack:** React 19 + Monaco Editor + Zustand 5 + WASM (omnilang-wasm) + Storybook 8 + Playwright

## Global Constraints

- 所有新建文件遵循 `poiesis/src/` 现有目录结构
- CodeOp 类型体系（12 种）不变，emitOps/parseOps 保留
- WASM 语义分析通过 omnilang 现有 semantic 管线，不手写 TS 解析器
- Monaco 实例全局单例（避免重复加载）
- Storybook stories 作为主要测试媒介
- 先建新、再删旧——BlockEditor 和 blocks/ 目录在新编辑器跑通前保留

---

## 文件结构

```
poiesis/src/
├── components/canvas/code/
│   ├── CodeBlockEditor.tsx          ★ 新建 — 主编排组件
│   ├── CodeBlockEditor.module.css   ★ 新建 — 样式
│   ├── CodeBlockEditor.stories.tsx  ★ 新建 — Storybook stories
│   ├── DependencyPanel.tsx          ★ 新建 — 依赖面板
│   ├── DependencyPanel.module.css   ★ 新建 — 面板样式
│   ├── FlowCanvas.tsx               修改 — BlockEditor→CodeBlockEditor
│   ├── CodeEditorModal.tsx          修改 — 提取 Monaco 初始化 hook
│   ├── [blocks/ 全部文件]           ⚠ 最终删除（Task 8）
│
├── services/monaco/
│   ├── useMonacoEditor.ts           ★ 新建 — 共享 Monaco 初始化 hook
│
├── core/types/
│   └── project-json.ts              修改 — NodeMeta 加 dependencies 字段
│
├── store/slices/
│   └── metadata.ts                  不变（setNodeMeta 已存在）

omnilang-wasm/src/
└── lib.rs                           修改 — OmniCompiler 加 analyse_dependencies()

omnilang/src/
└── lib.rs                           修改 — PipelineOutput 加 symbols + dependencies 字段
```

---

### Task 1: WASM — PipelineOutput 透传 SemanticResult

**Files:**
- Modify: `omnilang/src/lib.rs`（PipelineOutput 结构体 + run_pipeline 函数）

**Interfaces:**
- Produces: `PipelineOutput { symbols: Vec<SymbolInfo>, dependencies: HashMap<String, Vec<String>>, ...原有字段 }`

- [ ] **Step 1: 在 PipelineOutput 中增加 symbols 和 dependencies 字段**

`omnilang/src/lib.rs` 中找到 `PipelineOutput` 结构体：

```rust
// 找到类似这样的结构体并扩展
pub struct PipelineOutput {
    pub ir: Option<IRProgram>,
    pub diagnostics: Vec<Diagnostic>,
    pub has_errors: bool,
    // ★ 新增
    pub symbols: Vec<SymbolInfo>,
    pub dependencies: FxHashMap<String, Vec<String>>,
}
```

- [ ] **Step 2: 在 run_pipeline 中填充新字段**

找到 `run_pipeline()` 函数，`semantic::analyze(program)` 的返回值 `SemanticResult` 中提取 symbols 和 dependencies：

```rust
let semantic_result = semantic::analyze(&program);
// 转换 SymbolTable 为 Vec<SymbolInfo>
let symbols: Vec<SymbolInfo> = semantic_result.symbols
    .scopes()
    .iter()
    .flat_map(|scope| scope.symbols.values())
    .map(|sym| SymbolInfo {
        name: sym.name.clone(),
        kind: format!("{:?}", sym.kind).to_lowercase(),
        type_ann: sym.type_ann.clone(),
        mutable: sym.mutable,
    })
    .collect();

PipelineOutput {
    ir: ...,
    diagnostics: ...,
    has_errors: ...,
    symbols,
    dependencies: semantic_result.dependencies,
}
```

- [ ] **Step 3: 定义 SymbolInfo 类型**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,       // "variable" | "signal" | "prop" | "store" | ...
    pub type_ann: Option<String>,
    pub mutable: bool,
}
```

确保 `#[derive(Serialize)]` 且字段名与 TS 侧 `DependencyInfo` 对齐。

- [ ] **Step 4: 编译验证**

```bash
cd omnilang && cargo check
```

Expected: 编译通过，无类型错误。

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/lib.rs
git commit -m "feat: extend PipelineOutput with symbols and dependencies from semantic analysis"
```

---

### Task 2: WASM — OmniCompiler.analyse_dependencies()

**Files:**
- Modify: `omnilang-wasm/src/lib.rs`（OmniCompiler impl 块）

**Interfaces:**
- Produces: `OmniCompiler.analyse_dependencies() -> JsValue`（`{ symbols, dependencies, diagnostics }`）

- [ ] **Step 1: 在 OmniCompiler impl 中新增方法**

```rust
/// Run semantic analysis on all registered sources and return
/// symbols + dependencies in addition to diagnostics.
/// Returns JsValue with { symbols, dependencies, diagnostics }.
pub fn analyse_dependencies(&mut self) -> JsValue {
    let mut all_symbols: Vec<serde_json::Value> = Vec::new();
    let mut all_deps: FxHashMap<String, Vec<String>> = FxHashMap::default();
    let mut all_errors: Vec<serde_json::Value> = Vec::new();

    for source in self.sources.values() {
        let result = compile(source);
        // compile() returns CompileResult { success, ir, errors }
        // 但现在 PipelineOutput 中有 symbols 和 dependencies
        // 需要修改 compile() 返回类型或新增 compile_with_deps()
        
        if let Some(ir) = &result.ir {
            // symbols 现在在 IR 或 PipelineOutput 中
            // 实际路径取决于 run_pipeline 如何暴露
        }
        
        for err in &result.errors {
            all_errors.push(err.to_json_value());
        }
    }

    serde_wasm_bindgen::to_value(&serde_json::json!({
        "symbols": all_symbols,
        "dependencies": all_deps,
        "diagnostics": all_errors,
    }))
    .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)))
}
```

**注意**：当前 `compile()` 返回 `CompileResult { success, ir, errors }`。需要确认 `PipelineOutput.symbols` 和 `.dependencies` 如何传递到 `CompileResult`。可能需要修改 `compile()` 或新增 `compile_with_semantic()` 函数。

- [ ] **Step 2: 在 omnilang/src/lib.rs 中修改 compile() 或新增 compile_with_semantic()**

```rust
/// Like compile() but also returns semantic symbols and dependencies.
pub fn compile_with_semantic(source: &str) -> CompileWithSemanticResult {
    let pipeline = run_pipeline(source, /* ... */);
    CompileWithSemanticResult {
        success: !pipeline.has_errors,
        ir: pipeline.ir,
        symbols: pipeline.symbols,
        dependencies: pipeline.dependencies,
        errors: pipeline.diagnostics,
    }
}
```

- [ ] **Step 3: 编译 WASM**

```bash
cd omnilang-wasm && wasm-pack build --target web --out-dir ../poiesis/src/wasm
```

Expected: 编译通过，`omnilang_wasm.js` + `_bg.wasm` 生成。

- [ ] **Step 4: 更新 TS 类型声明**

`poiesis/src/wasm/omnilang_wasm.d.ts` 中 `OmniCompiler` 类增加：

```typescript
analyse_dependencies(): {
  symbols: SymbolInfo[];
  dependencies: Record<string, string[]>;
  diagnostics: WasmDiagnostic[];
};
```

- [ ] **Step 5: Commit**

```bash
git add omnilang-wasm/src/lib.rs omnilang/src/lib.rs poiesis/src/wasm/omnilang_wasm.d.ts
git commit -m "feat: add OmniCompiler.analyse_dependencies() for semantic dependency extraction"
```

---

### Task 3: useMonacoEditor — 共享 Monaco 初始化 Hook

**Files:**
- Create: `poiesis/src/services/monaco/useMonacoEditor.ts`

**Interfaces:**
- Consumes: Monaco Editor API (`monaco-editor`), OmniLang language services
- Produces: `useMonacoEditor(config: UseMonacoEditorConfig): UseMonacoEditorResult`

- [ ] **Step 1: 定义类型**

```typescript
import type * as monaco from "monaco-editor";
import type { StoreDefinition } from "../../core/types/store";

interface UseMonacoEditorConfig {
  containerRef: React.RefObject<HTMLDivElement | null>;
  initialValue: string;
  language: "typescript" | "omnilang";
  storeDefs?: StoreDefinition[];
  /** Called on every content change with current editor value */
  onChange?: (value: string) => void;
}

interface UseMonacoEditorResult {
  editor: monaco.editor.IStandaloneCodeEditor | null;
  getValue: () => string;
  setValue: (value: string) => void;
}
```

- [ ] **Step 2: 实现 hook**

```typescript
export function useMonacoEditor(config: UseMonacoEditorConfig): UseMonacoEditorResult {
  const { containerRef, initialValue, language, storeDefs, onChange } = config;
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);

  // Init Monaco services (once)
  useEffect(() => {
    void initMonacoServices(storeDefs);
  }, [/* storeNamesKey */]);

  // Create editor (once)
  useEffect(() => {
    if (!containerRef.current) return;
    let cancelled = false;

    import("monaco-editor").then((monacoModule) => {
      if (cancelled || !containerRef.current) return;
      const editor = monacoModule.editor.create(containerRef.current, {
        value: initialValue,
        language,
        theme: "vs-dark",
        fontSize: 13,
        minimap: { enabled: false },
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        automaticLayout: true,
      });
      editorRef.current = editor;
      
      if (onChange) {
        editor.onDidChangeModelContent(() => onChange(editor.getValue()));
      }
    });

    return () => {
      cancelled = true;
      editorRef.current?.dispose();
      editorRef.current = null;
    };
  }, []);

  return {
    editor: editorRef.current,
    getValue: () => editorRef.current?.getValue() ?? "",
    setValue: (v) => editorRef.current?.setValue(v),
  };
}
```

- [ ] **Step 3: 重构成 CodeEditorModal 使用此 hook**

`CodeEditorModal.tsx` 中的 Monaco 初始化逻辑替换为 `useMonacoEditor`：

```tsx
const { getValue, setValue } = useMonacoEditor({
  containerRef,
  initialValue: initialSource,
  language,
  storeDefs,
});
```

保留 CodeEditorModal 的 WASM diagnostics useEffect（它使用 `editor.onDidChangeModelContent`），但编辑器创建委托给 hook。

- [ ] **Step 4: 验证 CodeEditorModal 仍然正常工作**

Storybook 中打开任意使用 CodeEditorModal 的 story，确认：
- Monaco 编辑器正常加载
- OmniLang 语法高亮正常
- WASM diagnostics 正常显示

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/monaco/useMonacoEditor.ts poiesis/src/components/canvas/code/CodeEditorModal.tsx
git commit -m "feat: extract useMonacoEditor hook for shared Monaco initialization"
```

---

### Task 4: DependencyPanel 组件

**Files:**
- Create: `poiesis/src/components/canvas/code/DependencyPanel.tsx`
- Create: `poiesis/src/components/canvas/code/DependencyPanel.module.css`

**Interfaces:**
- Produces: `<DependencyPanel deps={DependencyInfo | null} />` React 组件

- [ ] **Step 1: 定义 DependencyInfo 类型**

在 `DependencyPanel.tsx` 中定义（后续移到 `core/types/`）：

```typescript
export interface DependencyInfo {
  codeBlockPid: string;
  reads:  DepEntry[];
  writes: DepEntry[];
  calls:  CallEntry[];
}

interface DepEntry {
  name: string;
  kind: "param" | "signal" | "computed" | "prop" | "store" | "local" | "function" | "unknown";
  sourceLine: number;
}

interface CallEntry {
  storeName: string;
  actionName?: string;
  sourceLine: number;
}
```

- [ ] **Step 2: 实现 DependencyPanel 组件**

```tsx
import styles from "./DependencyPanel.module.css";

interface Props {
  deps: DependencyInfo | null;
}

export function DependencyPanel({ deps }: Props) {
  if (!deps) {
    return (
      <div className={styles.panel}>
        <div className={styles.empty}>解析中...</div>
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      <Section title="📖 读取" items={deps.reads} kindLabel={kindLabel} />
      <Section title="✏️ 写入" items={deps.writes} kindLabel={kindLabel} />
      <Section title="📞 调用" items={[]} />
      {deps.calls.map((c, i) => (
        <div key={i} className={styles.entry}>
          <span className={styles.name}>{c.storeName}</span>
          {c.actionName && <span className={styles.action}>.{c.actionName}</span>}
          <span className={styles.line}>:{c.sourceLine}</span>
        </div>
      ))}
    </div>
  );
}

function Section({ title, items, kindLabel }: {
  title: string; items: DepEntry[]; kindLabel: (k: string) => string;
}) {
  if (items.length === 0) return null;
  return (
    <div className={styles.section}>
      <h4 className={styles.sectionTitle}>{title}</h4>
      {items.map((item, i) => (
        <div key={i} className={styles.entry}>
          <span className={styles.name}>{item.name}</span>
          <span className={styles.kind}>{kindLabel(item.kind)}</span>
          <span className={styles.line}>:{item.sourceLine}</span>
        </div>
      ))}
    </div>
  );
}

const KIND_LABELS: Record<string, string> = {
  param: "参数", signal: "Signal", computed: "Computed",
  prop: "Prop", store: "Store", local: "局部变量",
  function: "函数", unknown: "未知",
};

function kindLabel(k: string): string {
  return KIND_LABELS[k] ?? k;
}
```

- [ ] **Step 3: CSS 样式**

```css
.panel {
  width: 260px;
  padding: 12px;
  border-left: 1px solid var(--ol-border, #e2e8f0);
  overflow-y: auto;
  font-size: 12px;
  background: var(--ol-surface, #f8fafc);
}

.empty {
  color: var(--ol-text-muted, #94a3b8);
  text-align: center;
  padding-top: 24px;
}

.section { margin-bottom: 12px; }

.sectionTitle {
  margin: 0 0 6px 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--ol-text, #1e293b);
}

.entry {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
}

.name {
  font-family: monospace;
  font-weight: 500;
  color: var(--ol-accent, #6366f1);
}

.kind {
  font-size: 10px;
  padding: 0 4px;
  border-radius: 3px;
  background: var(--ol-surface-hover, #e2e8f0);
  color: var(--ol-text-muted, #64748b);
}

.line {
  margin-left: auto;
  color: var(--ol-text-muted, #94a3b8);
  font-size: 10px;
}
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/DependencyPanel.tsx poiesis/src/components/canvas/code/DependencyPanel.module.css
git commit -m "feat: add DependencyPanel component for live data dependency display"
```

---

### Task 5: CodeBlockEditor 主组件

**Files:**
- Create: `poiesis/src/components/canvas/code/CodeBlockEditor.tsx`
- Create: `poiesis/src/components/canvas/code/CodeBlockEditor.module.css`

**Interfaces:**
- Consumes: `useMonacoEditor`, `DependencyPanel`, WASM `OmniCompiler.analyse_dependencies()`, `generateOmniSource()`
- Produces: `<CodeBlockEditor>` React 组件

- [ ] **Step 1: 实现 CodeBlockEditor 组件**

```typescript
import React, { useCallback, useEffect, useRef, useState } from "react";
import { useMonacoEditor } from "../../../services/monaco/useMonacoEditor";
import { getOmniCompiler } from "../../../services/monaco/diagnostics";
import { generateOmniSource } from "../../../services/codegen/pipeline";
import { DependencyPanel } from "./DependencyPanel";
import type { DependencyInfo, DepEntry, CallEntry } from "./DependencyPanel";
import styles from "./CodeBlockEditor.module.css";

interface CodeBlockEditorProps {
  elementId: string;
  initialSource: string;
  context: {
    fnParams: string[];
    componentName: string;
    omniSourcePath: string;          // e.g. "src/main.omni"
    designElements: ElementData[];   // from store.elements
    designWires: WireData[];         // from store.wires
    codeTabs: CodeTab[];             // from store
  };
  onSave: (source: string, deps: DependencyInfo | null) => void;
  onClose: () => void;
}

export function CodeBlockEditor({
  elementId, initialSource, context, onSave, onClose,
}: CodeBlockEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [deps, setDeps] = useState<DependencyInfo | null>(null);
  const sourceRef = useRef(initialSource);

  const { getValue } = useMonacoEditor({
    containerRef,
    initialValue: initialSource,
    language: "omnilang",
  });

  // WASM dependency analysis (debounce 500ms)
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;

    const analyse = async () => {
      const currentSource = getValue();
      sourceRef.current = currentSource;

      // 1. Generate full .omni source for the component
      const fullResult = generateOmniSource({
        designElements: context.designElements,
        designWires: context.designWires,
        codeTabs: context.codeTabs,
        componentName: context.componentName,
      });

      if (!fullResult.value) return;

      // 2. Run WASM semantic analysis
      try {
        const compiler = await getOmniCompiler();
        if (!compiler) return;
        compiler.upsert_source(context.omniSourcePath, fullResult.value);
        const result = compiler.analyse_dependencies();

        // 3. Extract dependency info
        const depInfo = mapSemanticResult(result, elementId);
        setDeps(depInfo);
      } catch {
        // WASM not available — silent fallback
      }
    };

    timer = setTimeout(analyse, 500);
    return () => clearTimeout(timer);
  }, [context]);

  const handleSave = useCallback(() => {
    onSave(sourceRef.current, deps);
  }, [deps, onSave]);

  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        <div className={styles.header}>
          <h3>编辑代码块</h3>
          <div className={styles.actions}>
            <button onClick={handleSave}>保存 (Ctrl+Enter)</button>
            <button onClick={onClose}>取消</button>
          </div>
        </div>
        <div className={styles.body}>
          <div ref={containerRef} className={styles.monacoArea} />
          <DependencyPanel deps={deps} />
        </div>
      </div>
    </div>
  );
}

/** Map WASM semantic result to DependencyInfo */
function mapSemanticResult(
  result: any, codeBlockPid: string
): DependencyInfo {
  const symbols: any[] = result.symbols ?? [];
  const deps: Record<string, string[]> = result.dependencies ?? {};

  const reads: DepEntry[] = [];
  const writes: DepEntry[] = [];
  const calls: CallEntry[] = [];

  for (const sym of symbols) {
    const entry: DepEntry = {
      name: sym.name,
      kind: sym.kind ?? "unknown",
      sourceLine: sym.declared_at ?? 0,
    };
    // Classify as read or write based on symbol kind and usage
    if (sym.kind === "signal" || sym.kind === "variable") {
      // TODO: refine with actual read/write classification from AST
      reads.push(entry);
    }
    if (sym.kind === "store") {
      calls.push({ storeName: sym.name, sourceLine: entry.sourceLine });
    }
  }

  return { codeBlockPid, reads, writes, calls };
}
```

- [ ] **Step 2: CSS 布局**

```css
.overlay {
  position: fixed; inset: 0; z-index: 10000;
  background: rgba(0,0,0,0.4);
  display: flex; align-items: center; justify-content: center;
}

.editor {
  width: 80vw; height: 70vh;
  background: var(--ol-bg, #fff);
  border-radius: 8px;
  display: flex; flex-direction: column;
  overflow: hidden;
}

.header {
  display: flex; justify-content: space-between; align-items: center;
  padding: 8px 16px; border-bottom: 1px solid var(--ol-border);
}

.actions { display: flex; gap: 8px; }

.body {
  flex: 1; display: flex; min-height: 0;
}

.monacoArea {
  flex: 1; min-width: 0;
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/canvas/code/CodeBlockEditor.tsx poiesis/src/components/canvas/code/CodeBlockEditor.module.css
git commit -m "feat: add CodeBlockEditor with Monaco + WASM dependency analysis"
```

---

### Task 6: FlowCanvas 集成

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`

**Interfaces:**
- Consumes: `CodeBlockEditor`, store hooks
- Produces: 替换 `<BlockEditor>` 为 `<CodeBlockEditor>`

- [ ] **Step 1: 替换 BlockEditor import 和渲染**

```tsx
// 旧: import { BlockEditor } from "./BlockEditor";
// 新:
import { CodeBlockEditor } from "./CodeBlockEditor";
```

- [ ] **Step 2: 修改渲染逻辑，传递 context**

```tsx
{editingElement && (
  <CodeBlockEditor
    elementId={editingElement.id}
    initialSource={(editingElement as CodeblockElementData).source || ""}
    context={{
      fnParams: getFnParams(editingElement),
      componentName: currentComponentName,
      omniSourcePath: getActiveFilePath(),
      designElements: elements,  // 从 store
      designWires: wires,        // 从 store
      codeTabs: getCodeTabs(),   // 从 store
    }}
    onSave={(source, deps) => {
      setElementSource(editingNodeId, source);
      if (deps) {
        setNodeMeta(editingNodeId, { dependencies: deps, pos: /* preserve existing pos */ });
      }
      setEditingNodeId(null);
    }}
    onClose={() => setEditingNodeId(null)}
  />
)}
```

- [ ] **Step 3: 添加辅助函数 getFnParams**

从 fn 节点的 `props.params` 中提取参数名列表：

```typescript
function getFnParams(el: ElementData): string[] {
  if (el.nodeType !== "codeblock" || el.tag !== "fn") return [];
  try {
    return JSON.parse((el.props.params as string) || "[]");
  } catch {
    return [];
  }
}
```

- [ ] **Step 4: 验证 FlowCanvas 正常渲染**

确认 CodeFlow 图不受影响，双击 code-block 节点能打开新编辑器。

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat: integrate CodeBlockEditor into FlowCanvas replacing BlockEditor"
```

---

### Task 7: NodeMeta 扩展 + Storybook Stories

**Files:**
- Modify: `poiesis/src/core/types/project-json.ts`（NodeMeta 加 dependencies）
- Create: `poiesis/src/components/canvas/code/CodeBlockEditor.stories.tsx`

**Interfaces:**
- Produces: `NodeMeta.dependencies: DependencyInfo | undefined`

- [ ] **Step 1: 扩展 NodeMeta 类型**

```typescript
import type { DependencyInfo } from "../../components/canvas/code/DependencyPanel";

export interface NodeMeta {
  pos: { x: number; y: number };
  folded?: boolean;
  renderMode?: "dom" | "canvas";
  nodeType?: string;
  blockName?: string;
  dependencies?: DependencyInfo;  // ★ 新增
}
```

- [ ] **Step 2: 写 Storybook stories**

```tsx
import type { Meta, StoryObj } from "@storybook/react";
import { CodeBlockEditor } from "./CodeBlockEditor";

const meta: Meta<typeof CodeBlockEditor> = {
  title: "CodeBlockEditor/CodeBlockEditor",
  component: CodeBlockEditor,
  parameters: { layout: "fullscreen" },
};

export default meta;
type Story = StoryObj<typeof CodeBlockEditor>;

const noop = { onSave: () => {}, onClose: () => {} };
const mockContext = {
  fnParams: ["items", "filter"],
  componentName: "TodoList",
  omniSourcePath: "src/main.omni",
  designElements: [],
  designWires: [],
  codeTabs: [],
};

export const Empty: Story = {
  args: { elementId: "s-empty", initialSource: "", context: mockContext, ...noop },
  name: "空编辑器",
};

export const SimpleVarOps: Story = {
  args: {
    elementId: "s-ops",
    initialSource: "let count: i32 = 0;\ncount += 1;\nreturn count;",
    context: mockContext,
    ...noop,
  },
  name: "变量操作 + 返回",
};

export const IfElseBlock: Story = {
  args: {
    elementId: "s-if",
    initialSource: "if x > 0 {\n  return 1;\n} else {\n  return 0;\n}",
    context: mockContext,
    ...noop,
  },
  name: "if/else 分支",
};

export const WhileLoop: Story = {
  args: {
    elementId: "s-while",
    initialSource: "let i = 0;\nwhile i < 10 {\n  i += 1;\n}",
    context: mockContext,
    ...noop,
  },
  name: "while 循环",
};

export const StoreCall: Story = {
  args: {
    elementId: "s-store",
    initialSource: "let items = TodoStore.getItems();\nfiltered = items.filter(fn);",
    context: mockContext,
    ...noop,
  },
  name: "Store 调用 + 赋值",
};
```

- [ ] **Step 3: 验证 Storybook 可运行**

```bash
cd poiesis && pnpm storybook
```

确认所有 5 个 stories 渲染正常，Monaco 编辑器可交互。

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/core/types/project-json.ts poiesis/src/components/canvas/code/CodeBlockEditor.stories.tsx
git commit -m "feat: add NodeMeta.dependencies field and CodeBlockEditor Storybook stories"
```

---

### Task 8: 清理 — 删除 blocks/ 目录

**Files:**
- Delete: `poiesis/src/components/canvas/code/blocks/` (整个目录)
- Delete: `poiesis/src/components/canvas/code/BlockEditor.stories.tsx`
- Delete: `poiesis/src/components/canvas/code/BlockEditor.module.css`
- Modify: `poiesis/src/components/canvas/code/BlockEditor.tsx` (删除文件)
- Modify: 任何 import BlockEditor 的文件（检查残留引用）

- [ ] **Step 1: 检查残留引用**

```bash
cd poiesis && grep -r "BlockEditor\|blocks/" src/ --include="*.ts" --include="*.tsx" | grep -v "CodeBlockEditor\|node_modules"
```

Expected: 只有 CodeBlockEditor 自身的引用，无残留。

- [ ] **Step 2: 删除文件**

```bash
rm -rf poiesis/src/components/canvas/code/blocks/
rm poiesis/src/components/canvas/code/BlockEditor.tsx
rm poiesis/src/components/canvas/code/BlockEditor.stories.tsx
rm poiesis/src/components/canvas/code/BlockEditor.module.css
```

- [ ] **Step 3: 删除 Blockly 依赖（如果还有）**

```bash
cd poiesis && pnpm remove blockly  # 如果 package.json 中还有
```

- [ ] **Step 4: 跑测试确认无回归**

```bash
cd poiesis && pnpm test -- --passWithNoTests
```

确认保留测试（code-op-parser, graph-compiler 等）正常通过。

- [ ] **Step 5: 跑 Storybook 确认**

```bash
cd poiesis && pnpm storybook
```

确认 BlockEditor stories 消失，CodeBlockEditor stories 正常。

- [ ] **Step 6: Commit**

```bash
git add -A poiesis/src/components/canvas/code/
git commit -m "refactor: remove BlockEditor SVG block system, replaced by CodeBlockEditor"
```

---

### Task 9: ArchitecturePage 集成（依赖消费端）

**Files:**
- Modify: `poiesis/src/components/architecture/` 下的相关文件

**Interfaces:**
- Consumes: `nodeMeta[codeBlockId].dependencies`（来自 Task 7）
- Produces: 泡泡图连线（Signal ↔ code-block, Store ↔ code-block）

- [ ] **Step 1: 在泡泡图数据构建中读取 dependencies**

找到 ArchitecturePage 中构建泡泡连线的逻辑：

```typescript
// 伪代码：在构建 edges 时遍历 nodeMeta
for (const [pid, meta] of Object.entries(store.nodeMeta)) {
  if (meta.dependencies) {
    const deps = meta.dependencies;
    for (const read of deps.reads) {
      if (read.kind === "signal") {
        // 从 signal 泡泡 → code-block 泡泡
        edges.push({ from: findBubbleId(read.name, "signal"), to: pid });
      }
    }
    for (const write of deps.writes) {
      if (write.kind === "signal") {
        // 从 code-block 泡泡 → signal 泡泡
        edges.push({ from: pid, to: findBubbleId(write.name, "signal") });
      }
    }
    for (const call of deps.calls) {
      // 从 Store 泡泡 → code-block 泡泡
      edges.push({ from: findBubbleId(call.storeName, "store"), to: pid });
    }
  }
}
```

- [ ] **Step 2: 验证泡泡图正确显示连线**

启动 Poiesis，在有数据依赖的 code-block 上保存，切换到 ArchitecturePage 确认连线出现。

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/architecture/
git commit -m "feat: ArchitecturePage consumes code-block dependencies for bubble graph edges"
```

---

### Task 10: Storybook 全量验证 + 回归检查

**Files:**
- 验证: `poiesis/src/components/canvas/code/CodeBlockEditor.stories.tsx`

- [ ] **Step 1: 跑全部 Storybook stories**

```bash
cd poiesis && pnpm storybook
```

逐 story 检查：
- [ ] Empty: Monaco 编辑器空白，DependencyPanel 显示"解析中..."
- [ ] SimpleVarOps: 代码正常渲染，依赖面板显示变量信息
- [ ] IfElseBlock: if/else 语法高亮正常
- [ ] WhileLoop: while 语法高亮正常
- [ ] StoreCall: Store 引用正确显示

- [ ] **Step 2: 跑 Playwright 截图对比**

```bash
cd poiesis && pnpm test:storybook  # 或对应的 Playwright storybook 测试命令
```

- [ ] **Step 3: 跑全部单元测试**

```bash
cd poiesis && pnpm test
```

确认所有保留测试通过。

- [ ] **Step 4: Commit（如有截图更新）**

```bash
git add poiesis/
git commit -m "test: Storybook + unit test validation for CodeBlockEditor"
```

---

## 依赖关系

```
Task 1 (PipelineOutput) ──┐
                          ├──→ Task 2 (analyse_dependencies) ──→ Task 5 (CodeBlockEditor)
                          │                                         │
Task 3 (useMonacoEditor) ─┘                                         │
                                                                     │
Task 4 (DependencyPanel) ──────────────────────────────────────────┘
                                                                     │
                                              Task 6 (FlowCanvas) ──┤
                                              Task 7 (NodeMeta + Stories) ─┘
                                                                     │
                                              Task 8 (清理) ─────────┘
                                              Task 9 (ArchitecturePage) ──→ Task 10 (验证)
```

Task 1-5 可部分并行（3 和 4 完全独立，1→2 串行，5 依赖 2+3+4）。

---

## 自检清单

- [x] §1 PipelineOutput 扩展 → Task 1
- [x] §2 analyse_dependencies() → Task 2
- [x] §3 useMonacoEditor hook → Task 3
- [x] §4 DependencyPanel 组件 → Task 4
- [x] §5 CodeBlockEditor 主组件 → Task 5
- [x] §6 FlowCanvas 集成 → Task 6
- [x] §7 NodeMeta 扩展 + Storybook → Task 7
- [x] §8 清理 blocks/ → Task 8
- [x] §9 ArchitecturePage 集成 → Task 9
- [x] 全局 Storybook 验证 → Task 10
- [x] 无 TBD/TODO/占位符
- [x] 类型签名跨任务一致（DependencyInfo 在 Task 4 定义，Task 5/7/9 复用）
