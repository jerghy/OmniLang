# Element Reference System — Design Document

**Status**: Design | **Date**: 2026-07-29 | **Related**: ADR-028, ADR-034

## Motivation

OmniView needs a low-level primitive for high-frequency element manipulation — drag, mousemove-driven updates, resize handles, scroll-linked effects. The existing signal system triggers full reconciliation on every change, which is inappropriate for 60fps DOM updates.

The design follows the standard React pattern: **direct DOM manipulation during the high-frequency phase, signal/state update only at the end.**

## Current Implementation Status

| Layer | Status | Details |
|-------|--------|---------|
| Parser: `ref name: Element` declaration | ✅ Done | `parse_ref_decl()` in `top_level/component.rs:320` |
| Parser: `ref: name` on elements | ✅ Done | `view/element.rs:114-120` |
| AST: `RefDecl { name, type_ann }` | ✅ Done | `ast/view.rs:60-63` |
| AST: `ElementProp::Ref(String)` | ✅ Done | `ast/view.rs:143` |
| AST: `ComponentDef.refs: Vec<RefDecl>` | ✅ Done | `ast/view.rs:19` |
| IR: `RefDeclIR { name }` | ✅ Done | `ir/types.rs:702-704` |
| IR: `ComponentIR.refs: Vec<RefDeclIR>` | ✅ Done | `ir/types.rs:736` |
| IR: `ElementIR.ref_binding` | ❌ Missing | Needs new field |
| IR lowering: `RefDecl` → `RefDeclIR` | ✅ Done | `ir/lower/component.rs:249-255` |
| IR lowering: `ElementProp::Ref` → `ref_binding` | ❌ Broken | Currently lowered to `data-ref` attr |
| Codegen: OmniView | ❌ Missing | |
| Codegen: React | ❌ Missing | No `useRef` anywhere |
| Codegen: Flutter | ❌ Missing | |

## Language Syntax

### Declaration

```omni
component Draggable {
  signal x: Float = 0
  signal y: Float = 0

  ref box: Element        ← 声明一个类型为 Element 的引用
  ref list: Element       ← 可以有多个 ref

  <div @ref={box}>        ← 绑定到元素（@ref 属性）
    ...
  </div>
}
```

**规则**：
- `ref name: Element` — 组件级声明，与 signal/computed/fn 同级
- `@ref={name}` — 元素属性，绑定声明到 DOM 元素
- 每个 ref 必须恰好绑定一个元素（编译时检查）
- `ref` 不在 signal 反应系统中——修改 ref 的属性不会触发 reconciliation

### Ref API Surface

框架暴露一个跨后端的安全 API：

```omni
ref box: Element

// 属性读取
box.style                    // CSSStyleDeclaration (Web) / RenderBox (Flutter)
box.getBoundingClientRect()  // { x, y, width, height, top, right, bottom, left }

// 指针捕获（拖拽用）
box.capturePointer(pointerId)
box.releasePointer(pointerId)

// 滚动
box.scrollTo(x, y)
box.scrollBy(dx, dy)

// 焦点
box.focus()
box.blur()

// 尺寸
box.offsetWidth
box.offsetHeight
```

不暴露 `innerHTML`、`querySelector`、`addEventListener` —— 保持后端无关性。

### 使用模式

**拖拽移动元素（核心场景）**：

```omni
component Draggable {
  signal x: Float = 0
  signal y: Float = 0
  signal dragging: Bool = false

  ref box: Element

  fn startDrag(e) {
    dragging = true
    box.capturePointer(e.pointerId)
  }

  fn onDrag(e) {
    if !dragging { return }
    // 高频：直接操作 DOM style — 不触发 reconciliation
    box.style.transform = "translate({e.clientX}px, {e.clientY}px)"
  }

  fn endDrag(e) {
    dragging = false
    box.releasePointer(e.pointerId)
    // 最后一刻才更新 signal
    x = e.clientX
    y = e.clientY
  }

  <div
    @ref={box}
    @onPointerDown={startDrag}
    @onPointerMove={onDrag}
    @onPointerUp={endDrag}
    style:position="absolute"
    style:transform="translate({x}px, {y}px)"
  >
    Drag me
  </div>
}
```

**关键点**：拖拽期间 `box.style.transform = ...` 是直接 DOM 写入——绕过了 `style:transform="translate({x}px, {y}px)"` 的 signal 绑定。拖拽结束后 `x = e.clientX` 才更新 signal，React reconciliation 才会把元素放到最终位置。

## IR Design

### ElementIR 新增字段

```rust
// crates/omnilang-core/src/ir/types.rs

pub struct ElementIR {
    // ... 现有字段 ...
    /// Element ref binding name — connects this element to a component-level
    /// ref declaration. `None` means no ref bound. Maps to:
    ///   OmniView → __omni_el(nid) wrapper stored in a closure variable
    ///   React    → useRef + ref={} JSX attribute
    ///   Flutter  → GlobalKey
    #[serde(default)]
    pub ref_binding: Option<String>,
}
```

### IR Lowering 修复

```rust
// crates/omnilang-core/src/ir/lower/view.rs

// BEFORE (broken — lowers ref to data attribute):
ElementProp::Ref(r) => {
    props.get_or_insert_with(Default::default).insert(
        "data-ref".into(),
        ExprIR::LiteralExpr {
            value: serde_json::Value::String(r.clone()),
        },
    );
}

// AFTER (correct — sets ref_binding on the element):
ElementProp::Ref(r) => {
    el.ref_binding = Some(r.clone());
}
```

### Semantic 验证

新增编译时检查（`semantic/validate_component.rs`）：
1. `@ref={name}` 中的 `name` 必须在 `ComponentIR.refs` 中有对应的 `RefDeclIR`
2. 每个 `RefDeclIR` 必须恰好被一个元素引用
3. `ref` 声明不能与 signal/computed/fn 同名

## Codegen: React (useRef)

### 设计目标

将 `ref box: Element` + `<div @ref={box}>` 编译为：

```tsx
const boxRef = useRef<HTMLDivElement>(null);

// JSX:
<div ref={boxRef} ...>

// 事件 handler 中使用:
boxRef.current.style.transform = `...`;
boxRef.current.getBoundingClientRect();
```

### 变更文件

| 文件 | 变更 |
|------|------|
| `component/mod.rs` | hook 选择逻辑：检测 `comp.refs` → push `"useRef"` |
| `component/imports.rs` | 已有通用 hooks Vec，无需改动 |
| `view.rs` | JSX 生成：`@ref={name}` → `ref={nameRef}` |
| `component/state.rs` | **新增**：生成 `useRef` 声明 |
| `expr.rs` | `ref.method()` → `refNameRef.current.method()` |
| `component/mod.rs` | identifier collection: ref names → 不标记为 signal |

### 详细实现

#### Step 1: Hook 选择 (`component/mod.rs:84-96`)

```rust
// 现有逻辑
let mut hooks: Vec<String> = Vec::new();
if has_mutable_states { hooks.push("useState".to_string()); }
if has_computeds      { hooks.push("useMemo".to_string()); }
if has_fn_form_computed { hooks.push("useCallback".to_string()); }
if has_effects_or_lifecycles || has_motion { hooks.push("useEffect".to_string()); }

// 新增：ref 声明 → useRef
let has_refs = !comp.refs.is_empty();
if has_refs {
    hooks.push("useRef".to_string());
}
```

#### Step 2: Ref 声明生成 (`component/state.rs` — 新函数)

在 `gen_state_hooks` 中或作为独立函数：

```rust
pub(super) fn gen_ref_hooks(
    w: &mut CodeWriter,
    comp: &ComponentIR,
) {
    for ref_decl in &comp.refs {
        // const boxRef = useRef<HTMLDivElement>(null);
        w.emit_fmt(format_args!(
            "const {}Ref = useRef<HTMLDivElement>(null);",
            ref_decl.name
        ));
    }
    if !comp.refs.is_empty() {
        w.emit("");
    }
}
```

调用位置：`component/mod.rs` — 在 state hooks 之后、computed values 之前：

```rust
self.gen_state_hooks(&mut w, comp, &read_signals, &mutated_signals, &fn_names);
self.gen_ref_hooks(&mut w, comp);   // ← 新增
self.gen_effects(&mut w, comp, &store_signal_map, &signal_names);
```

#### Step 3: JSX `ref` 属性 (`view.rs`)

在 `gen_view_node_tsx` 的 Element 分支中，构建 attrs 时：

```rust
// 现有: HTML 元素属性生成
// 新增: ref binding → ref={nameRef}
if let Some(ref ref_name) = el.ref_binding {
    attrs.push(format!("ref={{{}Ref}}", ref_name));
}
```

**重要**：对于原生 HTML 元素，`ref={boxRef}` 绑定到 DOM 元素。对于自定义组件（已在 `component_map` 中的 tag），需要 `forwardRef` —— **Phase 1 先不支持组件 ref，仅支持原生 HTML 元素。**

#### Step 4: 表达式中的 ref 访问 (`expr.rs`)

OmniLang 中 `box.style.transform = "..."` 需要翻译为 `boxRef.current.style.transform = "..."`。

在 `format_expr_ir_tsx` 或表达式 walker 中：

```rust
// 当遇到标识符是 ref 名称时
fn is_ref_name(name: &str, comp: &ComponentIR) -> bool {
    comp.refs.iter().any(|r| r.name == name)
}

// FieldAccessExpr: box.style → boxRef.current.style
// MethodCallExpr: box.getBoundingClientRect() → boxRef.current.getBoundingClientRect()
// 模式: ref_name → ref_name + "Ref.current"
```

**核心转换规则**：

| OmniLang | React JSX |
|----------|-----------|
| `box` | `boxRef.current` |
| `box.style` | `boxRef.current.style` |
| `box.style.transform = "..."` | `boxRef.current.style.transform = "..."` |
| `box.getBoundingClientRect()` | `boxRef.current.getBoundingClientRect()` |
| `box.capturePointer(id)` | `boxRef.current.setPointerCapture(id)` |
| `box.focus()` | `boxRef.current.focus()` |

#### Step 5: Identifier collection 排除 ref

在 `collect_expr_identifiers` 和 `collect_stmt_identifiers` 中，ref 名称不应被标记为 signal 读取——否则会导致不必要的 `useState`：

```rust
// 在 component/mod.rs 的 read_signals 构建中
// ref 名称不应出现在 signal_names 中
let ref_names: FxHashSet<String> = comp.refs.iter().map(|r| r.name.clone()).collect();
// ... 在后续逻辑中排除 ref_names
```

### React useRef 实现总结

```
用户写:                         编译为:
───────────                    ───────────
ref box: Element           →   const boxRef = useRef<HTMLDivElement>(null);
<div @ref={box}>           →   <div ref={boxRef}>
box.style.transform = "x"  →   boxRef.current.style.transform = "x";
box.getBoundingClientRect() →  boxRef.current.getBoundingClientRect();
box.capturePointer(id)     →   boxRef.current.setPointerCapture(id);
```

## Codegen: OmniView (__omni_el wrapper)

将 `ref box: Element` + `<div @ref={box}>` 编译为：

```js
// 对每个 ref 声明生成 wrapper 对象
const box = (function() {
  const nid = "el-div-001";
  const el = function() { return __omni_el(nid); };
  return {
    get style() { const e = el(); return e ? e.style : null; },
    capturePointer(id) { const e = el(); if (e) e.setPointerCapture(id); },
    releasePointer(id) { const e = el(); if (e) e.releasePointer(id); },
    getBoundingClientRect() { const e = el(); return e ? e.getBoundingClientRect() : null; },
    focus() { const e = el(); if (e) e.focus(); },
    blur() { const e = el(); if (e) e.blur(); },
    get offsetWidth() { const e = el(); return e ? e.offsetWidth : 0; },
    get offsetHeight() { const e = el(); return e ? e.offsetHeight : 0; },
  };
})();
```

**关键**：使用 lazy getter + `__omni_el()` 缓存——拖拽期间每帧调用 `box.style` 时都是 O(1) 的缓存查找。

## Codegen: Flutter (GlobalKey)

已实施。`ref box: Element` → `GlobalKey` 绑定；ref API（scrollTo/focus/measure 等）经 `_omni_measure` / 生命周期直调映射。

**动态 style 绑定已表达式化**（Phase 3，`docs/design/ref-high-frequency-direct-channel.md` §6）：`style: { transform: "translate(" + dragX + "px, " + dragY + "px)" }` 不再**静默跳过**——D1/D2 信号存储为 `ValueNotifier<T>` 字段，mutation 直写 `_dragX.value = ...`（零 setState）；元素包裹 `ListenableBuilder(listenable: Listenable.merge([_dragX, _dragY]), ...)`，transform 条目映射 `Transform.translate(offset: Offset(_dragX.value, _dragY.value))`，width/height/padding/margin 映射 widget 参数，不可映射属性编译期显式诊断。局部重建，零整树 build。

## 实现计划

### Phase 1: 核心管线 (估计 3-4 天)

| # | 任务 | 文件 | 估时 |
|---|------|------|------|
| 1 | `ElementIR` 加 `ref_binding` 字段 | `ir/types.rs` | 5 min |
| 2 | 修复 IR lowering: `ElementProp::Ref` → `ref_binding` | `ir/lower/view.rs` | 15 min |
| 3 | Semantic: ref 引用验证 | `semantic/validate_component.rs` | 2h |
| 4 | OmniView codegen: `__omni_el` wrapper | `codegen/omniview/store.rs` | 2h |
| 5 | React: `useRef` hook 选择 + 导入 | `codegen/react/component/mod.rs` | 30 min |
| 6 | React: `useRef` 声明生成 | `codegen/react/component/state.rs` | 1h |
| 7 | React: JSX `ref` 属性 | `codegen/react/view.rs` | 30 min |
| 8 | React: 表达式 ref 转换 | `codegen/react/expr.rs` | 2h |
| 9 | 集成测试: 拖拽端到端 | `tests/` | 3h |

### Phase 2: 完善 (估计 2-3 天)

| # | 任务 |
|---|------|
| 10 | `@for` + `ref` 语义（ref 数组 vs 单元素） |
| 11 | Flutter codegen: `GlobalKey` |
| 12 | 组件 ref forwarding |
| 13 | TypeScript `.d.ts` 中的 ref 类型 |

---

## 与 @drag-* 的关系

`@drag-horizontal` / `@drag-vertical` 保持不变——它们是专用于"手势驱动 WAAPI 动画 scrubbing"的声明式语法糖。新的 `@ref` + `onPointerMove` 是更通用的底层原语。

两者不冲突：
- 想做一个简单的滑动解锁动画 → 用 `@drag-horizontal={slideAnim}`
- 想做一个拖拽移动元素 / resize handle / 自定义拖拽交互 → 用 `@ref` + `onPointerMove`

## 安全性考虑

`ref` API 只暴露受限的方法集——不暴露 `innerHTML`、`addEventListener`、`querySelector`。这防止了：
- XSS（无法通过 ref 注入 HTML）
- 事件监听器泄漏（不能用 ref 绕过框架的事件系统）
- DOM 遍历（不能用 ref 访问兄弟/父元素）

高频写入路径是 **信号 + 声明式 style 绑定**（`style: { transform: ... }`）——`ref.style` 的写仍是编译错误，
迁移提示为 `use style:{...} binding with signals instead`。style 绑定在 OmniView 目标生成
`el.style.setProperty(...)` 直接 DOM 写入（零 reconciliation）；在 React 目标走**直写通道**（Phase 1，
`docs/design/ref-high-frequency-direct-channel.md`）：D1/D2 信号 useRef 存储 + mutation 点直写
（零 reconciliation，与 OmniView 同级），B2 共享信号 useEffect 直写——v2 时代「退化为 useState +
style={{...}} 整组件重渲染」的代价陈述已被直写通道撤销；在 Flutter 目标走**ListenableBuilder 表达式化**
（Phase 3）：动态 style 条目从**静默跳过**（`as_str().unwrap_or("")` 空串）变为 ValueNotifier +
`ListenableBuilder(listenable: Listenable.merge([...]))` + Transform 映射，局部重建零整树 build，
不可映射条目编译期显式诊断。
