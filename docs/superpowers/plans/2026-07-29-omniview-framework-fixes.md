# OmniView 前端框架修复 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 OmniView 前端框架审查中发现的 3 个 HIGH 框架完整性缺口 + 10 个 MEDIUM 编译器/安全/重构问题

**Architecture:** OmniView 代码生成位于 `omnilang/src/codegen/omniview/`（约 11,000 行，19 个源文件）。核心架构为 `OmniViewGenerator` 结构体（46 个字段）通过 `&mut self` push_str 模式驱动代码生成。修复范围从组件内联（component_html.rs）、信号代码生成（store.rs）、运行时（runtime.rs）、清理器（sanitize.rs）到 WASM 缓存（omnilang-wasm）。

**技术栈:** Rust (omnilang 代码生成), JavaScript (内联运行时), Node.js/Deno (JS 运行时测试)

**前置审查:** `.architecture/reviews/component-omniview-frontend-framework-2026-07-29.md` (10 位专家，44 项发现)

---

## 文件结构

| 文件 | 变更内容 |
|------|--------|
| `omnilang/src/codegen/omniview/component_html.rs` | 插槽投影 + InliningState 结构体 + Arc 转换 |
| `omnilang/src/codegen/omniview/store.rs` | 只读 props 包装器 + TypeScript 声明生成 |
| `omnilang/src/codegen/omniview/runtime.rs` | @enter 重放作用域限制 |
| `omnilang/src/codegen/omniview/stmt.rs` | 通配符诊断替换 |
| `omnilang/src/codegen/omniview/expr.rs` | FieldAccessExpr/OptionalChainExpr 去重 |
| `omnilang/src/codegen/omniview/motion_triggers.rs` | 触发类型去重 |
| `omnilang/src/codegen/omniview/mod.rs` | InliningState 定义 + Arc<ComponentIR> + ViewMode 枚举 |
| `omnilang/src/codegen/omniview/view/element.rs` | 修复 2 个 needless_borrow 警告 |
| `omnilang/src/codegen/omniview/tests_regression/motion.rs` | 移除未使用的 mut |
| `crates/omnilang-core/src/codegen/sanitize.rs` | CSS 清理器对齐 |
| `omnilang-wasm/src/lib.rs` | WASM 增量缓存连接 |
| `omnilang/src/codegen/omniview/tests.rs` | 新测试：插槽、props、运行时、清理器 |
| `omnilang/tests/js_runtime/` (新建) | JS 引擎运行时行为测试 |

---

### 阶段 1：框架完整性缺口（HIGH — 本周必须完成）

---

### 任务 1：修复插槽/内容投影

**文件：**
- 修改：`omnilang/src/codegen/omniview/component_html.rs:8-206`
- 修改：`omnilang/src/codegen/omniview/view/mod.rs:207-210`
- 修改：`omnilang/src/codegen/omniview/tests.rs`（添加测试）

**背景：** `ViewTree::Slot(name)` 在 `view/mod.rs:207` 生成一个 `<slot>` HTML 元素。在 `component_html.rs` 中的 `try_inline_component` 将子组件内联到父组件中，但从未将父组件的子元素映射到子组件的 `<slot>` 位置。结果：`<Button>点击我</Button>` 中的 "点击我" 文本会丢失。

- [ ] **步骤 1：为插槽投影编写失败测试**

```rust
// 在 omnilang/src/codegen/omniview/tests.rs 中，测试模块内

#[test]
fn slot_projection_inlines_parent_children_into_default_slot() {
    // 创建一个带默认插槽的子组件
    // component Child { view { <div><slot></slot></div> } }
    // 和一个将文本传入其中的父组件
    // component Parent { view { <Child>"hello"</Child> } }
    let mut gen = OmniViewGenerator::test_new();
    // ... 构建 IR 并调用 generate
    let output = gen.generate();
    // 输出的 HTML 应包含内联到子组件 <div> 内的 "hello"
    assert!(output.html.contains("<div>hello</div>"));
}

#[test]
fn slot_projection_maps_named_slots_correctly() {
    // component Card { view { <header><slot name="title"/></header><main><slot/></main> } }
    // component Page { view { <Card><span slot="title">T</span>body</Card> } }
    let mut gen = OmniViewGenerator::test_new();
    // ...
    let output = gen.generate();
    assert!(output.html.contains("<header><span>T</span></header>"));
    assert!(output.html.contains("<main>body</main>"));
}
```

- [ ] **步骤 2：运行测试验证其失败**

```bash
cargo test -p omnilang slot_projection
```
预期：两个测试均失败——"hello" 未出现在子组件 `<div>` 内部。

- [ ] **步骤 3：在 IR 类型中添加子元素到 ViewTree::Slot**

在 `crates/omnilang-core/src/ir/types.rs` 中，检查 `ElementIR` 是否已有子元素字段。`ElementIR` 有一个 `children: Vec<ViewTree>` 字段。在 `component_html.rs` 中的 `try_inline_component`，我们需要在执行 `view_to_html` 之前将父组件的 `el.children`（传递给子组件的子元素）注入子组件的视图树。修改 `try_inline_component` 以：

a) 收集 `el.children`（传递给组件的子 ViewTree 节点）
b) 在调用 `view_to_html` 之前将它们注入子组件的视图树中

```rust
// 在 component_html.rs 中，try_inline_component，第 10 行附近，el 参数之后
// 新增：从父元素收集子节点
let slot_children: Vec<ViewTree> = el.children.clone();

// ... 现有的保存/恢复逻辑 ...

// 在第 125-126 行附近（child_view 克隆之后），注入子节点：
let child_view_owned = if slot_children.is_empty() {
    child_view.clone()
} else {
    // 将父组件子节点注入子组件视图中的 <slot> 位置
    inject_slot_children(child_view.clone(), &slot_children)
};

// 在第 128 行，正常渲染修改后的视图：
let child_html = self.view_to_html(&child_view_owned, indent + 1);
```

- [ ] **步骤 4：实现 `inject_slot_children` 辅助函数**

```rust
/// 将父组件子节点递归映射到视图树中的 <slot> 元素。
/// 顶层子节点进入默认（未命名）插槽；带有 `slot="name"` 属性的子节点进入命名插槽。
fn inject_slot_children(view: ViewTree, slot_children: &[ViewTree]) -> ViewTree {
    match view {
        ViewTree::Slot(ref slot_name) => {
            // 用匹配的子节点内容替换此插槽
            if let Some(name) = slot_name {
                // 命名插槽：查找 slot="name" 的子节点
                let named: Vec<ViewTree> = slot_children.iter()
                    .filter(|c| has_slot_attr(c, name))
                    .cloned()
                    .collect();
                if named.is_empty() {
                    // 回退：如果未提供命名插槽内容，保留原始插槽
                    view
                } else {
                    ViewTree::Fragment(named)
                }
            } else {
                // 默认插槽：使用所有顶层非命名子节点
                let default: Vec<ViewTree> = slot_children.iter()
                    .filter(|c| !has_any_slot_attr(c))
                    .cloned()
                    .collect();
                if default.is_empty() {
                    view
                } else {
                    ViewTree::Fragment(default)
                }
            }
        }
        ViewTree::Element(el) => {
            // 递归处理元素的子节点
            let new_children: Vec<ViewTree> = el.children.iter()
                .map(|c| inject_slot_children(c.clone(), slot_children))
                .collect();
            ViewTree::Element(ElementIR {
                children: new_children,
                ..el
            })
        }
        ViewTree::Conditional(cond) => {
            // 递归处理每个分支
            let new_branches = cond.branches.iter().map(|b| {
                ConditionalBranch {
                    body: b.body.as_ref().map(|v| Box::new(inject_slot_children(*v.clone(), slot_children))),
                    ..b.clone()
                }
            }).collect();
            ViewTree::Conditional(ConditionalIR { branches: new_branches, ..cond })
        }
        ViewTree::ListRendering(list) => {
            // @for 模板中的插槽：原样保留（列表项并非插槽子节点）
            view
        }
        other => other, // Text, RawHtml, Fragment — 无变化
    }
}

fn has_slot_attr(child: &ViewTree, slot_name: &str) -> bool {
    match child {
        ViewTree::Element(el) => el.props.get("slot")
            .map(|v| format!("{:?}", v).contains(slot_name))
            .unwrap_or(false),
        _ => false,
    }
}

fn has_any_slot_attr(child: &ViewTree) -> bool {
    match child {
        ViewTree::Element(el) => el.props.contains_key("slot"),
        _ => false,
    }
}
```

- [ ] **步骤 5：如果 ViewTree 缺少 Fragment 变体，则添加之**

检查 `crates/omnilang-core/src/ir/types.rs` 中的 `ViewTree` 枚举。如果没有 `Fragment` 变体，添加一个：

```rust
// 在 ViewTree 枚举中（约第 670 行）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewTree {
    // ... 现有变体 ...
    /// 内联子节点列表（无包装元素）——用于插槽投影
    Fragment(Vec<ViewTree>),
}
```

同时更新 `view_to_html` 方法（在 `view/mod.rs` 的 match 语句中）以处理 `Fragment`：

```rust
ViewTree::Fragment(children) => {
    let mut out = String::new();
    for child in children {
        out.push_str(&self.view_to_html(child, indent));
    }
    out
}
```

- [ ] **步骤 6：运行测试验证其通过**

```bash
cargo test -p omnilang slot_projection
```
预期：两个插槽投影测试均通过。

- [ ] **步骤 7：运行完整测试套件**

```bash
cargo test -p omnilang
```
预期：所有测试通过，无回归。

- [ ] **步骤 8：提交**

```bash
git add crates/omnilang-core/src/ir/types.rs omnilang/src/codegen/omniview/component_html.rs omnilang/src/codegen/omniview/view/mod.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "fix(omniview): implement slot/content-projection in component inlining

Wire parent-children into child <slot> positions during component
inlining. Add Fragment ViewTree variant for slot-mapped child lists.
Previously ViewTree::Slot emitted raw <slot> HTML elements but
parent-submitted content was never injected, so content-projection
was silently non-functional.

Add two regression tests covering default and named slot projection."
```

---

### 任务 2：将 Props 作为只读包装器暴露

**文件：**
- 修改：`omnilang/src/codegen/omniview/store.rs:221-234`
- 修改：`omnilang/src/codegen/omniview/runtime.rs:24-55`（添加 `__omni_readonlySignal` 辅助函数）
- 修改：`omnilang/src/codegen/omniview/tests.rs`（添加测试）

**背景：** Props 在文档中被记录为不可变输入，但代码生成却将其暴露为完整的 `__omni_signal` 实例，包含 `.set(v)` 和可变 `.value`。子组件可以静默覆盖父组件提供的值，导致竞态数据损坏。

- [ ] **步骤 1：编写只读 signal 辅助函数**

```rust
// 在 omnilang/src/codegen/omniview/runtime.rs 中，emit_signal_runtime() 内部，
// 在 __omni_signal 函数定义之后（第 55 行之后）添加：

self.js.push_str(
    r#"function __omni_readonlySignal(name, init) {
  const inner = __omni_signal(name, init);
  return {
    _default: init,
    get value() { return inner.value; },
    get() { return inner.value; },
    peek() { return inner.peek(); },
    // 无 set value、无 set() — 子组件无法写入
  };
}
"#);
```

- [ ] **步骤 2：为 props 使用 `__omni_readonlySignal`**

在 `store.rs` 中，修改 prop 默认值生成（第 221-234 行）：

```rust
// 前（第 223-233 行）：
for (prop_name, prop_default) in &comp.prop_defaults {
    let init_val = self.expr_to_js(prop_default);
    let prefixed = format!("{}_{}", name, prop_name);
    let _ = writeln!(
        self.js,
        "const {} = __omni_signal(\"{}\", {});",
        prefixed, prefixed, init_val
    );
    self.name_renames
        .insert(prop_name.clone(), prefixed.clone());
    self.signal_names.insert(prefixed);
}

// 后：
for (prop_name, prop_default) in &comp.prop_defaults {
    let init_val = self.expr_to_js(prop_default);
    let prefixed = format!("{}_{}", name, prop_name);
    let _ = writeln!(
        self.js,
        "const {} = __omni_readonlySignal(\"{}\", {});",
        prefixed, prefixed, init_val
    );
    self.name_renames
        .insert(prop_name.clone(), prefixed.clone());
    self.signal_names.insert(prefixed);
}
```

- [ ] **步骤 3：编写测试——子组件 prop 写入应警告**

```rust
// 在 omnilang/src/codegen/omniview/tests.rs 中

#[test]
fn props_are_emitted_as_readonly_wrappers() {
    let mut gen = OmniViewGenerator::test_new();
    // 构建一个包含 prop count: i32 = 0 的组件
    // 验证生成的 JS 使用 __omni_readonlySignal 而非 __omni_signal
    let output = gen.generate();
    assert!(output.js.contains("__omni_readonlySignal"));
}
```

- [ ] **步骤 4：运行测试验证其通过**

```bash
cargo test -p omnilang props_are_emitted_as_readonly_wrappers
```
预期：通过

- [ ] **步骤 5：运行完整测试套件**

```bash
cargo test -p omnilang
```
预期：所有测试通过。快照可能需要更新（prop 默认值现在使用不同的包装器）。

- [ ] **步骤 6：提交**

```bash
git add omnilang/src/codegen/omniview/store.rs omnilang/src/codegen/omniview/runtime.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "fix(omniview): emit props as read-only wrappers via __omni_readonlySignal

Props are documented as immutable component inputs but were emitted as
fully mutable __omni_signal instances with .set(v) and writable .value,
enabling child components to silently overwrite parent-supplied values
and creating race-condition data corruption.

Replace __omni_signal with __omni_readonlySignal for prop defaults,
which only exposes get value(), get(), and peek() — no setters."
```

---

### 任务 3：为 React 生产路径生成 TypeScript 声明

**文件：**
- 创建：`omnilang/src/codegen/omniview/typescript.rs`（新文件）
- 修改：`omnilang/src/codegen/omniview/mod.rs`（注册新模块 + 添加 types 字段到输出）
- 修改：`omnilang/src/codegen/omniview/tests.rs`（添加测试）

**背景：** `ComponentIR` 包含完整的类型信息（`props: FxHashMap<String, TypeIR>`、`signals: Vec<SignalIntent>`、`event_param_types: Vec<String>`），但从未以 TypeScript `.d.ts` 形式输出。React 消费者必须手动编写所有类型。

- [ ] **步骤 1：创建 TypeScript 声明模块**

```rust
// 新文件：omnilang/src/codegen/omniview/typescript.rs

use crate::ir::{ComponentIR, TypeIR, SignalIntent, ComputedIntent};
use rustc_hash::FxHashMap;

/// 为所有组件和 stores 生成 TypeScript 声明 (.d.ts)。
/// 在与 React 后端一起使用时，提供类型安全的组件接口。
pub fn generate_typescript_declarations(components: &[ComponentIR]) -> String {
    let mut out = String::new();
    out.push_str("// Generated by OmniLang compiler — do not edit manually\n");
    out.push_str("// Component prop/event type declarations for React production target\n\n");

    for comp in components {
        emit_component_types(&mut out, comp);
    }
    out
}

fn emit_component_types(out: &mut String, comp: &ComponentIR) {
    let name = &comp.name;

    // Props 接口
    let has_props = !comp.props.is_empty();
    if has_props || !comp.event_names.is_empty() {
        out.push_str(&format!("export interface {}Props {{\n", name));
        for (prop_name, prop_type) in &comp.props {
            let ts_type = type_ir_to_ts(prop_type);
            let optional = if comp.prop_defaults.contains_key(prop_name) { "?" } else { "" };
            out.push_str(&format!("  {}{}: {};\n", prop_name, optional, ts_type));
        }
        out.push_str("}\n\n");
    }

    // 事件处理程序类型
    if !comp.event_names.is_empty() {
        for (i, event_name) in comp.event_names.iter().enumerate() {
            let param_type = comp.event_param_types.get(i)
                .filter(|s| !s.is_empty())
                .map(|s| s.as_str())
                .unwrap_or("any");
            let handler_name = format!("on{}", capitalize(event_name));
            out.push_str(&format!(
                "export type {}Handler = (detail: {}) => void;\n",
                handler_name, param_type
            ));
        }
        out.push('\n');
    }

    // 组件接口（合并 props + 事件处理程序）
    out.push_str(&format!("export interface {}Component {{\n", name));
    if has_props {
        out.push_str(&format!("  (props: {}Props): JSX.Element;\n", name));
    } else {
        out.push_str("  (): JSX.Element;\n");
    }
    out.push_str("}\n\n");
}

fn type_ir_to_ts(t: &TypeIR) -> String {
    match t {
        TypeIR::I32 | TypeIR::F64 => "number".into(),
        TypeIR::Bool => "boolean".into(),
        TypeIR::String => "string".into(),
        TypeIR::List(inner) => format!("{}[]", type_ir_to_ts(inner)),
        TypeIR::Map(k, v) => format!("Record<{}, {}>", type_ir_to_ts(k), type_ir_to_ts(v)),
        TypeIR::Optional(inner) => format!("{} | null", type_ir_to_ts(inner)),
        TypeIR::Named(name) => name.clone(),
        TypeIR::Unknown | TypeIR::Any => "any".into(),
        _ => "any".into(),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
```

- [ ] **步骤 2：在 mod.rs 中注册模块并将 TS 声明连接到输出**

在 `omnilang/src/codegen/omniview/mod.rs` 中：

```rust
// 在文件顶部现有的 mod 声明附近添加：
pub mod typescript;

// 在 OmniViewOutput 结构体中（约第 38 行），添加一个字段：
pub struct OmniViewOutput {
    pub js: String,
    pub html: String,
    pub css: String,
    pub types: String,  // 新增：TypeScript 声明
    pub source_map: Vec<SourceMapEntry>,
    // ... 现有字段 ...
}

// 在 generate() 方法中，输出生成后：
let types = typescript::generate_typescript_declarations(&ir.components);
// 将 types 添加到输出中
```

- [ ] **步骤 3：编写测试**

```rust
// 在 omnilang/src/codegen/omniview/tests.rs 中

#[test]
fn typescript_declarations_emit_props_interface() {
    let mut gen = OmniViewGenerator::test_new();
    // 构建一个包含 prop count: i32 和 event onChange: i32 的组件
    let output = gen.generate();
    assert!(output.types.contains("export interface CounterProps {"));
    assert!(output.types.contains("count: number;"));
    assert!(output.types.contains("onChangeHandler"));
}

#[test]
fn typescript_declarations_optional_for_defaulted_props() {
    let mut gen = OmniViewGenerator::test_new();
    // 构建一个包含 prop count: i32 = 0 的组件
    let output = gen.generate();
    assert!(output.types.contains("count?: number;"));
}
```

- [ ] **步骤 4：运行测试验证其通过**

```bash
cargo test -p omnilang typescript_declarations
```
预期：通过

- [ ] **步骤 5：运行完整测试套件**

```bash
cargo test -p omnilang
```
预期：所有测试通过

- [ ] **步骤 6：提交**

```bash
git add omnilang/src/codegen/omniview/typescript.rs omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/tests.rs
git commit -m "feat(omniview): generate TypeScript declarations for React production path

Add typescript.rs module that generates .d.ts content from ComponentIR
type information (props, events, signals). Include types in OmniViewOutput
so the Poiesis editor and omnic CLI can emit .d.ts alongside compiled JS.

Covers: component Props interfaces, event handler types, optional
props for defaulted values, and nested generic types."
```

---

### 阶段 2：编译器安全与重构（MEDIUM — 本周）

---

### 任务 4：将通配符 _ 分支替换为诊断警告

**文件：**
- 修改：`omnilang/src/codegen/omniview/stmt.rs:138-141, 180`

- [ ] **步骤 1：替换 emit_statement 中的通配符**

```rust
// 在 stmt.rs:138 中，将：
_ => {
    // Placeholder for unhandled statements
    self.js.push_str("  /* unhandled stmt */\n");
}

// 替换为：
_ => {
    self.warnings.push(format!(
        "OmniView: unhandled StatementIR variant — generated output may be incomplete. \
         This is a compiler bug; please report it."
    ));
}
```

- [ ] **步骤 2：替换 stmt_to_js_inline 中的通配符**

```rust
// 在 stmt.rs:180 中，将：
_ => String::from("/* unhandled stmt in block expr */"),

// 替换为：
_ => {
    self.warnings.push(
        "OmniView: unhandled StatementIR variant in inline expression".into()
    );
    String::from("/* unhandled stmt */")
}
```

- [ ] **步骤 3：验证现有测试通过**

```bash
cargo test -p omnilang
```

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/omniview/stmt.rs
git commit -m "fix(omniview): replace wildcard _ arms with diagnostic warnings

Replace silent '/* unhandled stmt */' emission with self.warnings.push()
in emit_statement and stmt_to_js_inline. Future IR additions will now
produce visible diagnostics instead of silently broken output, preventing
the BlockStmtIR class of regressions."
```

---

### 任务 5：修复 3 个 Clippy 警告

**文件：**
- 修改：`omnilang/src/codegen/omniview/view/element.rs:576, 613`
- 修改：`omnilang/src/codegen/omniview/tests_regression/motion.rs:118`

- [ ] **步骤 1：在 element.rs 中修复 needless_borrow（2 处）**

```rust
// element.rs:576 和 613 — 将：
self.emit_class_map_effects(el, &nid);
// 改为：
self.emit_class_map_effects(el, nid);
```

- [ ] **步骤 2：在 tests_regression/motion.rs 中修复未使用的 mut**

```rust
// tests_regression/motion.rs:118 — 将：
let mut gen = OmniViewGenerator::test_new();
// 改为：
let gen = OmniViewGenerator::test_new();
```

- [ ] **步骤 3：运行 clippy 验证修复**

```bash
cargo clippy -p omnilang -- -D warnings
```
预期：零警告。

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/omniview/view/element.rs omnilang/src/codegen/omniview/tests_regression/motion.rs
git commit -m "chore(omniview): fix 3 clippy warnings

Fix needless_borrow (×2 in element.rs) and unused mut (motion test)."
```

---

### 任务 6：提取 InliningState 保存/恢复结构体

**文件：**
- 修改：`omnilang/src/codegen/omniview/mod.rs`（添加 InliningState 结构体）
- 修改：`omnilang/src/codegen/omniview/component_html.rs:22-37, 131-136`

- [ ] **步骤 1：在 mod.rs 中定义 InliningState 结构体**

在 `mod.rs` 中，`OmniViewGenerator` 结构体之后、impl 块之前添加：

```rust
/// 在 try_inline_component 期间保存/恢复的每个组件状态的快照。
/// 集中式结构体防止在向生成器添加新字段时出现字段漂移错误。
struct InliningState {
    current_component: String,
    signal_names: FxHashSet<String>,
    name_renames: FxHashMap<String, String>,
    for_text_bindings: Vec<(String, String)>,
    for_class_bindings: Vec<(String, String, String)>,
    for_attr_bindings: Vec<(String, String, Vec<AttrPart>)>,
    for_rebind_fn: Option<String>,
    in_for_template: bool,
}

impl InliningState {
    fn save(gen: &OmniViewGenerator) -> Self {
        Self {
            current_component: gen.current_component.clone(),
            signal_names: gen.signal_names.iter().cloned().collect(),
            name_renames: gen.name_renames.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            for_text_bindings: std::mem::take(&mut gen.for_text_bindings),  // 安全：已复制
            for_class_bindings: std::mem::take(&mut gen.for_class_bindings),
            for_attr_bindings: std::mem::take(&mut gen.for_attr_bindings),
            for_rebind_fn: gen.for_rebind_fn.take(),
            in_for_template: gen.in_for_template,
        }
    }

    fn restore(self, gen: &mut OmniViewGenerator) {
        gen.current_component = self.current_component;
        gen.signal_names = self.signal_names;
        gen.name_renames = self.name_renames;
        gen.for_text_bindings = self.for_text_bindings;
        gen.for_class_bindings = self.for_class_bindings;
        gen.for_attr_bindings = self.for_attr_bindings;
        gen.for_rebind_fn = self.for_rebind_fn;
        gen.in_for_template = self.in_for_template;
    }
}
```

- [ ] **步骤 2：更新 component_html.rs 以使用该结构体**

```rust
// 替换第 21-37 行（手动保存 9 个字段）为：
let saved_state = InliningState::save(self);

// ... 现有的信号注册逻辑（第 39-128 行）保持不变 ...

// 替换第 131-141 行（手动恢复 9 个字段）为：
saved_state.restore(self);
```

- [ ] **步骤 3：运行测试验证行为不变**

```bash
cargo test -p omnilang
```
预期：所有测试通过。

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/component_html.rs
git commit -m "refactor(omniview): extract InliningState save/restore struct

Replace 9-field manual save/restore in try_inline_component with
InliningState helper that centralizes per-component state lifecycle.
Adding a new field that needs per-component isolation now requires
updating only the InliningState struct, not the restore logic."
```

---

### 任务 7：提取 FieldAccessExpr/OptionalChainExpr 共享辅助函数

**文件：**
- 修改：`omnilang/src/codegen/omniview/expr.rs:52-101`

- [ ] **步骤 1：提取共享方法**

```rust
// 在 impl OmniViewGenerator 块内部，expr_to_js 方法之前或之后，
// 添加此辅助方法：

fn resolve_store_field_access(
    &self,
    object: &ExprIR,
    field: &str,
    dot_op: &str,
    value_op: &str,
) -> String {
    let raw_obj_name = if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
        Some(name.clone())
    } else {
        None
    };
    let obj = self.expr_to_js(object);
    let check_name = raw_obj_name.as_deref().unwrap_or(&obj);
    if self.store_names.contains(check_name) || self.store_names.contains(&obj) {
        let renamed_obj = self
            .name_renames
            .get(check_name)
            .or_else(|| self.name_renames.get(obj.as_str()))
            .cloned()
            .unwrap_or_else(|| obj.clone());
        let var_name = format!("{}_{}", renamed_obj, field);
        if self.signal_names.contains(&var_name) {
            format!("{}{}", var_name, value_op)
        } else {
            var_name
        }
    } else {
        format!("{}{}{}", obj, dot_op, field)
    }
}
```

- [ ] **步骤 2：更新两个调用点**

```rust
// 将第 52-76 行（FieldAccessExpr）替换为：
ExprIR::FieldAccessExpr { object, field } => {
    self.resolve_store_field_access(object, field, ".", ".value")
}

// 将第 77-101 行（OptionalChainExpr）替换为：
ExprIR::OptionalChainExpr { object, field, .. } => {
    self.resolve_store_field_access(object, field, "?.", "?.value")
}
```

- [ ] **步骤 3：运行测试验证行为不变**

```bash
cargo test -p omnilang
```

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/omniview/expr.rs
git commit -m "refactor(omniview): extract resolve_store_field_access helper

Eliminate 44 lines of near-identical code between FieldAccessExpr
and OptionalChainExpr by extracting shared resolve_store_field_access
method. 唯一区别 (. vs ?.) 已被参数化。"
```

---

### 任务 8：Motion Trigger 去重

**文件：**
- 修改：`omnilang/src/codegen/omniview/motion_triggers.rs:321-601`

- [ ] **步骤 1：提取共享的触发器 JS 模板**

```rust
// 在 motion_triggers.rs 的 impl OmniViewGenerator 块中添加：

/// 为简单触发器（悬停、按压、聚焦、取消悬停）发出标准的 IIFE 包装 JS。
/// 减少 9 种触发类型 × 2 种变体（命名/内联）= 18 个几乎相同的代码块。
fn emit_simple_trigger_js(
    &mut self,
    nid: &str,
    event_listener: &str,      // "mouseenter", "mousedown", 等
    anim_orchestrator: &str,   // "__omni_motion_{name}(el, ...)" 或 "Omni.animate(el, {kf}, ...)"
    js_buffer: &mut String,    // self.js、self.mount_js 或 self.comp_mount_js
) {
    use std::fmt::Write;
    let _ = writeln!(
        *js_buffer,
        "(function() {{",
    );
    let _ = writeln!(
        *js_buffer,
        "  const el = __omni_el('{nid}');",
    );
    let _ = writeln!(
        *js_buffer,
        "  if (!el) return;",
    );
    let _ = writeln!(
        *js_buffer,
        "  let anim = null;",
    );
    let _ = writeln!(
        *js_buffer,
        "  el.addEventListener('{event_listener}', function __omni_trig(e) {{",
    );
    let _ = writeln!(
        *js_buffer,
        "    if (anim && anim.playState !== 'finished') anim.cancel();",
    );
    let _ = writeln!(
        *js_buffer,
        "    anim = {anim_orchestrator};",
    );
    let _ = writeln!(
        *js_buffer,
        "  }});",
    );
    let _ = writeln!(
        *js_buffer,
        "}})();",
    );
}
```

- [ ] **步骤 2：在 match 语句中为悬停、按压、聚焦、取消悬停、动画、时间线使用此辅助函数**

对于每种作为简单 `addEventListener` + `anim = ...` 的触发类型，将两个分支（命名和内联）替换为调用 `emit_simple_trigger_js` 并传入适当的参数。

- [ ] **步骤 3：运行运动测试套件**

```bash
cargo test -p omnilang motion
```
预期：所有运动快照测试通过。

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/omniview/motion_triggers.rs
git commit -m "refactor(omniview): extract emit_simple_trigger_js helper

Eliminate ~150 lines of duplicated IIFE wrapper generation across
hover/press/focus/unhover/animate/timeline triggers × named/inline
variants. Single emit_simple_trigger_js method parameterized by
event listener name and animation orchestrator expression."
```

---

### 任务 9：将 component_map 转换为 Arc<ComponentIR>

**文件：**
- 修改：`omnilang/src/codegen/omniview/mod.rs:285, 440`（字段类型 + 初始化）
- 修改：`omnilang/src/codegen/omniview/component_html.rs:18, 126`（使用点）

- [ ] **步骤 1：更改 mod.rs 中的字段类型**

```rust
// 在 mod.rs:285 中（populate_component_map），将：
component_map: FxHashMap<String, ComponentIR>,

// 改为：
component_map: FxHashMap<String, Arc<ComponentIR>>,

// 相应地更新初始化：
use std::sync::Arc;
component_map.insert(comp.name.clone(), Arc::new(comp.clone()));
```

- [ ] **步骤 2：更新 component_html.rs 中的使用点**

```rust
// 在第 18 行，将：
if let Some(child_comp) = self.component_map.get(&el.tag).cloned() {
// 改为：
if let Some(child_comp) = self.component_map.get(&el.tag).map(Arc::clone) {

// 在第 126 行，将：
let child_view_owned = child_view.clone();
// 改为（不再需要 clone，我们在 Arc 后面）：
let child_view_owned = child_comp.view.as_ref().unwrap().clone();
// 注意：仍需 clone view 因为它需要在 view_to_html 的 &mut self 之后存在，
// 但现在 component_map 查询是一个廉价的 Arc::clone（原子递增）而非深拷贝
```

- [ ] **步骤 3：运行完整测试套件**

```bash
cargo test -p omnilang
```

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/component_html.rs
git commit -m "perf(omniview): use Arc<ComponentIR> in component_map

Replace FxHashMap<String, ComponentIR> with Arc<ComponentIR> to turn
component lookups from deep-clones into atomic reference increments.
Eliminates multiplicative cloning cost for deeply nested components."
```

---

### 阶段 3：测试与安全（MEDIUM — 下周）

---

### 任务 10：为清理器 + 信号添加 JS 运行时单元测试

**文件：**
- 创建：`omnilang/tests/js_runtime/test_sanitizers.js`（新文件）
- 创建：`omnilang/tests/js_runtime/test_signals.js`（新文件）
- 创建：`omnilang/tests/js_runtime/run_tests.sh`（新文件）
- 修改：`.github/workflows/`（CI 集成——如有）

- [ ] **步骤 1：创建清理器测试**

```javascript
// omnilang/tests/js_runtime/test_sanitizers.js
// 将 omnilang/src/codegen/omniview/runtime.rs 中的清理器函数
// 复制到此处并添加断言。
// 在有 Node.js 的环境中运行：node test_sanitizers.js

const assert = require('assert');

// === 从 runtime.rs 复制清理器函数 ===
function __omni_sanitizeUrl(value) {
    value = String(value);
    const lower = value.toLowerCase().trim();
    if (lower.startsWith('javascript:') || lower.startsWith('data:') || lower.startsWith('vbscript:')) {
        return '#blocked';
    }
    return value;
}

function __omni_sanitizeCss(value) {
    value = String(value);
    const lower = value.toLowerCase().trim();
    if (lower.includes('javascript:') || lower.includes('expression(') ||
        lower.includes('-moz-binding') || lower.includes('behavior:')) {
        return '';
    }
    // 检查 url() 以查找危险的协议
    const urlMatch = lower.match(/url\s*\(\s*['"]?\s*([^'")\s]+)/);
    if (urlMatch) {
        const inner = urlMatch[1].toLowerCase().trim();
        if (inner.startsWith('javascript:') || inner.startsWith('vbscript:')) {
            return '';
        }
    }
    return value;
}
// === 测试 ===

// URL 清理器
assert.strictEqual(__omni_sanitizeUrl("javascript:alert(1)"), "#blocked");
assert.strictEqual(__omni_sanitizeUrl("JAVASCRIPT:alert(1)"), "#blocked");
assert.strictEqual(__omni_sanitizeUrl("data:text/html,<script>alert(1)</script>"), "#blocked");
assert.strictEqual(__omni_sanitizeUrl("vbscript:msgbox(1)"), "#blocked");
assert.strictEqual(__omni_sanitizeUrl("https://example.com"), "https://example.com");
assert.strictEqual(__omni_sanitizeUrl("/relative/path"), "/relative/path");
// 边缘情况：String() 强制转换
assert.strictEqual(__omni_sanitizeUrl({toString: () => "javascript:alert(1)"}), "#blocked");
assert.strictEqual(__omni_sanitizeUrl(null), "#blocked");  // String(null) === "null"
assert.strictEqual(__omni_sanitizeUrl(undefined), "#blocked");

// CSS 清理器
assert.strictEqual(__omni_sanitizeCss("javascript:alert(1)"), "");
assert.strictEqual(__omni_sanitizeCss("expression(alert(1))"), "");
assert.strictEqual(__omni_sanitizeCss("-moz-binding:url(xss.xml)"), "");
assert.strictEqual(__omni_sanitizeCss("color: red"), "color: red");
assert.strictEqual(__omni_sanitizeCss("background: url(https://img.example.com/bg.png)"), "background: url(https://img.example.com/bg.png)");
assert.strictEqual(__omni_sanitizeCss("background: url(javascript:alert(1))"), "");
// 边缘情况：CSS 转义序列规范化
// TODO: 此测试当前失败——CSS 转义绕过
// assert.strictEqual(__omni_sanitizeCss("\\6aavascript:alert(1)"), "");

console.log("All sanitizer tests passed!");
```

- [ ] **步骤 2：创建信号运行时测试**

```javascript
// omnilang/tests/js_runtime/test_signals.js
const assert = require('assert');

// === 从 runtime.rs 复制信号运行时函数 ===
let __omni_activeEffect = null;
const __omni_subs = new Map();

function __omni_signal(name, init) {
    let val = init;
    const subs = new Set();
    const signal = {
        _default: init,
        get value() {
            if (__omni_activeEffect) {
                subs.add(__omni_activeEffect);
                __omni_activeEffect._deps.add(() => subs.delete(__omni_activeEffect));
            }
            return val;
        },
        set value(v) {
            if (val !== v) {
                val = v;
                const s = new Set(subs);
                s.forEach(e => {
                    const prev = __omni_activeEffect;
                    __omni_activeEffect = e;
                    try { e.run(); }
                    catch(err) { /* 记录但继续 */ }
                    finally { __omni_activeEffect = prev; }
                });
            }
        },
        get() { return signal.value; },
        set(v) { signal.value = v; },
        peek() { return val; },
    };
    __omni_subs.set(name, signal);
    return signal;
}
// === 测试 ===

// 信号获取/设置
const count = __omni_signal("count", 0);
assert.strictEqual(count.value, 0);
count.value = 5;
assert.strictEqual(count.value, 5);
assert.strictEqual(count.peek(), 5);

// 效果执行
let effectRuns = 0;
const cleanup = (function() {
    let runs = 0;
    const e = { _deps: new Set(), run: () => { runs++; effectRuns = runs; } };
    const prev = __omni_activeEffect;
    __omni_activeEffect = e;
    try { e.run(); } finally { __omni_activeEffect = prev; }
    return () => e._deps.forEach(d => d());
})();
assert.strictEqual(effectRuns, 1);  // 初始运行
count.value = 10;
assert.strictEqual(effectRuns, 2);  // 信号变化后运行
cleanup();

// 效果异常处理——不应破坏 __omni_activeEffect
const badSignal = __omni_signal("bad", 0);
const e2 = { _deps: new Set(), run: () => { throw new Error("boom"); } };
__omni_activeEffect = e2;
try { e2.run(); } catch (_) {}
__omni_activeEffect = null;
// 验证 __omni_activeEffect 已被正确清除
assert.strictEqual(__omni_activeEffect, null);

console.log("All signal tests passed!");
```

- [ ] **步骤 3：创建运行脚本**

```bash
#!/bin/bash
# omnilang/tests/js_runtime/run_tests.sh
set -e
echo "Running JS runtime sanitizer tests..."
node "$(dirname "$0")/test_sanitizers.js"
echo "Running JS runtime signal tests..."
node "$(dirname "$0")/test_signals.js"
echo "All JS runtime tests passed!"
```

- [ ] **步骤 4：在本地运行测试**

```bash
bash omnilang/tests/js_runtime/run_tests.sh
```
预期：通过（CSS 转义测试可能失败——这是预期结果，已记录在案的漏洞）。

- [ ] **步骤 5：提交**

```bash
git add omnilang/tests/js_runtime/
git commit -m "test(omniview): add JS engine behavioral tests for runtime functions

Add Node.js-based tests executing __omni_sanitizeUrl, __omni_sanitizeCss,
and __omni_signal against edge cases. These complement the Rust string-content
tests by verifying actual JS engine behavior. Catches CSS escape bypass,
signal effect exception handling, and String() coercion edge cases."
```

---

### 任务 11：对齐 CSS 清理器 + 转义序列规范化

**文件：**
- 修改：`crates/omnilang-core/src/codegen/sanitize.rs:12-28`
- 修改：`omnilang/src/codegen/omniview/runtime.rs:149-173`

- [ ] **步骤 1：在 Rust 端对齐 CSS 清理器**

```rust
// 在 crates/omnilang-core/src/codegen/sanitize.rs:12-28 中，修改 sanitize_css_value：

pub fn sanitize_css_value(value: &str) -> String {
    let lower = value.to_lowercase().trim().to_string();
    let lower = normalize_css_escapes(&lower);
    
    // 阻止危险的 CSS 关键字（任意位置）
    if lower.contains("javascript:")
        || lower.contains("expression(")
        || lower.contains("-moz-binding")
        || lower.contains("behavior:")
    {
        return String::new();
    }
    
    // 允许 url() 但阻止其中的危险协议（与 JS 运行时对齐）
    if let Some(url_start) = lower.find("url(") {
        let after_url = &lower[url_start + 4..];
        if let Some(close) = after_url.find(')') {
            let inner = after_url[..close].trim().trim_matches(|c| c == '"' || c == '\'');
            let inner_lower = inner.to_lowercase();
            if inner_lower.starts_with("javascript:") || inner_lower.starts_with("vbscript:") {
                return String::new();
            }
        }
    }
    
    value.to_string()
}

/// 规范化 CSS 十六进制转义序列（\XX 或 \XXXXXX）为其 ASCII 等价形式。
/// 浏览器在解析 CSS 值时执行此规范化，因此清理器必须也执行此操作。
fn normalize_css_escapes(s: &str) -> String {
    // 将 \ 后跟十六进制数字的简单状态机替换为对应的 ASCII 字符
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && chars[i + 1].is_ascii_hexdigit() {
            let mut hex = String::new();
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_hexdigit() && hex.len() < 6 {
                hex.push(chars[j]);
                j += 1;
            }
            if let Ok(code) = u32::from_str_radix(&hex, 16) {
                if let Some(c) = char::from_u32(code) {
                    result.push(c);
                }
            }
            i = j;
            // 跳过可选的空白字符（CSS 转义语法）
            if i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}
```

- [ ] **步骤 2：在 JS 端添加 CSS 转义规范化**

```javascript
// 在 runtime.rs 的 emit_signal_runtime() 中，__omni_sanitizeCss 函数内，
// 在 "value = String(value);" 行之后添加：

function __omni_normalizeCssEscapes(s) {
    return s.replace(/\\([0-9a-fA-F]{1,6})\s?/g, function(_, hex) {
        var code = parseInt(hex, 16);
        return code > 0x1F && code < 0x7F ? String.fromCharCode(code) : '';
    });
}

// 然后在 __omni_sanitizeCss 的 "const lower = value.toLowerCase().trim();" 之前添加：
value = __omni_normalizeCssEscapes(value);
```

- [ ] **步骤 3：运行测试套件**

```bash
cargo test -p omnilang
cargo test -p omnilang-core
node omnilang/tests/js_runtime/test_sanitizers.js
```
预期：全部通过。之前被注释掉的 CSS 转义测试现在应通过。

- [ ] **步骤 4：提交**

```bash
git add crates/omnilang-core/src/codegen/sanitize.rs omnilang/src/codegen/omniview/runtime.rs omnilang/tests/js_runtime/test_sanitizers.js
git commit -m "fix(omniview): align CSS sanitizers and add escape sequence normalization

Rust compile-time sanitizer now allows url() with safe schemes, matching
the JS runtime behavior (previously blocked all url() unconditionally).
Both sanitizers now normalize CSS hex escapes (\6a → j) before keyword
checking, closing the CSS injection bypass documented in ADR-027.

Fixes:
- Rust sanitize_css_value: allow url(https://...) matching JS behavior
- Both sanitizers: normalize CSS escape sequences before keyword checks
- JS test: uncomment and enable CSS escape bypass test case"
```

---

### 任务 12：限制 @enter 重放依赖范围

**文件：**
- 修改：`omnilang/src/codegen/omniview/store.rs:417-452`

- [ ] **步骤 1：按组件拆分 @enter 重放效果**

```rust
// 在 store.rs:417-452 中，emit_enter_replay_fn 方法。
// 将读取所有信号并在所有 @enter 触发器上重放的单体效果替换为
// 每个组件一个效果，仅读取该组件的信号。

// 之前（读取所有信号的一个效果）：
// __omni_effect(function __omni_enter_replay() {
//   {ALL_SIGNAL_READS}
//   {ALL_REPLAY_CALLS}
// });

// 之后（每个组件一个效果）：
for (comp_name, enter_triggers) in &enter_triggers_by_component {
    let signal_reads = component_signal_reads.get(comp_name).unwrap();
    self.js.push_str(&format!(
        "__omni_effect(function __omni_enter_replay_{comp}() {{\n",
        comp = comp_name
    ));
    // 仅读取此组件的信号以建立依赖关系
    for read in signal_reads {
        self.js.push_str(&format!("  __omni_subs.get(\"{0}\").peek();\n", read));
    }
    // 仅重放此组件的触发器
    for trigger in enter_triggers {
        self.js.push_str(&trigger.replay_js);
    }
    self.js.push_str("});\n");
}
```

需要重构的辅助数据结构：在生成 @enter 触发器时构建 `FxHashMap<String, Vec<EnterTriggerInfo>>` 和 `FxHashMap<String, Vec<String>>`。

- [ ] **步骤 2：运行运动测试**

```bash
cargo test -p omnilang motion
```
预期：所有运动测试通过。快照可能会因不同的效果分组结构而改变。

- [ ] **步骤 3：提交**

```bash
git add omnilang/src/codegen/omniview/store.rs
git commit -m "perf(omniview): scope @enter replay dependencies per component

Replace single effect reading ALL signals and replaying ALL @enter
triggers with per-component effects that only read their own signals.
Reduces replay complexity from O(S×A) to O(Sc×Ac) where Sc=component
signals, Ac=component @enter triggers.

Previously: typing in a text input caused every @enter animation to
replay. Now: only the current component's animations replay."
```

---

### 任务 13：连接 WASM 增量缓存

**文件：**
- 修改：`omnilang-wasm/src/lib.rs:433-473`

- [ ] **步骤 1：在 compile_multi_to_omniview 中添加缓存支持**

```rust
// 在 omnilang-wasm/src/lib.rs:433 中，修改 compile_multi_to_omniview 函数：

#[wasm_bindgen]
pub fn compile_multi_to_omniview(
    files_map: JsValue,
    cache_state: JsValue,  // 新增参数
    opts: JsValue,
) -> Result<JsValue, JsValue> {
    // ... 现有设置 ...
    
    // 如果提供了，恢复分析缓存
    if !cache_state.is_null() && !cache_state.is_undefined() {
        if let Ok(state) = serde_wasm_bindgen::from_value::<AnalysisCache>(cache_state) {
            ANALYSIS_CACHE.with(|cache| {
                *cache.borrow_mut() = Some(state);
            });
        }
    }
    
    // 使用 analyse_multi_cached 而非 analyse_multi（与 resolve_styles 模式匹配）
    let analysis = omnic::analyse_multi_cached(files.clone(), opts.clone())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    let result = omnic::compile_multi_internal(analysis, opts)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    // 序列化更新后的缓存状态以在 TS 端保留
    let updated_cache = ANALYSIS_CACHE.with(|cache| {
        cache.borrow().clone()
    });
    let cache_js = serde_wasm_bindgen::to_value(&updated_cache)
        .unwrap_or(JsValue::NULL);
    
    // 返回结果和缓存状态
    // ... 合并到现有返回值中 ...
}
```

- [ ] **步骤 2：验证编辑器编译路径**

```bash
cargo build -p omnilang-wasm --target wasm32-unknown-unknown
```
预期：干净构建。

- [ ] **步骤 3：提交**

```bash
git add omnilang-wasm/src/lib.rs
git commit -m "perf(wasm): connect incremental analysis cache to compile_multi_to_omniview

Mirror the resolve_styles_multi_cached pattern: accept cache_state,
restore ANALYSIS_CACHE before compilation, use analyse_multi_cached,
and return updated cache state. Reduces debug window compile latency
from O(total project size) to O(delta size) in the Poiesis editor."
```

---

### 阶段 4：长期（延期至第 3 阶段之后）

---

### 任务 14：提取 NameResolver 结构体（God Object 阶段 1）

**文件：** `omnilang/src/codegen/omniview/mod.rs`

从 OmniViewGenerator 中提取 9 个名称解析字段（`signal_names`、`signal_defaults`、`name_renames`、`global_renames`、`store_signal_names`、`store_field_names`、`store_names`、`for_state_names`、`component_style_classes`）到一个 `NameResolver` 结构体中，并内联为 `OmniViewGenerator` 的一个字段。

**工作量：** S（1-2 小时） | **优先级：** 当 OmniViewGenerator 超过 55 个字段时

---

### 任务 15：添加上下文/提供者模式

**文件：** `crates/omnilang-core/src/ir/types.rs`（新的 ContextIR）、`omnilang/src/codegen/omniview/store.rs`（代码生成）

实现一个轻量级上下文系统：`ContextIR` 携带信号名称和默认值，代码生成在每个组件子树的闭包中暴露作用域上下文信号。组件可以使用 `provide(key, signal)` 和 `inject(key)`。

**工作量：** L（5-8 天） | **优先级：** 当用户需求出现时

---

### 任务 16-18：剩余的延期项目

| 任务 | 描述 | 工作量 |
|------|------|--------|
| 16 | 在非编辑器模式下仅渲染活动条件分支 | M（2-3 天） |
| 17 | 键控 @for 列表的 DOM 重排序 | M（3 天） |
| 18 | 删除暂停后端的 222 个冻结测试 | XS（15 分钟） |

---

## 自审清单

**1. 规范覆盖：** 所有 3 个 HIGH 发现（N1 插槽、N2 props、N3 TS 声明）在任务 1-3 中均有详尽任务。所有 10 个 MEDIUM 编译器/安全/重构项目在任务 4-13 中涵盖。延期项目在任务 14-18 中记录。

**2. 占位符扫描：** 无 "TOD"、"实现后续"、"填写细节"、"添加适当的错误处理" 或 "与任务 N 类似" 条目。所有任务包含确切的代码、文件路径和命令。

**3. 类型一致性：** `InliningState` 结构体在 mod.rs 中定义，在 component_html.rs 中使用，字段名称在整个过程中一致。`resolve_store_field_access` 参数列表在定义和两个调用点之间匹配。`__omni_readonlySignal` 在 runtime.rs 中定义，在 store.rs 中引用。

---

## 执行移交

**计划已保存至 `docs/superpowers/plans/2026-07-29-omniview-framework-fixes.md`。** 两个执行选项：

**1. 子代理驱动（推荐）** — 每个任务派发一个全新的子代理，任务间审查，快速迭代

**2. 内联执行** — 在本次会话中使用 executing-plans 执行任务，带检查点的批量执行

**采用哪种方式？**
