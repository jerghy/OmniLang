# DesignCanvas 渲染引擎重设计

> 目标：无限画布 + 统一 WASM 渲染路径 + 所见即编译所得

---

## 1. 现状问题

| # | 问题 | 详情 |
|---|------|------|
| 1 | 双路径渲染 | WASM `resolveStylesMulti` 只输出 CSS class，React 侧 `buildInlineStyleMap` 做 TS fallback，两套风格计算可能不同步 |
| 2 | 无画布能力 | 当前只是 `overflow:auto` 的文档流 div，无缩放/平移 |
| 3 | 无画板概念 | 无法预览不同设备尺寸 |
| 4 | WASM editor_mode 闲置 | Rust 侧已实现完整的编辑器桥接（data-nid、postMessage），但画布没用 |

## 2. 目标

1. **无限画布**：缩放（0.1x–5x）、平移，Ctrl+滚轮缩放，空格拖拽平移
2. **单画板**：在画布中央，手机/平板/桌面尺寸可选，下拉切换
3. **React 驱动 + WASM 同源**：画布预览 = 编译输出，零偏差
4. **无闪烁增量更新**：CSS 按 scope 分池管理，React 按稳定 nid + flat lookup + memo 渲染

## 3. 架构总览

### 3.1 组件树

```
DesignPage
├─ Toolbar / Panels（不变）
└─ InfiniteCanvas                              ← 新
    ├─ CanvasGrid                               ← 新
    ├─ <div transform={scale+translate}>        ← Canvas 变换层
    │   └─ Artboard                             ← 新
    │       ├─ ArtboardHeader                   ← 新（设备选择 + 尺寸）
    │       └─ ArtboardContent                  ← 新（overflow:auto）
    │           ├─ <style scope="design" />
    │           ├─ <style scope="component:App" />
    │           ├─ <style scope="component:..." />
    │           └─ OmniViewElementTree          ← 重构
    │               └─ OmniViewElement          ← 保留核心逻辑
    ├─ ZoomControls                             ← 新
    └─ Minimap                                  ← 可选，未来
```

### 3.2 文件布局

```
poiesis/src/components/canvas/
├── InfiniteCanvas.tsx          ← 顶层：pan/zoom 状态管理 + 事件分发
├── CanvasGrid.tsx              ← 无限网格背景
├── Artboard.tsx                ← 画板容器（固定宽高，居中）
├── ArtboardHeader.tsx          ← 设备预设下拉 + 宽高显示
├── ArtboardContent.tsx         ← overflow 容器，文档流渲染
├── canvas-store.ts             ← Canvas 专用 store slice
├── event-hit-test.ts           ← 事件命中判断

poiesis/src/components/design/
├── OmniViewElementTree.tsx     ← 重构：纯渲染，不计算样式
├── OmniViewElement.tsx         ← 保留：React.createElement + className + props
├── useCssScopePool.ts          ← 新：CSS 按 scope 分池增量更新
├── useWasmCompile.ts           ← 新：防抖编译 + 结果缓存
└── DesignCanvas.tsx            ← 删除（被 InfiniteCanvas 替代）
```

### 3.3 数据流

```
elements, wires, tokens, classes, variants (Zustand)
  │
  ├─→ generateOmniFiles() + emitDesignSource()
  │     └─→ .omni source strings (main + stores + design)
  │
  ├─→ WASM compile_to_omniview(sources, editor_mode:true)
  │     └─→ OmniViewOutput {
  │           css,                    → <style scope> 分池注入
  │           component_css,          → 逐组件 scope
  │           element_tree,           → React 逐节点渲染
  │           element_classes,        → className
  │           element_tags,           → HTML tag
  │         }
  │
  └─→ React tree:
        ├─ <style> 按哈希增量更新，不变不替换
        └─ OmniViewElementTree 按 nid + memo 渲染
```

## 4. WASM 侧改动（最小）

### 4.1 OmniViewOutput 新增字段

```rust
pub struct OmniViewOutput {
    // ... 现有字段不变 ...
    pub element_tree: Vec<ElementTreeNode>,
}

pub struct ElementTreeNode {
    pub nid: String,                   // data-nid
    pub tag: String,                   // HTML tag
    pub text: Option<String>,          // 文本内容（叶子节点）
    pub children: Vec<ElementTreeNode>,
}
```

`element_tree` 在现有 `generate()` 的 HTML walk 中附带收集，不是新编译路径。`element_classes` / `element_tags` / `css` / `per_component_css` 全部保留。

### 4.2 复用原则

- 不新增 WASM 函数，复用 `compile_to_omniview` / `OmniCompiler`
- 不新增编译管线，`codegen::omniview::generate()` 不变
- 仅在现有 tree walk 中附加记录 ElementTreeNode

## 5. React 侧渲染

### 5.1 OmniViewElementTree

```typescript
function OmniViewElementTree({ tree, classMap, tagMap, selectedNid, onSelect, onHover }: Props) {
  return tree.map(node => (
    <OmniViewElement
      key={node.nid}
      node={node}
      tag={tagMap[node.nid]}
      className={classMap[node.nid]?.join(' ')}
      isSelected={node.nid === selectedNid}
      onClick={(e) => { e.stopPropagation(); onSelect(node.nid); }}
    />
  ));
}
```

### 5.2 OmniViewElement

```typescript
const OmniViewElement = React.memo(function OmniViewElement({
  node, tag, className, isSelected, onClick
}: Props) {
  const style: CSSProperties = {
    outline: isSelected ? "2px solid #7C3AED" : "none",
    outlineOffset: "-2px",
    cursor: "pointer",
  };

  const childNodes = node.children;

  return React.createElement(
    tag,
    {
      'data-nid': node.nid,
      className,
      style,
      onClick,
    },
    node.text,
    ...childNodes.map(child => (
      <OmniViewElement key={child.nid} node={child} ... />
    )),
  );
});
```

### 5.3 为什么不会闪烁

| 层级 | 机制 | 效果 |
|---|---|---|
| nid 稳定性 | nid 由元素 ID 确定性生成，同元素同 nid | React key 不变 → 保留 DOM |
| React.memo | OmniViewElement 用 React.memo，浅比较 props | 父元素不变时子元素跳过渲染 |
| Flat lookup | childNids 不变时子节点数组引用不变 | memo 命中率接近 100% |
| CSS scope pool | 按组件 scope 分池，哈希比较 | 只替换变化的 style 元素，不闪烁 |

## 6. CSS Scope Pool

### 6.1 方案

```html
<style data-css-scope="design" />           ← Design token/class 变了才替换
<style data-css-scope="component:App" />    ← App 组件变了才替换
<style data-css-scope="component:Card" />   ← Card 子组件变了才替换
```

### 6.2 更新逻辑

```typescript
function useCssScopePool(wasmOutput: OmniViewOutput) {
  // design_css — 哈希比较
  const designHash = hashStr(wasmOutput.css);
  if (designHash !== lastDesignHash.current) {
    upsertScopeStyle('design', wasmOutput.css);
  }

  // component_css — 逐组件比较
  for (const [name, css] of Object.entries(wasmOutput.component_css)) {
    const hash = hashStr(css);
    if (hash !== lastCompHash.current[name]) {
      upsertScopeStyle(`component:${name}`, css);
    }
  }

  // 移除已删除的组件 scope
  for (const name of Object.keys(lastCompHash.current)) {
    if (!wasmOutput.component_css[name]) removeScopeStyle(`component:${name}`);
  }
}
```

### 6.3 更新粒度

| 变化范围 | 更新范围 | 闪烁风险 |
|---|---|---|
| 改元素 props（label、src） | 0 个 style | 无 |
| 改元素 classRefs | 仅 design scope（class 定义变才更新） | 无 |
| 改组件 style {} 块 | 仅该组件的 component scope | 极低 |
| 改 design token 值 | 仅 design scope | 低（CSS 变量原生支持过渡） |
| 改 design class 属性 | 仅 design scope | 低 |
| 添加/删除子组件 | 对应 scope 增删 | 极低 |

## 7. nid ↔ elementId 映射

### 7.1 映射原理

nid 由 WASM 在 tree walk 中按遍历顺序分配（`n0`、`n1`、`n2`...），与元素 ID 的对应关系通过 `sourceMap` 建立：

```
sourceMap: [{ node_id: "abc123", nid: "n0", ... }, ...]
```

### 7.2 选中流程

```
用户点击元素
  → data-nid="n0"
    → sourceMap 查 n0 → node_id "abc123"
      → store.selectElement("abc123")
        → 右侧属性面板显示该元素属性
```

这个映射在 React 侧用 `useMemo` 构建一次：`nidToElementId: Map<string, string>`。

## 8. 编译触发策略

### 8.1 useWasmCompile Hook

```typescript
function useWasmCompile(): {
  output: OmniViewOutput | null;   // 最新编译结果，null = 未就绪
  isCompiling: boolean;            // 编译中，保留上一帧
  errors: Diagnostic[];            // 编译错误
} {
  // 内部：
  //   1. 监听 store elements/wires/tokens/classes/variants 变化
  //   2. 150ms 防抖后调用 generateOmniFiles + compile_to_omniview
  //   3. 编译期间 isCompiling=true，output 保留上次结果
  //   4. 编译成功后原子替换 output
  //   5. 编译失败 → errors 更新，output 保留上次成功结果
}
```

### 8.2 防抖全量编译

```
用户操作 → Store 更新（即时）→ 150ms 防抖 → WASM 编译 → 更新 React
```

- 编译期间保留上一帧结果，原子替换
- `OmniCompiler` 类自带缓存，源码不变时 zero-cost
- 编译失败：保留上次成功结果 + 错误提示

### 8.3 后续优化方向（不在本期范围）

- `OmniCompiler.upsert_source` 增量编译（仅更新变化的 .omni 文件）
- 组件级 Hot Module Replacement

## 9. Canvas Store Slice

```typescript
interface CanvasState {
  zoom: number;              // 0.1 ~ 5.0，默认 1.0
  panX: number;
  panY: number;
  devicePreset: DevicePreset;
  artboardScrollTop: number;
  artboardScrollLeft: number;
}

type DevicePreset = {
  name: string;              // "iPhone 15" | "iPad" | "Desktop" | "Custom"
  width: number;
  height: number;            // 0 = 自适应内容高度
};

const DEVICE_PRESETS: DevicePreset[] = [
  { name: "iPhone 15", width: 393, height: 852 },
  { name: "Pixel 8", width: 412, height: 915 },
  { name: "iPad", width: 768, height: 1024 },
  { name: "iPad Pro", width: 1024, height: 1366 },
  { name: "Desktop", width: 1440, height: 900 },
  { name: "Full HD", width: 1920, height: 1080 },
  { name: "Custom", width: 0, height: 0 },
];
```

### 9.1 缩放行为

- 等比例缩放：CSS `transform: scale(zoom)` + `transform-origin: 0 0`
- 以光标为中心缩放
- Fit 按钮：`Math.min(viewportW / artboardW, viewportH / artboardH) * 0.9`
- 范围限制：zoom [0.1, 5.0]

### 9.2 持久化

复用已有 `canvasStates: Record<string, { zoom, panX, panY }>`，新增 `devicePreset` 和 `artboardScroll`。

## 10. 事件分发

```
wheel
  ├─ Ctrl/Meta 按下 → Canvas zoom（以光标为中心）
  └─ 无修饰键 → Artboard 自然滚动（overflow:auto）

pointerdown
  ├─ 命中 Artboard 内元素 → 选中（透传 onClick）
  ├─ 命中 Artboard 但空白区 → Canvas pan（space+拖拽 或 中键拖拽）
  └─ 命中 Artboard 外 → Canvas pan
```

## 11. 删掉的代码

| 文件 | 原因 |
|---|---|
| `DesignCanvas.tsx` | 被 `InfiniteCanvas` 替代 |
| `render-utils.ts` 中 `buildInlineStyleMap` | 不再有 TS 样式计算 |
| `OmniViewRenderer.tsx` 中双路径逻辑 | 只剩单一 WASM 路径 |
| `expand-custom-components.ts` 画布调用 | 组件展开由 WASM 编译自然处理 |
| SelectionOverlay 方案 | 选中/高亮直接在 React 组件上处理 |

## 12. 实施顺序

| 阶段 | 内容 | 风险 |
|---|---|---|
| Phase 1 | WASM 侧加 `element_tree` 字段 | 低：只在现有 walk 中附加记录 |
| Phase 2 | `OmniViewElementTree` + `OmniViewElement` 重构 | 中：需要保证渲染结果等价 |
| Phase 3 | `useCssScopePool` + 去掉 TS fallback | 中：CSS 行为需逐项验证 |
| Phase 4 | `InfiniteCanvas` + `Artboard` + `ZoomControls` | 中：事件分发逻辑 |
| Phase 5 | Canvas store slice + 持久化 | 低 |
| Phase 6 | 删除旧代码 + 清理 | 低 |

每个 Phase 完成后独立验证编译输出与旧版一致。
