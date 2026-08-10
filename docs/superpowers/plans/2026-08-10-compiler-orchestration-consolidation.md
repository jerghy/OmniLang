# 编译器编排层收口 + S 级修复 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复架构评审（`.architecture/reviews/component-omnilang-compiler-2026-08-10.md`）实证确认的 Critical 两项 + S 级修复 6 项：程序级 pass 编排单一化、跨文件 required-prop 校验、URL 消毒器 C0 缺口、hidden 布尔属性、缓存容量、WASM 缓存路由、画布 success 语义、IRProgram 版本对齐。

**Architecture:** 全部为纯提取/纯增量修复。核心是把三处逐字复制的程序级 pass 序列（`analyse_multi` / `analyse_multi_cached` / `run_pipeline`）收敛为单一 `run_program_passes` 函数（先例：既有 `run_ir_passes`），然后把跨文件 required-prop 校验移入该函数；其余任务各自独立（sanitizer 过滤集放宽、布尔属性集增补、缓存容量参数、WASM 路由一行改、画布门控 errors-only、Default 版本对齐），互不依赖。

**Tech Stack:** Rust（crates/omnilang-core + omnilang + omnilang-wasm）、Python 门禁脚本（verify-semantic-universe.py）、Node（js_runtime 测试）。

## Global Constraints

- 全部修改为**纯提取或行为修复**：除任务 2（新增跨文件 E0011 诊断）与任务 3（新增拦截）外，不改变任何既有诊断行为、不改变生成代码输出（任务 3/4 例外：skeleton/toast 无关，hidden 走既有布尔分支、sanitize 只放宽拦截面——**既有测试断言必须保持全绿**）
- **不触碰批次 1 未提交代码**：工作树中 `omnilang/src/codegen/flutter/{mod,ref,view}.rs` 为未提交修改（B1 Task 5）——本计划任何任务不得修改这些文件，避免与批次 1 收口冲突
- 测试基线：`cargo test -p omnilang-core`、`cargo test -p omnilang`、`cargo test -p omnilang-wasm`、`cargo clippy -- -D warnings` 全绿；rustfmt 通过
- 中文诊断消息政策（ADR-056 决策 1）：新增诊断消息必须中文（本计划任务 2/3 无新增消息文本——E0011 复用既有、sanitize 无消息）
- 每次任务结束 `cargo fmt` + 独立 commit（commit-at-task 粒度）
- 门禁脚本修改（任务 4）：`python scripts/verify-semantic-universe.py` 必须 0 problems

---

### Task 1: 提取 `run_program_passes`——程序级 pass 序列单一化

**Files:**
- Modify: `crates/omnilang-core/src/analyse.rs:100-185`（新增函数，放在 `run_ir_passes` 之后）
- Modify: `crates/omnilang-core/src/analyse.rs:497-520`（analyse_multi 三处调用点 ①）
- Modify: `crates/omnilang-core/src/analyse.rs:789-812`（analyse_multi_cached ②）
- Modify: `crates/omnilang-core/src/lib.rs:113-136`（run_pipeline ③）
- Test: `crates/omnilang-core/src/analyse.rs`（tests mod，:836 起）

**Interfaces:**
- Consumes: 既有 `crate::ir::optimize::optimize(&mut IRProgram)`、`crate::semantic::validate_theme::validate_theme(&mut IRProgram, &mut Vec<Diagnostic>)`、`validate_event_names`、`validate_form_load`、`validate_feedback_modal`（签名均为 `(&IRProgram | &mut IRProgram, &mut Vec<Diagnostic>)`）
- Produces: `pub(crate) fn run_program_passes(ir: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>)`——三个管线入口的唯一程序级 pass 序列；任务 2 将在此函数中加入跨文件 prop 校验

- [ ] **Step 1: 写对等测试（先验证当前两条路径诊断一致）**

在 `crates/omnilang-core/src/analyse.rs` 的 `mod tests`（:836 起，`use super::*;` 已含 `analyse_multi`；追加 `use crate::run_pipeline;`）末尾加：

```rust
/// 管线对等：单文件 run_pipeline 与多文件 analyse_multi 必须产出
/// (kind, message) 相同的程序级诊断（历史缺陷：validate_event_names
/// 曾漏接线单文件路径，单文件 @onTotallyBogus 静默 "No issues found"）。
#[test]
fn pipeline_parity_program_level_diagnostics() {
    let src = r#"import omniview;

component App {
    view {
        <div @onTotallyBogus={f}></div>
    }
    fn f() {}
}
"#;

    // 单文件路径
    let single = crate::run_pipeline(src);
    // 多文件路径（同一源码作唯一文件）
    let mut sources: FxHashMap<String, String> = FxHashMap::default();
    sources.insert("main.omni".to_string(), src.to_string());
    let path_mapping = crate::path_mapping::derive_path_mapping(&sources);
    let multi = analyse_multi(&sources, &path_mapping);

    // 事件名诊断 kind 为 E0006（error.rs:97，validate_event_names.rs:203 实证）
    let ev = |ds: &[Diagnostic]| -> Vec<(String, String)> {
        ds.iter()
            .filter(|d| d.kind == DiagnosticKind::E0006)
            .map(|d| (format!("{:?}", d.kind), d.message.clone()))
            .collect()
    };

    let single_ev = ev(&single.diagnostics);
    let multi_ev = ev(&multi.diagnostics);
    assert!(
        !single_ev.is_empty(),
        "单文件路径必须产出 E0006 事件名诊断，got: {:?}",
        single.diagnostics
    );
    assert_eq!(single_ev, multi_ev, "两条管线的事件名诊断必须一致");
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p omnilang-core pipeline_parity_program_level_diagnostics -- --nocapture`
Expected: FAIL——`assert!(!single_ev.is_empty())` 失败（当前单文件路径缺 UnknownEventName 诊断，正是 lib.rs:120-126 注释记载的历史场景；若此断言意外通过，说明该场景已修复，则改用另一个程序级诊断场景（如 `@themes` 缺失 `@theme` 配对的 validate_theme 硬错误）重写 fixture 直到失败成立）

- [ ] **Step 3: 提取 `run_program_passes`**

在 `crates/omnilang-core/src/analyse.rs` 的 `run_ir_passes` 函数（:185 结束）之后新增：

```rust
/// Program-level passes on the merged IR. Owned by this one function so
/// every pipeline entry point — single-file `run_pipeline`, multi-file
/// `analyse_multi`, cached `analyse_multi_cached` — sees identical
/// diagnostics in identical order. A pass added here reaches all paths;
/// a pass hand-appended at one call site silently diverges (historical
/// incident: validate_event_names was retroactively added to run_pipeline
/// after single-file paths shipped missing diagnostics).
pub(crate) fn run_program_passes(ir: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    // Phase 3c: IR optimization passes
    crate::ir::optimize::optimize(ir);

    // Phase 3d: Program-level theme validation (@themes ↔ @theme pairing,
    // design D1). Runs on the merged IR so cross-file pairing is visible;
    // also populates DesignIR.theme_signal for every design.
    crate::semantic::validate_theme::validate_theme(ir, diagnostics);

    // Phase 3e: Event-name validation (Task 4.2, D-B) — unknown event names
    // warn with the nearest legal suggestion. Runs on the merged IR so
    // cross-file custom `event` declarations and component tags are visible
    // (a per-file pass cannot tell an imported component's custom-event
    // binding apart from a typo'd DOM event).
    crate::semantic::validate_event_names::validate_event_names(ir, diagnostics);

    // Phase 3f: Form/data tri-state validation (Domain A, Task 3) — consumes
    // FormIntentIR/LoadIntentIR collected during lowering, checks V-F2~F8.
    // V-F1/V-F6 are reported at the lowering scan (ir/lower/component.rs).
    crate::semantic::validate_form_load::validate_form_load(ir, diagnostics);

    // Phase 3f2: Feedback-modal validation (batch 1, domain 10) — consumes
    // ModalIntentIR/SkeletonBindingIR/ErrorBoundaryBindingIR, checks V1-V14
    // (V4/V11 duplicate bindings are reported at lowering).
    crate::semantic::validate_feedback_modal::validate_feedback_modal(ir, diagnostics);
}
```

- [ ] **Step 4: 三处调用点替换**

**调用点 ①** `crates/omnilang-core/src/analyse.rs:497-520`——把 `// Phase 3c: IR optimization passes` 起至 `validate_feedback_modal` 调用（:520）整段替换为：

```rust
    // Phase 3c-3f2: program-level passes on the merged IR (sequence owned
    // by run_program_passes — see its doc comment).
    run_program_passes(&mut merged_ir, &mut diags);
```

**调用点 ②** `crates/omnilang-core/src/analyse.rs:789-812`——同样替换（`optimize` 起至 `validate_feedback_modal`）。

**调用点 ③** `crates/omnilang-core/src/lib.rs:113-136`——把 `crate::ir::optimize::optimize(&mut ir);` 起至 `validate_feedback_modal` 调用整段替换为：

```rust
    // Program-level passes (optimize + theme/event/form/feedback-modal
    // validation) — sequence owned by analyse::run_program_passes so
    // single-file and multi-file paths see identical diagnostics.
    crate::analyse::run_program_passes(&mut ir, &mut diagnostics);
```

> 三处原有的 `lints::run_all` 调用（lib.rs:146-155、analyse.rs:522-540、analyse.rs:814-...）**保持原位不动**（其签名三处形状不同：programs 参数与 lint_to_diag 转换），只提取 4-pass 序列 + optimize。

- [ ] **Step 5: 跑测试确认通过**

Run: `cargo test -p omnilang-core`（含新对等测试）+ `cargo test -p omnilang`
Expected: 全绿；`pipeline_parity_program_level_diagnostics` PASS（单文件与多文件事件名诊断一致）

- [ ] **Step 6: clippy + fmt + commit**

```bash
cargo clippy -p omnilang-core --all-features -- -D warnings
cargo fmt
git add crates/omnilang-core/src/analyse.rs crates/omnilang-core/src/lib.rs
git commit -m "refactor(analyse): 提取 run_program_passes——程序级 pass 序列单一化（三处编排点收敛）"
```

---

### Task 2: 跨文件 required-prop 校验移到程序级（E0011 必报）

**Files:**
- Modify: `crates/omnilang-core/src/analyse.rs:100-105`（从 `run_ir_passes` 移除 `validate_required_props`）
- Modify: `crates/omnilang-core/src/analyse.rs`（`run_program_passes` 内新增调用，任务 1 的函数体）
- Test: `crates/omnilang-core/src/analyse.rs`（tests mod）

**Interfaces:**
- Consumes: 任务 1 的 `run_program_passes(ir: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>)`；既有 `crate::semantic::validate_props::validate_required_props(&IRProgram) -> Vec<Diagnostic>`（validate_props.rs:18——用 `program.iter_all_components()` 建 component_map，merged IR 上运行时含全部文件的组件声明）
- Produces: 行为变化——跨文件 `<Button />` 缺 required prop 从「编译零诊断」变为 E0011 硬错误（同文件行为不变）

- [ ] **Step 1: 写失败测试（跨文件缺失 required prop 必须报 E0011）**

在 `crates/omnilang-core/src/analyse.rs` 的 `mod tests` 末尾追加：

```rust
/// 跨文件 required-prop：a.omni 引用 b.omni 声明的组件且缺 required prop
/// 时必须报 E0011——per-file 运行时 component_map 看不见另一文件的声明，
/// 曾静默编译成功（评审 HIGH-1，实证复现）。
#[test]
fn cross_file_required_prop_errors() {
    let lib_src = r#"import omniview;

component Button {
  prop title: String;

  view {
    <button>{title}</button>
  }
}
"#;
    let app_src = r#"import omniview;
import { Button } from "./b";

component App {
  view {
    <Button />
  }
}
"#;
    let mut sources: FxHashMap<String, String> = FxHashMap::default();
    sources.insert("a.omni".to_string(), app_src.to_string());
    sources.insert("b.omni".to_string(), lib_src.to_string());
    let path_mapping = crate::path_mapping::derive_path_mapping(&sources);

    let out = analyse_multi(&sources, &path_mapping);
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.message.contains("required prop 'title'")),
        "跨文件缺失 required prop 必须报 E0011，got: {:?}",
        out.diagnostics
    );
    assert!(!out.success, "跨文件缺 required prop 时 success 必须为 false");
}

/// 同文件对照（回归保护）：单文件内缺 required prop 仍报 E0011。
#[test]
fn same_file_required_prop_still_errors() {
    let src = r#"import omniview;

component Button {
  prop title: String;

  view {
    <button>{title}</button>
  }
}

component App {
  view {
    <Button />
  }
}
"#;
    let mut sources: FxHashMap<String, String> = FxHashMap::default();
    sources.insert("same.omni".to_string(), src.to_string());
    let path_mapping = crate::path_mapping::derive_path_mapping(&sources);

    let out = analyse_multi(&sources, &path_mapping);
    assert!(
        out.diagnostics
            .iter()
            .any(|d| d.message.contains("required prop 'title'")),
        "同文件缺失 required prop 必须报 E0011，got: {:?}",
        out.diagnostics
    );
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p omnilang-core cross_file_required_prop_errors -- --nocapture`
Expected: FAIL——`assert!(...any(...))` 失败（当前跨文件零诊断；同文件测试此时 PASS，证明缺陷仅跨文件）

- [ ] **Step 3: 从 `run_ir_passes` 移除、加入 `run_program_passes`**

**移除** `crates/omnilang-core/src/analyse.rs:100-105`：

```rust
    // Validate required props at component call sites
    let prop_errors = crate::semantic::validate_props::validate_required_props(ir_program);
    for err in prop_errors {
        diagnostics.push(err);
    }
```

**加入** `run_program_passes`（任务 1 的函数体）——放在 `optimize` 调用之后、`validate_theme` 之前：

```rust
    // Validate required props at component call sites — runs on the merged
    // IR so cross-file instantiations (import { X } from "./b") see the
    // declaring component's props; a per-file run silently skipped them
    // (E0011 must fire for cross-file call sites too).
    let prop_errors = crate::semantic::validate_props::validate_required_props(ir);
    diagnostics.extend(prop_errors);
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p omnilang-core` + `cargo test -p omnilang`
Expected: 全绿——两个新测试 PASS；既有测试无回归（单文件路径经 run_pipeline → lowering → run_ir_passes 不再跑 prop 校验，但 run_pipeline 现在经 run_program_passes 跑——同文件行为不变）

- [ ] **Step 5: 端到端实证（可选但推荐——用 CLI 复现评审场景）**

```bash
cargo build -p omnic
# 用评审时的 fixture：a.omni import b.omni 缺 title
omnic build a.omni b.omni -t react -o /tmp/out
```
Expected: 输出 `required prop 'title' not provided for component 'Button'`（此前是 `Compiled 4 file(s)` 静默成功）

- [ ] **Step 6: clippy + fmt + commit**

```bash
cargo clippy -p omnilang-core --all-features -- -D warnings
cargo fmt
git add crates/omnilang-core/src/analyse.rs
git commit -m "fix(analyse): 跨文件 required-prop 校验移到程序级 merged IR——E0011 跨文件必报（评审 HIGH-1）"
```

---

### Task 3: URL 消毒器 C0 控制字符缺口修复

**Files:**
- Modify: `crates/omnilang-core/src/codegen/sanitize.rs:148-159`（`is_dangerous_url`）
- Modify: `omnilang/src/codegen/omniview/runtime.rs:169-186`（`__omni_sanitizeUrl` 发射串）
- Test: `crates/omnilang-core/src/codegen/sanitize.rs`（`test_is_dangerous_url`，:402-415）
- Test: `omnilang/tests/js_runtime/test_sanitizers.js`（JS 副本——**同步修改**）
- Test: `omnilang/src/codegen/omniview/tests_regression/sanitizers.rs`（生成文本断言）

**Interfaces:**
- Consumes: 既有 `is_dangerous_url(&str) -> bool`、`__omni_sanitizeUrl(url)`（发射字符串）
- Produces: 行为变化——`"\u{1}javascript:alert(1)"`、`"\u{0B}javascript:..."`、`"\u{7F}data:..."` 等 C0/DEL 前缀载荷从「放行」变为「拦截」；合法 URL（https/mailto/相对路径/锚点）行为不变

- [ ] **Step 1: 写失败测试**

**Rust 侧**——在 `crates/omnilang-core/src/codegen/sanitize.rs` 的 `test_is_dangerous_url`（:402）中追加：

```rust
        // C0/DEL control-char bypass: lexer decodes \u{1}/\u{0B}/\u{7F} into
        // the string value, browsers strip leading C0 controls + spaces
        // before scheme parsing — the filter must strip ALL control chars,
        // not just CR/LF/TAB (review MEDIUM-1, previously fixed twice).
        assert!(is_dangerous_url("\u{1}javascript:alert(1)"));
        assert!(is_dangerous_url("\u{0B}javascript:alert(1)"));
        assert!(is_dangerous_url("\u{7F}data:text/html,<script>alert(1)</script>"));
        assert!(is_dangerous_url("\u{1}\u{1}javascript:void(0)"));
```

**JS 侧**——在 `omnilang/tests/js_runtime/test_sanitizers.js` 的共享断言集（文件底部 `// ─── Shared assertion set ───` 区域，对 `__omni_sanitizeUrl` 的断言列表）追加：

```js
  // C0/DEL control-char bypass (review MEDIUM-1)
  assert.strictEqual(__omni_sanitizeUrl('javascript:alert(1)'), '#blocked');
  assert.strictEqual(__omni_sanitizeUrl('javascript:alert(1)'), '#blocked');
  assert.strictEqual(__omni_sanitizeUrl('data:text/html,<script>alert(1)</script>'), '#blocked');
```

（若 JS 测试文件的断言集不是直接函数调用形式，先读文件底部 40 行照既有结构加条目。）

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p omnilang-core is_dangerous_url -- --nocapture` 和 `cd omnilang/tests/js_runtime && node test_sanitizers.js`
Expected: 两处 FAIL——`\u{1}javascript:` 断言失败（当前过滤只剥 `\n\r\t`）

- [ ] **Step 3: 实现——放宽两处过滤集**

**Rust 侧** `sanitize.rs:154`——把过滤闭包改为：

```rust
    // Strip CR, LF, TAB — browsers strip these before URL navigation.
    // Also strip all C0 controls + DEL: the lexer decodes \u{1}/\u{0B}/
    // \u{7F} into string values and browsers strip leading C0 controls
    // and spaces before scheme parsing, so "\u{1}javascript:" would
    // execute as "javascript:" (fixed twice for CR/LF/TAB — generalize
    // to the full class, review MEDIUM-1).
    let stripped: String = lower
        .chars()
        .filter(|c| !c.is_control())
        .collect();
```

**JS 侧** `omnilang/src/codegen/omniview/runtime.rs` 发射串（:175）——把正则改为：

```rust
  // Strip CR, LF, TAB and all C0 controls + DEL before scheme matching —
  // browsers (Chromium, WebKit, Gecko) strip these characters from URLs
  // before navigation, so "java\nscript:alert(1)" or "javascript:"
  // would execute as "javascript:alert(1)".
  var normalized = trimmed.replace(/[\x00-\x1F\x7F]/g, '');
```

**同步 JS 测试副本** `omnilang/tests/js_runtime/test_sanitizers.js` 顶部的 `__omni_sanitizeUrl` 函数体（该文件是 runtime 的镜像副本，必须同改，否则共享断言集对副本失败）。

**同步文本断言** `omnilang/src/codegen/omniview/tests_regression/sanitizers.rs`——若该文件断言发射串含 `[\r\n\t]` 模式，更新为 `[\x00-\x1F\x7F]`（先 `grep -n "\\\\r\\\\n\\\\t\|r\\n\\t" omnilang/src/codegen/omniview/tests_regression/sanitizers.rs` 确认有无该断言再改）。

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p omnilang-core` + `cargo test -p omnilang` + `cd omnilang/tests/js_runtime && node test_sanitizers.js`
Expected: 全绿——新断言 PASS；既有 sanitize 断言（合法 URL 放行、\n\r\t 案例）无回归

- [ ] **Step 5: 端到端实证（可选）**

```bash
cargo build -p omnic
# fixture: <a href="\u{1}javascript:alert(1)">x</a> → 生成产物 grep 应含 __omni_sanitizeUrl 且运行时返回 #blocked
```
（编译期 is_dangerous_url 拦截发生在静态 URL 校验路径——生成代码中 href 字面量不会出现 javascript:）

- [ ] **Step 6: clippy + fmt + commit**

```bash
cargo clippy -p omnilang-core --all-features -- -D warnings && cargo clippy -p omnilang --all-features -- -D warnings
cargo fmt
git add crates/omnilang-core/src/codegen/sanitize.rs omnilang/src/codegen/omniview/runtime.rs omnilang/tests/js_runtime/test_sanitizers.js omnilang/src/codegen/omniview/tests_regression/sanitizers.rs
git commit -m "fix(sanitize): URL 消毒器剥除全部 C0 控制字符 + DEL——\u{1}javascript: 绕过闭环（评审 MEDIUM-1）"
```

---

### Task 4: `hidden`/`autoplay` 布尔属性入册 + 域 7 断言升级

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:19-33`（`HTML_BOOLEAN_ATTRS` 13→15）
- Modify: `omnilang/src/codegen/omniview/view/element.rs:126-136`（已知属性白名单 match 增补 hidden/autoplay）
- Modify: `scripts/verify-semantic-universe.py`（域 7 布尔断言：存在性 → 集合包含）
- Test: `omnilang/src/codegen/omniview/tests.rs`（新增 codegen 测试）

**Interfaces:**
- Consumes: 既有 `is_boolean_attr = HTML_BOOLEAN_ATTRS.contains(&name)`（element.rs:139）→ DOM property 分支（:154-161，`el.hidden = !!(...)` effect）；白名单 match（:126-136）→ 静态/动态属性发射
- Produces: 行为变化——`<div hidden={showModal}>` 从「warning + 静态 data-hidden（不工作）」变为「`el.hidden = !!(showModal)` 响应式 effect（工作）」；`<video autoplay={flag}>` 同理

- [ ] **Step 1: 写失败测试**

在 `omnilang/src/codegen/omniview/tests.rs` 的 tests mod 末尾追加：

```rust
/// 评审 systems-architect MEDIUM-2：hidden 是最常见的可见性切换模式，
/// 但不在 HTML_BOOLEAN_ATTRS/白名单——曾落 _ => fallback 发射静态
/// data-hidden（属性不工作，B2 bug 类活跃实例）。
#[test]
fn dynamic_hidden_emits_dom_property_effect() {
    let src = r#"import omniview;

component App {
  signal open = false;
  view {
    <div hidden={open}></div>
  }
}
"#;
    let out = crate::codegen::omniview::generate_for_test(src);
    let js = out.files.values().next().map(|f| f.content.clone()).unwrap_or_default();
    // 布尔属性必须走 DOM property（data-hidden 静态发射 = 属性不工作）
    assert!(
        js.contains("el.hidden = !!"),
        "hidden 动态绑定必须生成 el.hidden = !!() effect, got:\n{js}"
    );
    assert!(
        !js.contains("data-hidden"),
        "hidden 不得落 data-* fallback, got:\n{js}"
    );
}
```

> 若 `generate_for_test` 不存在，改查 `omnilang/src/codegen/omniview/tests.rs` 现有测试的入口函数（grep `fn generate` 或 `pub fn` 找最接近的 helper）照用；若需通过 `crate::compile_to_target_json` 走完整管线，用 `omnilang::compile_to_target` 亦可。

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p omnilang dynamic_hidden_emits_dom_property_effect -- --nocapture`
Expected: FAIL——`js.contains("el.hidden = !!")` 失败且 `data-hidden` 出现（当前 fallback 行为）

- [ ] **Step 3: 实现——两处增补**

**①** `element.rs:19-33` 的 `HTML_BOOLEAN_ATTRS` 末尾（`"open",` 之后）追加：

```rust
    "hidden", // 2026-08-10: 最常见的可见性切换——此前落 data-* fallback 不工作（评审 MEDIUM-2）
    "autoplay", // 2026-08-10: 媒体自动播放（parser 关键字已有，codegen 缺失）
```

**②** 白名单 match（:126-136 区域，`"open" | "poster"` 附近）追加两个分支项：

```rust
                | "hidden" | "autoplay"
```

（插入到该 arm 的既有 `"open" | "poster"` 处——保持字母序风格。）

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p omnilang dynamic_hidden_emits_dom_property_effect -- --nocapture` + `cargo test -p omnilang`
Expected: 新测试 PASS；既有测试无回归（open 布尔分支既有测试保持）

- [ ] **Step 5: 域 7 断言升级为集合包含**

修改 `scripts/verify-semantic-universe.py` 域 7 布尔属性断言（:647-649 附近，现为存在性检查「keywords non-empty」）——追加集合包含断言：

```python
    # 2026-08-10 升级（评审 MEDIUM-2）：存在性断言不足以捕获双源漂移——
    # parser 关键字（15 项含 hidden/autoplay）⊆ codegen 布尔集 ∪ 已知属性
    # 白名单，否则新关键字可静默落入 data-* fallback（B2 bug 类）。
    def _codegen_boolean_words(text: str) -> set[str]:
        """HTML_BOOLEAN_ATTRS 数组字面量中的字符串条目。"""
        m = re.search(r"HTML_BOOLEAN_ATTRS: &\[&str\] = &\[(.*?)\];", text, re.S)
        if not m:
            return set()
        return set(re.findall(r'"([^"]+)"', m.group(1)))

    codegen_bool = _codegen_boolean_words(element_src)
    missing = keywords - codegen_bool
    if missing:
        fail(
            "directives: parser 布尔关键字不在 codegen HTML_BOOLEAN_ATTRS: "
            + ", ".join(sorted(missing))
        )
```

（`keywords`/`element_src`/`fail` 为既有变量/函数——先读 :640-660 确认变量名再写。）

Run: `python scripts/verify-semantic-universe.py`
Expected: 0 problems（hidden/autoplay 已在 codegen 集内）

- [ ] **Step 6: clippy + fmt + commit**

```bash
cargo fmt
git add omnilang/src/codegen/omniview/view/element.rs omnilang/src/codegen/omniview/tests.rs scripts/verify-semantic-universe.py
git commit -m "fix(codegen-omniview): hidden/autoplay 入布尔属性集——最常见的可见性切换恢复响应式（评审 MEDIUM-2）+ 域 7 断言升级为集合包含"
```

---

### Task 5: `MAX_CACHE_ENTRIES` 容量与项目规模对齐

**Files:**
- Modify: `crates/omnilang-core/src/analyse.rs:238`（常量注释）+ `:711` 附近（逐出循环）
- Test: `crates/omnilang-core/src/analyse.rs`（tests mod）

**Interfaces:**
- Consumes: `analyse_multi_cached(sources: &FxHashMap<String,String>, ..., cache: &mut IncrementalCache)`——`sources.len()` 在逐出点可及
- Produces: 新增 `fn cache_capacity(file_count: usize) -> usize`（`MAX_CACHE_ENTRIES.max(file_count)`）；逐出循环用其容量——项目文件数超过 50 时缓存不再逐出（15.6× 回归修复）

- [ ] **Step 1: 写失败测试**

在 `crates/omnilang-core/src/analyse.rs` 的 `mod tests` 末尾追加：

```rust
/// 评审 performance HIGH：MAX_CACHE_ENTRIES=50 低于旗舰项目 102 文件——
/// 每击键逐出最旧条目，下击键付全量重解析（实测 15.6× 回归）。
/// 容量必须不低于项目文件数。
#[test]
fn cache_capacity_scales_with_project_size() {
    assert_eq!(cache_capacity(10), MAX_CACHE_ENTRIES, "小项目用常量上限");
    assert_eq!(cache_capacity(50), 50);
    assert_eq!(cache_capacity(102), 102, "旗舰项目 102 文件不得逐出");
    assert_eq!(cache_capacity(500), 500);
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p omnilang-core cache_capacity_scales_with_project_size`
Expected: FAIL——`cache_capacity` 未定义（编译错误即失败形态）

- [ ] **Step 3: 实现**

**①** `analyse.rs:238` 常量旁（或附近）新增函数：

```rust
/// Cache capacity: the LRU must never evict below the project's own file
/// count — a bound under the project size turns the cache into a churn
/// generator (measured 15.6× regression at 102 files, review performance
/// HIGH; the deliberate memory cap intent is preserved above the floor).
fn cache_capacity(file_count: usize) -> usize {
    MAX_CACHE_ENTRIES.max(file_count)
}
```

**②** `:711` 逐出循环——把

```rust
                while cache.insertion_order.len() > MAX_CACHE_ENTRIES {
```

改为

```rust
                let capacity = cache_capacity(sources.len());
                while cache.insertion_order.len() > capacity {
```

（`sources` 是 `analyse_multi_cached` 的参数，逐出点在函数体内可及。）

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p omnilang-core`（新测试 + 既有缓存测试 `cached_run_keeps_diagnostics_of_unchanged_files` 等）
Expected: 全绿

- [ ] **Step 5: clippy + fmt + commit**

```bash
cargo clippy -p omnilang-core --all-features -- -D warnings
cargo fmt
git add crates/omnilang-core/src/analyse.rs
git commit -m "perf(analyse): 缓存容量不低于项目文件数——MAX_CACHE_ENTRIES=50 之下的 15.6× 逐出回归修复（评审 performance HIGH）"
```

---

### Task 6: WASM 对象 API 多文件分支走增量缓存

**Files:**
- Modify: `omnilang-wasm/src/lib.rs:674-675`（`compile_to_omniview` 多文件分支）
- Test: `omnilang-wasm/src/lib.rs`（既有测试区域，:810 附近）

**Interfaces:**
- Consumes: `ANALYSIS_CACHE`（omnilang-wasm thread_local `RefCell<IncrementalCache>`，:341）；`omnilang::compile_multi_internal_cached(sources, target, path_mapping, editor_mode, debug_mode, store_mode: Option<&str>, cache: &mut IncrementalCache) -> InternalCompileResult`（omnilang/src/lib.rs:322）
- Produces: 行为变化——多文件 `compile_to_omniview` 从每次全冷编译（`compile_multi_internal` → 未缓存 `analyse_multi`）变为增量缓存（`analyse_multi_cached`）；结果级缓存（`self.cached_compile_result`）保持不变

- [ ] **Step 1: 写失败测试（行为等价 + 缓存路径可达）**

在 `omnilang-wasm/src/lib.rs` 的 tests mod 追加（先 `grep -n "mod tests" omnilang-wasm/src/lib.rs` 确认位置与既有 helper）：

```rust
    /// 评审 performance MEDIUM：多文件 compile_to_omniview 曾绕过增量缓存
    /// 走全冷 analyse_multi——断言两次连续调用（同 sources）结果一致且
    /// 第二次经 cached 路径不崩溃（结果级缓存已覆盖等价性；此测试防
    /// 接线回归——改回未缓存分支时结果仍一致但失去增量，故以编译
    /// 通过 + 结果一致 + cache 状态存在为守卫）。
    #[test]
    fn multi_file_compile_to_omniview_uses_cached_path() {
        let mut compiler = OmniCompiler::new();
        compiler.upsert_source("a.omni".to_string(), "component A { view { <div>a</div> } }".to_string());
        compiler.upsert_source("b.omni".to_string(), "component B { view { <div>b</div> } }".to_string());

        let r1 = compiler.compile_to_omniview(false, false);
        let r2 = compiler.compile_to_omniview(false, false);
        let (v1, v2): (String, String) = (
            serde_wasm_bindgen::from_value(r1).unwrap(),
            serde_wasm_bindgen::from_value(r2).unwrap(),
        );
        assert_eq!(v1, v2, "两次多文件编译结果必须一致");
    }
```

> API 实证：`OmniCompiler::new()`（:610）+ `upsert_source(path, source)`（:620，`self.sources.insert`）。`serde_wasm_bindgen::from_value` 仅当结果类型可反序列化时可用——否则改为比较 `JsValue` 的 JSON 字符串（`js_sys::JSON::stringify`），并对照既有测试（:947 `OmniCompiler::new()` 用法）确认断言形态。

- [ ] **Step 2: 跑测试确认当前通过（行为守卫先行）**

Run: `cargo test -p omnilang-wasm multi_file_compile_to_omniview_uses_cached_path`
Expected: PASS（当前行为等价——测试是回归守卫，实现改动后才真正验证缓存路径）

- [ ] **Step 3: 实现——多文件分支改走 cached**

`omnilang-wasm/src/lib.rs:674-675`——把

```rust
        } else {
            // Multi-file: use compile_multi_internal with structured result
            let result = omnilang::compile_multi_internal(
                self.sources.clone(),
                omnilang::codegen::Target::OmniView,
                omnilang::path_mapping::derive_path_mapping(&self.sources),
                editor_mode,
                debug_mode,
                None,
            );
            to_js_value(&result)
        };
```

改为

```rust
        } else {
            // Multi-file: route through the incremental cache — the uncached
            // compile_multi_internal paid full cold analysis per call
            // (review performance MEDIUM; 150ms debug-window double-compile
            // pathology).
            let result = ANALYSIS_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                omnilang::compile_multi_internal_cached(
                    self.sources.clone(),
                    omnilang::codegen::Target::OmniView,
                    omnilang::path_mapping::derive_path_mapping(&self.sources),
                    editor_mode,
                    debug_mode,
                    None,
                    &mut cache,
                )
            });
            to_js_value(&result)
        };
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p omnilang-wasm`（全量——含既有测试）
Expected: 全绿

- [ ] **Step 5: clippy + fmt + commit**

```bash
cargo clippy -p omnilang-wasm --all-features -- -D warnings
cargo fmt
git add omnilang-wasm/src/lib.rs
git commit -m "perf(wasm): 多文件 compile_to_omniview 走增量缓存——未缓存分支接线修复（评审 performance MEDIUM）"
```

---

### Task 7: 画布 resolve_styles success 改为 errors-only

**Files:**
- Modify: `omnilang/src/lib.rs:185`（`resolve_styles` success 门控）
- Modify: `omnilang/src/lib.rs:217-220`（`resolve_styles_multi` success 门控）
- Test: `omnilang/tests/resolve_styles_test.rs`

**Interfaces:**
- Consumes: `resolve_styles(source: &str) -> StyleResolveResult`（omnilang/src/lib.rs:179）、`resolve_styles_multi(...)`（:206）——`StyleResolveResult` 定义于 `omnilang/src/codegen/omniview/mod.rs:110`，含 `success: bool` + `warnings: Vec<String>`（:127，warnings 经此字段透出，展示不受影响）
- Produces: 行为变化——warning 输入（如未知属性动态绑定）从 `success=false` 变为 `success=true`（与编译路径 :464 errors-only 一致）；warnings 仍在结果信封内透出

- [ ] **Step 1: 写失败测试**

在 `omnilang/tests/resolve_styles_test.rs` 末尾追加：

```rust
/// 评审 systems-architect MEDIUM-1：画布 success 曾把 warnings 计入——
/// B2 后未知属性动态绑定产生 warning，同一输入 CLI 构建成功而画布
/// 显示失败。success 必须只按 errors 门控（与编译路径一致）。
#[cfg(feature = "codegen-omniview")]
#[test]
fn test_resolve_styles_warning_does_not_flip_success() {
    // 裸未知属性 + 动态值 → B2 warning（unknown attribute 'foo'），
    // 但无 error——画布必须报 success。
    let source = r#"import omniview;

component App {
  signal count = 0;
  view {
    <div foo={count}></div>
  }
}
"#;
    let result = omnilang::resolve_styles(source);
    assert!(
        !result.warnings.is_empty(),
        "fixture 必须产生 warning（未知属性动态绑定），got: {:?}",
        result.warnings
    );
    assert!(
        result.success,
        "仅 warning 无 error 时画布 success 必须为 true，warnings: {:?}",
        result.warnings
    );
}
```

> 若该 fixture 不触发 warning（`foo` 可能被当作透传），先用 `omnic build` 或既有 B2 测试（`omnilang/src/codegen/omniview/view/element.rs` 的 fallback warning 测试）确认触发条件，换成 `class: unknownClass`（既有 `test_resolve_styles_warns_unknown_class` 已证实的 warning 源）。

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p omnilang --test resolve_styles_test test_resolve_styles_warning_does_not_flip_success`
Expected: FAIL——`result.success` 为 false（当前 `warnings.is_empty()` 参与）

- [ ] **Step 3: 实现——两处门控去掉 warnings 条件**

**①** `omnilang/src/lib.rs:185`：

```rust
        success: !pipeline.has_errors,
```

（删除 `&& gen_output.warnings.is_empty()`；warnings 仍经 `result.warnings` 字段透出——检查 `StyleResolveResult` 定义确认有 warnings 字段，若结构体无此字段则保留接收值在注释中说明由调用方另取。）

**②** `omnilang/src/lib.rs:217-220`：

```rust
                success: errors
                    .iter()
                    .all(|d| d.severity != crate::error::Severity::Error),
```

（删除 `&& output.warnings.is_empty()`。）

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p omnilang --test resolve_styles_test` + `cargo test -p omnilang`
Expected: 新测试 PASS；既有 resolve_styles 测试无回归（error fixture 仍 success=false）

- [ ] **Step 5: 端到端核对（可选）**

`grep -rn "resolve_styles" poiesis/src/` 确认编辑器侧无 `success` 假阴性依赖（若发现依赖 warnings 的调用点，登记不改——超出本任务范围）。

- [ ] **Step 6: clippy + fmt + commit**

```bash
cargo clippy -p omnilang --all-features -- -D warnings
cargo fmt
git add omnilang/src/lib.rs omnilang/tests/resolve_styles_test.rs
git commit -m "fix(editor-bridge): resolve_styles success 按 errors-only 门控——warning 不再翻转画布成功态（评审 MEDIUM-1）"
```

---

### Task 8: `IRProgram::default()` 版本对齐 "1.0"

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:1145-1152`（`impl Default for IRProgram`）
- Test: `crates/omnilang-core/src/ir/types.rs`（tests mod）或 `crates/omnilang-core/src/analyse.rs`（tests mod）

**Interfaces:**
- Consumes: 无（`IRProgram::default()` 现产 `version: "0.1"`）
- Produces: `Default::default().version == "1.0"`——与生产构造点（`ir/lower/mod.rs:333`、`analyse.rs:483/:755` 的 `"1.0"`）一致；`fixtures_shared.rs` 5 处与 `optimize.rs` 测试 2 处显式 `"0.1"` 为手工构造、不受影响

- [ ] **Step 1: 写失败测试**

在 `crates/omnilang-core/src/ir/types.rs` 的 tests mod 末尾追加（若 types.rs 无 tests mod，放 `analyse.rs` tests mod）：

```rust
#[test]
fn default_ir_program_version_matches_production() {
    // 评审 rust-expert LOW-2：Default 曾产 "0.1" 而全部生产构造点用 "1.0"——
    // 取 Default 的测试/工具静默得到过期版本（version 是序列化 WASM 契约）。
    assert_eq!(IRProgram::default().version, "1.0");
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p omnilang-core default_ir_program_version_matches_production`
Expected: FAIL——`"0.1" != "1.0"`

- [ ] **Step 3: 实现**

`crates/omnilang-core/src/ir/types.rs:1148`：

```rust
            version: "1.0".to_string(),
```

（仅此一行；`fixtures_shared.rs`/`optimize.rs` 测试的显式 `"0.1"` 构造**不改**——它们是历史快照输入，与 Default 无耦合。若后续测试断言 `version == "0.1"` 而失败，按失败信息定位——先跑 Step 4 确认。）

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test -p omnilang-core` + `cargo test -p omnilang`
Expected: 全绿——新测试 PASS；fixtures/optimize 既有测试无回归

- [ ] **Step 5: clippy + fmt + commit**

```bash
cargo clippy -p omnilang-core --all-features -- -D warnings
cargo fmt
git add crates/omnilang-core/src/ir/types.rs
git commit -m "fix(ir): IRProgram::default() 版本对齐 \"1.0\"——与生产构造点一致（评审 LOW-2）"
```

---

## 执行顺序与依赖

```
Task 1 (run_program_passes 提取) ──► Task 2 (跨文件 prop 移入)   [1→2 硬依赖]
Task 3 ─┐
Task 4 ─┤
Task 5 ─┼── 相互独立，可与 Task 1-2 并行或顺序执行（每任务独立 commit）
Task 6 ─┤
Task 7 ─┤
Task 8 ─┘
```

- **批次 2 开始门 = 本计划全部 8 任务完成**（评审结论：编排层收口必须在批次 2 前落地）
- 任务 2 依赖任务 1（`run_program_passes` 存在）；其余任务零依赖
- 每任务独立可测试、独立可回滚（纯增量/纯提取）

## 不在本计划范围（后续批次窗口）

- M 级项（评审 Important）：方法表统一 + 跨目标对等矩阵（#76/#28/#88）、per-pass 脏度门控、意图 IR source_span 收集、vue/dioxus E_CODEGEN 门禁、注入语料注入形状扩展、内建函数表（navigate 前置）——批次 2-3 窗口单独计划
- 批次 1 收口（Task 5 提交/Task 8 demo 迁移/Flutter 锚点）——执行既有批次 1 计划
- 路线图尾部语料门控修剪（容器查询/@capture/canvas/分页）——用户拍板项
