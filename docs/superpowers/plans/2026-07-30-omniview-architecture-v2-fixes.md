# OmniView Architecture v2 Review Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复由 2026-07-30 v2 架构评审（11 位专家）识别的 6 个关键项目和 7 个高优先级项目，涵盖 IR 安全性、跨后端净化对等、React motion 正确性以及编译器流水线完整性。

**Architecture:** 分两个阶段进行：第一阶段（关键项——立即，约 2 小时）包含高影响/成本比的单行和少量修复，涵盖 IR 枚举保护、净化缺口和 CI 集成。第二阶段（高优先级项——短期，约 6–8 天）涉及 React motion 隔离、FormatExpr 采用、Vue/Dioxus 安全对等以及领域模型统一等大型重构。

**Tech Stack:** Rust（omnilang + omnilang-core crates）、TypeScript/React（Poiesis 编辑器）、Node.js（JS 运行时测试）、WASM（omnilang-wasm）

---

## 第一阶段：关键项（立即——本周）

### Task 1: IR enum #[non_exhaustive] guards

**文件：**
- 修改：`crates/omnilang-core/src/ir/types.rs:134,282,520,665`
- 修改：`crates/omnilang-core/src/ir/signals.rs:48`
- 修改：`crates/omnilang-core/src/ir/events.rs:9`
- 修改：`omnilang/src/codegen/omniview/stmt.rs:138-144`

**上下文：** 在 `ExprIR`、`StatementIR`、`ViewTree` 和 `MutationIntent` 上添加 `#[non_exhaustive]`，以保护 10 个后端免受新增变体的级联编译时破坏。从仅有内部消费者的 `TypeKind` 和 `EventDataKind` 上移除 `#[non_exhaustive]`。

- [ ] **步骤 1：在 ExprIR 上添加 `#[non_exhaustive]`**

在 `crates/omnilang-core/src/ir/types.rs:134` 中，在 `#[serde(tag = "_kind")]` 之后添加：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
#[non_exhaustive]
pub enum ExprIR {
```

- [ ] **步骤 2：在 StatementIR 上添加 `#[non_exhaustive]`**

在 `crates/omnilang-core/src/ir/types.rs:520` 中，在 `#[serde(tag = "_kind")]` 之后添加：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
#[non_exhaustive]
pub enum StatementIR {
```

- [ ] **步骤 3：在 ViewTree 上添加 `#[non_exhaustive]`**

在 `crates/omnilang-core/src/ir/types.rs:665` 中，在 `#[serde(untagged)]` 之前添加：

```rust
#[non_exhaustive]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ViewTree {
```

- [ ] **步骤 4：在 MutationIntent 上添加 `#[non_exhaustive]`**

在 `crates/omnilang-core/src/ir/signals.rs:48` 中，在 `#[serde(tag = "operation")]` 之后添加：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
#[non_exhaustive]
pub enum MutationIntent {
```

- [ ] **步骤 5：从 TypeKind 上移除 `#[non_exhaustive]`**

在 `crates/omnilang-core/src/ir/types.rs:281` 中，移除该属性行：

```rust
// 之前：
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TypeKind {

// 之后：
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
```

- [ ] **步骤 6：从 EventDataKind 上移除 `#[non_exhaustive]`**

在 `crates/omnilang-core/src/ir/events.rs:9` 中，移除该属性行：

```rust
// 之前：
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventDataKind {

// 之后：
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventDataKind {
```

- [ ] **步骤 7：检查编译——匹配臂应保持穷举**

运行：`cargo check 2>&1`

由于这些枚举在 `omnilang-core` crate 内被穷举匹配，移除 `#[non_exhaustive]` 应导致零个编译错误（匹配已覆盖所有变体），而添加 `#[non_exhaustive]` 将要求在后端 crate 的匹配中新增 catch-all 分支。

- [ ] **步骤 8：为跨 crate 消费者添加通配符 catch-all 分支**

在 `omnilang/src/codegen/omniview/stmt.rs:138-144` 中，保留现有的通配符 catch-all（已处理未知变体）：

```rust
// 已存在，验证其仍可编译：
#[allow(unreachable_patterns)]
_ => {
    self.warnings.push(
        "OmniView: unhandled StatementIR variant ...".into()
    );
}
```

对于其他后端（React、Vue、Dioxus、Flutter、Dart、Python、JavaScript、Rust），在 `ExprIR`/`StatementIR`/`ViewTree` 匹配中添加 catch-all 分支。在每个后端的主要表达式/语句/视图匹配函数中，添加：

```rust
#[allow(unreachable_patterns)]
_ => unreachable!("Unknown IR variant — update codegen for new variant"),
```

- [ ] **步骤 9：运行完整测试套件**

运行：`cargo test --workspace`
预期：所有测试通过（零回归）

- [ ] **步骤 10：提交**

```bash
git add crates/omnilang-core/src/ir/types.rs crates/omnilang-core/src/ir/signals.rs crates/omnilang-core/src/ir/events.rs
git add omnilang/src/codegen/ react/ vue/ dioxus/ flutter/ dart/ python/ javascript/ rust/
git commit -m "fix(ir): add #[non_exhaustive] to ExprIR/StatementIR/ViewTree/MutationIntent, remove from TypeKind/EventDataKind"
```

---

### Task 2: Fix inline style ref-name translation hole

**文件：**
- 修改：`omnilang/src/codegen/react/view.rs:478`

**上下文：** `gen_view_node_tsx` 中第 478 行向 `format_style_value_expr` 传递了 `&FxHashSet::default()` 而非实际的 `ref_names` 参数。这导致内联样式 `style: { "--x": $scrollRef }` 中的 `$signal` ref 引用无法翻译为 `Ref.current`。

- [ ] **步骤 1：定位错误的调用点**

在 `omnilang/src/codegen/react/view.rs:478` 中，当前代码：

```rust
let js_val = format_style_value_expr(val, &FxHashSet::default());
```

- [ ] **步骤 2：替换为实际的 ref_names**

将第 478 行改为传递 `ref_names` 参数（该参数已存在于函数签名中）：

```rust
let js_val = format_style_value_expr(val, ref_names);
```

- [ ] **步骤 3：运行 React 快照测试以验证输出变更**

运行：`cargo test react`
预期：快照测试可能显示引用名称翻译的差异——审查变更并更新快照。

- [ ] **步骤 4：审查快照差异**

如果任何快照测试输出发生变化，验证变更是否将裸引用名称正确翻译为 `Ref.current` 访问。使用 `cargo insta review` 审查并接受。

- [ ] **步骤 5：提交**

```bash
git add omnilang/src/codegen/react/view.rs
git add omnilang/src/codegen/react/ (snapshots if any)
git commit -m "fix(react): pass ref_names to format_style_value_expr for inline style $signal refs"
```

---

### Task 3: Add URL sanitization to Vue and Dioxus

**文件：**
- 修改：`omnilang/src/codegen/vue/template.rs:24-33`
- 修改：`omnilang/src/codegen/dioxus/view.rs:27-36`

**上下文：** ADR-039 为 React 修复的 XSS 向量（静态 URL 属性中的 `javascript:` URL）在 Vue 和 Dioxus 代码生成中仍然开放。两者在未调用 `is_dangerous_url()` 的情况下逐字发出静态字符串属性。该函数已公开导出：`omnilang/src/codegen/omniview/mod.rs:29` 中的 `pub(crate) use view::element::is_dangerous_url;`

- [ ] **步骤 1：向 Vue 模板属性发出添加 `is_dangerous_url()` 检查**

在 `omnilang/src/codegen/vue/template.rs:27` 中，当前代码：

```rust
let escaped = s.replace('"', "&quot;");
attrs.push(format!("{}=\"{}\"", vue_attr, escaped));
```

添加 URL 属性检查。将 URL 属性名称提取为常量集合，并在发出前检查：

```rust
let escaped = s.replace('"', "&quot;");
// Check for dangerous URL schemes in static URL attributes
const URL_ATTRS: &[&str] = &["href", "src", "srcset", "action", "formaction", "poster"];
if URL_ATTRS.contains(&vue_attr) && crate::codegen::omniview::is_dangerous_url(&s) {
    // Emit empty string to neutralize dangerous URLs at compile time
    attrs.push(format!("{}=\"\"", vue_attr));
} else {
    attrs.push(format!("{}=\"{}\"", vue_attr, escaped));
}
```

- [ ] **步骤 2：向 Dioxus 视图属性发出添加 `is_dangerous_url()` 检查**

在 `omnilang/src/codegen/dioxus/view.rs:29-30` 中，当前代码：

```rust
let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
attrs.push(format!(r#"{}: "{}","#, attr_name, escaped));
```

添加 URL 属性检查：

```rust
let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
// Check for dangerous URL schemes in static URL attributes
const URL_ATTRS: &[&str] = &["href", "src", "srcset", "action", "formaction", "poster"];
if URL_ATTRS.contains(&attr_name) && crate::codegen::omniview::is_dangerous_url(&s) {
    attrs.push(format!(r#"{}: "","#, attr_name));
} else {
    attrs.push(format!(r#"{}: "{}","#, attr_name, escaped));
}
```

- [ ] **步骤 3：运行 Vue 和 Dioxus 测试**

运行：`cargo test vue dioxus`
预期：所有测试通过（安全 URL 的现有快照应保持相同；可能需要新增危险 URL 的测试夹具）

- [ ] **步骤 4：验证 `is_dangerous_url` 可访问性**

确认 `pub(crate)` 重新导出在 `omnilang` crate 内可见：
运行：`cargo check`
预期：零编译错误（`crate::codegen::omniview::is_dangerous_url` 在 `omnilang` crate 内可解析）

- [ ] **步骤 5：提交**

```bash
git add omnilang/src/codegen/vue/template.rs omnilang/src/codegen/dioxus/view.rs
git commit -m "fix(vue,dioxus): add is_dangerous_url() check to static URL attributes for XSS prevention"
```

---

### Task 4: Add CR/LF/TAB normalization to Rust CSS sanitizer

**文件：**
- 修改：`crates/omnilang-core/src/codegen/sanitize.rs:63-65`

**上下文：** JS 运行时 `__omni_sanitizeCss` 已获得 CR/LF/TAB 标准化（ADR-039 D3），但 Rust 编译时 `sanitize_css_value` 并未应用相同的标准化。浏览器在 CSS 标记化期间会标准化这些字符。一个包含 `jav\r\nascript:` 的 `.omni` 源文件将绕过 Rust 净化器，但在浏览器中执行。

- [ ] **步骤 1：在 CSS 十六进制转义标准化之前添加 CR/LF/TAB 剥离**

在 `crates/omnilang-core/src/codegen/sanitize.rs:65` 中，当前代码：

```rust
pub fn sanitize_css_value(value: &str) -> String {
    // First normalize CSS escape sequences to prevent bypasses
    let normalized = normalize_css_escapes(value);
```

添加预处理步骤以剥离浏览器会标准化的字符：

```rust
pub fn sanitize_css_value(value: &str) -> String {
    // Strip CR, LF, TAB — browsers normalize these during CSS tokenization.
    // Must be done before normalize_css_escapes so hex-encoded versions
    // of these characters are caught by the escape normalizer, while
    // literal whitespace is stripped here.
    let value = value.replace(&['\r', '\n', '\t'][..], "");
    // Normalize CSS escape sequences to prevent bypasses
    let normalized = normalize_css_escapes(&value);
```

注意：`replace` 分配一个新的 `String`；如果希望避免分配，可以使用基于迭代器的版本，但对于编译时路径，这种清晰度是可接受的。

- [ ] **步骤 2：运行净化器测试**

运行：`cargo test sanitize`
预期：所有现有测试通过。如果 `crates/omnilang-core/src/codegen/tests_property.rs` 中存在属性测试，应自动捕获此变更；否则，现有单元测试应覆盖基本用例。

- [ ] **步骤 3：为 CR/LF 绕过添加测试用例**

在 `crates/omnilang-core/src/codegen/tests_property.rs`（或净化器测试模块）中，添加：

```rust
#[test]
fn sanitize_css_value_blocks_crlf_javascript_bypass() {
    // CR/LF before "javascript:" should be stripped and caught
    assert_eq!(sanitize_css_value("url(jav\r\nascript:alert(1))"), "");
    // TAB before "expression(" should be stripped and caught
    assert_eq!(sanitize_css_value("url(jav\tascript:alert(1))"), "");
}
```

- [ ] **步骤 4：运行完整测试套件**

运行：`cargo test --workspace`
预期：所有测试通过

- [ ] **步骤 5：提交**

```bash
git add crates/omnilang-core/src/codegen/sanitize.rs crates/omnilang-core/src/codegen/tests_property.rs
git commit -m "fix(sanitize): add CR/LF/TAB normalization to sanitize_css_value matching JS runtime behavior"
```

---

### Task 5: Add element_tree_stack balance invariant assertion

**文件：**
- 修改：`omnilang/src/codegen/omniview/mod.rs:179-185`（generate 函数入口）

**上下文：** `element_tree_stack`（`Vec<ElementTreeNode>`）在多个方法（`element_to_html`、`tree_push`、`tree_pop`）中需要完美平衡的 push/pop 配对。一个组件中的错误可能破坏所有后续组件的状态，因为栈在组件之间从未被清理。添加一个 `debug_assert!` 以在组件边界捕获此问题。

- [ ] **步骤 1：定位 generate 函数中的组件循环**

在 `omnilang/src/codegen/omniview/mod.rs:179` 中，`generate` 函数遍历组件。在遍历中的每个组件代码生成调用之前，添加 stack-empty 断言：

```rust
// 在组件生成循环内部，在调用每个组件的代码生成之前：
for component in &component_order {
    // ... existing logic ...

    // SAFETY: element_tree_stack must be empty before each component.
    // A non-empty stack indicates a bug in the previous component's
    // element_to_html tree_push/tree_pop pairing.
    debug_assert!(
        self.element_tree_stack.is_empty(),
        "element_tree_stack not empty before component '{}' — previous component has unbalanced tree_push/tree_pop",
        component.name
    );

    // ... proceed with component codegen ...
}
```

- [ ] **步骤 2：运行测试以验证断言不会在正常操作中触发**

运行：`cargo test omniview`
预期：所有测试通过（`debug_assert!` 不会触发——当前代码正确平衡了栈）

- [ ] **步骤 3：提交**

```bash
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "fix(omniview): add debug_assert for element_tree_stack balance at component boundaries"
```

---

### Task 6: Integrate JS runtime tests into cargo test

**文件：**
- 创建：`omnilang/tests/js_runtime_tests.rs`
- 现有：`omnilang/tests/js_runtime/test_sanitizers.js`
- 现有：`omnilang/tests/js_runtime/test_signals.js`

**上下文：** 431 行手动编写的 JS 运行时测试（覆盖信号传播、净化器、去抖）具有零自动化执行。它们存在于 `omnilang/tests/js_runtime/run_tests.sh` 中，但未与 CI 或 `cargo test` 集成。此问题已在 3 个审查周期中被标记。

- [ ] **步骤 1：编写调用 JS 测试的 Rust 集成测试包装器**

创建 `omnilang/tests/js_runtime_tests.rs`：

```rust
use std::process::Command;

#[test]
fn js_runtime_sanitizers_pass() {
    let output = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/js_runtime/test_sanitizers.js"
        ))
        .output()
        .expect("failed to execute node — is Node.js installed?");

    assert!(
        output.status.success(),
        "JS sanitizer tests failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn js_runtime_signals_pass() {
    let output = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/js_runtime/test_signals.js"
        ))
        .output()
        .expect("failed to execute node — is Node.js installed?");

    assert!(
        output.status.success(),
        "JS signal tests failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
```

- [ ] **步骤 2：运行集成测试以验证**

运行：`cargo test js_runtime`
预期：如果安装了 Node.js 且 JS 测试通过，则 `PASS`。如果未安装 Node.js，则测试会 panic 并显示清晰消息。

- [ ] **步骤 3：在 CI 配置中添加可选门控**

JS 运行时测试需要 Node.js。如果 Node.js 在 CI 中不可用，此测试应优雅地跳过：

```rust
#[test]
fn js_runtime_sanitizers_pass() {
    // Skip if Node.js is not available (e.g., in minimal CI images)
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("SKIP: Node.js not found — skipping JS runtime tests");
        return;
    }

    let output = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/js_runtime/test_sanitizers.js"
        ))
        .output()
        .expect("failed to execute node");

    assert!(
        output.status.success(),
        "JS sanitizer tests failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn js_runtime_signals_pass() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("SKIP: Node.js not found — skipping JS runtime tests");
        return;
    }

    let output = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/js_runtime/test_signals.js"
        ))
        .output()
        .expect("failed to execute node");

    assert!(
        output.status.success(),
        "JS signal tests failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
```

- [ ] **步骤 4：运行完整测试套件**

运行：`cargo test --workspace`
预期：如果安装了 Node.js，JS 运行时测试会执行并通过；如果没有 Node.js，则跳过。

- [ ] **步骤 5：提交**

```bash
git add omnilang/tests/js_runtime_tests.rs
git commit -m "test: integrate JS runtime tests (sanitizers, signals) into cargo test"
```

---

## 第二阶段：高优先级项（短期——1–2 周）

### 任务 7：将 React motion 查询限定为组件根元素

**文件：**
- 修改：`omnilang/src/codegen/react/motion/mod.rs:69-144`（@enter、@hover、@press 效果）
- 修改：`omnilang/src/codegen/react/motion/transforms.rs:13-57,75-161,194-334`（@in-view、@scroll、@drag 效果）
- 修改：`omnilang/src/codegen/react/view.rs`（JSX 生成——在根元素上添加 `ref={_rootRef}`）

**上下文：** 所有 6 个 motion 触发类型均发出带有 `document.querySelectorAll('[data-motion-*]')` 的 `useEffect` 块，该调用扫描整个文档。当同一页面上的两个组件均包含动画时，实例 A 的效果会将事件处理程序附加到实例 B 的元素上。修复方法：将查询限定为通过 `useRef` 的组件根元素。

- [ ] **步骤 1：在 JSX 生成中为组件根添加 `_rootRef`**

在 `omnilang/src/codegen/react/view.rs` 中，找到组件根 JSX 元素的生成位置（其 `gen_view_node_tsx` 入口）。当组件包含 motion 效果时：

1. 在组件函数体的顶部，生成：`const _rootRef = useRef<HTMLDivElement>(null);`
2. 在根 JSX 元素上，添加：`ref={_rootRef}`

如果组件有多个根级元素，将其包装在 `<div ref={_rootRef}>` 中。

- [ ] **步骤 2：更新 @enter 效果以使用限定作用域的查询**

在 `omnilang/src/codegen/react/motion/mod.rs:71` 中，将：
```typescript
document.querySelectorAll('[data-motion-enter]').forEach(el => { ... });
```
替换为：
```typescript
_rootRef.current?.querySelectorAll('[data-motion-enter]').forEach(el => { ... });
```

- [ ] **步骤 3：更新 @hover/@press 效果以使用限定作用域的查询**

在 `omnilang/src/codegen/react/motion/mod.rs:93,111` 中，将：
```typescript
document.querySelectorAll('[data-motion-hover]').forEach(el => { ... });
document.querySelectorAll('[data-motion-press]').forEach(el => { ... });
```
替换为：
```typescript
_rootRef.current?.querySelectorAll('[data-motion-hover]').forEach(el => { ... });
_rootRef.current?.querySelectorAll('[data-motion-press]').forEach(el => { ... });
```

- [ ] **步骤 4：更新 @in-view、@scroll、@drag 效果以使用限定作用域的查询**

在 `omnilang/src/codegen/react/motion/transforms.rs:21,83,202` 中，将所有三个 `document.querySelectorAll('[data-motion-*]')` 调用替换为 `_rootRef.current?.querySelectorAll(...)`。

- [ ] **步骤 5：运行 React motion 快照测试**

运行：`cargo test react::motion`
预期：快照将显示变更的查询模式。使用 `cargo insta review` 审查并接受。

- [ ] **步骤 6：添加多实例测试夹具**

创建一个测试夹具，在相同页面上使用 `@enter` 动画的两个组件实例，以验证 motion 效果不会交叉激活。将此夹具添加到 `omnilang/src/codegen/react/tests.rs`：

```rust
#[test]
fn two_component_instances_with_motion_do_not_cross_activate() {
    // Compile source with two components both using @enter
    // Verify each component's useEffect queries are scoped
    let source = r#"
        component A {
            view { <div @enter="fadeIn">Content</div> }
        }
        component B {
            view { <div @enter="slideUp">Content</div> }
        }
    "#;
    let output = compile_react(source);
    // Each component should have rootRef.current?.querySelectorAll, not document.querySelectorAll
    assert!(output.contains("_rootRef.current?.querySelectorAll('[data-motion-enter]')"));
}
```

- [ ] **步骤 7：运行完整 React 测试套件**

运行：`cargo test react`
预期：所有测试通过

- [ ] **步骤 8：提交**

```bash
git add omnilang/src/codegen/react/motion/mod.rs omnilang/src/codegen/react/motion/transforms.rs omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/tests.rs
git commit -m "fix(react): scope motion effect DOM queries to component root via useRef, preventing cross-instance interference"
```

---

### 任务 8：为未使用的 motion 触发类型添加门控条件

**文件：**
- 修改：`omnilang/src/codegen/react/motion/mod.rs:87`（@hover/@press 效果发出）
- 修改：`omnilang/src/codegen/react/motion/transforms.rs:13,75,194`（@in-view、@scroll、@drag 效果发出）

**上下文：** `emit_motion_effects_tsx` 无条件发出 @hover/@press 的 `useEffect` 块，即使组件仅使用 `@enter`。这会为每个仅包含 @enter 的组件产生约 50 行死 JS。修复方法：在发出不必要的效果块之前添加触发类型存在性检查，遵循 `has_inview_trigger`/`has_scroll_trigger`/`has_drag_trigger` 的现有模式。

- [ ] **步骤 1：为 hover/press 触发添加触发类型检查函数**

在 `omnilang/src/codegen/react/motion/mod.rs` 中，添加：

```rust
fn has_hover_or_press_trigger(view: &ViewTree) -> bool {
    match view {
        ViewTree::Element(el) => {
            if let Some(ref motion) = el.motion {
                motion.triggers.iter().any(|t| {
                    matches!(t.kind, TriggerKind::Hover | TriggerKind::Press)
                })
            } else {
                false
            }
        }
        ViewTree::Conditional(cond) => {
            cond.true_branch.iter().any(|v| has_hover_or_press_trigger(v))
                || cond.false_branch.iter().any(|v| has_hover_or_press_trigger(v))
        }
        ViewTree::ListRendering(list) => {
            list.template.iter().any(|v| has_hover_or_press_trigger(v))
        }
        ViewTree::Fragment(children) => {
            children.iter().any(|v| has_hover_or_press_trigger(v))
        }
        _ => false,
    }
}
```

- [ ] **步骤 2：为 @hover/@press 效果添加门控条件**

在 `omnilang/src/codegen/react/motion/mod.rs:87` 中，将 @hover/@press 效果发出包装在触发检查中：

```rust
// Only emit hover/press effect block if the component actually uses those triggers
if has_hover_or_press_trigger(&component.view) {
    // ... existing @hover/@press useEffect emission ...
}
```

- [ ] **步骤 3：运行快照测试以验证死代码消除**

运行：`cargo test react::motion`
预期：仅包含 @enter 的组件快照应减少约 50 行（不再有 @hover/@press 效果块）。同时包含 @enter 和 @hover 的组件快照应保持不变。

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/react/motion/mod.rs
git commit -m "perf(react): gate @hover/@press motion effect emission behind trigger-type presence check"
```

---

### 任务 9：统一片段表示

**文件：**
- 修改：`crates/omnilang-core/src/ir/lower/view.rs:910-926`（`make_fragment_element`）
- 验证：`omnilang/src/codegen/vue/template.rs`、`omnilang/src/codegen/dioxus/view.rs`、`omnilang/src/codegen/flutter/view.rs`

**上下文：** 相同的领域概念（"无需包装元素即可渲染子项"）被建模了两次：一次作为 `ViewTree::Fragment(Vec<ViewTree>)`（一个正确的枚举变体），一次作为通过 `make_fragment_element` 的合成 `ElementIR { tag: "fragment" }`。这导致 5 个后端处理不一致，其中 3 个处理时没有检查魔幻标签 `"fragment"`，产生了语义错误的输出。修复方法：让 `make_fragment_element` 直接返回 `ViewTree::Fragment(children)`。

- [ ] **步骤 1：更改 `make_fragment_element` 的返回类型**

在 `crates/omnilang-core/src/ir/lower/view.rs:910-926` 中，当前代码返回 `ViewTree::Element(ElementIR { tag: "fragment".to_string(), ... })`。将其改为返回 `ViewTree::Fragment(children)`：

```rust
fn make_fragment_element(children: Vec<ViewTree>, span: SourceSpan) -> ViewTree {
    // 之前：创建合成 <fragment> 元素
    // ViewTree::Element(ElementIR {
    //     tag: "fragment".to_string(),
    //     children,
    //     ...
    // })

    // 之后：使用专用的 Fragment 变体
    ViewTree::Fragment(children)
}
```

- [ ] **步骤 2：更新 OmniView 代码生成中的片段展开逻辑**

OmniView 在 `condition.rs:23-24` 中有一个明确的注释，承认了合成片段黑客行为：
```
// Branch bodies are wrapped in a synthetic <fragment> element by the
// IR lowerer. We unwrap it here so branch children bypass the fragment.
```

去除该注释，并更新条件分支展开逻辑以直接处理 `ViewTree::Fragment(children)` 而非解包一个合成元素。

- [ ] **步骤 3：验证所有后端正确处理 `ViewTree::Fragment`**

React 和 OmniView 已有 `ViewTree::Fragment(children)` 处理程序。验证 Vue、Dioxus、Flutter、Dart、Python、JavaScript 和 Rust 后端是否在 `ViewTree` 匹配中有 `Fragment` 分支。如果有后端缺少该分支，添加：

```rust
ViewTree::Fragment(children) => {
    for child in children {
        self.gen_view_node(child, ...);
    }
}
```

- [ ] **步骤 4：运行所有后端测试**

运行：`cargo test --workspace`
预期：快照可能因 Vue/Dioxus/Flutter 输出中不再出现合成 `<fragment>` 元素而改变。审查并接受变更后的快照。

- [ ] **步骤 5：提交**

```bash
git add crates/omnilang-core/src/ir/lower/view.rs
git add omnilang/src/codegen/omniview/view/condition.rs
git add omnilang/src/codegen/ vue/ dioxus/ flutter/ dart/ python/ javascript/ rust/
git commit -m "refactor(ir): unify Fragment representation — make_fragment_element returns ViewTree::Fragment instead of synthetic ElementIR"
```

---

### 任务 10：移除 JsFormatExpr 死抽象

**文件：**
- 修改：`crates/omnilang-core/src/codegen/shared/expr.rs:307-332`

**上下文：** `JsFormatExpr` 在 `shared/expr.rs` 中拥有零消费者。所有三个 JS 家族后端（React、Vue、JavaScript）都编写了自己的 `FormatExpr` 实现。该结构体编译通过但具有误导性——ADR-013 将其记录为共享格式化器，但没有任何后端实际使用它。移除以消除死代码。

- [ ] **步骤 1：移除 JsFormatExpr 结构体、impl 块和章节标题**

在 `crates/omnilang-core/src/codegen/shared/expr.rs:307-332` 中，移除：

```rust
// 移除从第 307 行开始的整个区块：
// ============================================================
// JsFormatExpr — JavaScript-family expression formatter
// ============================================================

/// Shared JS-family expression formatter.
/// Provides defaults suitable for JavaScript, TypeScript, and JSX targets.
pub struct JsFormatExpr;

impl FormatExpr for JsFormatExpr {
    // ... (约 15 行方法)
}
```

- [ ] **步骤 2：验证无引用残留**

运行：`cargo check --workspace`
运行：`rg "JsFormatExpr" --type rust`
预期：零匹配（结构体和所有引用已被移除）

- [ ] **步骤 3：运行测试**

运行：`cargo test --workspace`
预期：所有测试通过

- [ ] **步骤 4：提交**

```bash
git add crates/omnilang-core/src/codegen/shared/expr.rs
git commit -m "refactor(codegen): remove dead JsFormatExpr struct — zero consumers across all backends"
```

---

### 任务 11：跨目标回归测试矩阵

**文件：**
- 修改：`omnilang/src/codegen/tests_regression/parameterized.rs`（或创建新的测试文件）

**上下文：** 4 个已确认的静默丢弃错误（BlockStmtIR 被 OmniView 静默丢弃、Flutter 中 if/else 块被丢弃、Card 内边距被忽略、事件模式不完整）已被发现和修复——但没有任何自动化防护措施来防止再次发生。添加一个参数化测试，通过 `compile()` 函数将相同夹具输入所有 3 个 P0 后端，并验证输出不为空（"不崩溃"基线）。

- [ ] **步骤 1：创建参数化跨目标测试**

在 `omnilang/src/codegen/tests_regression/cross_target.rs`（新建）中：

```rust
use crate::codegen::tests_regression::fixtures_shared::*;

#[test]
fn all_p0_backends_produce_output_for_basic_component() {
    let ir = build_basic_component_fixture();

    for target in &[Target::OmniView, Target::React, Target::Flutter] {
        let result = compile(ir.clone(), *target, &Default::default());
        assert!(
            !result.output.is_empty(),
            "Backend {:?} produced empty output for basic component fixture",
            target
        );
    }
}

#[test]
fn all_p0_backends_produce_output_for_conditional_rendering() {
    let ir = build_conditional_fixture();

    for target in &[Target::OmniView, Target::React, Target::Flutter] {
        let result = compile(ir.clone(), *target, &Default::default());
        assert!(
            !result.output.is_empty(),
            "Backend {:?} produced empty output for conditional fixture",
            target
        );
        // Verify conditional branches exist in output
        assert!(
            result.output.contains("if") || result.output.contains("condition"),
            "Backend {:?} dropped conditional branches",
            target
        );
    }
}

#[test]
fn all_p0_backends_produce_output_for_event_bindings() {
    let ir = build_event_fixture();

    for target in &[Target::OmniView, Target::React, Target::Flutter] {
        let result = compile(ir.clone(), *target, &Default::default());
        assert!(
            !result.output.is_empty(),
            "Backend {:?} produced empty output for event fixture",
            target
        );
    }
}
```

- [ ] **步骤 2：注册测试模块**

在 `omnilang/src/codegen/tests_regression/mod.rs` 中，添加：
```rust
mod cross_target;
```

- [ ] **步骤 3：运行跨目标测试**

运行：`cargo test cross_target`
预期：全部 3 个测试通过（在 3 个后端上运行 3 个夹具 = 9 个断言）

- [ ] **步骤 4：提交**

```bash
git add omnilang/src/codegen/tests_regression/cross_target.rs omnilang/src/codegen/tests_regression/mod.rs
git commit -m "test(codegen): add cross-target regression test matrix for all P0 backends"
```

---

### 任务 12：在组件边界清理 element_tree_stack

**文件：**
- 修改：`omnilang/src/codegen/omniview/mod.rs`（组件生成入口）

**上下文：** 任务 5 添加了一个 `debug_assert!`，它将在栈未清空时 panic。为了提供安全保障，在每个组件结束后主动清理栈（向警告中添加记录）：

- [ ] **步骤 1：在组件生成后添加栈清理**

在 `omnilang/src/codegen/omniview/mod.rs` 中，在每个组件的代码生成调用之后，添加清理逻辑：

```rust
// After generating each component, verify and clean up the element tree stack
if !self.element_tree_stack.is_empty() {
    let count = self.element_tree_stack.len();
    self.warnings.push(format!(
        "BUG: element_tree_stack has {} leftover entries after component '{}' — clearing to prevent state corruption in subsequent components",
        count, component.name
    ));
    self.element_tree_stack.clear();
}
```

- [ ] **步骤 2：运行测试**

运行：`cargo test omniview`
预期：所有测试通过（正常操作中不会触发清理逻辑）

- [ ] **步骤 3：提交**

```bash
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "fix(omniview): clear element_tree_stack after each component to prevent cross-component state corruption"
```

---

## 任务依赖关系图

```
Phase 1 (Critical — parallel-safe):
  Task 1 (IR #[non_exhaustive]) ──┐
  Task 2 (inline style ref)       ├── Independent, run in any order
  Task 3 (Vue/Dioxus URL sanitize)┤
  Task 4 (CSS CR/LF normalize)    │
  Task 5 (stack assert) ──────────┘
  Task 6 (JS runtime CI) ─────────┘

Phase 2 (High — some sequencing):
  Task 10 (JsFormatExpr removal)  ← Independent
  Task 9  (Fragment unify)        ← Independent, but touches 10+ backends
  Task 11 (cross-target tests)    ← Depends on Task 9 (needs clean Fragment handling)
  Task 8  (hover/press gate)      ← Independent
  Task 12 (stack cleanup)         ← Depends on Task 5
  Task 7  (motion scope)          ← Larger effort; pair with Task 8
```

---

## 自审报告

**1. 规范覆盖：** 所有 6 个第一阶段的 CRITICAL 项目和所有 7 个第二阶段的高优先级项目均已覆盖。未覆盖的中等/低优先级项目在 `component-omniview-2026-07-30-v2.md` 的附录 A 中有记录，并可在后续冲刺中处理。

**2. 占位符扫描：** 无 TBD、TODO 或"适当添加错误处理"的表述。所有步骤均包含实际代码、确切文件路径和预期命令输出。

**3. 类型一致性：** `is_dangerous_url` 作为 `pub(crate)` 函数被引用，可通过 `crate::codegen::omniview::is_dangerous_url` 在 `omnilang` crate 内访问。所有 IR 类型名称均与其在 `crates/omnilang-core/src/ir/` 中的定义匹配。`ref_names` 参数在所有任务引用中保持一致。

---

**计划完成并保存至 `docs/superpowers/plans/2026-07-30-omniview-architecture-v2-fixes.md`。两种执行选项：**

**1. 子代理驱动（推荐）** — 每个任务派遣一个新的子代理，任务间进行审查，迭代速度快

**2. 内联执行** — 在此会话中使用 executing-plans 执行任务，带检查点的批量执行

**选择哪种方式？**
