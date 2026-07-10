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

## 十六、`@monaco-editor/react` 与 React 19 不兼容 ★

### 问题

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

`App.tsx` L69-97 的项目初始化解析循环只处理 `parseResult.designElements`，不处理 `parseResult.storeDef`。Store 文件打开后不会自动加载。

修复：在 `ParsedFile` 接口加 `storeDef` 字段 → 解析时提取 → 批量 apply 后遍历 `parsed` 调用 `loadStoreDef()` + `registerStorePids()`。
