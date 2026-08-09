# 批次 4：数据能力实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**日期**: 2026-08-09
**上游 spec**: `docs/superpowers/specs/2026-08-09-semantic-batch4-data-capabilities-design.md`（唯一详细级输入——本计划只引用章节号，不重复设计内容）
**基线 spec**: `docs/superpowers/specs/2026-08-09-semantic-capability-baseline-design.md`（§5 批次路线图 / §6 验收 / §8 治理）
**参照先例**: `docs/superpowers/plans/2026-08-08-semantic-form-load-phase2.md`（8 任务 TDD 结构——@form/@load 同簇范式）
**执行方式**: subagent-driven + 逐任务审查（每任务一个 subagent 会话，完成后两阶段审查再放行下一任务）

**Goal:** @load v0 边界补齐（多例/参数/Reset）+ 分页 + 无限滚动 + 轮询 + std/timing 防抖节流，三目标 codegen 收口，DataFetching 语料迁移与新 demo 落地。

**Architecture:** 在既有 @load 单例机制（`LoadIntentIR` + `validate_form_load` + 三端 `load_intent` 门控）之上做增量扩展：① IR 从 `Option<LoadIntentIR>` 升级为 `Vec<LoadIntentIR>`（多例化）；② 语义层新增 V1-V12 验证网（含 2 条 warning 诚实降级）；③ 分页/无限滚动/轮询为编译器自生成（v0 先例 + 避免双事实库竞争 + ADR-071，spec §2.2 决策）；④ std/timing 为纯语义 stdlib 模块（与 std/async 平行），各端映射各自地道形态。

**Tech Stack:** Rust（omnilang-core / omnilang）、React 生成（tsc 门禁）、Flutter 生成（dart analyze 门禁）、Playwright 浏览器验证、verify 双脚本 + 注入语料。

## Global Constraints

- 诊断消息**中文 verbatim**（ADR-056 政策，spec §3 表格消息草案逐字落地）
- **语义化即切换，不留旧残留**（ADR-075 原则 8）：DataFetching 迁移后零旧形态（status/resultText/errorMsg 信号删除，不留手写状态机残留，spec §8.1）；`@load` 匿名 v0 形态保留为活证据（LoadDemo 不动）
- **登记先行**（ADR-074）：semantic-universe/vocabulary/verify 脚本登记在 Task 1（先于语法实施），与 form-load 先例的登记后置不同——域 13 为**新域**，必须先登记；代码锚点存在性断言随引入任务接线（Task 2/5）
- `cargo fmt` + `cargo clippy --workspace --all-targets -- -D warnings` 零警告（rust 规则）
- 门禁终态：`cargo test --workspace` 全绿；verify-react GATE PASSED（豁免不新增）；Flutter dart analyze 0 error；docs-gate 0 failed；verify-semantic-universe.py / verify-vocabulary.py 0 problem；注入语料 855 基线 + 28 元组；Playwright 全量 0 failure
- **ts-rs 绑定同步**：IR 形态变更（`LoadIntentIR` 升级）后必须跑 `cargo test export_bindings --lib -p omnilang` 刷新 poiesis `src/commands/__generated__/`（freshness gate ci.yml:31-38）——Task 3 强制
- **spec §9 开放项全部已裁决（P1-P8），计划直接执行不再重开**：P1 Page<T> 信封约定 `{ items: Array<T>, hasMore: Bool }`；P2 页码式分页（page 从 1 起）；P3 轮询 × 分页 v0 不组合（验证拦截）；P4 Flutter Reset/分页组件渐进升级 AsyncNotifier（非 Reset 实例维持 FutureProvider）；P5 data 可空化（Reset 语义载体）——**兼容性检查点：@for over null data 的生成行为本计划在每个 codegen 任务显式覆盖**；P6 防抖节流 trailing-only 默认；P7 无 pause 参数；P8 双例各自独立 Reset
- 破坏性变更注记：React data 初值 `[]` → `null`（spec §2.1）；既有 React 快照 fixture 需同步更新（Task 6）；Flutter 渲染 `asData?.value ?? const []` 天然 null 安全（既有模式，Task 8 显式断言）

---

### Task 1: 语义契约常量层 + 登记先行（universe 域 9 扩展 / 新域 13 / vocabulary 6 词 / verify 锚点）

**Files:**
- Create: `crates/omnilang-core/src/semantic/load_contract.rs`
- Modify: `crates/omnilang-core/src/semantic/mod.rs`（`pub mod load_contract;`）
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 9 扩展 3 行 + 新域 13 定义与 6 行登记——spec §5.1/§5.2 表格）
- Modify: `docs/superpowers/specs/vocabulary-registry.md`（6 词 4 字段 + 排除词注记——spec §5.3）
- Modify: `scripts/verify-semantic-universe.py`（域 9 断言块扩展 + 域 13 断言块）
- Modify: `scripts/verify-vocabulary.py`（6 词断言）
- Test: `crates/omnilang-core/src/semantic/load_contract.rs` 内 `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: 无（登记先行的新概念层；Task 2-11 消费）
- Produces（verify 脚本断言锚点 = 本文件常量，ADR-074 `rust_str_array` 提取模式，SEMANTIC_EVENT_NAMES 先例）:
  - `pub const LOAD_RESERVED_NAMES: &[&str]` = `["loading", "error", "data", "retry", "reset"]`（匿名实例派生信号全集，含新增 reset——spec §2.1 信号表）
  - `pub const LOAD_PAGINATED_SUFFIXES: &[&str]` = `["Page", "HasMore", "LoadingMore", "LoadMore"]`（命名分页派生信号后缀——spec §2.2 信号表）
  - `pub const LOAD_PARAM_WORDS: &[&str]` = `["paginated", "scroll", "interval"]`（@load 局部模式词——spec §2.2/§2.3/§2.4 BNF）
  - `pub const TIMING_FUNCS: &[&str]` = `["debounce", "throttle"]`（spec §2.5）
  - `pub const PAGE_TYPE_NAME: &str` = `"Page"`（spec §2.2 内建信封）

- [ ] **Step 1: 写失败测试**（load_contract.rs）

```rust
/// 批次 4 语义契约锚点——verify 脚本经 rust_str_array 提取断言（ADR-074）。
/// 任何新 @load 派生信号/模式词必须在此登记，且同时更新 universe 域 9/13 登记册。
#[test]
fn load_contract_matches_design() {
    // spec §2.1 匿名派生信号（含新增 reset）
    assert_eq!(LOAD_RESERVED_NAMES, &["loading", "error", "data", "retry", "reset"]);
    // spec §2.2 分页派生信号后缀
    assert_eq!(LOAD_PAGINATED_SUFFIXES, &["Page", "HasMore", "LoadingMore", "LoadMore"]);
    // spec §2.2/2.3/2.4 @load 模式词
    assert_eq!(LOAD_PARAM_WORDS, &["paginated", "scroll", "interval"]);
    // spec §2.5 std/timing
    assert_eq!(TIMING_FUNCS, &["debounce", "throttle"]);
    assert_eq!(PAGE_TYPE_NAME, "Page");
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core load_contract`
Expected: FAIL（`load_contract` 模块/常量未定义）

- [ ] **Step 3: 最小实现**（load_contract.rs 常量 + mod.rs 注册）

- [ ] **Step 4: universe/vocabulary 登记 + verify 脚本断言**（spec §5.1-§5.4）

- semantic-universe.md 域 9（数据三态）追加 3 行：命名实例多例 / `{name}*` 5 派生信号（Loading/Error/Data/Retry/Reset）/ reset 匿名信号；状态值域含「留生态」（spec §8 治理）
- semantic-universe.md 新域 13「数据深化」定义 + 6 行：paginated / Page\<T\> 信封 / `{name}Page`+`HasMore`+`LoadingMore`+`LoadMore` 派生信号 / scroll / interval / debounce-throttle；状态列按 spec §5.2 表格标注；留生态行附语言边界注记（spec §6：游标分页/分页库绑定/乐观更新/缓存策略）
- vocabulary-registry.md 6 词（debounce、throttle、paginated、scroll、interval、Page\<T\>）4 字段登记 + 排除词注记（refetchInterval/refreshInterval/infinite_scroll_pagination = 平台形态，禁止借词——spec §5.3）
- verify-semantic-universe.py：域 9 断言块新增 3 行（锚点 = `rust_str_array(load_contract.rs, "LOAD_RESERVED_NAMES")` 与登记册行双向一致，既有 `VALIDATE_RULES` 模式）；域 13 新增断言块（锚点 = LOAD_PAGINATED_SUFFIXES / LOAD_PARAM_WORDS / TIMING_FUNCS / PAGE_TYPE_NAME 四常量 + 登记册行双向一致）
- verify-vocabulary.py：6 词四字段断言（`assert_set` 既有模式，spec §5.4）

- [ ] **Step 5: 验证 + Commit**

Run: `cargo test -p omnilang-core load_contract` + `python scripts/verify-semantic-universe.py`（0 problem）+ `python scripts/verify-vocabulary.py`（0 problem）
Expected: 全 PASS

```bash
git add crates/omnilang-core/src/semantic/load_contract.rs crates/omnilang-core/src/semantic/mod.rs docs/superpowers/specs/semantic-universe.md docs/superpowers/specs/vocabulary-registry.md scripts/verify-semantic-universe.py scripts/verify-vocabulary.py
git commit -m "feat(semantic): 批次4 契约常量层 + 登记先行——universe 域9 扩展/新域13/vocabulary 6词/verify 锚点（Task 1）"
```

**验证门禁**: cargo test（load_contract）+ verify 双脚本 0 problem
**依赖**: 无（登记先行——ADR-074，先于 Task 2 语法）

---

### Task 2: 语法形态（@load 命名/参数/模式词解析）

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/view.rs`（`LoadDirective { fn_name, span }` → `LoadDirective { spec: LoadSpec, span }`）
- Modify: `crates/omnilang-core/src/parser/view/directives.rs`（`parse_load_directive` 重写，:101 处）
- Modify: `crates/omnilang-core/src/parser/view/mod.rs`（AtLoad 分发，如有接线）
- Modify: `scripts/verify-semantic-universe.py`（域 13 编译器锚点存在性断言接线：`parse_load_directive` 源码含 "paginated"/"interval" 字样——spec §5.2 锚点「parser @load 参数解析」的落地）
- Test: `crates/omnilang-core/src/parser/view/` 既有 tests 目录模式

**Interfaces:**
- Consumes: Task 1 的 `LOAD_PARAM_WORDS` 语义（文档侧；解析器直接按 spec §2 BNF 实现）
- Produces:
  - `pub enum LoaderRef { Fn(String), Call(CallExpr) }`（spec §2.1 `fn_ref := identifier | call_expr`）
  - `pub struct LoadMode { pub paginated: bool, pub scroll: bool, pub interval: Option<u64> }`
  - `pub struct LoadSpec { pub name: Option<String>, pub loader: LoaderRef, pub mode: LoadMode }`
  - `ViewNode::LoadDirective { spec: LoadSpec, span }`

- [ ] **Step 1: 写失败测试**（解析器 tests，spec §2.1-§2.4 BNF 逐条）

```rust
#[test]
fn parse_named_load_with_call_params() {
    // @load stats={fetchStats(token)} → name=Some("stats"), loader=Call(fetchStats, [token])
    let ast = parse_view(r#"@load stats={fetchStats(token)}"#);
    let LoadDirective { spec, .. } = ast.unwrap();
    assert_eq!(spec.name.as_deref(), Some("stats"));
    assert!(matches!(spec.loader, LoaderRef::Call(_)));
    assert!(!spec.mode.paginated && spec.mode.interval.is_none());
}

#[test]
fn parse_paginated_scroll_interval_combined() {
    // @load history={fetchHistory} paginated scroll interval: 5000
    // → name=Some("history"), mode 全置（paginated=true, scroll=true, interval=Some(5000)）
}

#[test]
fn parse_anonymous_v0_form_unchanged() {
    // @load={fetchItems} → name=None（v0 兼容，LoadDemo 活证据）
}

#[test]
fn parse_error_bad_interval() {
    // interval: 0 / interval: -5 / interval: abc → ParseError（V1 中文消息草案逐字）
}

#[test]
fn parse_error_missing_loader() {
    // @load items={} → ParseError
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core parse_load`
Expected: FAIL（`LoadSpec`/`LoaderRef` 未定义，旧 `LoadDirective { fn_name }` 形态不匹配）

- [ ] **Step 3: 最小实现**（spec §2.1-§2.4 BNF）

`parse_load_directive` 重写：`@load` 之后 lookahead——`ident =` → 命名形式；否则 `{ fn_ref }`。`fn_ref`：identifier 或 call_expr（函数名 + 实参列表）。命名形式后可选模式词序列：`paginated` / `scroll` / `interval : INTEGER`（毫秒，正整数；非法 → ParseError，消息按 spec §3 V1/V5 草案）。既有 v0 匿名路径（`@load={fn}`）零行为变化。AST 节点形态按 Interfaces 块。parse 失败沿用既有 `collect_at_text` 兜底路径检查（mod.rs :65 附近），确认无回归。

- [ ] **Step 4: 运行确认通过 + verify 锚点接线**

Run: `cargo test -p omnilang-core parse_load` + `cargo test -p omnilang-core parser`（全量解析回归）+ `python scripts/verify-semantic-universe.py`（域 13 新锚点断言 PASS）
Expected: 全 PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/view.rs crates/omnilang-core/src/parser/view/directives.rs crates/omnilang-core/src/parser/view/mod.rs scripts/verify-semantic-universe.py
git commit -m "feat(parser): @load 命名/参数/模式词解析——LoadSpec/LoaderRef/LoadMode + V1/V5 解析错误（Task 2）"
```

**验证门禁**: cargo test（parse_load + parser 全量）+ verify-semantic-universe.py
**依赖**: Task 1（登记先行；锚点接线本任务完成）

---

### Task 3: IR + lowering（LoadIntentIR 多例化 + LoadMode）

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs`（:950 `LoadIntentIR { fn_name }` → `{ name/loader/mode }`；:993 `load_intent: Option<LoadIntentIR>` → `Vec<LoadIntentIR>`，保留 `#[serde(default)]`）
- Modify: `crates/omnilang-core/src/ir/lower/view.rs`（LoadDirective → LoadIntentIR 转换，含 call 实参表达式 lowering）
- Modify: `crates/omnilang-core/src/ir/lower/component.rs`（组件收集处升级 Vec——spec §2.1 收集点）
- Modify: `crates/omnilang-core/src/semantic/validate_form_load.rs`（编译适配：Option → Vec 迭代 + V-F6「@load 至多一个」→「匿名 @load 至多一个」；验证逻辑主体 Task 4）
- Modify: `crates/omnilang-core/src/codegen/shared/`（如有 `load_intent` 消费点编译适配）
- Test: `crates/omnilang-core/src/ir/lower/tests.rs` 既有模式 + snapshot

**Interfaces:**
- Consumes: Task 2 的 `LoadSpec`/`LoaderRef`/`LoadMode`
- Produces:
  - `pub enum LoaderIR { Fn(String), Call(CallExprIR) }`
  - `pub struct LoadModeIR { pub paginated: bool, pub scroll: bool, pub interval: Option<u64> }`
  - `pub struct LoadIntentIR { pub name: Option<String>, pub loader: LoaderIR, pub mode: LoadModeIR }`
  - `ComponentIR.load_intent: Vec<LoadIntentIR>`（序列化契约——ts-rs 绑定刷新）
- 破坏性变更注记：IR 形态变化波及 React/Flutter/OmniView 三端 `load_intent` 消费点编译错误——各 codegen 任务（6-11）分别修复，本任务仅保证 core 编译 + 三端编译错误清单留待后续任务（先确认编译错误仅来自 codegen 消费点，无其他散落）

- [ ] **Step 1: 写失败测试**（lower tests）

```rust
#[test]
fn multi_load_intents_collected_in_order() {
    // 组件内 2 个命名 @load（items/stats）→ load_intent.len() == 2 且顺序保持（spec §2.1 多例）
    let ir = lower_program(r#"@load items={fetchOrders} @load stats={fetchStats(token)}"#);
    assert_eq!(ir.components[0].load_intent.len(), 2);
    assert_eq!(ir.components[0].load_intent[0].name.as_deref(), Some("items"));
    assert_eq!(ir.components[0].load_intent[1].name.as_deref(), Some("stats"));
}

#[test]
fn load_call_expr_preserved_in_ir() {
    // @load stats={fetchStats(token)} → loader=Call(fetchStats, [token])，参数表达式保留
}

#[test]
fn load_mode_flags_lowered() {
    // paginated scroll interval: 5000 → mode { paginated: true, scroll: true, interval: Some(5000) }
}

#[test]
fn anon_and_named_mix_collected() {
    // @load={fn} + @load items={fn2} → vec![None, Some("items")]（v0 匿名与命名共存）
}

#[test]
fn serde_snapshot_backward_compat() {
    // 旧 JSON 快照（单例形态）反序列化不 panic（#[serde(default)]——WASM 序列化契约）
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core multi_load_intents`
Expected: FAIL（编译错误：`LoadIntentIR` 字段不匹配 / 非 Vec）

- [ ] **Step 3: 最小实现**（spec §2.1 IR 方向）

types.rs 形态按 Interfaces 块；lower/view.rs LoadDirective → LoadIntentIR（call 实参走既有表达式 lowering）；lower/component.rs 收集点改为追加 Vec；validate_form_load.rs 适配迭代（V-F6 语义微调：匿名至多一个，命名不限——spec §3 V6）。`cargo test export_bindings --lib -p omnilang` 刷新 ts-rs 绑定（Global Constraints）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang-core`（全量——既有 fixture 中单例断言若断言形态变化需同步）
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs crates/omnilang-core/src/ir/lower/ poiesis/src/commands/__generated__/ crates/omnilang-core/src/semantic/validate_form_load.rs
git commit -m "feat(ir): LoadIntentIR 多例化——Vec + name/loader/mode + V-F6 匿名至多一个（Task 3）"
```

**验证门禁**: cargo test -p omnilang-core + ts-rs 绑定刷新（export_bindings）
**依赖**: Task 2

---

### Task 4: 语义验证 V1-V11（validate_load 重写）

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_form_load.rs`（:144 `validate_load` 重写为 Vec 迭代 + V1-V11 检查；:219 `validate_retry_call_arity` 区域扩展）
- Test: 同文件 tests（既有 `validate_program_with_view` 模式）

**Interfaces:**
- Consumes: Task 1 `LOAD_RESERVED_NAMES`/`LOAD_PAGINATED_SUFFIXES`/`TIMING_FUNCS`；Task 3 `Vec<LoadIntentIR>`
- Produces: 诊断（`Severity::Error` 为主 + V10/V11 `Severity::Warning`——中文消息逐字按 spec §3 表）

- [ ] **Step 1: 写失败测试**（spec §3 V1-V11 逐条，消息逐字断言）

```rust
#[test]
fn named_load_conflicts_with_reserved() {
    // @load items={f} 且既有信号 items → Error，消息含「@load 实例名 "@items" 与既有信号冲突」（V2）
}

#[test]
fn duplicate_named_load_rejected() {
    // 两个 @load items → Error「重复的 @load 实例名 "@items"」（V3）
}

#[test]
fn derived_signal_conflict_detected() {
    // @load items={f} 且既有信号 itemsLoading → Error（V4 派生信号全名冲突——5 后缀 + 分页 4 后缀）
}

#[test]
fn interval_must_be_positive() {
    // interval: 0 与 interval: -100 → Error「@load interval 必须为正整数（毫秒）」（V5）
}

#[test]
fn anonymous_paginated_rejected() {
    // @load={f} paginated → Error「@load paginated 必须使用命名实例」（V7）
}

#[test]
fn scroll_requires_paginated() {
    // @load items={f} scroll → Error「@load scroll 需要与 paginated 一起使用」（V8）
}

#[test]
fn multiple_anonymous_rejected() {
    // 两个匿名 @load → Error（V6——升级版 V-F6 消息，spec §3）
}

#[test]
fn debounce_throttle_args_validated() {
    // debounce(fn, 0) / debounce(fn, -5) / debounce(123, 300) → Error（V9：ms 正整数字面量 + 首参可调用）
    // debounce(fn, 300) / throttle(fn, 1000) → 无诊断
}

#[test]
fn non_scrollable_container_warns() {
    // scroll 模式但 @for 容器无高度约束 → Warning（V10「…可能不可滚动——检查容器高度」）
}

#[test]
fn pagination_signature_unconfirmable_warns() {
    // paginated 且 loader 为 call 表达式 → Warning（V11「分页 loader 签名无法静态确认——请声明 fn {name}(page: i32) -> Page<T>」）
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core named_load_conflicts`
Expected: FAIL（V1-V11 未实现——既有验证仅 V-F2..F8）

- [ ] **Step 3: 最小实现**（spec §3 V1-V12 表——V12 运行时归 Task 7，不在此）

`validate_load` 改为对 `component.load_intent` Vec 迭代：V1（解析已拦，兜底重检名字合法性）、V2（实例名 vs `LOAD_RESERVED_NAMES` + 既有信号/函数名/派生信号全名冲突，消息含 `LOAD_RESERVED_NAMES` 建议）、V3 重复命名、V4 派生冲突（`LOAD_PAGINATED_SUFFIXES` 拼接）、V5、V6、V7、V8、V9（`TIMING_FUNCS` 名称解析 + 实参形态检查）、V10/V11 warning。既有 V-F2..F8 保留。消息逐字对齐 spec §3 表格。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang-core validate_form_load` + `cargo test -p omnilang-core`（全量回归）
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_form_load.rs
git commit -m "feat(semantic): @load 验证网 V1-V11——命名/派生冲突/interval/多匿名/分页滚动组合/debounce 参数（Task 4）"
```

**验证门禁**: cargo test -p omnilang-core 全量
**依赖**: Task 3（Vec IR）；Task 1（常量）

---

### Task 5: std/timing stdlib（debounce/throttle 语义注册）

**Files:**
- Create: `crates/omnilang-core/src/stdlib/timing.rs`
- Modify: `crates/omnilang-core/src/stdlib/mod.rs`（注册表追加 module `"std/timing"`）
- Modify: `scripts/verify-semantic-universe.py`（域 13 debounce/throttle 行 stdlib 注册表锚点存在性断言接线——spec §5.2 锚点「stdlib 注册表」）
- Test: `crates/omnilang-core/src/stdlib/tests.rs`（既有注册表测试模式，std/async 先例）

**Interfaces:**
- Consumes: Task 1 `TIMING_FUNCS` 语义（文档侧）
- Produces: stdlib 注册条目——module `"std/timing"`，函数 `debounce`/`throttle`，签名 `(callable, positive_int_literal) -> callable`（V9 语义已在 Task 4 验证；本任务注册类型/arity 供类型检查与三端 codegen 统一解析）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn std_timing_module_registered() {
    // stdlib 注册表含 module "std/timing"，函数 debounce/throttle（spec §2.5 import 形态）
    let m = stdlib_lookup("std/timing").expect("std/timing 必须注册");
    assert!(m.functions().any(|f| f.name == "debounce"));
    assert!(m.functions().any(|f| f.name == "throttle"));
}

#[test]
fn debounce_type_checked_as_callable_return() {
    // 类型检查器：debounce(doSearch, 300) → 函数类型（可作 @onInput 处理器）
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core std_timing`
Expected: FAIL（`std/timing` 未注册）

- [ ] **Step 3: 最小实现**（spec §2.5 + §5.2 锚点）

`timing.rs` 按 stdlib/mod.rs 注册表模式登记 `debounce`/`throttle`（module 名 `"std/timing"`、snake_case、arity 2、类型签名 `(fn, int) -> fn`）；mod.rs 挂载。verify-semantic-universe.py 域 13 该行锚点 = stdlib 注册表源码含 "std/timing"（既有 `rust_str_array` 或源码包含断言，按脚本既有模式选）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang-core std_timing` + `cargo test -p omnilang-core stdlib` + `python scripts/verify-semantic-universe.py`
Expected: 全 PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/stdlib/timing.rs crates/omnilang-core/src/stdlib/mod.rs scripts/verify-semantic-universe.py
git commit -m "feat(stdlib): std/timing debounce/throttle 语义注册 + 域13 锚点接线（Task 5）"
```

**验证门禁**: cargo test（std_timing + stdlib）+ verify-semantic-universe.py
**依赖**: Task 1（可并行于 Task 2-4）

---

### Task 6: React codegen 基础深化（多例/参数/Reset/data 可空化 + @for null 兜底）

**Files:**
- Modify: `omnilang/src/codegen/react/component/mod.rs`（:269/:278/:291/:560/:1071 `load_intent` 门控升级 Vec 迭代）
- Modify: `omnilang/src/codegen/react/component/function.rs`（:296 @load 块重写：每实例 5 派生 useState + 参数 deps 重载 + reset callback）
- Modify: `omnilang/src/codegen/react/component/jsx.rs`（data 渲染点 `?? []` 兜底——spec §2.1 React 渲染形态）
- Modify: `omnilang/src/codegen/react/component/state.rs` / `store_slices.rs`（按需）
- Test: `omnilang/src/codegen/react/` 既有 tests + snapshot fixture 更新（data 初值 `[]` → `null` 破坏性变更）

**Interfaces:**
- Consumes: Task 3 IR（`Vec<LoadIntentIR>`）
- Produces: 命名实例 5 信号（`{name}Loading`/`{name}Error`/`{name}Data`/`{name}Retry`/`{name}Reset`——spec §2.1 信号表）+ 匿名 5 信号（含新增 reset）+ 参数重载（useEffect deps 含参数字符串化）+ Reset 不重载（P5）

- [ ] **Step 1: 写失败测试**（react tests，产物字符串断言）

```rust
#[test]
fn react_multi_load_generates_per_instance_signals() {
    // @load items={fetchOrders} @load stats={fetchStats} → 产物含 itemsLoading/statsLoading/itemsError/statsError/itemsData/statsData/itemsRetry/statsRetry/itemsReset/statsReset
    let out = compile_react(r#"@load items={fetchOrders} @load stats={fetchStats}"#);
    for s in ["itemsLoading", "statsLoading", "itemsData", "statsData", "itemsReset", "statsReset"] {
        assert!(out.contains(s), "{s} 信号缺失");
    }
}

#[test]
fn react_param_signal_change_auto_reloads() {
    // @load stats={fetchStats(token)} → useEffect deps 含 token（信号变更自动重载，spec §2.1 参数绑定）
}

#[test]
fn react_reset_sets_data_null_no_refetch() {
    // reset callback → setItemsData(null)/setItemsError(null)/setItemsLoading(false)，产物无 refetch 调用（P5 不自动重载）
}

#[test]
fn react_anon_reset_signal_emitted() {
    // 匿名 @load → reset 信号（v0 无此信号——新增，spec §2.1）
}

#[test]
fn react_for_over_null_data_renders_empty() {
    // ★ 兼容性检查点（主审查要求）：@for item in data 且 data 为 null（Reset 后）→
    // 迭代产物为 (data ?? []).map(...)（spec §2.1 渲染兜底），null 不崩溃
    let out = compile_react(r#"@for item in data"#);
    assert!(out.contains("?? []"), "@for over null data 必须生成空数组兜底");
}

#[test]
fn react_data_initial_null_breaking_change() {
    // data 初值从 [] 改为 null（破坏性变更——既有快照 fixture 同步更新）
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang --features codegen-react react_multi_load`
Expected: FAIL（单例门控产物不匹配 / 编译错误待修）

- [ ] **Step 3: 最小实现**（spec §2.1 + §4.1 映射）

`load_intent` 门控升级：对 Vec 迭代，每实例生成独立 state/effect/retry/reset 组；参数绑定 = loader call 实参中信号名进入 useEffect deps（变更自动重载）；Reset = 清 data/error/loading 三态置初值，不触发 fetch；data 初值 `null`；渲染点与 @for 迭代点统一 `?? []` 兜底。v0 匿名实例产物除新增 reset 外零变化（LoadDemo 不动）。快照 fixture 同步（`[]` → `null`）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang --features codegen-react` + `scripts/verify-react.sh`（GATE PASSED，tsc 覆盖新产物 fixture）
Expected: 全 PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/
git commit -m "feat(codegen-react): @load 多例/参数重载/Reset/data 可空化 + @for null 兜底（Task 6）"
```

**验证门禁**: cargo test --features codegen-react + verify-react GATE（tsc）
**依赖**: Task 3（IR Vec）；Task 4（验证先行，负例测试锚点）

---

### Task 7: React codegen 分页/滚动/轮询 + debounce/throttle + V12 运行时

**Files:**
- Modify: `omnilang/src/codegen/react/component/mod.rs` / `function.rs`（paginated 分支：page/hasMore/loadingMore 三 state + loadMore callback + append；scroll 分支：sentinel div + IntersectionObserver effect + 卸载 disconnect——spec §2.2/§2.3）
- Modify: `omnilang/src/codegen/react/component/effect.rs`（interval 分支：setInterval + unmount clearInterval——spec §2.4）
- Modify: `omnilang/src/codegen/react/expr.rs` / `utils.rs`（std/timing 映射：`useMemo(() => debounce(fn, ms), [fn])` 稳定引用——spec §2.5 + §4.1）
- Modify: `omnilang/src/codegen/react/types.rs`（`Page<T>` TS interface 生成——spec §2.2 信封）
- Test: react tests + snapshot + V12 运行时断言

**Interfaces:**
- Consumes: Task 3 IR（mode 标志）；Task 5 std/timing 注册
- Produces: 分页 4 派生信号（`{name}Page` 从 1 / `{name}HasMore` / `{name}LoadingMore` / `{name}LoadMore()` 页码+1 append——spec §2.2 信号表）+ `Page<T> { items: T[]; hasMore: boolean }` interface + append 失败 → `{name}Error` 保持已加载数据、LoadMore 可重试

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn react_pagination_state_machine_generated() {
    // @load history={fetchHistory} paginated → 产物含 historyPage/historyHasMore/historyLoadingMore/historyLoadMore + Page 初值 1
}

#[test]
fn react_load_more_appends_and_increments() {
    // LoadMore() → setHistoryPage(p+1) + 数据 concat append 产物（spec §2.2 追加语义）
}

#[test]
fn react_paginated_reset_resets_page() {
    // Reset → setHistoryPage(1) + setHistoryData(null)（P8：分页 Reset 页码回 1）
}

#[test]
fn react_append_failure_keeps_loaded_data() {
    // 追加页失败 → historyError 置错 + 已加载数据保留 + LoadMore 可重试（spec §2.2 失败语义）
}

#[test]
fn react_scroll_sentinel_observer_generated() {
    // paginated scroll → 最后一项 sentinel div + IntersectionObserver + 卸载 disconnect（spec §2.3）
    // 双守卫：LoadingMore || !HasMore 时不触发
}

#[test]
fn react_interval_cleanup_on_unmount() {
    // interval: 5000 → setInterval + clearInterval 同 effect（spec §2.4 卸载清理；每轮 = reload retry 语义）
}

#[test]
fn react_debounce_stable_reference() {
    // debounce(doSearch, 300) → useMemo(() => debounce(doSearch, 300), [doSearch]) 产物（spec §2.5 trailing-only 默认）
}

#[test]
fn react_page_envelope_interface_emitted() {
    // Page<T> interface { items: T[]; hasMore: boolean } 生成（spec §2.2 信封）
}

#[test]
fn react_v12_missing_has_more_honest_degradation() {
    // 运行时信封校验：响应无 hasMore → 错误态 + 控制台消息（V12——诚实降级，绝不静默）
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang --features codegen-react react_pagination_state_machine`
Expected: FAIL

- [ ] **Step 3: 最小实现**（spec §2.2/§2.3/§2.4/§2.5 + §4.1 映射逐行）

分页：三 state + `loadMore` useCallback（页码+1、concat append、`Page<T>` 解包 items 入 `{name}Data`）；信封校验（V12）在 fetch 响应处理处；scroll：sentinel 渲染于 @for 列表末、IntersectionObserver 触发 `loadMore`（`{name}LoadingMore || !{name}HasMore` 双守卫）、effect cleanup disconnect；interval：`setInterval` 回调 = reload（复用 retry 语义）；debounce/throttle：expr 层 std/timing 名称映射到 `useMemo` 稳定包装（依赖 `[fn]`），trailing-only（P6）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang --features codegen-react` + `scripts/verify-react.sh`
Expected: GATE PASSED

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/
git commit -m "feat(codegen-react): 分页状态机/无限滚动哨兵/轮询/防抖节流映射 + V12 信封校验（Task 7）"
```

**验证门禁**: cargo test --features codegen-react + verify-react GATE（tsc）
**依赖**: Task 6；Task 5（std/timing 注册）

---

### Task 8: Flutter codegen 基础深化（多例/参数/Reset + AsyncNotifier 渐进升级 + null 兜底）

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（:115-126 ctx.retry_expr / :269-277 context 携带 / :2162-2173 retry ref 映射——多实例派生信号 + reset + params）
- Modify: `omnilang/src/codegen/flutter/stmt.rs`（参数信号 `ref.watch` 进 provider body）
- Modify: `omnilang/src/codegen/flutter/tests.rs`（产物断言 + snapshot）
- Test: flutter/tests.rs 既有模式 + `generated_code_validation` 门禁

**Interfaces:**
- Consumes: Task 3 IR（Vec）
- Produces: 命名实例 `{name}Async` provider（spec §4.2 命名）+ 参数信号 `ref.watch` 自动重载 + Reset 组件 AsyncNotifier 升级（P4：`setState(AsyncData(null))`——`ref.invalidate` ≠ Reset，spec §2.1 注记）+ 非 Reset 实例维持 FutureProvider

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn flutter_multi_load_per_instance_providers() {
    // 2 命名 @load → 产物含 itemsAsync/statsAsync/itemsRetry/statsRetry + reset 回调（spec §4.2）
}

#[test]
fn flutter_param_signal_watched_in_provider_body() {
    // @load stats={fetchStats(token)} → provider body 内 ref.watch(token)（变更自动重载）
}

#[test]
fn flutter_reset_upgrades_to_async_notifier() {
    // 组件含 Reset 用点 → AsyncNotifier + setState(AsyncData(null))（P4 渐进升级；无 Reset 的组件维持 FutureProvider）
}

#[test]
fn flutter_for_over_null_data_falls_back_empty() {
    // ★ 兼容性检查点：@for item in data 且 data 为 null → asData?.value ?? const [] 产物（既有模式显式断言，spec §2.1）
}

#[test]
fn flutter_anon_reset_emitted() {
    // 匿名 @load → reset（v0 无）
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang --features codegen-flutter flutter_multi_load`
Expected: FAIL

- [ ] **Step 3: 最小实现**（spec §2.1 + §4.2 映射）

view.rs/stmt.rs 升级：对 Vec 迭代生成每实例 provider/派生命名；参数绑定 = provider body `ref.watch(参数信号)`；含 Reset 用点的实例生成 AsyncNotifier 子类（`setState(AsyncData(null))` 实现 Reset，不触发重载）；无 Reset 实例维持 FutureProvider（渐进式，P4）。渲染点与 @for 迭代点 `asData?.value ?? const []` 兜底（既有模式固化断言）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang --features codegen-flutter` + `dart analyze`（`scripts/verify-flutter.sh` 或等价入口，0 error）
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/
git commit -m "feat(codegen-flutter): @load 多例/参数 watch/Reset AsyncNotifier 渐进升级 + null 兜底（Task 8）"
```

**验证门禁**: cargo test --features codegen-flutter + dart analyze 0 error
**依赖**: Task 3

---

### Task 9: Flutter codegen 分页/滚动/轮询 + debounce/throttle

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（分页 AsyncNotifier：state = `AsyncData([...旧页, ...新页])` + hasMore/loadingMore 字段 + `loadMore()` + `reset()`——spec §2.2 + §4.2）
- Modify: `omnilang/src/codegen/flutter/stmt.rs` / 表达式区（ScrollController listener + `pixels >= maxScrollExtent - 阈值` sentinel——spec §2.3；Timer.periodic 轮询 + cancel——spec §2.4；debounce/throttle → Timer 闭包包装——spec §2.5 + §4.2）
- Modify: `omnilang/src/codegen/flutter/types.dart` 生成（`Page<T>` class：items/hasMore——spec §2.2 信封）
- Test: flutter/tests.rs + snapshot + dart analyze

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn flutter_paginated_notifier_generated() {
    // paginated → AsyncNotifier 含 page/hasMore/loadingMore 状态 + loadMore 追加（页码+1）+ 初值 page=1
}

#[test]
fn flutter_scroll_controller_sentinel() {
    // paginated scroll → ScrollController listener + maxScrollExtent 阈值 + dispose 清理（spec §2.3 阈值语义）
}

#[test]
fn flutter_interval_timer_periodic_dispose() {
    // interval: 5000 → Timer.periodic + dispose cancel（spec §2.4）
}

#[test]
fn flutter_page_envelope_class_emitted() {
    // Page<T> class { items; hasMore } 生成（spec §2.2）
}

#[test]
fn flutter_debounce_timer_wrapper() {
    // debounce(doSearch, 300) → Timer 延迟闭包包装（spec §2.5 trailing-only）
}
```

- [ ] **Step 2: 运行确认失败 → Step 3: 最小实现**（spec §2.2/§2.3/§2.4/§2.5 + §4.2 映射，形态按 Interfaces 块）

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang --features codegen-flutter` + `dart analyze` 0 error
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/
git commit -m "feat(codegen-flutter): 分页 AsyncNotifier/滚动阈值/轮询 Timer/防抖节流映射 + Page<T> class（Task 9）"
```

**验证门禁**: cargo test --features codegen-flutter + dart analyze 0 error
**依赖**: Task 8；Task 5

---

### Task 10: OmniView codegen 基础深化（多 load 槽位/参数/Reset/null 兜底）

**Files:**
- Modify: `omnilang/src/codegen/omniview/scope.rs`（:35 `load_intent: Option<LoadIntentIR>` → `Vec<LoadIntentIR>` + mode 携带；:78/:100/:123 消费点适配）
- Modify: `omnilang/src/codegen/omniview/store.rs`（槽位扩展：`{name}_loading`/`{name}_error`/`{name}_data`/`{name}_retry`/`{name}_reset` + 匿名 reset——spec §4.3 槽位表）
- Modify: `omnilang/src/codegen/omniview/view.rs`（data 槽位 null 兜底 + 参数信号监听重载）
- Test: omniview tests + snapshot

**Interfaces:**
- Consumes: Task 3 IR（Vec）
- Produces: 每实例 5 槽位（snake_case 命名）+ reset 回调（data=null/error=null/loading=false，不自动重载——P5）+ 参数信号变更重载

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn omniview_multi_load_store_slots() {
    // 2 命名 @load → store 含 items_loading/items_error/items_data/items_retry/items_reset + stats_* 五槽（spec §4.3）
}

#[test]
fn omniview_reset_slot_sets_data_null() {
    // reset 回调 → data=null/error=null/loading=false（P5，不触发重载）
}

#[test]
fn omniview_for_over_null_data_guarded() {
    // ★ 兼容性检查点：@for item in data 且 data 槽位为 null → 迭代前 null 检查（无崩溃、渲染空闲态）
}

#[test]
fn omniview_param_signal_change_reloads() {
    // @load stats={fetchStats(token)} → token 信号变更触发重载
}
```

- [ ] **Step 2: 运行确认失败 → Step 3: 最小实现**（spec §2.1 + §4.3 映射——scope.rs Vec 化 + store 槽位生成 + 渲染兜底）

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang --features codegen-omniview`（按既有 feature 名；若为默认后端则 `cargo test -p omnilang omniview`）
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/
git commit -m "feat(codegen-omniview): @load 多例槽位/参数重载/Reset + @for null 兜底（Task 10）"
```

**验证门禁**: cargo test omniview codegen 全量
**依赖**: Task 3

---

### Task 11: OmniView codegen 分页/滚动/轮询 + debounce/throttle JS 映射

**Files:**
- Modify: `omnilang/src/codegen/omniview/store.rs`（分页槽位：`{name}_page`/`{name}_has_more`/`{name}_loading_more` + `{name}_load_more` 回调——spec §4.3 槽位表）
- Modify: `omnilang/src/codegen/omniview/view.rs`（sentinel IntersectionObserver——spec §2.3；setInterval + clearInterval——spec §2.4；debounce/throttle → setTimeout 闭包——spec §2.5 + §4.3）
- Test: omniview tests + snapshot

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn omniview_pagination_slots() {
    // paginated → store 含 history_page(初值1)/history_has_more/history_loading_more + history_load_more 回调（页码+1 append）
}

#[test]
fn omniview_scroll_sentinel_js() {
    // paginated scroll → sentinel + IntersectionObserver + 卸载 disconnect + 双守卫
}

#[test]
fn omniview_interval_js() {
    // interval: 5000 → setInterval + clearInterval 清理（每轮 reload）
}

#[test]
fn omniview_debounce_js() {
    // debounce(fn, 300) → setTimeout 闭包稳定引用
}
```

- [ ] **Step 2: 运行确认失败 → Step 3: 最小实现**（spec §2.2/§2.3/§2.4/§2.5 + §4.3 映射）

- [ ] **Step 4: 运行确认通过**

Run: cargo test omniview codegen 全量
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/
git commit -m "feat(codegen-omniview): 分页槽位/滚动哨兵/轮询/防抖节流 JS 映射（Task 11）"
```

**验证门禁**: cargo test omniview codegen 全量
**依赖**: Task 10；Task 5

---

### Task 12: demo 迁移 + 新 demo（语义化即切换 + Playwright 8 断言）

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/DataFetching.omni`（spec §8.1 迁移表逐行）
- Create: `test/projects/learn-omniview/src/components/PaginationDemo.omni`（spec §8.2）
- Create: `test/projects/learn-omniview/src/components/PollingDemo.omni`（spec §8.2——std/timing 首用点）
- Modify: `test/projects/learn-omniview/src/pages/UiComponents.omni`（挂载 PaginationDemo/PollingDemo）
- Modify: `test/projects/learn-omniview/playwright/`（8 断言——spec §7.3 清单）
- Modify: `scripts/verify-demo-coverage.py`（新 demo 登记——spec §5.4）

**Interfaces:**
- Consumes: Task 6-11 三端 codegen；Task 1 登记册
- Produces: 零旧形态语料 + 新能力活证据（PaginationDemo：按钮分页 + scroll 无限滚动双形态；PollingDemo：interval 轮询 + debounce 搜索）

- [ ] **Step 1: 按 §8.1 迁移表重写 DataFetching.omni**

逐行：删除 `status`/`resultText`/`errorMsg` 信号；保留 `url` 信号作参数源（spec §8.1 表）；`loadData()` action → `async fn loadDirect() -> String`；`loadViaApi()` → `async fn loadViaApi() -> String`；删除 `fn reset()`；四态 @if → `directLoading`/`directError`/`directData == null` 空闲态（data 可空化后自然表达）；按钮 → `directRetry`/`apiRetry`/`directReset`（P8：双例各自独立 Reset）。匿名 @load 不删（活证据）。不写手写状态机（ADR-075 原则 8——旧形态硬化由新语义形态直接表达）。

- [ ] **Step 2: 新建 PaginationDemo.omni**（spec §8.2）

`fn fetchHistory(page: i32) -> Page<String>` 分页 loader + `@load history={fetchHistory} paginated`（按钮分页形态）+ `@load feed={fetchFeed} paginated scroll`（无限滚动形态）；@for + LoadMore 按钮 + 底部 sentinel 区域。

- [ ] **Step 3: 新建 PollingDemo.omni**（spec §8.2）

`@load prices={fetchPrices} interval: 5000` 轮询 + `@onInput={debounce(doSearch, 300)}` 搜索防抖（`import { debounce } from "std/timing"`——首用点）。

- [ ] **Step 4: 接线 + Playwright 8 断言**（spec §7.3）

UiComponents.omni 挂载两个新组件（PageSwitcher 标签形态）；Playwright 新增/扩展 8 断言：多例独立（items/stats 互不干扰）/ 参数重载（改 url 信号 → 数据刷新）/ Reset（data → null → 空闲态渲染且 @for 不崩溃）/ 分页（LoadMore 追加）/ 无限滚动（滚动接近底部自动加载）/ 轮询（interval 生效）/ debounce（输入停顿后触发一次）/ 空闲态（Reset 后 `@if data == null && !loading` 分支渲染）。verify-demo-coverage.py 登记两个新 demo。

- [ ] **Step 5: 验证 + Commit**

Run: 三目标编译（`cargo build --release -p omnic && omnic build --target react|flutter|omniview` 按项目既有用法）+ Playwright 全量（含 8 新断言）+ `python scripts/verify-demo-coverage.py`
Expected: 0 failures

```bash
git add test/projects/learn-omniview/ scripts/verify-demo-coverage.py
git commit -m "refactor(demo): DataFetching 双例迁移（§8.1）+ PaginationDemo/PollingDemo 新建 + Playwright 8 断言（Task 12）"
```

**验证门禁**: 三目标编译 + Playwright 全量 + verify-demo-coverage.py
**依赖**: Task 6-11（三端 codegen 全部就绪）

---

### Task 13: 文档同步 + 注入语料 28 元组 + 收口终态

**Files:**
- Modify: `docs/omniview/16-forms-and-async-data.md`（spec §8.1 双例形态/Reset/分页/轮询/防抖节流章节——学习文档与语料同步，ADR-075）
- Modify: `docs/omniview/12-standard-library.md`（std/timing 模块登记）
- Modify: `docs/omniview/07-events-and-bindings.md`（如需：debounce/throttle 事件处理器形态）
- Modify: `omnilang/src/codegen/tests_regression/injection.rs`（28 元组 pattern-missing 断言——spec §7.5 清单）
- Test: 注入语料全绿 + docs-gate + 五层终态

- [ ] **Step 1: 注入语料 28 元组**（spec §7.5 清单逐条，injection.rs 既有 pattern-missing 模式）

- 5 命名派生信号：`{name}Loading/Error/Data/Retry/Reset` 逐字产物断言（跨 5 后端全链路可见——parse → semantic → diagnostics 不因目标差异丢失）
- 4 参数绑定：参数信号重载产物断言
- 2 Reset：Reset 语义产物断言（React setData(null) / Flutter AsyncData(null) / OmniView data=null）
- 5 paginated：分页信号/状态机产物断言
- 2 paginated scroll：sentinel 产物断言
- 2 interval：轮询产物断言
- 4 debounce/throttle：std/timing 映射产物断言
- 4 错误路径：V2/V6/V7/V8 诊断消息逐字跨后端断言（spec §3 表格 verbatim）

- [ ] **Step 2: 文档同步**（spec §8.1 文档面）

16-forms-and-async-data.md 按迁移后形态重写 DataFetching 章节 + 新增 @load 深化（多例/参数/Reset/分页/无限滚动/轮询）与 std/timing 章节；12-standard-library.md 登记 std/timing；07-events-and-bindings.md 补 debounce/throttle 事件处理器形态（如有用点）。

- [ ] **Step 3: 收口验证**（五层门禁终态）

Run: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test export_bindings --lib -p omnilang`（绑定新鲜度）+ verify-react GATE + Flutter dart analyze（0 error）+ `python scripts/verify-docs.py --check`（0 failed）+ verify 双脚本 + 注入语料（855 基线 + 28 元组）+ Playwright 全量
Expected: 全绿

- [ ] **Step 4: Commit**

```bash
git add docs/omniview/ omnilang/src/codegen/tests_regression/injection.rs
git commit -m "docs(batch4): 16-forms/12-stdlib 同步 + 注入语料 28 元组 + 收口终态（Task 13）"
```

**验证门禁**: 五层全绿（上一步命令清单）
**依赖**: Task 12；Task 4（错误路径消息锚点）

---

## 阶段顺序（任务依赖图）

```
阶段 A 登记+语法链（顺序）：Task 1 → Task 2 → Task 3 → Task 4
阶段 B stdlib（可并行）：Task 5 ∥ Task 2-4（仅依赖 Task 1）
阶段 C 三目标 codegen（三链并行）：
  React 链：Task 4 → Task 6 → Task 7
  Flutter 链：Task 4 → Task 8 → Task 9
  OmniView 链：Task 4 → Task 10 → Task 11
  Task 5 → Task 7 / Task 9 / Task 11（std/timing 映射）
阶段 D demo 迁移：Task 12（依赖 Task 6-11 全部）
阶段 E 文档+注入+收口：Task 13（依赖 Task 12）
```

可并行标注：Task 5 与 Task 2-4 并行；Task 6/8/10 三链并行（互不触碰文件——React/Flutter/OmniView 各自目录）；Task 7/9/11 同理。审查者逐任务放行，阶段 C 内三链可交错。

## 登记先行任务（ADR-074）

Task 1 为登记先行任务（semantic-universe 域 9 扩展 + 新域 13 + vocabulary 6 词 + verify 脚本断言锚点），先于 Task 2 语法实施——与 form-load 先例（登记后置）不同，因域 13 为**新域**必须先行。代码锚点存在性断言随引入任务接线：Task 2（parser 参数解析锚点）、Task 5（stdlib 注册表锚点）。verify-demo-coverage.py 新 demo 登记在 Task 12（demo 存在后）。全部登记册断言锚点 = 编译器代码常量（`rust_str_array` 提取，SEMANTIC_EVENT_NAMES 先例），保证每任务边界 verify 双脚本全绿。

## 语义化即切换任务（旧形态硬化/迁移提示/demo 迁移）

- **Task 12 为切换任务**：DataFetching.omni 按 spec §8.1 迁移表逐行切换——旧四态信号（status/resultText/errorMsg）删除、手写 reset 删除、双 action 语义化为双 @load 实例 + 独立 Reset（P8），不留旧残留（ADR-075 原则 8）
- **数据可空化破坏性变更**：React data 初值 `[]` → `null`（Task 6 快照 fixture 同步 + 渲染 `?? []` 兜底）；Flutter `asData?.value ?? const []`（Task 8 显式断言）；OmniView data 槽位 null 兜底（Task 10）——**@for over null data 生成行为已在 Task 6/8/10 各设独立 RED 用例显式覆盖**（兼容性检查点）
- **LoadDemo.omni 零变更**：v0 匿名 @load 活证据（spec §8.2 注记）
- 迁移提示面：V1-V11 诊断消息全部中文 verbatim 带建议（spec §3 表）；无旧形态共存期

## 门禁终态（五层最终验收口径）

| 层 | 命令 | 口径 |
|---|---|---|
| 1 tsc | `scripts/verify-react.sh` | GATE PASSED（豁免不新增） |
| 2 dart analyze | `scripts/verify-flutter.sh` 或等价入口 | 0 error |
| 3 Playwright | learn-omniview 全量套件 | 0 failure（含 Task 12 的 8 新断言） |
| 4 docs-gate | `python scripts/verify-docs.py --check` | 248 blocks 0 failed |
| 5 注入语料 | `cargo test -p omnilang tests_regression::injection` | 855 基线 + 28 元组全绿 |

+ `cargo test --workspace` 全绿 + `cargo clippy --workspace --all-targets -- -D warnings` 零警告 + verify 双脚本 0 problem + ts-rs 绑定新鲜（export_bindings）+ 语义化即切换零残留（grep DataFetching 旧信号名 0 命中）。

## 风险与降级注记（spec §6 语言边界落地方式）

- **轮询 × 分页不组合**（P3）：V8 拦截 + 文档注记，触发条件 = 真实语料（记录于 semantic-universe 域 13 留生态行）
- **分页库绑定为未来增强路径**（spec §2.2 决策注记）：v0 自生成（三理由），若复杂度失控触发重评——留生态注记
- **游标分页/乐观更新/缓存策略/prefetch/请求去重竞态/滚动恢复**：留生态记录（universe 状态列 + 触发条件），乐观更新触发条件 = 写路径语义化
- **V10/V11 为 warning**（可编译但诚实提示）：容器不可滚动 / 分页签名不可静态确认——绝不静默（ADR-054）
- **V12 运行时信封校验**（React 实现）：缺 hasMore → 错误态 + 控制台消息，诚实降级
- **半实现信号记录**（非本批次）：form.submitting 恒 false / Flutter form.valid 恒 true——仅记录，不处理
- **回归风险**：IR 形态变化波及三端消费点编译错误——Task 3 后先确认错误仅限 codegen 消费点；快照 fixture 破坏性变更（`[]` → `null`）集中在 Task 6 处理

## 收口验证（全部任务完成后）

Run: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` + verify-react GATE + Flutter dart analyze + docs-gate + verify 双脚本 + 注入语料 + Playwright 全量
Expected: 全绿；learn-omniview 零旧 DataFetching 形态；spec §10 验收第 1 条（能力矩阵数据深化行全绿）达成——批次 4 交付。
