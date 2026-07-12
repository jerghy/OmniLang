# 常见混淆与陷阱

> 开发 Poiesis 时最容易出错的地方。**必读。**

## 一、磁盘文件 ≠ 编辑器内存状态 ★ 最重要！

### 问题

Poiesis 编辑器有自己的内存模型（Zustand Store）。打开 `.omni` 文件时，编辑器将内容解析到 Store 中。**之后的所有 UI 编辑操作都只修改 Store，不写磁盘。**

### 出错场景

1. 你直接在 VS Code 中修改了 `design.omni`（比如加了 padding）
2. 回到 Poiesis，点"保存项目 (Ctrl+S)"
3. **Poiesis 用 Store 中的旧内容覆盖了你的磁盘修改！**

### 正确做法

| 场景 | 正确流程 |
|------|---------|
| 要修改 .omni 文件内容 | 关掉编辑器中的对应 Tab → 编辑磁盘文件 → 在编辑器中重新打开 Tab |
| 要通过 UI 修改 | 用 VariantEditor / ClassEditor 修改 → 点保存 |
| 不确定当前是哪份数据 | 用 `invoke_command read_file` 读磁盘版本，用 `eval_js` 读 Store 版本对比 |

### 为什么会这样

编辑器没有文件监听（file watcher）。在编辑器打开期间，磁盘文件的变化不会被自动同步。

## 二、属性命名：kebab-case ↔ camelCase

### 规则

| 位置 | 格式 | 示例 |
|------|------|------|
| `.omni` 磁盘文件 | kebab-case | `font-size`, `padding-top`, `border-radius` |
| Store / TypeScript 内存 | camelCase | `fontSize`, `paddingTop`, `borderRadius` |

### 转换位置

- **camelCase → kebab-case**：`design-emitter.ts` 行 15-17 `camelToKebab()`
- **kebab-case → camelCase**：`parser.ts` 中解析属性名时

### 出错表现

- `.omni` 中写了 `font-size` 但 Store 中查到的是 `fontSize` → 正常，已转换
- Store 中生成了 `font-size`（kebab）作为属性名 → **bug**，Store 中必须是 camelCase
- `.omni` 中出现了 `fontSize`（camelCase）→ **bug**，磁盘必须是 kebab

## 三、Variant 分支属性不是"级联继承"

### 误解

很多人以为 `size.lg` 会自动继承 `size.md` 的属性（因为是"更大的尺寸"）。

### 真相

```omni
variant Button {
    @dim size: sm | md | lg;
    base { /* 共享属性 */ }
    size.sm { padding: 4px 8px; font-size: 12px; }
    size.md { padding: 8px 16px; font-size: 14px; }
    size.lg { font-size: 16px; }  // ← 只有 font-size，没有 padding
}

// Button(size=lg) 的渲染结果:
//   base + size.lg
//   = display:inline-flex + ... + font-size:16px
//   padding? → base 中没定义 → size.lg 中没定义 → 浏览器默认 1px 6px ❌
```

**每个分支是独立的、平级的。`size.lg` 不从 `size.md` 或 `size.sm` 继承任何东西。**

### 正确做法

每个分支必须显式定义该维度应有的**所有**属性：

```omni
size.lg {
    padding-top: token(spaceMd);
    padding-bottom: token(spaceMd);
    padding-left: token(spaceLg);
    padding-right: token(spaceLg);
    font-size: token(fontSizeLg);
}
```

## 四、base 的"已封锁"属性

在 VariantEditor 中，base 分支的有些属性旁显示"已封锁（分支可覆盖）— 点击解锁"。

这是 `@inherit` 机制的 UI 表示：

- **封锁（locked）**：属性锁定在 base 中，所有分支只读继承，不可覆盖
- **解锁（unlocked）**：分支可以覆盖此属性

`toggleInheritedProp(variantId, propName)` 切换封锁状态。

对应的 .omni 语法：
```omni
variant Button {
    @inherit border-radius, cursor;  // 这两个属性被封锁
    base {
        border-radius: token(radiusMd);
        cursor: "pointer";
    }
}
```

## 五、Poiesis 项目 vs OmniLang 项目

| | Poiesis 概念 | OmniLang 概念 |
|----|------------|-------------|
| 项目标识 | `project.json` 所在的目录 | `.omnilang.toml` 所在的目录（可选） |
| 源码目录 | 约定 `src/` | 约定 `src/` |
| 元数据 | `project.json`（编辑器状态） | `.omnilang.toml`（编译配置） |
| 创建方式 | Poiesis 新建项目 | `omnic create` |

Poiesis 可以打开任何有 `.omni` 文件的目录作为项目。`project.json` 是 Poiesis 自己创建的，和 `.omnilang.toml` 无关。

## 六、design.omni 的 Tab 类型

打开 `design.omni` 时，中央区域不是普通 Canvas，而是**设计系统编辑器**（TokenEditor + ClassEditor + VariantEditor）。

判断逻辑在 `DesignPage.tsx`：
```typescript
// 如果 TabType === "design" → 渲染 DesignTabContent
// 如果 TabType === "component" → 渲染 Canvas + Artboard
```

TabType 在 `openTab()` 时根据文件名推断。

## 七、保存 = 多文件写入

点"保存项目"时会发生：

1. `sync_store_state`：Store 状态 JSON → Rust `store_snapshot`
2. `save_project`：
   - 将 `store_snapshot` 写入 `project.json`
   - (其他 `.omni` 文件通过前端 `generateOmniFiles` → `write_file` 逐文件写入)
3. 标记 `isDirty = false`

**不是只写一个文件！** project.json + 所有脏的 .omni 文件都会被写。

## 八、撤销只能撤销编辑器操作

Undo stack 记录的是编辑器内的操作（拖拽、属性修改、样式变更），按 `tabId` 分栈。

以下操作**不在** undo stack 中：
- 保存项目
- 切换 Tab
- 打开/关闭文件
- 编译

所以 Ctrl+Z 不能"撤销保存"。

## 九、编译缓存失效

### WASM 侧

`OmniCompiler` 内部维护增量缓存。`upsert_source()` 或 `remove_source()` 会使缓存失效。

### Rust 侧

`file_list_cache` 按目录 `mtime` 判断文件列表是否变化。如果 `.omni` 文件内容变了但 mtime 没变 → 缓存不失效 → 用旧结果。

## 十、componentProtocol.ts 中的 defaultParams

```typescript
button: {
  recommendedVariant: {
    designName: "AppDesign",
    variantName: "Button",
    defaultParams: { color: "primary", size: "md" }
  }
}
```

这些 `defaultParams` 是**拖入按钮时的初始 variant 参数**。如果用户打开的项目中 `AppDesign.Button` variant 的 `@default` 不同，编辑器以 `@default` 为准。

## 十一、CSS Scope Pool 机制

`useCssScopePool.ts` 管理画布预览中的 `<style>` 标签：

- 编译输出的 CSS 按来源分 scope：`design`（全局）、`component:XXX`（每个组件）
- 每个 scope 一个 `<style>` 标签
- CSS 内容通过**哈希比较**增量更新：内容不变 → 不替换 DOM 元素
- 这是**无闪烁渲染**的关键——避免每次编译都导致 FOUC

## 十二、元素选中的 data-nid

WASM 编译时 `editor_mode: true` 会给每个 DOM 元素注入 `data-nid` 属性：

```html
<button data-nid="el-button-011" class="Button danger lg">...</button>
```

编辑器通过 `data-nid` 做：
- 点击元素 → 反向查找 ElementData
- 选中高亮（outline）
- HUD 定位

nid 格式：`el-{tag}-{序号}`（在编辑器渲染时生成，不是 PID）。

## 十三、保存时 `compileDesignSegments` 损坏多段值 ★

### 问题

在 VariantEditor 中编辑后保存，再次打开文件时，某些属性值出现多余的引号或 `+` 操作符。

### 原因

`design-emitter.ts` 中 `compileDesignSegments()` 对**多段值**（text + var-ref）的处理有陷阱：

```typescript
// 原始 .omni 中的 transition 值（正确）：
transition: ""all " " + token(durationFast);

// parseStyleValue 解析后：
segments: [
  { type: "text", value: '""all " " + ' },  // ← 已含 OmniLang 引号语法 + 末尾的 " + "
  { type: "var-ref", name: "durationFast" }
]

// ❌ 旧 compileDesignSegments：对每个 text segment 再用引号包裹
text segment → `"${s.value}"` → 多余的引号 → 损坏
```

### 正确做法

多段值时 text segment **按原样输出**，只去掉末尾的 ` + `：

```typescript
// 多段值分支
if (s.type === "text") return s.value.replace(/\s*\+\s*$/, "");  // 不去包裹引号
if (s.type === "var-ref") return `token(${s.name})`;
// join: " + "
```

### 如何识别损坏

保存后用 `read_app_file` 或直接打开 `design.omni`，检查多段值：
- 正确：`""all " " + token(durationFast)`（两个引号开头）
- 损坏：`"""all " " + " + token(durationFast)`（三个引号开头，多了 `"` 和多余空格）

### 影响范围

所有包含**字符串拼接的 token 引用**：`transition`、`border` 等属性的多段值。每次保存都会累积损坏（见 §七 磁盘文件 ≠ 编辑器内存状态 — 保存会写回损坏值）。

## 十四、`isPropLocked` 在 `inheritedProps` 未定义时误锁所有输入 ★

### 问题

当卡片组件（SizeCard、SpacingCard 等）在非 VariantEditor 场景中使用时（如 InlineStylePanel），`inheritedProps` 不会被传入，值为 `undefined`。

### 原因

`LockIcon.tsx` L25-27 的 `isPropLocked` 实现：

```typescript
export function isPropLocked(propName: string, inheritedProps?: string[]): boolean {
  return !inheritedProps?.includes(propName);
}

// inheritedProps = undefined 时：
// undefined?.includes("width") → undefined
// !undefined → true  ← 属性被锁！
```

结果：所有 ScrubInput 被包裹在 `<span style="opacity:0.45; pointer-events:none">` 中，完全无法交互。

### 修复

`LockIcon.tsx` L26：加 `if (!inheritedProps) return false` 提前返回。

```typescript
export function isPropLocked(propName: string, inheritedProps?: string[]): boolean {
  if (!inheritedProps) return false;  // ← 未传入 → 不锁
  return !inheritedProps.includes(propName);
}
```

## 十五、`parseStyleAttribute` 的 `split(",")` 截断 CSS 函数值 ★

### 问题

`parser-view.ts` L434 在解析 `style: { }` 属性时，用简单的 `split(",")` 拆分键值对：

```typescript
const semiStyle = styleContent
  .split(",")  // ← 会在 rgba(0,0,0,0.1) 中的逗号处截断！
```

导致含逗号的 CSS 值损坏：
```
原始: boxShadow: "0 4px 6px rgba(0,0,0,0.1)"
截断后 → "0 4px 6px rgba(0"   ← 从第 3 个逗号处断开
```

### 修复

`parser-view.ts` L413-436：替换为 `splitTopLevelCommas()`，只在引号外的逗号处拆分（状态机跟踪引号开关状态）。

## 十六、Monaco Editor 集成陷阱（多个） ★

### 16.1 `@monaco-editor/react` 与 React 19 不兼容

在 Poiesis（React 19 + Vite）中安装 `@monaco-editor/react`，双击节点弹出编辑器时报错：

```
Invalid hook call. Hooks can only be called inside of the body of a function component.
TypeError: Cannot read properties of null (reading 'useState')
```

### 原因

`@monaco-editor/react` v4.7.0 的 peer dependency 声明为 `"react": "^17.0.0 || ^18.0.0"`，**不包含 React 19**。当 React 19 运行时，库内部的 hook 调用因 React 版本不匹配而失败（`useState` 读取到 null）。

### 修复

不使用 `@monaco-editor/react`，直接用 `monaco-editor` 原生 API：

1. **依赖**：`package.json` 只装 `monaco-editor`（不装 `@monaco-editor/react`）
2. **Worker 配置**：在 Vite 中必须用 `?worker` 语法导入 web workers，否则语法高亮不工作：

```typescript
// CodeEditorModal.tsx 顶部（不可省略）
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";

(self as { MonacoEnvironment?: unknown }).MonacoEnvironment = {
  getWorker(_: string, label: string) {
    if (label === "typescript" || label === "javascript") return new tsWorker();
    if (label === "css") return new cssWorker();
    if (label === "html") return new htmlWorker();
    if (label === "json") return new jsonWorker();
    return new editorWorker();
  },
};
```

3. **编辑器初始化**：用 `useRef<HTMLDivElement>` + `useEffect(() => { monaco.editor.create(container, opts) }, [])` 创建，不通过 React 组件树渲染
4. **值的双向同步**：`editor.onDidChangeModelContent()` → `ref.current`，外部值变更时 `editor.setValue()` 同步回编辑器

### 完整文件参考

`poiesis/src/components/canvas/code/CodeEditorModal.tsx`。

### 16.2 `languages.typescript` 在 Monaco 0.55 被标记为 `deprecated`

Monaco 0.55.1 的 TypeScript 类型中，`languages.typescript` 的类型是 `{ deprecated: true }`，**不是**包含 `typescriptDefaults`/`ScriptTarget`/`ModuleResolutionKind` 的命名空间。直接访问 → TS2341 编译错误。

**修复**：用 runtime cast 绕过类型检查 `(languages as Record<string, unknown>).typescript`（见 `omnilang-types.ts` L310 `getTsDefaults()`）。运行时 API 仍可用，只是类型定义移到了 module-level contributions。

**影响**：`addExtraLib()` 注入 `.d.ts`、`setCompilerOptions()` 配置 TS 选项、`CompletionItemKind` 等全部需要通过 cast 访问。

涉及文件：`poiesis/src/services/monaco/omnilang-types.ts` L300-335。

### 16.3 Monarch `cases` 不支持正则表达式

Monarch tokenizer 的 `cases` 块中，case label 是**字面字符串前缀**，不是正则。以下写法会在编辑器打开时直接抛异常：

```typescript
// ❌ 崩溃：Monarch 把 "@(pid|token|...)\\b" 当作字面匹配目标名
cases: {
  "@(pid|token|class|...)\\b": "annotation",
}
```

**错误信息**：`omnilang: the @ match target '(pid|token|...) is not defined`

**修复**：对于所有 `@` 开头的 token，直接用简单规则 `[/@[a-zA-Z_][a-zA-Z0-9_-]*/, "annotation"]`，不需要按前缀细分（见 `omnilang-language.ts` L80）。

涉及文件：`poiesis/src/services/monaco/omnilang-language.ts` L80-96。

### 16.4 `triggerCharacters` 对特殊字符不可靠

`registerCompletionItemProvider` 的 `triggerCharacters: ["|"]` 在 Monaco 0.55 中不触发补全——`|` 被 TypeScript language service 消费或忽略。对于特殊字符（`|`、`(`、`{` 等）的触发，改用**字母别名**（如 `lambda` → `|param| expr`、`arrow` → `() => { }`），比 `triggerCharacters` 更可靠。

涉及文件：`poiesis/src/services/monaco/omnilang-snippets.ts` L46-65。

## 十七、xyflow v12 `useOnSelectionChange` 在元素更新时重置选择 ★

### 问题

在 CodeFlowCanvas 中编辑节点属性（如 FN 节点名称）后，节点选中状态丢失，右侧面板显示 "Select a node to edit"。

### 原因

`@xyflow/react` v12 的 `useOnSelectionChange` 依赖 ReactFlow **内部选择状态**。当 `elements` 因 `setProp` 变化时，ReactFlow 重算节点引用 → 内部选择被清空 → `onChange` 收到空数组 → 覆盖 `selectedNodeIds`。

链条：`setProp → elements 引用变化 → nodes 重算 → ReactFlow 内部选择重置 → useOnSelectionChange 清空 selectedNodeIds`

### 修复

`FlowCanvas.tsx` L142-148：用 `onNodeClick` + `onPaneClick` 手动管理 `selectedNodeIds`，**移除 `useOnSelectionChange`**。

```typescript
onNodeClick={(_event, node) => setSelectedNodeIds([node.id])}
onPaneClick={() => setSelectedNodeIds([])}
```

同时在 nodes 的 `useMemo` 中根据 `selectedNodeIds` 给节点加 `selected` 属性实现视觉高亮。

## 十八、`parser-finalize.ts` 的 `buildParseResult` 漏传 `storeDef`

`parser-finalize.ts` L97-113 的 `buildParseResult()` 需要包含 `storeDef: ctx.storeDef`。缺少这行 → parser 解析出 `StoreDefinition` 但被丢弃 → `App.tsx` 的 `parseResult.storeDef` 永远是 `undefined` → store 文件打开后无数据。

## 十九、Store 文件启动时必须从 `parseResult` 加载

`App.tsx` L26-263 `loadOmniFiles()` 的项目初始化解析循环必须处理 `parseResult.storeDef`。

**关键陷阱 1**：纯 Store 文件（只有 `store { ... }` 块、无 `view { ... }` 块）经过 `parseOmniSource()` 解析后，`designElements` 为 `null`。旧代码 `if (!parseResult.designElements) return null` 会直接丢弃这类文件，导致 `storeDef` 也被忽略。

修复：改为 `if (!parseResult.designElements && !parseResult.storeDef) return null`。

**关键陷阱 2**：`loadOmniFiles()` 原来的执行顺序是先 `openTab()`（触发 React 渲染 → StorePage mount → useEffect 创建空的 default Store），再异步解析文件 → 加载 `storeDef`。此时 StorePage 已用空 Store 覆盖了 `storeDefs[storeName]`，从磁盘解析出来的信号/动作被丢弃。

修复：改为先解析所有文件 → 加载 `storeDef` 到 `storeDefs` → 再 `openTab()`。这样 StorePage 挂载时 `storeDefs[storeName]` 已有完整数据，useEffect 跳过创建。

**关键陷阱 3**：`scanDirs` 必须包含 `"src/stores"` 和 `"src/lib"`，否则在这些目录下的 `.omni` 文件在重载后不会被扫描到（标签页不会出现）。`App.tsx` L27。

**涉及文件**：`App.tsx` L27, L44-49, L76, L104-118, `StorePage.tsx` L44-52

## 二十、ReactFlow `fitView` 单节点过度放大 ★

### 问题

CodeFlowCanvas 中只有一个 DesignPreviewNode（240×360）时，`fitView` 会把 viewport 放大到 ~2x 来填满画布区域，导致预览节点变成巨型浮窗。

### 原因

`@xyflow/react` 的 `fitView` 默认让所有节点填满视口。当画布里只有一个小节点时，缩放倍率 = `viewportSize / nodeSize`，可达 2~3x。

### 修复

`FlowCanvas.tsx` L336：加 `fitViewOptions={{ maxZoom: 1 }}` 限制自动适配的最高缩放倍率。代码节点多时 fitView 仍可缩小以展示全貌（maxZoom 只限制放大，不影响缩小）。

### 通用原则

任何时候在 ReactFlow 中有一个尺寸固定的占位/预览节点，都应该设 `fitViewOptions.maxZoom` 防止它被放大。值取 `1` 即可（保持节点实际尺寸），或取 `1.5` 允许适度放大。

## 二十一、内联样式锁是属性级的，不是 section 级的 ★

### 旧机制（已废弃）

`InlineStylePanel.tsx` 曾使用 section 级 🔒/🔓 开关：每个分组（尺寸/间距/背景/边框/圆角/文字）一把锁。解锁一个 section → 该 section 内所有属性（如间距的 padding×6 + margin×6 = 12 个）全部可编辑。问题：只想改 `margin-bottom` 却被迫解锁整个间距卡片，空属性显示 `0px` 易误触导致覆盖 class 样式。

### 新机制（当前）

`InlineStylePanel.tsx` 不再有 section 锁。所有卡片始终展示，每个属性独立判断：

| 状态 | 显示 | 行为 |
|------|------|------|
| 属性值空（无 inline 值） | 灰显 `opacity: 0.4`，ScrubInput 可拖拽 | 从 class/variant 继承，拖拽即设置 inline 值 |
| 属性值存在 | 正常亮起 + 右侧 × 按钮 | 点击 × → 清除该属性 → 回到空/灰显状态 |

**关键**：每个属性独立——`padding` 有值但 `margin` 为空时，padding 亮起+×，margin 灰显。不再有"解锁一个 section 就全亮"的问题。

### 涉及文件

- `InlineStylePanel.tsx` L1-124 — 移除 `unlocked`/`LockedSection`/`lock`/`unlock`，始终渲染所有卡片
- `SpacingCard.tsx` L54-96 `SpacingValue` + L270-290 统一简写 — 空值 `opacity:0.4` + × 清除
- `SizeCard.tsx` L204-240 — W/H 同理
- `BackgroundCard.tsx` L91-143 — 背景色同理
- `BorderCard.tsx` L175-227 — 粗细/样式/颜色同理
- `RadiusCard.tsx` L243-261 + L338-372 — 圆角 + CornerInput 同理
- `TypographyBasics.tsx` L169-197 + L217-250 — 字号/颜色同理

## 二十二、Code 面板 `useMemo` + `getState()` 不响应 Store 变化 ★

### 问题

Code 面板（`DesignPage.tsx` L83-118）用 `useMemo` 生成 `.omni` 源码预览，但依赖数组只放了 `[showCode]`。打开 Code 面板后编辑画布、切换 Tab，代码内容不更新——必须关闭再打开。

### 原因

`useMemo` 内部用 `usePoiesisStore.getState()` 读取 Store 快照。`getState()` 不是响应式的——它只是读取当前值，不会让 `useMemo`"知道"数据变了。`useMemo` 的重新计算只由 **依赖数组** 驱动，而 `[showCode]` 只在打开/关闭抽屉时变化。

```typescript
// ❌ getState() 不会触发 useMemo 重新计算
const codeSource = useMemo(() => {
  const state = usePoiesisStore.getState();  // 静态快照
  // ... 生成代码 ...
}, [showCode]);  // ← 只有 showCode 变化时才重算

// ✅ 用 Zustand selector 订阅依赖的数据源
const elements = usePoiesisStore(s => s.elements);
const activeDesign = usePoiesisStore(s => s.activeDesign);
// ... 其他依赖 ...
const codeSource = useMemo(() => {
  if (!showCode) return "";  // 关闭时不计算
  const state = usePoiesisStore.getState();  // 读取最新值
  // ... 生成代码 ...
}, [showCode, elements, activeDesign, ...]);  // ← 数据变 → 重算
```

### 涉及文件

`DesignPage.tsx` L83-118 — `codeSource` + `codeFileName` 的 useMemo 定义。

## 二十三、Codegen `defaultValue` 双来源 + 类型感知格式化 ★

### 问题

Store 中 signal 的 `defaultValue` 有两个来源，格式不一致：

| 来源 | `signal name: String = "all"` 的 defaultValue | `signal count: i32 = 0` 的 defaultValue |
|------|-----------------------------------------------|----------------------------------------|
| Parser (`collectDefaultValue()` L661-669) | `"all"`（含引号，token 原始文本） | `0` |
| StorePage UI（`StoreMembersPanel.tsx` L251-253） | `all`（用户输入的原始文本，无引号） | `0` |

旧 emitter 直接用 `${sig.defaultValue}` 拼接，导致 UI 创建/编辑的 String signal 生成 `= all` 而非 `= "all"`。

### 修复

新增 `formatDefaultValue(value, type)`（`source-emitter.ts` L21-42），类型感知地格式化默认值：

- `String` 或 `String?` → 包裹双引号（已含引号则不重复）
- 其他类型（`i32`, `bool`, `Array<...>`, struct 字面量）→ 原样输出

应用于 5 个发射位置：
`source-emitter.ts` L78, L198, L265, L274 + `pipeline.ts` L463。

### 涉及文件

- `source-emitter.ts` L21-42 — `formatDefaultValue()` 定义
- `source-emitter.ts` L78 — `emitStoreDefinition` standalone（Code 面板 store 路径）
- `pipeline.ts` L463 — `emitStoreBlock`

## 二十四、内联 Store 的 Key 大小写陷阱 ★

### 问题

内联 Store 保存在组件 `.omni` 文件中，保存成功、磁盘文件也有内容。但刷新/重开后 Store 面板显示"还没有 Store"。

### 原因

Parser 和 Zustand 对 inline store 的 key 使用了**不同的 componentName**：

```
Parser (parser.ts L535-537):
  ctx.currentComponent = "Main"       // 来自源码 component Main { ... }
  → inlineStoreDefs["Main.TestStore"]

Zustand (StoreDrawer.tsx, useSaveProject.ts):
  activeTab.componentName = "main"    // 来自文件名 main.omni
  → inlineStoreDefs["main.TestStore"]

加载时 key 不匹配 → 查不到 → 面板显示空
```

### 为什么会出现大小写差异

`source-emitter.ts emitComponent()` 中组件名首字母大写：`const pascalName = comp.name.charAt(0).toUpperCase() + comp.name.slice(1)`。所以 disk 上 `component main {`→`component Main {`。Parser 读到 `Main`，但 tab 的 `componentName` 来自 `fileName.replace(/\.omni$/, "")` = `"main"`。

### 修复

加载 inlineStoreDefs 的三处代码全部 re-key：提取 store 短名（`def.name` 中 `.` 之后的部分），用 `componentName`（来自 tab/filename）重建 key。

```
App.tsx L104-118:
  const dotIdx = def.name.indexOf(".");
  const storeName = dotIdx >= 0 ? def.name.slice(dotIdx + 1) : def.name;
  const rekeyed = { ...def, name: `${p.componentName}.${storeName}` };
  loadInlineStoreDef(rekeyed);
```

同样修复在 `useProjectInit.ts` L199+ 和 `DesignTabBar.tsx` L206-214。

### 涉及文件

- `parser.ts` L535-537 — 构建 qualified name
- `source-emitter.ts` L250-255 — emitComponent 首字母大写
- `App.tsx` L104-118 — 加载时 re-key
- `useProjectInit.ts` L199+ — 加载时 re-key
- `DesignTabBar.tsx` L206-214 — 加载时 re-key

### 注解格式混淆

**陷阱**：旧代码中 `// @token id=tk_1`、`// @class id=cls_1`、行尾 `// @pid:xxx` 已废弃。
**正确**：所有注解统一为 `// @pid:xxx`独占一行，在声明之上。

**陷阱**：手动写 omni 时忘记 `// @<type>`。只有 fn/action body 内部需要。
**正确**：component / store / design / view / style / `<tag>` / `@if` / `@for` 等顶层/模板构造有自明关键词，不需要标注。只在 body 内部的代码语句前加标注。

## 二十五、`serde_wasm_bindgen` 把 `HashMap` 序列化为 JS `Map` ★

### 问题

WASM 编译函数返回 `InternalCompileResult { files: FxHashMap<String, String> }`。TS 代码用 `result.files['index.js']` 读取 → 返回 `undefined`。`Object.keys(result.files)` → `[]`。

### 原因

`serde_wasm_bindgen::to_value()` 把 Rust `HashMap`/`FxHashMap` 序列化为 **JavaScript `Map`** 对象，不是普通 Object：

```javascript
const m = new Map(); m.set('a', 1);
m['a']     // → undefined ❌
m.get('a') // → 1 ✅
Object.keys(m) // → []，Map 的 key 不可枚举
JSON.stringify(m) // → "{}"
```

`OmniViewResult.files` 是 struct（`{js, html, css}`），序列化为普通 Object → 括号记法有效。
`InternalCompileResult.files` 是 `HashMap<String, String>` → JS Map → 括号记法无效。

### 修复

TS 侧统一处理两种类型（`rustCompiler.ts` L336-341）：

```typescript
const getFile = (key: string): string => {
  if (files instanceof Map) {
    return (files as Map<string, string>).get(key) || '';
  }
  return (files as Record<string, string>)[key] || '';
};
```

同理 `result.components`、`result.nid_map` 等所有 HashMap 字段都需要做 Map 检查。

### 涉及文件

- `wasm.rs` L168-218 — `compile_multi_to_omniview` 返回 `InternalCompileResult`
- `rustCompiler.ts` L305-352 — `compileForDebugWindow` 中的 getFile 和 components Map 处理

## 二十六、Tauri v2 跨窗口事件必须指定 `target: { kind: 'Any' }` ★

### 问题

Debug 窗口 `index.html` 中用裸 `window.__TAURI_INTERNALS__.invoke('plugin:event|listen', { event: '...', handler: ... })` 监听事件，Tauri v2 返回 `"invalid args: command listen missing required key target"`。

### 原因

Tauri v2 的 `plugin:event|listen` 要求显式指定 `target` 字段。省略 target 时 Tauri 不知道要监听哪个窗口的事件。

### 修复

```javascript
// ❌ Tauri v1 风格（v2 报错）
internals.invoke('plugin:event|listen', {
  event: 'debug:compile-result',
  handler: callbackId
});

// ✅ Tauri v2
internals.invoke('plugin:event|listen', {
  event: 'debug:compile-result',
  target: { kind: 'Any' },
  handler: callbackId
});
```

可选 target 值：`{ kind: 'Any' }`（任意窗口）、`{ kind: 'Window', label: 'main' }`（指定窗口）、`{ kind: 'App' }`（App 级事件）。

### 注意

`@tauri-apps/api/event` 的 `listen()` 函数不需要手动加 target——库内部自动处理。只有裸 `__TAURI_INTERNALS__.invoke('plugin:event|listen', ...)` 需要。

### 涉及文件

- `debug-window/index.html` L118-125 — debug window 的 listen 调用（裸 invoke）
- `useDebugWindow.ts` L121-130 — 主窗口的 listen（用 `@tauri-apps/api`，不需要 target）

## 二十七、WASM 构建输出 `public/wasm/` 但 Vite 从 `src/wasm/` 加载 ★

### 问题

修改了 `omnilang/src/wasm.rs` → 运行 `build-wasm.sh` → 重启 App → 浏览器仍加载旧 WASM。JS 中新增的函数不存在。

### 原因

`build-wasm.sh` 将 WASM 输出到 `poiesis/public/wasm/`。但 `rustCompiler.ts` L133 用 `import('../wasm/omnilang.js')` 导入，Vite 从 `poiesis/src/wasm/` 解析这个路径。两个目录不同步。

### 修复

`build-wasm.sh` 末尾追加 cp 命令，构建后自动同步：

```bash
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang_bg.wasm" "$REPO_ROOT/poiesis/src/wasm/"
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang.js" "$REPO_ROOT/poiesis/src/wasm/"
```

### 调试

可以用 `md5sum` 对比两个目录的 WASM 文件确认是否同步。

### 涉及文件

- `scripts/build-wasm.sh` L15-18 — 同步逻辑
- `rustCompiler.ts` L133 — `import('../wasm/omnilang.js')` → Vite 解析到 `src/wasm/`

## 二十八、子组件 prop 无默认值 → 父组件需手动 emit signal 声明 ★

### 问题

主组件 `<MyCard id="mycard-inst-..."/>` 传入 `id` prop，MyCard 组件的 `.omni` 源码中未声明该 prop（无 `props { id: string }`）。WASM 编译后的 JS 报错 `Uncaught ReferenceError: MyCard_id is not defined`。此错误使后续的 debug bridge 等 JS 全部不执行。

### 原因

`component_html.rs` L134-143 的 prop 绑定生成 `__omni_effect(() => { MyCard_id.set(...) })`。但 signal `MyCard_id` 只在子组件 prop 有**默认值**（`child_comp.prop_defaults`）时才声明（`store.rs` L179-184）。没有默认值的 prop → `MyCard_id` 未声明 → ReferenceError。

### 修复

`component_html.rs` L138-148：对传入子组件的每个 prop，在父组件的 `signal_names` 中注册并用 `push_str` emit `const MyCard_id = __omni_signal("MyCard_id", undefined);` 声明。关键在于这段代码必须在**父组件上下文恢复之后**执行（`self.signal_names = saved_signal_names` 之后），否则注册的 signal 会被恢复操作覆盖。

```rust
if !self.signal_names.contains(&child_prefixed) {
    self.signal_names.insert(child_prefixed.clone());
    self.js.push_str(&format!(
        "const {} = __omni_signal(\"{}\", undefined);\n",
        child_prefixed, child_prefixed
    ));
}
```

### 涉及文件

- `component_html.rs` L107-152 — 上下文保存/恢复 + prop 绑定
- `store.rs` L179-184 — prop_defaults 循环（仅处理有默认值的 prop）

## 二十九、componentDefs 填充 — 必须改 4 个路径，漏一个全丢 ★

### 问题

给 `componentDefs[name]` 添加了新字段（如 `computeds`），在面板编辑后数据正常，但刷新/重开后数据丢失——面板显示 "No computed yet"。

### 原因

`componentDefs` 的数据在 **4 个独立的代码路径**中被填充，每个路径都有自己的逻辑：

| 路径 | 文件 | 触发时机 | 处理范围 |
|------|------|---------|---------|
| 1 | `App.tsx` L238-267 | 项目启动批量解析所有 .omni | main + 非 main 组件 |
| 2 | `useProjectInit.ts` L126-143 | `openProject` 后读取 main.omni | 仅 main |
| 3 | `useProjectInit.ts` L218-235 | main.omni 不存在于 file list 时的 fallback | 仅 main |
| 4 | `component-service.ts` L82-119 | 自定义组件注册文件加载 | 非 main 自定义组件 |

**这 4 个路径互不感知。** 如果你在路径 1 加了 `computeds` 映射但路径 2-4 没加 → 走路径 2 加载时 computeds 为空。如果你修了路径 1-3 但忘了路径 4 → 自定义组件加载后丢 computeds。

### 更糟的：路径 1 对 main 跳过 componentDefs

`App.tsx` L238-254 原来对 `mainPath` 只设置 `activeFileContent`/`activeFilePath`，main 的 componentDefs 完全不走路径 1。必须显式添加 main 的 componentDefs 创建（但不能设 `elements`/`wires`/`rootOrder`，否则 main 会出现在自定义组件面板中）。

### 正确做法

1. 新增了 componentDefs 字段后，检查并更新**全部 4 个路径**
2. 路径 1 对 main：创建 componentDefs 但不含 elements/wires/rootOrder
3. 路径 1 对非 main：保留 `existing?.xxx ?? []` 模式，让已编辑数据不被覆盖
4. 每个路径都从 `ParseResult.xxx` 取对应的解析字段
5. `getUnifiedComponentMeta()`（`store/index.ts` L85-104）会遍历所有 componentDefs 做自定义组件面板——main 和 store 不属于组件面板，通过 `filePath.startsWith("src/components/")` 过滤（L97）

### 涉及文件

- `App.tsx` L225-267 — 批量解析循环（路径 1）
- `useProjectInit.ts` L126-143 — main.omni 正常加载（路径 2）
- `useProjectInit.ts` L218-235 — main.omni fallback 加载（路径 3）
- `component-service.ts` L82-119 — 自定义组件加载（路径 4）
- `store/index.ts` L85-104 — `getUnifiedComponentMeta()` 过滤逻辑

## 三十、useSaveProject 保存时覆盖 componentDefs ★

### 问题

在 ComponentMembersPanel 中编辑完 prop/signal/computed 后点保存 → 数据被清空为 `[]`。

### 原因

`useSaveProject.ts` L165-178 在保存时对非 main 组件更新 `componentDefs[name]`，旧代码直接写死了空数组：

```typescript
// ❌ 旧代码：直接覆盖为空
store.setComponentDef(componentName, {
  ...currentDef,
  signals: [],  // ← 清空面板数据！
  props: [],
  events: [],
  fns: [],
  elements: store.elements,
  wires: store.wires,
});
```

同时 main 组件被 `if (componentName !== "main")` 完全跳过，main 的 componentDefs 从不持久化。

### 修复

- 非 main：用 `currentDef.signals || []`、`currentDef.props || []` 等保留已编辑数据
- main：同样需要更新 componentDefs（新增 `else` 分支）
- 两者都不应覆盖通过面板编辑的字段

### 涉及文件

- `useSaveProject.ts` L165-178 — 保存时的 componentDef 更新逻辑
