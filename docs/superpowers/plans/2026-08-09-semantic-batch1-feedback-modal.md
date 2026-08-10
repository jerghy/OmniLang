# 批次 1 反馈与模态 实施计划（2026-08-09-semantic-batch1-feedback-modal）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 落实语义基线能力矩阵维度 8「反馈与模态」——`@modal` 元素指令、`showToast` 内建语义函数、`@skeleton` 加载占位形态、`@errorBoundary` 降级边界，三目标（React/Flutter/OmniView）codegen + 五层门禁 + learn-omniview 迁移。

**Architecture:** 沿既有语义语法管线（lexer → parser/view → IR → semantic 验证 → 三目标 codegen）新增四个语义构造：`@modal` 挂 dialog 元素（受控 open 信号形态）、`showToast` 为全新内建函数（内建函数表 + 调用点验证 + 三目标分派）、`@skeleton` 为 @load loading 分支内元素指令、`@errorBoundary` 为元素指令 + `@slot fallback` 降级槽。语义化即切换（ADR-075）：手写 Modal/NotificationStore/NotificationsPanel 迁移删除，编译器生成容器与行为。登记先行（ADR-074）：域 10「反馈模态」+ 4 词汇登记排在语法实施之前。

**Tech Stack:** Rust（omnilang-core / omnilang）、React 生成（tsc 门禁）、Flutter 生成（dart analyze 门禁 + 既有 Riverpod 3.0）、OmniView 生成、Playwright 浏览器验证（react-test-app 套件）。

## Global Constraints

1. **诊断消息 verbatim**：V1-V14 消息文本以批次 spec §3 表格为唯一真源（`"@modal"` 引号包裹指令名 + `——` 分隔，对齐 `validate_form_load.rs` 既有风格）。禁止改写措辞。
2. **五层门禁口径**（spec §7）：tsc = verify-react GATE PASSED（零新增豁免）；dart analyze = ≤ 基线 warning 数（零新增；Flutter 降级注记以代码注释呈现，不产生 warning）；Playwright = 既有套件全绿 + 10 项新浏览器行为断言（spec §7 清单）；docs-gate = 248+ blocks 0 failed；注入语料 = 855 元组扩展约 27 元组（spec §7 清单）+ verify-semantic-universe.py 域 10 断言 + verify-vocabulary.py 新词汇断言 + verify-demo-coverage.py 新 demo 接入。
3. **ts-rs freshness**：IR 结构变更后必须 `cargo test export_bindings --lib -p omnilang` + `git diff --exit-code poiesis/src/commands/__generated__/` 干净。
4. **无新第三方依赖**：React 端 toast 零依赖自生成（spec §4.2）、Flutter 端骨架屏静态占位不引 shimmer 依赖（spec §4.3）、@modal 不引模态库（spec §4.1）。
5. **语义化即切换（ADR-075 原则 8）**：迁移后旧手写形态不留并存（NotificationStore/NotificationsPanel 删除，不双轨保留）。
6. **`cargo fmt` + `cargo clippy --workspace --all-targets -- -D warnings` 零警告**；`cargo test --workspace` 全绿。
7. **spec 未决问题已全部裁决**（spec §9 注记 + 2026-08-09 拍板）：Flutter 桌面端 Esc 由平台 pop 行为承担（不阻塞）；@skeleton 形状自适应不进 v0；showToast 位置参数/手动关闭按钮不进 v0；fallback 槽不暴露错误对象；V9 硬约束保持；dialog 关闭时内容保留差异保持目标原生行为（React 保留 / Flutter pop 销毁），以注记呈现。

---

## 1. 背景与范围

本批次实施批次 spec（`docs/superpowers/specs/2026-08-09-semantic-batch1-feedback-modal-design.md`）的四个进语言能力项（引用 spec 章节，不重复设计）：

| 能力项 | spec 章节 | 本计划任务落点 |
|---|---|---|
| `@modal` 元素指令（dialog 宿主 + 受控 open 信号 + 焦点陷阱/Esc/遮罩点击/a11y 内建 + 命名槽投影 + @enter 复用） | §2.1 / §3 V1-V5,V13 / §4.1 | Task 1-6、Task 8（Modal.omni 迁移） |
| `showToast` 内建语义函数（kind/duration/堆叠/保留名） | §2.2 / §3 V6-V8 / §4.2 | Task 1-6、Task 8（手写通知删除） |
| `@skeleton` 占位形态（@load loading 分支硬约束） | §2.3 / §3 V9-V10 / §4.3 | Task 1-6、Task 8（LoadDemo 用点） |
| `@errorBoundary` 降级边界（@slot fallback + Flutter 诚实降级） | §2.4 / §3 V11-V12,V14 / §4.4 | Task 1-6、Task 8（ErrorHandlingDemo 用点） |
| 登记册：域 10 + 4 词汇 + verify 断言 | §5.1 / §5.2 | Task 0（登记先行）、Task 7（收口） |
| 语言边界记录：tooltip 留生态 / 关闭动画 / Flutter 组件级错误边界 | §6 | Task 0（登记）、Task 7（文档）、§7 注记 |
| demo 迁移 8 语料位 | §8 | Task 8 |

执行方式：subagent-driven + 逐任务审查；每任务一个 subagent 会话，TDD（RED 测试先行 → 确认失败 → 最小实现 → 门禁 → commit）。

---

## 2. 任务划分

### Task 0: 登记先行——域 10「反馈模态」+ vocabulary 4 词汇 + verify 断言扩展（ADR-074）

**Files:**
- Modify: `docs/superpowers/specs/semantic-universe.md`（新域 10「反馈模态」章节，结构照域 7/8/9 先例：语义域定义段 + 语义集条目 4 列表 + 已实现白名单 2 列表 + 机制级条目注记）
- Modify: `docs/superpowers/specs/vocabulary-registry.md`（4 词汇登记）
- Modify: `scripts/verify-semantic-universe.py`（域 10 断言：条目存在性 + tooltip「留生态」状态值域断言）
- Modify: `scripts/verify-vocabulary.py`（@modal/showToast/@skeleton/@errorBoundary 词汇断言）

**Interfaces:**
- Consumes: 批次 spec §5.1/§5.2 登记表（唯一真源）
- Produces: `semantic-universe.md` 域 10 章节（后续任务验证/文档引用的登记事实源）；verify 脚本域 10 断言函数（Task 7 全量启用）

- [ ] **Step 1: 写失败测试（登记断言先行）**

在 `scripts/verify-semantic-universe.py` 新增 `assert_domain10()`（或等价函数），断言：
1. `## 域 10:` 章节存在，且含 6 条目：`showToast` / `@modal` / `焦点陷阱` / `@skeleton` / `@errorBoundary` / `tooltip`
2. `tooltip` 行实现状态列 = `留生态`（状态值域新增值，基线 spec §8 治理：现「支持/部分/未支持」扩展第四值）
3. 条目行含诊断行为列（非空）

在 `scripts/verify-vocabulary.py` 新增断言：`@modal`/`showToast`/`@skeleton`/`@errorBoundary` 四条词汇记录存在，且各自「禁止借词」列含 spec §5.2 排除词（如 dialog 行禁止借词含 `showModal`/`showDialog`）。

Run: `python scripts/verify-semantic-universe.py --check`
Expected: FAIL（域 10 尚未登记，断言报「域 10 不存在」）

- [ ] **Step 2: 登记域 10**

按 spec §5.1 表在 `semantic-universe.md` 新增 `## 域 10: 反馈模态` 章节：语义域定义段（一段话：为什么这些条目是一个语义面——命令式短暂通知 + 模态对话框 + 加载占位 + 渲染降级，均属「告诉用户发生了什么/让用户完成关键抉择」的反馈语义）+ 语义集条目 4 列表（6 条目 verbatim，含 tooltip 行状态=`留生态`）+ 已实现白名单 2 列表（dialog 元素 / showToast 调用 / @modal/@skeleton/@errorBoundary——注意：白名单行加注「编译器事实源随 Task 1-6 落地，白名单交叉断言以存在性检查承担」）。tooltip 行附语言边界注记 + 触发条件（spec §6 第一行 verbatim）。

- [ ] **Step 3: 登记 4 词汇**

按 spec §5.2 表在 `vocabulary-registry.md` 新增 4 行（来源域=域 10 反馈模态 / 目标映射 verbatim / 禁止借词 verbatim）。另按 spec §5.2 判定注记补一行说明：`@modal` 绑定的 open 信号为编译器受控信号（唯一真源判定，不注册词汇）；`showToast` 保留名判定同域 6 先例（编译器保留名判定，非注册词汇）。

- [ ] **Step 4: 验证**

Run: `python scripts/verify-semantic-universe.py --check` + `python scripts/verify-vocabulary.py --check` + `python scripts/verify-docs.py --check`
Expected: 0 problems（域 10 断言通过；docs-gate 不受影响——新章节不算 block 失败）

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/semantic-universe.md docs/superpowers/specs/vocabulary-registry.md scripts/verify-semantic-universe.py scripts/verify-vocabulary.py
git commit -m "docs(universe): 域 10 反馈模态登记 + 4 词汇 + verify 断言扩展（登记先行 ADR-074，批次 1 Task 0）"
```

**验证门禁：** verify 双脚本 0 problems + verify-docs 0 failed
**依赖：** 无（批次第一批）

---

### Task 1: 语法面——lexer token + parser 指令/元素 + showToast 调用解析

**Files:**
- Modify: `crates/omnilang-core/src/lexer/keywords.rs:82-83`（`"@form" => Some(TokenKind::AtForm)` 同列加三个新指令）
- Modify: `crates/omnilang-core/src/lexer/token.rs:66-81`（AtStore…AtEnter 同列加 `AtModal`/`AtSkeleton`/`AtErrorBoundary`）
- Modify: `crates/omnilang-core/src/parser/view/directives.rs`（`parse_load_directive` :101 同列新增 `parse_modal_directive` / `parse_skeleton_directive` / `parse_error_boundary_directive`）
- Modify: `crates/omnilang-core/src/parser/view/element.rs`（`@drag` 分派 :711-734 同模式挂三个新指令分派；dialog 元素解析确认走透传通道）
- Modify: `crates/omnilang-core/src/parser/view/mod.rs`（新指令接入元素解析主循环）
- Modify: `crates/omnilang-core/src/parser/expr.rs`（或现有调用表达式解析处：`showToast(...)` 调用识别为内建调用节点，形态照 std 函数调用）
- Test: `crates/omnilang-core/src/parser/view/directives.rs` 内嵌测试（参照既有 `#[cfg(test)]` 风格）或 `omnilang/tests/golden_parse.rs`

**Interfaces:**
- Consumes: Task 0（域 10 已登记，语法实施有登记事实源）
- Produces: `TokenKind::AtModal/AtSkeleton/AtErrorBoundary`（Task 2 IR 消费）；`ViewNode::Modal { open_signal, slots }` / `ViewNode::Skeleton { shape }` / `ViewNode::ErrorBoundary { slots }`（Task 2 lowering 消费）；`Expr::BuiltinToastCall { msg, kind, duration }`（Task 2-6 消费）

- [ ] **Step 1: 写失败测试（解析 RED）**

在 `omnilang/tests/golden_parse.rs` 或 directives.rs 测试区新增用例：

```rust
#[test]
fn parse_modal_dialog_shape() {
    // `<dialog @modal={isOpen}>…</dialog>` 解析为 Modal 视图节点，
    // open_signal = "isOpen"，@slot header/body/footer 收集为命名槽
}

#[test]
fn parse_skeleton_directive_shapes() {
    // `<div @skeleton />` → Skeleton{shape: None(默认 text)}
    // `<div @skeleton="card" />` → Skeleton{shape: Some("card")}
}

#[test]
fn parse_error_boundary_directive() {
    // `<div @errorBoundary>…@slot fallback{…}…</div>` → ErrorBoundary 节点 + fallback 槽
}

#[test]
fn parse_show_toast_builtin_call() {
    // `showToast("x")` / `showToast("x", {kind:"error", duration:0})`
    // → BuiltinToastCall 表达式（msg/kind/duration 三字段）
}
```

Run: `cargo test -p omnilang-core parse_modal_dialog_shape parse_skeleton_directive_shapes parse_error_boundary_directive parse_show_toast_builtin_call`
Expected: FAIL（TokenKind/解析分支不存在，编译错或未匹配）

- [ ] **Step 2: 最小实现——lexer**

`keywords.rs` 在 `"@form"` 同列加 `"@modal" => Some(TokenKind::AtModal)` / `"@skeleton" => Some(TokenKind::AtSkeleton)` / `"@errorBoundary" => Some(TokenKind::AtErrorBoundary)`；`token.rs` 在 AtForm 同列加三个 `TokenKind` 变体。注意 `@errorBoundary` 的词汇切分：确认 lexer 按最长匹配取 token（`@error` + `Boundary` 不会误切——与 `@form`/`@for` 共前缀先例同机制，若 lexer 需最长匹配调整则在此任务一并处理）。

- [ ] **Step 3: 最小实现——parser 指令**

`directives.rs` 新增三个 `parse_*_directive`（形态照 `parse_load_directive` 的严格形状先例，spec §2.1/§2.3/§2.4 BNF）：
- `@modal` = `@modal` `=` `{` IDENT `}`（spec §2.1 BNF ModalDirective）
- `@skeleton` = `@skeleton`（布尔）或 `@skeleton` `=` STRING（spec §2.3 BNF SkeletonDirective）
- `@errorBoundary` = 裸 `@errorBoundary`（无值，spec §2.4 BNF ErrorBoundaryDirective）

`element.rs` 在 `@drag` 分派同列挂三个新指令：元素解析时遇到 `@modal` 标记宿主元素为 dialog 要求；`dialog` 标签名解析保持透传通道（spec §2.1 冲突检查表：`to_html_tag` 无 dialog 映射——透传即 `<dialog>`，Flutter 端由 @modal 处理器接管，本任务不动 codegen）。

`@slot` 槽解析确认现状：`@slot Ident { … }` 与 `@slot Ident` 两种形态已存在（Modal.omni 现用三命名槽先例）——命名槽不限 header/body/footer（spec §2.1 规则 3），parser 层不设白名单。

- [ ] **Step 4: 最小实现——showToast 调用解析**

在表达式解析处识别 `showToast` 标识符调用：`showToast` `(` Expr `,`? `{` kind? duration? `}`? `)`（spec §2.2 BNF ShowToastCall/ToastOptions）。kind 键名 `kind`/`duration` 冒号参数风格（`over: 400ms` 先例）。选项对象缺省容错（合法形态 4：默认 kind / 三 kind / duration 0 / 省略 options，spec §7 注入语料清单）。解析层不校验合法性（语义层 V6/V7 承担），仅结构捕获。

- [ ] **Step 5: 验证门禁**

Run: Step 1 四个用例全绿 + `cargo test -p omnilang-core` + `cargo fmt` + `cargo clippy -p omnilang-core --all-targets -- -D warnings`
Expected: 全绿；既有 2046+ tests 无回归（新 token/指令不影响旧语法）

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(parser): @modal/@skeleton/@errorBoundary 指令 token 与解析 + showToast 内建调用解析（批次 1 Task 1）"
```

**验证门禁：** `cargo test -p omnilang-core` + clippy 零警告
**依赖：** Task 0

---

### Task 2: IR + lowering——ModalIntentIR/SkeletonBindingIR/ErrorBoundaryBindingIR + ToastCall IR + 槽材料化 + ts-rs

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs`（`LoadIntentIR` :950 / `FormIntentIR` :930 / `DragBindingIR` :685 / `SlotDeclIR` :905 同列新增；`ElementIR` :694 加字段）
- Modify: `crates/omnilang-core/src/ir/lower/component.rs`（`collect_form_intent` :417 同列新增 modal/skeleton/errorBoundary 收集 + showToast 调用收集）
- Modify: `crates/omnilang-core/src/ir/slot_materialize.rs`（`slot_collision_names` :19 / `materialize_view_slots` :41——@modal 三命名槽与 @slot fallback 复用既有机制，仅确认槽名传递路径）
- Modify: `crates/omnilang-core/src/ir/expr.rs`（或现有表达式 IR 定义处：BuiltinToastCall 节点）
- Test: `crates/omnilang-core/src/ir/lower/component.rs` 内嵌测试 或 `omnilang/tests/compilation_test.rs`

**Interfaces:**
- Consumes: Task 1 的 ViewNode 形态
- Produces: `ModalIntentIR { open_signal: String, slots: Vec<SlotDeclIR> }`（ElementIR 新字段 `modal: Option<ModalIntentIR>`）；`SkeletonBindingIR { shape: Option<String> }`（ElementIR 新字段）；`ErrorBoundaryBindingIR { fallback: Option<SlotDeclIR> }`（ElementIR 新字段）；`Expr::BuiltinToastCall { msg: String, kind: Option<ToastKindIR>, duration: Option<u64> }`——Task 3-6 消费

- [ ] **Step 1: 写失败测试（IR/lowering RED）**

在 `omnilang/tests/compilation_test.rs`（或 lower 内嵌测试）新增：

```rust
#[test]
fn lower_modal_intent_captures_open_signal_and_slots() {
    // 源: <dialog @modal={isOpen}><@slot header…/><@slot body…/></dialog>
    // 断言: 该 ElementIR.modal 为 Some，open_signal=="isOpen"，slots 含 header/body
}

#[test]
fn lower_skeleton_and_error_boundary_bindings() {
    // `<div @skeleton="card"/>` → ElementIR.skeleton == Some(SkeletonBindingIR{shape:Some("card")})
    // `<div @errorBoundary>…@slot fallback{…}…</div>` → ElementIR.error_boundary.fallback 为 Some
}

#[test]
fn lower_show_toast_call_expr() {
    // fn 体内 `showToast("保存成功", {kind:"success"})` → Expr::BuiltinToastCall{msg:"保存成功", kind:Some(Success), duration:None}
}

#[test]
fn export_bindings_freshness() {
    // `cargo test export_bindings --lib -p omnilang` 后
    // `git diff --exit-code poiesis/src/commands/__generated__/` 干净（IR 变更同步 ts-rs 绑定）
}
```

Run: 前三用例 → FAIL（IR 类型不存在）；第四用例 → FAIL（__generated__ diff 非空）

- [ ] **Step 2: 最小实现——IR 类型**

`ir/types.rs`：`ModalIntentIR` / `SkeletonBindingIR` / `ErrorBoundaryBindingIR` 三个 struct（字段见 Interfaces，`#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]` 照既有 IR 类型）；`ElementIR` 加 `modal: Option<ModalIntentIR>` / `skeleton: Option<SkeletonBindingIR>` / `error_boundary: Option<ErrorBoundaryBindingIR>` 三字段（照 `drag_binding: Option<DragBindingIR>` :738 模式）；表达式 IR 加 `BuiltinToastCall` 变体 + `ToastKindIR` 枚举（Info/Success/Error）。

- [ ] **Step 3: 最小实现——lowering 收集**

`lower/component.rs`：在 `collect_form_intent` 同列新增三个收集函数（遍历视图节点，识别 Task 1 的 ViewNode::Modal/Skeleton/ErrorBoundary，填入 ElementIR 字段）；表达式遍历处识别 `BuiltinToastCall` 并校验调用点（spec §2.2 规则 3：事件处理器与函数体，不在组件声明处——声明处使用即抛 lowering 错误，此错误经语义层 V8 体系呈现，见 Task 3）。`slot_materialize.rs` 确认 @modal 三命名槽与 @slot fallback 走既有材料化路径（槽名 collision 检查照常生效），无需改逻辑——仅验证。

- [ ] **Step 4: ts-rs 同步**

Run: `cargo test export_bindings --lib -p omnilang` → 重新生成 `poiesis/src/commands/__generated__/` 绑定 → `git diff --exit-code poiesis/src/commands/__generated__/` 干净

- [ ] **Step 5: 验证门禁**

Run: `cargo test -p omnilang-core` + `cargo test --workspace`（确认无回归）+ clippy 零警告
Expected: 全绿

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(ir): Modal/Skeleton/ErrorBoundary 绑定 IR + showToast 调用 IR + lowering 收集 + ts-rs 同步（批次 1 Task 2）"
```

**验证门禁：** `cargo test --workspace` + clippy 零警告 + `git diff --exit-code poiesis/src/commands/__generated__/` 干净
**依赖：** Task 1

---

### Task 3: 语义验证——validate_feedback_modal.rs（V1-V14 全量）

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_feedback_modal.rs`（V1-V14 校验 + 消息 verbatim spec §3）
- Modify: `crates/omnilang-core/src/semantic/mod.rs`（`pub mod validate_feedback_modal;` 注册 + 验证管道接入）
- Modify: `crates/omnilang-core/src/semantic/collect.rs`（`collect_fn_symbols` :306 / `collect_block_symbols` :350——`showToast` 保留名检查符号收集）
- Modify: `crates/omnilang-core/src/semantic/validate_form_load.rs`（对照 V-F8 保留名冲突先例 :112-172 的检查形态，不修改既有逻辑）
- Test: `crates/omnilang-core/src/semantic/validate_feedback_modal.rs` 内嵌测试 或 `omnilang/tests/event_name_validation.rs` 风格（诊断消息断言测试）

**Interfaces:**
- Consumes: Task 2 IR 结构
- Produces: V1-V14 诊断（14 条，错误 11 + 警告 3）；`showToast` 保留名诊断（V8 形态，spec §2.2 规则 7）——Task 4-6 codegen 前置门

- [ ] **Step 1: 写失败测试（V1-V14 全量 RED）**

新增测试模块 `validate_feedback_modal`（内嵌或独立测试文件），逐条断言消息 verbatim（spec §3 表格是唯一真源，测试里逐字抄录）：

```rust
#[test]
fn v1_modal_value_must_be_boolean_signal() {
    // `<dialog @modal={123}>` → 诊断含 `"@modal" 的值必须是布尔信号`
}
#[test]
fn v2_modal_only_on_dialog() {
    // `<div @modal={isOpen}>` → 诊断含 `"@modal" 只能用于 dialog 元素`
}
#[test]
fn v3_bare_dialog_requires_modal() {
    // `<dialog>…</dialog>`（无 @modal）→ 诊断含 `dialog 元素必须绑定 "@modal"`
}
#[test]
fn v4_duplicate_modal() {
    // `<dialog @modal={a} @modal={b}>` → 诊断含 `元素不能重复绑定 "@modal"`
}
#[test]
fn v5_modal_not_in_for_loop() {
    // `<div @for={items}><dialog @modal={isOpen}/></div>` → 诊断含 `"@modal" 不能用在 @for 循环内`
}
#[test]
fn v6_unknown_toast_kind() {
    // `showToast("x", {kind:"warn"})` → 诊断含 `未知 toast 类型 "warn"——合法类型: info/success/error`
}
#[test]
fn v7_negative_toast_duration() {
    // `showToast("x", {duration:-1})` → 诊断含 `"showToast" 的 duration 不能为负数`
}
#[test]
fn v8_show_toast_reserved_name() {
    // `fn showToast() {}` / `signal showToast: bool` / `store showToast` / 组件名 showToast
    // → 诊断含 `"showToast" 是内建语义函数，不能用作组件声明名`
    // 注：spec §2.2 规则 7 覆盖 fn/store/信号/组件四类声明位点，全部断言
}
#[test]
fn v9_skeleton_only_in_load_loading_branch() {
    // `<div @skeleton />` 脱离 @load loading 分支（含在 @else 分支、普通视图、另一组件内）
    // → 诊断含 `"@skeleton" 只能用在 @load 的 loading 分支内（@if loading）`
}
#[test]
fn v10_unknown_skeleton_shape() {
    // `<div @skeleton="table" />` → 诊断含 `未知骨架形状 "table"——合法形状: text/line/circle/card`
}
#[test]
fn v11_duplicate_error_boundary() {
    // `<div @errorBoundary @errorBoundary>` → 诊断含 `元素不能重复绑定 "@errorBoundary"`
}
#[test]
fn v12_error_boundary_without_fallback_warns() {
    // `<div @errorBoundary><p>x</p></div>`（无 fallback 槽）→ warning 含 `"@errorBoundary" 没有提供 fallback 槽`
}
#[test]
fn v13_modal_without_heading_warns() {
    // `<dialog @modal={isOpen}><p>无标题</p></dialog>` → warning 含 `"@modal" 内容没有标题`
    // （有 Heading 类元素 → 无此 warning——对照用例）
}
#[test]
fn v14_duplicate_fallback_slot() {
    // 两个 @slot fallback → 诊断含 `"fallback" 槽重复声明——降级内容只能有一个`
}
```

Run: `cargo test -p omnilang-core validate_feedback_modal`
Expected: FAIL（validate_feedback_modal 模块不存在）

- [ ] **Step 2: 最小实现——校验器**

新建 `validate_feedback_modal.rs`，按 spec §3 表逐条实现：

- **@modal 五错误 + 一警告（V1-V5, V13）**：
  - 绑定值必须为布尔信号读引用（V1——检查表达式形态：信号引用且类型为 bool，spec §2.1 规则 2）
  - 宿主元素双向校验（V2/V3——@modal 仅 dialog 标签；dialog 标签必须带 @modal；spec §2.1 规则 1「双向校验」）
  - 重复绑定（V4——同元素两个 @modal，V11 同机制）
  - @for 循环内禁止（V5——@modal 元素的祖先链上存在 @for 节点即触发）
  - 无 Heading 类元素警告（V13——dialog 子树内查找 Heading 类元素（Heading/Title 组件或 h1-h6 元素），无则警告；spec §2.1 规则 5）
- **showToast 两错误 + 保留名（V6-V8）**：
  - kind 合法值校验（V6——非法值硬错误，消息内嵌用户写的值）
  - duration 非负（V7——负数硬错误；0 合法=不自动关闭，spec §2.2 规则 2）
  - 保留名检查（V8——照 `validate_form_load.rs` V-F8 先例 :112-134 的双位点形态：信号/计算属性名 + 函数/组件名；collect.rs 侧把 `showToast` 加入保留符号集）
- **@skeleton 一错误 + 一形状（V9-V10）**：
  - loading 分支约束（V9——@skeleton 元素祖先链须位于 `@load` 的 `@if loading` 直系或嵌套子树上；spec §2.3 规则 2「直系或嵌套子树」）
  - 形状值白名单（V10——text/line/circle/card，默认 text；spec §2.3 规则 1）
- **@errorBoundary 一错误 + 一警告 + 一重复槽（V11, V12, V14）**：
  - 重复绑定（V11）、无 fallback 警告（V12——warning 级）、fallback 槽至多一个（V14——跨两个 @slot fallback 声明检测，照 slot 材料化的槽名碰撞路径）

诊断消息逐字用 spec §3 表文本；错误消息风格对齐 `validate_form_load.rs`（`"@modal"` 引号包裹 + `——` 分隔）。验证管道在 `semantic/mod.rs` 注册（照 validate_form_load/validate_drag 接入点）。

- [ ] **Step 3: 验证门禁**

Run: Step 1 全部用例绿 + `cargo test -p omnilang-core`（含既有诊断测试无回归——消息格式未动）+ clippy 零警告
Expected: 全绿；V12/V13 为 warning 级——确认既有 warning 计数通道（diagnostics 分级）不把 warning 当 error 阻塞

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(semantic): validate_feedback_modal V1-V14 全量校验 + showToast 保留名（批次 1 Task 3）"
```

**验证门禁：** `cargo test -p omnilang-core` + clippy 零警告
**依赖：** Task 2

---

### Task 4: React codegen——@modal/showToast/@skeleton/@errorBoundary 四件套

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`（@modal 受控 dialog 生成 + @skeleton 占位生成 + @errorBoundary 包装 + 命名槽投影）
- Modify: `omnilang/src/codegen/react/stmt.rs`（showToast 调用点生成 + ToastProvider 根挂载注入）
- Modify: `omnilang/src/codegen/react/expr.rs`（BuiltinToastCall 表达式 → toast 调用）
- Modify: `omnilang/src/codegen/react/mod.rs`（ToastProvider/ErrorBoundary 运行时支持模块拼接、shimmer keyframes 注入）
- Modify: `omnilang/src/codegen/react/types.rs`（如需要——aria-labelledby id 生成类型）
- Test: `omnilang/tests/compilation_test.rs` 或 `omnilang/src/codegen/react/tests.rs`（生成代码快照断言）

**Interfaces:**
- Consumes: Task 2 IR / Task 3 验证通过
- Produces: React 生成形态（spec §4.1/§4.2/§4.3/§4.4 表 verbatim）：受控 `<dialog>` + `useLayoutEffect` showModal/close 同步 + `onCancel` preventDefault + 遮罩点击 `e.target === ref.current` + `role="dialog" aria-modal="true" aria-labelledby={titleId}`；ToastProvider + `.omni-toast--{kind}`；shimmer keyframes；ErrorBoundary class

- [ ] **Step 1: 写失败测试（React 生成 RED）**

在 `omnilang/tests/compilation_test.rs` 新增 React 目标快照用例（照既有 `#[test]` 编译+断言生成文本风格）：

```rust
#[test]
fn react_modal_generates_controlled_dialog() {
    // 源: <dialog @modal={isOpen}>…三槽…</dialog>
    // 断言生成含: open={isOpen} / useLayoutEffect / showModal() / close() /
    //   onCancel / e.target === /* ref */ / role="dialog" / aria-modal="true" / aria-labelledby
}

#[test]
fn react_show_toast_generates_provider_and_call() {
    // 源: fn save() { showToast("保存成功", {kind:"success"}) }
    // 断言生成含: ToastProvider 根挂载 / toast({ message: "保存成功", kind: "success", duration: 3000 })
    //   / .omni-toast--success 容器类
}

#[test]
fn react_skeleton_generates_shimmer_placeholder() {
    // 源: @load loading 分支内 <div @skeleton="card"/>
    // 断言生成含: 占位 div + @keyframes（shimmer 渐变扫光）+ 形状类
}

#[test]
fn react_error_boundary_generates_class() {
    // 源: <div @errorBoundary>…@slot fallback{…}…</div>
    // 断言生成含: getDerivedStateFromError / componentDidCatch / fallback 槽渲染为 error UI
}
```

Run: `cargo test -p omnilang react_modal_generates_controlled_dialog`（逐用例）
Expected: FAIL（生成文本缺失）

- [ ] **Step 2: 最小实现——@modal（spec §4.1 React 行）**

`view.rs`：遇到 `ElementIR.modal` 生成受控 `<dialog>` 形态——`open={openSig}` + ref + `useLayoutEffect(() => { openSig ? dialogRef.current.showModal() : dialogRef.current.close() }, [openSig])` + `onCancel={e => { e.preventDefault(); setOpenSig(false) }}` + 遮罩点击 `onClick={e => { if (e.target === dialogRef.current) setOpenSig(false) }}` + `role="dialog" aria-modal="true"`；`aria-labelledby` 指向 dialog 内第一个 Heading 类元素（编译器自生成 id），无标题则省略（V13 警告已由 Task 3 发）。三个命名槽按文档序投影（header/body/footer 为语义提示名，槽名不限三个——spec §2.1 规则 3）；匿名子内容兜底进 body 槽。`@enter` 组合：条件变 true 时自动重播先例复用（spec §2.1 规则 4——docs/omniview/11-motion-animation.md 条件切换重播），本任务仅确认 @enter 触发器与 open 信号同源时的重播路径不冲突。

- [ ] **Step 3: 最小实现——showToast（spec §4.2 React 行）**

`mod.rs`/`stmt.rs`：根挂载自生成 `ToastProvider`（容器 + 信号列表 + 定时器，零依赖）；调用点生成 `toast({ message, kind, duration })`；容器类 `.omni-toast` + `.omni-toast--{kind}`（spec §2.2 规则 6 类名 verbatim）。堆叠：多条同时展示（自生成容器自然支持）。`duration: 0` = 不自动关闭。默认 duration 3000（未提供 options 或 duration 时）。

- [ ] **Step 4: 最小实现——@skeleton（spec §4.3 React 行）**

`view.rs`：@load loading 分支内 `@skeleton` 元素生成占位 div + 编译器自带 shimmer keyframes（渐变扫光动画，零依赖；spec §2.3 规则 3）；形状 text（段落条）/line/circle/card 各生成对应占位结构（默认 text）。

- [ ] **Step 5: 最小实现——@errorBoundary（spec §4.4 React 行）**

`mod.rs`：自生成 `ErrorBoundary` class（`getDerivedStateFromError` + `componentDidCatch`），`@slot fallback` 槽 → error UI；无 fallback 槽 → 渲染空内容（V12 警告已由 Task 3 发）。捕获范围注记（不捕获事件处理器/异步——React 官方语义，spec §2.4 规则 1）：以生成代码注释呈现。

- [ ] **Step 6: verify-react GATE**

Run: `bash scripts/verify-react.sh`（或等价既有入口）+ Step 1 用例全绿
Expected: GATE PASSED——零新增豁免（新语法样例全量进入 React 生成校验，spec §7 tsc 行）

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(codegen-react): @modal 受控 dialog + showToast ToastProvider + @skeleton shimmer + @errorBoundary（批次 1 Task 4）"
```

**验证门禁：** `cargo test -p omnilang` + verify-react GATE PASSED + clippy 零警告
**依赖：** Task 3

---

### Task 5: Flutter codegen——四件套 + 诚实降级注记

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（@modal showDialog + FocusTraversalGroup + 槽投影；@skeleton 静态占位；@errorBoundary 空壳）
- Modify: `omnilang/src/codegen/flutter/stmt.rs`（showToast → ScaffoldMessenger.showSnackBar + SnackBarAction 降级）
- Modify: `omnilang/src/codegen/flutter/expr.rs`（BuiltinToastCall → SnackBar 参数）
- Test: `omnilang/tests/generated_code_validation.rs`（flutter 目标生成断言，照 `flutter_output_compiles_with_dart_analyze` 先例）

**Interfaces:**
- Consumes: Task 2 IR / Task 3 验证通过
- Produces: Flutter 生成形态（spec §4.1/§4.2/§4.3/§4.4 Flutter 行 verbatim）：`showDialog(context, barrierDismissible: true, builder: (_) => FocusTraversalGroup(child: Dialog(child: …槽投影)))` + open 信号驱动的 Navigator push/pop（幂等 guard）；`ScaffoldMessenger.of(context).showSnackBar(SnackBar(...))` + `SnackBarAction("关闭")` 降级；静态占位灰块/圆；@errorBoundary 空壳 + 代码注释注记

- [ ] **Step 1: 写失败测试（Flutter 生成 RED）**

在 `omnilang/tests/generated_code_validation.rs` 新增（照既有 flutter 断言风格）：

```rust
#[test]
fn flutter_modal_generates_show_dialog_with_focus_traversal() {
    // 源: <dialog @modal={isOpen}>…三槽…</dialog>
    // 断言生成含: showDialog / barrierDismissible: true / FocusTraversalGroup /
    //   Dialog( / 槽投影内容 / Navigator push/pop 幂等 guard（isOpen 信号驱动）
}

#[test]
fn flutter_show_toast_generates_snack_bar() {
    // 源: showToast("网络不可用", {kind:"error", duration:0})
    // 断言生成含: ScaffoldMessenger.of(context).showSnackBar / SnackBar(content: Text("网络不可用")) /
    //   duration 映射（duration:0 → 超长时长 + SnackBarAction("关闭")）
}

#[test]
fn flutter_skeleton_generates_static_placeholder() {
    // 源: @load loading 分支内 <div @skeleton="circle"/>
    // 断言生成含: 静态占位（Container 灰底圆）+ 降级注记注释（web shimmer 为增强）
}

#[test]
fn flutter_error_boundary_generates_stub_with_note() {
    // 源: <div @errorBoundary>…</div>
    // 断言生成含: 空壳 + 注释（数据级错误请走 @load error 信号 + ErrorWidget.builder 全局兜底指引）
    // 注: 注释不产生 dart analyze warning（spec §7 dart analyze 行）
}
```

Run: `cargo test -p omnilang flutter_modal_generates_show_dialog_with_focus_traversal`（逐用例）
Expected: FAIL

- [ ] **Step 2: 最小实现——@modal（spec §4.1 Flutter 行）**

`view.rs`：`ElementIR.modal` 生成 `showDialog(context: context, barrierDismissible: true, builder: (_) => FocusTraversalGroup(child: Dialog(child: …)))`；打开/关闭由 openSig 信号驱动的 Navigator push/pop（幂等 guard——信号重复 true 不重复 push，spec §4.1 注记）；三命名槽投影进 Dialog child（槽名不限三个，匿名内容进 body——与 React 端同语义）。Esc 关闭：移动端系统返回键触发 pop（行为对等注记，spec §4.1 Flutter 行）；桌面端 Esc 实现形态 = 平台 pop 行为承担（Global Constraints 7 裁决，不新增按键监听）。@modal 关闭时内容销毁（pop 销毁，spec §6 最后一行：保持目标原生行为，以注记呈现——生成代码注释）。

- [ ] **Step 3: 最小实现——showToast（spec §4.2 Flutter 行）**

`stmt.rs`/`expr.rs`：`ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg), duration: …))`；kind → SnackBar 内容/主题色（kind 三色板，spec §2.2 规则 1）；**队列单条**（同时只显示一条，后续排队——降级注记，spec §4.2 Flutter 行）；`duration: 0` → 超长时长 + `SnackBarAction(label: "关闭", onPressed: …)`（spec §2.2 规则 5 降级）。

- [ ] **Step 4: 最小实现——@skeleton + @errorBoundary（spec §4.3/§4.4 Flutter 行）**

`skeleton`：静态占位灰块/圆（`Container` 灰底，无 shimmer 依赖）+ 生成代码注释注记（动画形态为 web 增强，spec §4.3 降级注记）。
`errorBoundary`：空壳 + 生成代码注释注记（数据级错误请走 @load error 信号；ErrorWidget.builder 全局兜底为文档指引——spec §4.4 Flutter 行 + §6 触发条件）。注释以 Dart 注释呈现，不产生 warning。

- [ ] **Step 5: 验证门禁**

Run: `cargo test --features codegen-flutter -p omnilang`（含 `learn_omniview_flutter_output_passes_dart_analyze` 0 错误 + `learn_omniview_flutter_warnings_are_tracked` ≤ 基线 warning 数，spec §7 dart analyze 行）
Expected: PASS（零新增 warning；降级注记以注释呈现）

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(codegen-flutter): @modal showDialog+FocusTraversalGroup + showToast SnackBar 降级 + @skeleton 静态占位 + @errorBoundary 空壳注记（批次 1 Task 5）"
```

**验证门禁：** `cargo test --features codegen-flutter -p omnilang` 全绿（dart analyze 0 错误 + warning ≤ 基线）
**依赖：** Task 3

---

### Task 6: OmniView codegen——四件套

**Files:**
- Modify: `omnilang/src/codegen/omniview/component_html.rs`（@modal 自绘遮罩 + 对话框容器 + 焦点循环生成；@skeleton 占位 + shimmer；@errorBoundary try 兜底）
- Modify: `omnilang/src/codegen/omniview/expr.rs`（showToast 调用 → toast 容器信号推送）
- Modify: `omnilang/src/codegen/omniview/runtime.rs`（toast 全局容器 + 定时器 + 焦点陷阱运行时支持）
- Test: `omnilang/tests/compilation_test.rs` 或 `omnilang/src/codegen/omniview/tests.rs`

**Interfaces:**
- Consumes: Task 2 IR / Task 3 验证通过
- Produces: OmniView 生成形态（spec §4.1/§4.2/§4.3/§4.4 OmniView 行 verbatim）：自绘遮罩 + 对话框容器 + tabindex 焦点循环 + Esc/遮罩点击处理 + role 属性；自绘 toast 容器（全局挂载）+ 信号列表 + 定时器；同 React（web 通道 CSS 动画）；try 兜底渲染 fallback 槽（ErrorHandlingDemo 先例）

- [ ] **Step 1: 写失败测试（OmniView 生成 RED）**

```rust
#[test]
fn omniview_modal_generates_self_drawn_overlay_with_focus_trap() {
    // 源: <dialog @modal={isOpen}>…三槽…</dialog>
    // 断言生成含: 遮罩元素 / 对话框容器 / tabindex 焦点循环（Tab/Shift+Tab 不逸出）/
    //   Esc 处理 / 遮罩点击关闭 / role="dialog" aria-modal="true"
}

#[test]
fn omniview_show_toast_generates_global_container() {
    // 源: showToast("保存成功")
    // 断言生成含: 全局挂载 toast 容器 / 信号列表 / 定时器（3000ms 自动消失）
}

#[test]
fn omniview_skeleton_generates_shimmer_placeholder() {
    // 源: @load loading 分支内 <div @skeleton/>
    // 断言生成含: 占位 div + shimmer 动画（web 通道 CSS）
}

#[test]
fn omniview_error_boundary_generates_try_fallback() {
    // 源: <div @errorBoundary>…@slot fallback{…}…</div>
    // 断言生成含: try 兜底渲染 fallback 槽（ErrorHandlingDemo 先例形态）
}
```

Run: `cargo test -p omnilang omniview_modal_generates_self_drawn_overlay_with_focus_trap`（逐用例）
Expected: FAIL

- [ ] **Step 2: 最小实现（spec §4.1/§4.2/§4.3/§4.4 OmniView 行）**

`component_html.rs`：
- **@modal**：自绘遮罩 + 对话框容器 + tabindex 焦点循环（编译器生成 Tab/Shift+Tab 循环逻辑——三端中唯一需手写焦点逻辑的目标，spec §4.1 OmniView 行）+ Esc/遮罩点击处理（信号置 false）+ `role="dialog"` `aria-modal="true"` + aria-labelledby（指向第一个 Heading 类元素，同 React 语义）；三命名槽投影 + 匿名内容进 body。焦点陷阱运行时支持挂 `runtime.rs`。
- **showToast**：全局挂载 toast 容器（自绘）+ 信号列表 + 定时器（duration 默认 3000 / 0 = 不自动关闭）+ kind 三色样式 + 堆叠展示。
- **@skeleton**：占位 + shimmer（web 通道 CSS 动画，同 React）。
- **@errorBoundary**：try 兜底渲染 fallback 槽（ErrorHandlingDemo 现有 try/catch 先例形态）；无 fallback → 空内容（V12 警告已发）。

- [ ] **Step 3: 验证门禁**

Run: Step 1 用例全绿 + `cargo test -p omnilang` + clippy 零警告
Expected: 全绿

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(codegen-omniview): @modal 自绘遮罩+焦点循环 + showToast 全局容器 + @skeleton + @errorBoundary try 兜底（批次 1 Task 6）"
```

**验证门禁：** `cargo test -p omnilang` + clippy 零警告
**依赖：** Task 3

---

### Task 7: 登记册收口 + docs/omniview 新语法文档 + 注入语料扩展

**Files:**
- Modify: `docs/omniview/`（新语法文档或既有文档新节：@modal/@skeleton/@errorBoundary 元素指令 + showToast 内建函数——随 §8 迁移同步，docs-gate 覆盖）
- Modify: 注入语料（855 元组矩阵——新增约 27 元组，spec §7 清单：@modal 合法 5 + 非法 6 + 裸 dialog；showToast 合法 4 + 非法 3；@skeleton 合法 2 + 非法 2；@errorBoundary 合法 2 + 非法 2 + 警告 1）
- Modify: `scripts/verify-semantic-universe.py`（Task 0 断言扩展为全量：白名单交叉断言启用）
- Modify: `scripts/verify-demo-coverage.py`（新 demo 接入——域 10 条目展示缺口审计）
- Test: `python scripts/verify-docs.py --check`（docs-gate）

**Interfaces:**
- Consumes: Task 0 登记 / Task 1-6 语法落地
- Produces: 文档事实源（Task 8 迁移与 Playwright 断言的语法依据）；注入语料矩阵扩展（spec §7 验收第 5 层）

- [ ] **Step 1: 写失败测试（docs 与注入语料 RED）**

1. docs：在 `docs/omniview/` 新增（或既有文档扩展）@modal/showToast/@skeleton/@errorBoundary 语法文档节——含合法/非法示例与诊断消息引用（docs 内容以 spec §2/§3 为唯一真源，不新增设计）。
2. 注入语料：在 855 元组矩阵注入点新增 27 元组（spec §7 清单 verbatim：合法 5+4+2+2 / 非法 6+3+2+2 / 警告 1——@errorBoundary 无 fallback 的 V12 警告 1 元组 + 裸 `<dialog>` 归入 @modal 非法 6）。注入语料测试入口定位：`grep -rn "插入点\|载荷\|注入" omnilang/ scripts/`（form-load 计划同款处理——实施者按既有矩阵结构扩展）。

Run: `python scripts/verify-docs.py --check` + 注入语料回归
Expected: FAIL（新文档节未覆盖/语料缺元组——按既有 docs-gate 计数口径验证）

- [ ] **Step 2: 文档 + 语料落地**

按 Step 1 落地；docs-gate 计数从 248+ blocks 扩展（新增 blocks 全绿，spec §7 docs-gate 行）。

- [ ] **Step 3: verify 全量启用**

`verify-semantic-universe.py`：Task 0 的存在性断言扩展为全量交叉断言（域 10 白名单 ↔ 编译器事实源——@modal/@skeleton/@errorBoundary/showToast 现已在 parser/IR/codegen 落地，spec §5.1 已实现白名单表为真源）；`verify-demo-coverage.py`：域 10 条目与 demo 用点关联审计（Task 8 完成后 0 缺口——本任务先跑出缺口清单交给 Task 8）。

Run: `python scripts/verify-semantic-universe.py --check` + `python scripts/verify-vocabulary.py --check` + `python scripts/verify-demo-coverage.py`
Expected: 0 problems（demo 缺口清单输出，Task 8 消化）

- [ ] **Step 4: Commit**

```bash
git commit -m "docs(omniview): 反馈模态语法文档 + 注入语料 27 元组 + verify 全量断言（批次 1 Task 7）"
```

**验证门禁：** verify 三脚本 0 problems + `python scripts/verify-docs.py --check` 0 failed（248+ blocks）
**依赖：** Task 0, Task 1-6

---

### Task 8: learn-omniview demo 迁移（8 语料位）+ Playwright 10 断言 + 门禁终态

**Files:**
- Modify: `test/projects/learn-omniview/src/components/Modal.omni`（重写：`@modal={isOpen}` + @slot header/body/footer；删除手写 handleKeyDown/handleOverlayClick/handleContentClick/close 逻辑；保留 @enter 打开动画用点）
- Delete: `test/projects/learn-omniview/src/stores/NotificationStore.omni`、`test/projects/learn-omniview/src/components/NotificationsPanel.omni`（语义化即切换，spec §8）
- Modify: `test/projects/learn-omniview/src/components/EventShorthand.omni`（**spec §8 未列隐藏依赖**：import NotificationStore + add/clear/unreadCount 用点——迁移为 showToast 内建调用）
- Modify: `test/projects/learn-omniview/src/pages/UiComponents.omni`（showToast 按钮直接调用；Modal 触发沿用）
- Modify: `test/projects/learn-omniview/src/pages/MotionDemo.omni`（`@if isOpen mode:"fade"` 内联模态 → `@modal` + @enter 组合用点）
- Modify: `test/projects/learn-omniview/src/components/LoadDemo.omni`（loading 分支加 `@skeleton` 用点）
- Modify: `test/projects/learn-omniview/src/components/ErrorHandlingDemo.omni`（新增 @errorBoundary + @slot fallback 用点——渲染级降级演示）
- Modify: `test/projects/learn-omniview/src/main.omni`、`src/design.omni`（旧 Modal/NotificationsPanel 引用同步；Modal* 样式类 :2031-2060 核对，删除死类）
- Modify: `test/projects/learn-omniview/build/react-test-app/tests/`（Playwright 新增 10 项断言，spec §7 清单；既有 46+ 用例全绿）
- Test: `cargo build --release -p omnic && omnic build --target react|flutter|omniview` + Playwright 全量套件 + 五层门禁终态

**Interfaces:**
- Consumes: Task 1-7 全部（真机验证载体）
- Produces: 零手写反馈样板语料；10 项浏览器行为断言；门禁终态达成

- [ ] **Step 1: 定位全部用点**

Run: `grep -rn "Modal\|NotificationsPanel\|NotificationStore\|showToast" test/projects/learn-omniview/src --include="*.omni"`
Expected: 覆盖 spec §8 清单 8 语料位 + EventShorthand.omni 隐藏依赖（UiComponents.omni:8,13,35,52 / main.omni:11,32 / design.omni:2031-2060 / EventShorthand.omni:9,12,46-49,71）

- [ ] **Step 2: Modal.omni 重写**

按 spec §8 首行：`<dialog @modal={isOpen} @enter={fadeIn}>` + `@slot header/body/footer` 三槽投影（ModalHeader/ModalBody/ModalFooter 结构保留为槽内容或简化）；删除全部手写焦点/Esc/遮罩逻辑（tabindex / @onKeyDown={handleKeyDown} / stopPropagation / handleOverlayClick / handleContentClick / close 函数——编译器内建，spec §2.1 意图一句话）；保留 @enter 打开动画用点（打开动画复用，spec §2.1 规则 4）。style 块空注释更新（原「模态框特有的弹窗布局和覆盖层」——覆盖层逻辑已内建，按生成形态核对样式类归属）。

- [ ] **Step 3: 通知语义化即切换（spec §8 第 2/3 行）**

- **删除** `NotificationStore.omni` + `NotificationsPanel.omni`（showToast 内建，旧手写形态不双轨保留——ADR-075）
- `UiComponents.omni`：`<NotificationsPanel />` 用点删除；新增 showToast 按钮（直接调用 `showToast("操作成功", {kind:"success"})` 等——三 kind 演示用点）
- `EventShorthand.omni`：`NotificationStore.add("Hello from shorthand")` → `showToast("Hello from shorthand")`；unreadCount 显示用点 → 删除或改 toast 语义（实施时按页面上下文定，不留对已删 store 的引用）
- `main.omni`：`@store NotificationStore` 接线删除（:11,32）

- [ ] **Step 4: 其余三用点（spec §8 第 4-7 行）**

- `MotionDemo.omni`：`@if isOpen mode:"fade"` 内联模态 → `@modal={isOpen}` + @enter 组合用点（打开动画；关闭动画为语言边界，spec §6 第 2 行）
- `LoadDemo.omni`：loading 分支 `@if loading { … }` 内加 `@skeleton` 用点（text/card 形状——保持既有 data-testid="load-status" 断言不破坏）
- `ErrorHandlingDemo.omni`：新增 @errorBoundary + @slot fallback 用点（渲染级降级演示：故障组件子树 → fallback 槽内容可见；既有逻辑异常演示保留）
- `design.omni`：Modal* 样式类核对（ModalBody/ModalFooter/ModalHeader/ModalTitle/Modalcontent :2031-2060——保留供槽内容用类或删除死类；Overlay/CloseBtn 相关类随手写 overlay 删除核清）

- [ ] **Step 5: 三目标编译 + Playwright 全量 + 10 项新断言**

Run: `cargo build --release -p omnic && omnic build --target react|flutter|omniview`（按项目既有 omnic 用法）+ Playwright 全量套件（`test/projects/learn-omniview/build/react-test-app/tests/`）
Expected: 0 failures + 10 项新断言（spec §7 浏览器行为断言清单 verbatim）：
1. @modal 打开：点击触发按钮 → dialog 出现在 top layer，页面背景不可交互
2. 焦点陷阱：打开后焦点在 dialog 内；Tab 循环不逸出
3. Esc 关闭：按 Esc → dialog 关闭，焦点回触发按钮（React 原生行为）
4. 遮罩点击关闭：点击遮罩 → dialog 关闭（语言语义）
5. a11y：打开态 dialog 具 role="dialog" + aria-modal="true" + aria-labelledby 指向标题
6. showToast 出现：触发 → toast 文本可见、kind 样式类 `.omni-toast--success` 命中
7. showToast 自动消失：约 3000ms 后消失
8. showToast 堆叠：连续触发多条 → 同时可见多条
9. @skeleton：@load loading 态显示占位（text/card 形状断言），loaded 后替换真实内容
10. @errorBoundary：故障组件子树渲染 → fallback 槽内容可见

- [ ] **Step 6: 门禁终态（五层全绿）**

Run:
```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test export_bindings --lib -p omnilang && git diff --exit-code poiesis/src/commands/__generated__/
bash scripts/verify-react.sh            # GATE PASSED 零新增豁免
cargo test --features codegen-flutter -p omnilang   # dart analyze 0 错误 ≤ 基线 warning
python scripts/verify-docs.py --check   # 248+ blocks 0 failed（新鲜 binary——先 cargo build -p omnic）
python scripts/verify-semantic-universe.py --check  # 0 problems（含域 10 断言）
python scripts/verify-vocabulary.py --check          # 0 problems
python scripts/verify-demo-coverage.py               # 0 缺口
# 注入语料回归全绿（855+27 元组）
# Playwright 全量（46+10 用例）
```
Expected: 全绿——spec §7 门禁表逐项达成

- [ ] **Step 7: Commit**

```bash
git add test/projects/learn-omniview/src test/projects/learn-omniview/build/react-test-app/tests
git commit -m "refactor(demo): 反馈模态语义化即切换——Modal @modal 化 + 通知 showToast 化 + @skeleton/@errorBoundary 用点 + Playwright 10 断言（批次 1 Task 8）"
```

**验证门禁：** 五层门禁终态全绿（Step 6 命令集）
**依赖：** Task 1-7

---

## 3. 阶段顺序

```
Task 0（登记先行）→ Task 1（语法面）→ Task 2（IR+lowering）→ Task 3（语义验证）
→ Task 4（React codegen）┐
→ Task 5（Flutter codegen）├→ Task 7（登记收口+docs+语料）→ Task 8（demo 迁移+Playwright+门禁终态）
→ Task 6（OmniView codegen）┘
```

- 严格串行链：Task 0 → 1 → 2 → 3（编译器管线顺序，不可并行）
- **可并行**：Task 4 / Task 5 / Task 6（三目标 codegen 互相独立，均依赖 Task 3 完成——subagent-driven 执行时可并行派发）
- Task 7 依赖 Task 0 + 1-6（verify 全量断言需编译器事实源落地）；Task 8 依赖全部前序

## 4. 登记先行（ADR-074）

- **Task 0 独立承担登记先行**：域 10「反馈模态」6 条目 + tooltip「留生态」状态值域 + vocabulary 4 词汇（含禁止借词）在 Task 1 语法实施**之前**登记——spec §5.1/§5.2 为唯一真源。
- 约束满足声明：plan-writing-guidance 硬约束 3「universe/vocabulary 登记任务必须在对应语法实施任务之前或与之同任务」——Task 0 在 Task 1 之前，明确满足。
- Task 0 的 verify 断言先以「存在性 + 状态值域」口径启用（编译器事实源未落地前白名单交叉断言不可运行）；Task 7 在语法落地后启用全量交叉断言（spec §5.1 已实现白名单表）。

## 5. 语义化即切换（ADR-075，逐语料位）

| 语料位 | 现形态 | 迁移任务 |
|---|---|---|
| `src/components/Modal.omni` | 手写 overlay + tabindex + @onKeyDown Esc + stopPropagation 遮罩点击 | Task 8 Step 2（@modal 化，删手写逻辑，留 @enter） |
| `src/stores/NotificationStore.omni` | 手写通知 store | Task 8 Step 3 **删除**（showToast 内建） |
| `src/components/NotificationsPanel.omni` | 手写通知面板 | Task 8 Step 3 **删除** |
| `src/components/EventShorthand.omni` | NotificationStore 用点（**spec §8 未列隐藏依赖**） | Task 8 Step 3（迁移为 showToast 调用，防断链） |
| `src/pages/UiComponents.omni` | 接线 Modal/NotificationsPanel | Task 8 Step 3（showToast 按钮 + Modal 沿用） |
| `src/pages/MotionDemo.omni` | @if isOpen mode:"fade" 内联模态 | Task 8 Step 4（@modal + @enter 组合） |
| `src/components/LoadDemo.omni` | @load 三态 | Task 8 Step 4（loading 分支 + @skeleton） |
| `src/components/ErrorHandlingDemo.omni` | 纯逻辑异常演示 | Task 8 Step 4（新增 @errorBoundary + @slot fallback 用点） |
| `src/main.omni` / `src/design.omni` | 旧 Modal/NotificationsPanel 引用 + Modal* 样式类 | Task 8 Step 3/4（引用同步 + 死类核清） |

纪律：迁移后旧手写形态不留并存（不双轨保留）；`showToast` 保留名（V8）确保用户声明不与内建冲突。

## 6. 门禁终态（五层）

| 门禁 | 验收口径 | 执行任务 |
|---|---|---|
| tsc | verify-react GATE PASSED，零新增豁免（新语法样例全量进 React 生成校验） | Task 4 Step 6 / Task 8 Step 6 |
| dart analyze | ≤ 基线 warning 数（零新增；Flutter 降级注记以代码注释呈现） | Task 5 Step 5 / Task 8 Step 6 |
| Playwright | 既有 46+ 用例全绿 + 10 项新浏览器行为断言（spec §7 清单） | Task 8 Step 5 |
| docs-gate | 248+ blocks 0 failed（learn-omniview 文档随 §8 迁移同步） | Task 7 Step 1-2 / Task 8 Step 6 |
| 注入语料 | 855 元组扩展约 27 元组 + verify-semantic-universe 域 10 + verify-vocabulary 新词汇 + verify-demo-coverage 新 demo 接入 | Task 7 Step 1-3 / Task 8 Step 6 |

## 7. 风险与降级注记

1. **Flutter 三处诚实降级**（spec §4 降级注记列，均以代码注释呈现不产生 warning）：
   - showToast 队列单条（堆叠差异）+ `duration: 0` → 超长时长 + SnackBarAction「关闭」
   - @skeleton 静态占位（shimmer 为 web 增强）
   - @errorBoundary 空壳 + 数据级错误走 @load error 信号指引
2. **语言边界记录**（spec §6，Task 0 登记 + Task 7 文档）：tooltip 留生态（批次 5 评估）；关闭动画（无 @leave 机制，@modal 关闭过渡不可表达）；Flutter 组件级错误边界留生态（触发条件：Flutter 生态出现组件级事实标准）。
3. **spec §9 未决问题裁决后无阻塞项**：Flutter 桌面端 Esc = 平台 pop 行为承担；@skeleton 形状自适应/位置参数/error 对象暴露均不进 v0；V9 硬约束保持；dialog 关闭内容保留差异保持目标原生行为。
4. **实现期风险**：showToast 为全新内建函数（仓库零先例）——内建函数表 + 调用点验证 + 三目标分派需参照 std/async 内建函数既有调用解析模式，Task 1 实施者定位；`@errorBoundary` token 最长匹配（@error + Boundary 切分）需 Task 1 确认；EventShorthand.omni 隐藏依赖断链风险由 Task 8 Step 3 消化（grep 已确认用点位）。
5. **既有测试回归面**：新 token/指令/IR 字段的增量为 additive——但 ElementIR 加字段与 ts-rs 绑定、以及既有 golden 快照（react/flutter/omniview tests.rs snapshots）需全量回归，Task 2 Step 4 / Task 8 Step 6 承担。

## 8. 批次收口清单（评审遗留登记）

> Task 6（OmniView codegen）终审「有条件通过」——Important 缺陷（@errorBoundary 子节点绑定死寂）已修复并补回归测试；Minor 1（条件分支标题 aria-labelledby 悬空）已按裁决以代码注释说明限制。以下 Minor 2-5 依裁决记入本清单，随批次收口（Task 7/8）跟进或按已知限制存档（2026-08-09 登记）。

### Minor 2（防御性加固，可 defer）：焦点循环边界（view/modal.rs 焦点陷阱 IIFE）

- `[tabindex]:not([tabindex="-1"])` 只排除精确 `"-1"`（`tabindex="-2"` 等仍被选入 focusables）
- 无 visibility/display 过滤（`display:none` 元素可被选入 focusables）
- 空 focusables 时 Tab 逃出对话框
- 未覆盖 contenteditable / iframe / 带 controls 的媒体元素
- 定性：a11y 健壮性，非功能故障

### Minor 3（防御性加固，可 defer）：escape_js_string 转义缺口（view/modal.rs / view/element.rs @errorBoundary 共用）

- 现仅转义 `` \ `` / `"` / `\n`；JS 双引号字符串字面量中裸 `\r` / U+2028 / U+2029 是 SyntaxError
- Windows CRLF 源若未经归一化，boundary 内容含裸 `\r` 会产出非法 JS
- 已核实 parser 大概率已归一化（当前不可达）；兜底方案：`.replace('\r', "\\r")` 一行

### Minor 4（非阻塞，可 defer）：toast 队列只 push 不移除（expr.rs TOAST_RUNTIME_JS）

- `__omni_toastQueue.push(item)` 永不 shift，无界 toast 场景内存持续增长
- `item.remove()` 后引用仍留在队列；超时回调里 shift 即可

### Minor 5（非阻塞，随批次跟进）：测试缺口（tests.rs）

- boundary 无交互子节点用例——恰为 Important 缺陷盲区，**已补**：`omniview_error_boundary_binds_children_after_content_install`（@onPress + 插值，断言绑定晚于内容安装）
- 仍缺：toast 三种 kind 颜色/前缀未断言；@skeleton 只测 card 形状（text/line/circle 未测）；焦点循环首/末元素、shift+tab 边界未测

### Low（已核实，保持现状）

- @skeleton shape 白名单在语义层（V10 为 Error 级诊断），class 名拼接未转义属纵深防御范畴——当前不可达，保持白名单作为唯一防线。

---

### Task 1（语法面）Minor 6-9（2026-08-09 登记）

- **Minor 6**：showToast 选项值用 `expect_close` 的 `—— found ...` 英文后缀（错误消息语言未完全统一）
- **Minor 7**：`parse_show_toast_call` 中 `name` 参数仅在错误路径使用（消息内联 name 而非按函数区分）
- **Minor 8**：`@modal`/`@load` 等在 view 解析失败后恢复路径未覆盖（`@modal` 出现在正文中的回退行为无测试）
- **Minor 9**：`ElementProp::Modal` 的 open_signal 用 `Expr::VarRef` 而非语义层信号引用类型（保持解析器层一致性取舍）

### Task 2（IR）Minor 2（2026-08-09 登记）

- u64→i64 回绕边界：`Literal::Int(n) as i64` 对 > i64::MAX 字面量回绕 + `wrapping_neg` 翻正（如 `-18446744073709551615` → 1）——V7 是最终裁决者，实际不可达，仅记录

### Task 3（语义验证）Minor 2-7（2026-08-09 登记）

- **Minor 2**：模块注释「五处声明位」vs 实现六处（inline_stores 未列入）——一行注释修正
- **Minor 3**：V8 测试只覆盖 signal/fn/component 3 处声明位，computeds/store_refs/inline_stores 3 处未测
- **Minor 4**：error_diag/warning_diag 第三份逐字拷贝（validate_event_names → validate_form_load → validate_feedback_modal）——后续批次建议提取共享 helper
- **Minor 5**：expr_refs_loading 按名匹配边缘（用户自声明 loading 信号 → V9 假阴）——窄边缘，计划内可接受
- **Minor 6**：subtree_has_heading 对每个子节点独立 re-walk（效率微项）
- **Minor 7**：V13 测试对同一输入调用两次 crate::compile（效率微项）

### Task 4（React codegen）遗留（2026-08-09/10 登记）

- **labelledby 悬空**：用户标题自带 id 时 aria-labelledby 仍指向合成 omni-modal-title（id 未被注入，悬空引用）——如需回退指向用户 id 后续单独裁决
- **ToastProvider children**（Task 5 交叉发现，已修复跟进）：`function ToastProvider()` 零参数不渲染 `{children}`——@types/react 19 TS2559（曾由 allowance 掩盖）+ 功能吞视图；修复 = 接收 children 参数 + body 渲染 + allowance 自过期删除

### Task 5（Flutter codegen）审查遗留（2026-08-09/10 登记）

- **I1（记录不阻塞）**：`_modalShown` 单共享 bool——同组件两个 @modal 时 B 的降沿可能 pop 掉 A（「关闭全部」场景）。spec/plan 均未约束多模态/组件（V4 只禁同元素重复、V5 禁 @for 内）；迁移语料单模态。修复方向：per-dialog 幂等键（信号或 route 身份）替代共享 bool
- **M2**：flutter/view.rs 3976 行（+172）——预存结构性债，format_element 是未来拆分点
- **M4**：addPostFrameCallback 每次重建重注册（轮询无操作，一次触发，可忽略）——一行注释防评审疑问

### Task 7（登记收口）Minor 2（2026-08-09 登记）

- 焦点陷阱与 @modal 共用 `@modal\s*=` 计数致审计展示重复——可在 shown 列表去重或并入豁免
