# 阶段 1：事件意图化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 语义事件分级 + 旧名一步到位切换 + schema 增强（pointerId/buttons/progress）+ key 枚举 + enter/leave 别名 + 坐标基准元素局部化，三目标 codegen 收口。

**Architecture:** 既有事件机制已成熟（canonical 表 + `EventDataKind` schema + 三端 `field_extract_*` + `used_fields` 提取 + `invalid_event_field` lint），本计划在其上收口：① 新增「语义事件分级」概念层（14 个语义事件名 + 旧名映射），旧名产生硬诊断 + 迁移提示（spec 原则 8）；② schema 增强与坐标基准定义（元素局部坐标 D1）；③ key 枚举验证（D2）。React/OmniView 走 `field_extract_react`/`field_extract_js` 扩展，Flutter 走 view.rs 事件字段区。

**Tech Stack:** Rust（omnilang-core / omnilang）、React 生成（tsc 门禁）、Flutter 生成（dart analyze 门禁）、Playwright 浏览器验证。

## Global Constraints

- 诊断消息**中文 verbatim**（ADR-056 政策，如「未知事件名」先例）
- **语义化即切换，不留旧残留**（spec 原则 8）：旧名（@onClick/@onMouseEnter/@onMouseLeave）落地即硬错误 + 迁移提示，无透传共存期
- 新增语法先登记 semantic-universe（ADR-074）——本计划 Task 9 在实现完成后登记并接线断言
- `cargo fmt` + `cargo clippy --workspace --all-targets -- -D warnings` 零警告（rust 规则）
- 门禁：`cargo test --workspace` 全绿；verify-react GATE PASSED（豁免不新增）；Flutter dart analyze ≤ 673 warnings；docs-gate 248 blocks 0 failed；Playwright 全量 0 failure
- **设计修正（YAGNI）**：`@onPointer*` schema 移除 `dx/dy`（@drag 已覆盖增量语义，pointer 原始事件增量需跨事件快照状态、零消费者）；最终 schema `{x, y, pointerId, buttons}`（x/y = 元素局部坐标）。spec §3.1 同步修正（Task 9 一并做）
- 破坏性变更：`@onClick`/`@onMouseEnter`/`@onMouseLeave` 变编译错误——learn-omniview 迁移面 13 处（onClick 7 + onMouseEnter 3 + onMouseLeave 3），`@onChange` 13 处逐个判别（输入框→`@onInput`；select/checkbox/radio 保留 `@onChange`）

---

### Task 1: 语义事件分级常量 + 旧名映射表（core 概念层）

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/event_names.rs`（新增常量 + 映射函数）
- Test: 同文件 `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `pub const SEMANTIC_EVENT_NAMES: &[&str]`（14 个语义事件名，on 前缀 camelCase 拼写）；`pub fn legacy_spelling(semantic: &str) -> Option<&'static str>`（语义事件 → 被替换的旧拼写；无旧名的返回 None，如 onInput/onKeyDown）；`pub fn is_semantic_event(name: &str) -> bool`
- Consumes: 无（新概念层，Task 2/3/9 消费）

- [ ] **Step 1: 写失败测试**（event_names.rs tests 追加）

```rust
/// 语义事件全集——Task 2 硬诊断与 Task 9 登记册锚点。
/// press/input/change/submit/key(down/up)/pointer(down/move/up)/scroll/focus/blur/enter/leave
const SEMANTIC_NAMES: &[&str] = &[
    "onPress", "onInput", "onChange", "onSubmit", "onKeyDown", "onKeyUp",
    "onPointerDown", "onPointerMove", "onPointerUp",
    "onScroll", "onFocus", "onBlur", "onEnter", "onLeave",
];

#[test]
fn semantic_names_match_design_contract() {
    for n in SEMANTIC_NAMES {
        assert!(SEMANTIC_EVENT_NAMES.contains(n), "{n} 必须是语义事件");
    }
    assert_eq!(SEMANTIC_EVENT_NAMES.len(), 14, "首批 14 个事件名/9 语义族");
}

#[test]
fn legacy_spelling_maps_old_names() {
    assert_eq!(legacy_spelling("onPress"), Some("onClick"));
    assert_eq!(legacy_spelling("onEnter"), Some("onMouseEnter"));
    assert_eq!(legacy_spelling("onLeave"), Some("onMouseLeave"));
    assert_eq!(legacy_spelling("onInput"), None, "onInput 无旧名");
    assert_eq!(legacy_spelling("onKeyDown"), None);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core semantic_event_names -- --nocapture`
Expected: FAIL（`SEMANTIC_EVENT_NAMES`/`legacy_spelling` 未定义）

- [ ] **Step 3: 最小实现**（event_names.rs，`canonical_event_name` 下方追加）

```rust
/// 首批语义事件（spec 2026-08-08 §3.1）——语义化即切换，旧名由
/// `legacy_spelling` 映射到硬诊断迁移提示。新增语义事件必须在此登记，
/// 且同时更新 Task 9 的 semantic-universe 域 3 登记册（门禁双向断言）。
pub const SEMANTIC_EVENT_NAMES: &[&str] = &[
    "onPress", "onInput", "onChange", "onSubmit", "onKeyDown", "onKeyUp",
    "onPointerDown", "onPointerMove", "onPointerUp",
    "onScroll", "onFocus", "onBlur", "onEnter", "onLeave",
];

/// 语义事件 → 被取代的旧 DOM 拼写；无旧名（原名即语义名）返回 None。
/// 旧名在语义验证阶段产生硬错误 + 「use @onX instead」迁移提示。
#[must_use]
pub fn legacy_spelling(semantic: &str) -> Option<&'static str> {
    match semantic {
        "onPress" => Some("onClick"),
        "onEnter" => Some("onMouseEnter"),
        "onLeave" => Some("onMouseLeave"),
        _ => None,
    }
}

/// 语义事件分级查询——canonical 表合法但非语义的事件名维持透传。
#[must_use]
pub fn is_semantic_event(name: &str) -> bool {
    SEMANTIC_EVENT_NAMES.contains(&name)
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang-core semantic_event_names`
Expected: PASS（2 tests）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/event_names.rs
git commit -m "feat(semantic): 语义事件分级常量表 + 旧名映射（14 事件名/9 语义族，Task 1）"
```

---

### Task 2: 旧名硬诊断 + 迁移提示（validate_event_names 升级）

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_event_names.rs`（`walk_view_tree` 的 `canonical_event_name(name).is_none()` 检查之后追加旧名检查）
- Test: 同文件追加（或 `semantic/tests` 目录既有模式）

**Interfaces:**
- Consumes: Task 1 的 `legacy_spelling` / `is_semantic_event`
- Produces: 诊断 `DiagnosticKind::E0006`（复用既有 code），severity Error（新语义错误通道；检查 E0006 现有用法，若 E0006 为 warning 语义则复用 validate_drag 的硬错误诊断通道——实施时以 `Severity::Error` + 中文消息为准）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn legacy_event_names_hard_error_with_migration_hint() {
    // @onClick={fn} → error，消息含「use @onPress」
    let diags = validate_program_with_view(&[("onClick", "fn")]);
    assert!(diags.iter().any(|d| d.severity == Severity::Error
        && d.message.contains("onClick") && d.message.contains("onPress")));
}

#[test]
fn semantic_event_names_pass_validation() {
    // 14 个语义事件名全部通过（无诊断）
    let diags = validate_program_with_view(SEMANTIC_EVENT_NAMES);
    assert!(diags.is_empty());
}

#[test]
fn custom_events_exempt_from_legacy_check() {
    // 组件自定义事件（event onX）不受旧名检查影响
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core legacy_event_names_hard_error`
Expected: FAIL（旧名仍合法，无诊断）

- [ ] **Step 3: 最小实现**（validate_event_names.rs `walk_view_tree` Element 分支）

```rust
// 既有：canonical_event_name(name).is_none() → warning（未知事件名）
// 追加（语义事件分级，原则 8）：旧名 → 硬错误 + 迁移提示。
// 命中判定：某语义事件 s 的 legacy_spelling 与用户拼写 canonical 同值——
// "onClick" 与 "click" 的 canonical 都是 "click"，裸拼写同样命中。
for s in SEMANTIC_EVENT_NAMES {
    if let Some(legacy) = legacy_spelling(s) {
        if canonical_event_name(legacy) == canonical_event_name(name) {
            diagnostics.push(error_diag(
                component,
                el.source_span,
                format!("事件名 \"@{name}\" 已弃用——使用 \"@{s}\""),
            ));
        }
    }
}
```

`error_diag` 照抄 `warning_diag` 模式（validate_event_names.rs 底部），仅 `severity: Severity::Error`（既有 E0006 通道，中文消息）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang-core legacy_event_names` + `cargo test -p omnilang-core semantic_event_names_pass`
Expected: PASS

- [ ] **Step 5: 全量回归 + Commit**

Run: `cargo test -p omnilang-core`（既有 1000+ tests，注意既有 fixture 含 @onClick 的需先修——若失败则检查 `fixtures_shared` 用点，属预期破坏性变更）

```bash
git add crates/omnilang-core/src/semantic/validate_event_names.rs
git commit -m "feat(semantic): 旧事件名硬诊断 + 迁移提示——@onClick→@onPress/@onMouseEnter→@onEnter（Task 2）"
```

---

### Task 3: @onEnter/@onLeave 别名接入 canonical 表

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/event_names.rs`（canonical 表 + SUGGESTIONS + IR_EVENT_NAMES + ALIAS_GROUPS）
- Test: 同文件

**Interfaces:**
- Consumes: 无
- Produces: `canonical_event_name("onEnter") == Some("mouseenter")`、`"onLeave" == "mouseleave"`（codegen 后端零改动——React `to_react_event`/Flutter `event_to_flutter_callback` 已按 canonical 接线）

- [ ] **Step 1: 写失败测试**（ALIAS_GROUPS 追加，测试强制表同步）

```rust
const ALIAS_GROUPS: &[&[&str]] = &[
    // ... 既有
    &["onEnter", "enter", "onMouseEnter", "mouseenter"],
    &["onLeave", "leave", "onMouseLeave", "mouseleave"],
];
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core alias_groups_resolve_to_single_canonical`
Expected: FAIL（"onEnter" → None 触发 panic）

- [ ] **Step 3: 最小实现**

```rust
// canonical_event_name match 追加（mouse 族）
"enter" | "mouseenter" => Some("mouseenter"),
"leave" | "mouseleave" => Some("mouseleave"),
// EVENT_NAME_SUGGESTIONS 追加 onEnter/onLeave（若 IR_EVENT_NAMES 增长则一并更新）
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang-core alias_groups` + `cargo test -p omnilang-core every_ir_event`
Expected: PASS（IR_EVENT_NAMES 未新增事件则既有测试足够；若 added 则同步列表）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/event_names.rs
git commit -m "feat(semantic): @onEnter/@onLeave 别名接入 canonical 表（enter→mouseenter 族，Task 3）"
```

---

### Task 4: PointerEvent schema 扩展（pointerId/buttons）+ 坐标基准元素局部化

**Files:**
- Modify: `crates/omnilang-core/src/ir/events.rs`（`event_data_schema` PointerEvent 分支）
- Modify: `crates/omnilang-core/src/codegen/event.rs`（`field_extract_js` PointerEvent 分支——x/y 改 offsetX/offsetY，+pointerId/buttons）
- Modify: `omnilang/src/codegen/react/utils.rs`（`field_extract_react` PointerEvent 分支——x/y 改 `e.nativeEvent.offsetX`，+pointerId/buttons）
- Test: 三处各自 tests + 生成产物断言

**Interfaces:**
- Consumes: 无
- Produces: PointerEvent schema = `x,y(元素局部 Float),button,pressure,tiltX,tiltY,pointerType,pointerId(Int),buttons(Int),alt,ctrl,shift,meta`；OmniView 提取 `raw.offsetX/offsetY/pointerId/buttons`；React 提取 `raw.nativeEvent.offsetX` 等
- **破坏性变更注记**：x/y 从视口坐标（clientX/Y）改为元素局部坐标（offsetX/Y）——既有 `e.x` 消费者语义变化（learn-omniview @onPointerMove 用点需核对，属 D1 坐标基准既定变更，Task 8 一并处理）

- [ ] **Step 1: 写失败测试**（events.rs tests + field_extract_js tests）

```rust
#[test]
fn pointer_schema_has_semantic_fields() {
    let schema = event_data_schema(&EventDataKind::PointerEvent);
    let names: Vec<&str> = schema.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"pointerId"));
    assert!(names.contains(&"buttons"));
    assert!(!names.contains(&"dx"), "dx/dy 已 YAGNI 移除（@drag 覆盖增量）");
}
// field_extract_js：x → "{raw}.offsetX"（元素局部坐标 D1）
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core pointer_schema`
Expected: FAIL（schema 无 pointerId）

- [ ] **Step 3: 最小实现**

```rust
// events.rs PointerEvent 分支追加（保留既有字段）
f("pointerId", EventFieldType::Int),
f("buttons", EventFieldType::Int),

// event.rs / react/utils.rs PointerEvent 分支：
(EventDataKind::PointerEvent, "x") => (format!("{}.offsetX", raw), "0".into()),
(EventDataKind::PointerEvent, "y") => (format!("{}.offsetY", raw), "0".into()),
(EventDataKind::PointerEvent, "pointerId") => (format!("{}.pointerId", raw), "0".into()),
(EventDataKind::PointerEvent, "buttons") => (format!("{}.buttons", raw), "0".into()),
// React 侧：raw 是合成事件 e —— (e.nativeEvent.offsetX, 0)
```

**V5 消息增强（spec §3.5）**：`invalid_event_field.rs` 消息补迁移提示——DOM 形状字段名（clientX/ctrlKey/…) 报错时附加「可改用 e.x（元素局部坐标）/e.ctrl」；匹配方式：schema 内语义字段与 DOM 字段名的对照小表（x↔clientX、y↔clientY、ctrl↔ctrlKey、shift↔shiftKey、alt↔altKey、meta↔metaKey）。

- [ ] **Step 4: 运行确认通过 + tsc 验证**

Run: `cargo test -p omnilang-core pointer` + `cargo test -p omnilang --features codegen-react`（React 产物快照断言）
Expected: PASS；`scripts/verify-react.sh` 中新增产物 fixture 的 tsc 通过（`offsetX` 在 lib.dom 的 PointerEvent 上存在）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/events.rs crates/omnilang-core/src/codegen/event.rs omnilang/src/codegen/react/utils.rs
git commit -m "feat(events): PointerEvent schema +pointerId/buttons + 坐标基准元素局部（offsetX/Y，Task 4）"
```

---

### Task 5: KeyEvent key 枚举验证（语义 pass）

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_semantic_keys.rs`
- Modify: `crates/omnilang-core/src/semantic/mod.rs`（pass 注册）
- Test: 新文件 tests

**Interfaces:**
- Consumes: `event_data_schema(KeyEvent)`（既有）+ 表达式遍历（`collect_ref_usage` 或 handler body 扫描模式，@drag 的 `used_fields` 先例）
- Produces: `pub fn is_valid_semantic_key(k: &str) -> bool` + 语义键近邻建议（复用 `nearest_event_suggestion` 的 score 模式或简单包含表）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn semantic_key_validation() {
    assert!(is_valid_semantic_key("Enter"));
    assert!(is_valid_semantic_key("Escape"));
    assert!(is_valid_semantic_key("ArrowUp"));
    assert!(!is_valid_semantic_key("enter"), "大小写敏感——键名规范");
    assert!(!is_valid_semantic_key("Entr"));
}
// 集成：@onKeyDown handler 内 e.key == "entr" → error + 建议「Enter」
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core semantic_key`
Expected: FAIL（`is_valid_semantic_key` 未定义）

- [ ] **Step 3: 最小实现**

```rust
/// 语义键名枚举（D2）——e.key 字符串本身跨平台一致，价值在验证与建议。
/// v0 表（覆盖 demo 用点 + 导航/表单高频键；扩展按 lib.dom KeyboardEvent.key 规范）：
const SEMANTIC_KEYS: &[&str] = &[
    "Enter", "Escape", "Backspace", "Delete", "Tab", "CapsLock",
    "Shift", "Control", "Alt", "Meta", "Space",
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
    "Home", "End", "PageUp", "PageDown",
    // 单字母 A-Z / 单数字 0-9 经谓词接受（keys.len() == 1 && is_alphanumeric）
];
pub fn is_valid_semantic_key(k: &str) -> bool {
    SEMANTIC_KEYS.contains(&k)
        || (k.len() == 1 && k.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()))
}

pub fn nearest_key_suggestion(bad: &str) -> Option<&'static str> {
    // 复用 nearest_event_suggestion 的前缀权重 score 模式，对 SEMANTIC_KEYS 求最近
    SEMANTIC_KEYS.iter().copied().min_by_key(|k| {
        let lcp = common_prefix_len(&bad.to_ascii_lowercase(), &k.to_ascii_lowercase());
        bad.len() + k.len() - 2 * lcp
    })
}
```

pass 逻辑：walk handler body，找 `e.key` 比较字面量（== "X"）的常量，非枚举值 → Error + 建议。字面量之外的比较（变量）跳过（v0 范围，V6 警告项）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p omnilang-core validate_semantic_keys` + `cargo test -p omnilang-core`（全量回归）
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_semantic_keys.rs crates/omnilang-core/src/semantic/mod.rs
git commit -m "feat(semantic): KeyEvent key 语义键枚举验证 + 近邻建议（D2，Task 5）"
```

---

### Task 6: ScrollEvent schema +progress 字段

**Files:**
- Modify: `crates/omnilang-core/src/ir/events.rs`（ScrollEvent 分支 +progress）
- Modify: `crates/omnilang-core/src/codegen/event.rs`（progress 提取：`scrollTop/(scrollHeight-clientHeight)` 或 0 兜底）
- Modify: `omnilang/src/codegen/react/utils.rs`（field_extract_react progress）
- Modify: `omnilang/src/codegen/flutter/view.rs`（事件字段区 progress——Flutter 无统一 scrollTop 概念，用 `ScrollMetrics` 时实施定位；若不可达则 Flutter 诊断 + 诚实降级注记）
- Test: 各端产物断言

**Interfaces:**
- Consumes: 无
- Produces: ScrollEvent schema = `x,y,progress(Float)`；progress 语义 = 滚动进度 0..1（顶部 0 / 底部 1；不可测时 0 兜底）

- [ ] **Step 1: 写失败测试**（schema + field_extract_js）

```rust
#[test]
fn scroll_schema_has_progress() {
    let schema = event_data_schema(&EventDataKind::ScrollEvent);
    assert!(schema.iter().any(|f| f.name == "progress"));
}
```

- [ ] **Step 2: 运行确认失败 → Step 3: 最小实现**（与 Task 4 同模式）

```rust
// events.rs：f("progress", EventFieldType::Float)
// event.rs：progress 提取——
//   "({target?.scrollHeight - {target?.clientHeight} > 0 ? {target?.scrollTop / ({target?.scrollHeight - {target?.clientHeight}) : 0)"
// react：同式（e.currentTarget）
// flutter：ScrollNotification.metrics（extentBefore/(maxScrollExtent)），不可达则诊断
```

- [ ] **Step 4: 验证 + Commit**（同 Task 4 模式，flutter 分支带 dart analyze 验证）

```bash
git commit -m "feat(events): ScrollEvent schema +progress 滚动进度字段（Task 6）"
```

---

### Task 7: Flutter 端事件字段映射补齐

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（事件字段提取区 ~2124-2259：PointerEvent pointerId/buttons/localPosition 映射、ScrollEvent progress）
- Test: `omnilang/src/codegen/flutter/tests.rs` 产物断言 + `generated_code_validation` 门禁

**Interfaces:**
- Consumes: Task 4/6 的 schema
- Produces: Flutter 生成事件对象：PointerMoveEvent `localPosition.dx/dy` → x/y、`pointerId`/`buttons` 直取；ScrollNotification progress 按 Task 6

- [ ] **Step 1: 写失败测试**（flutter/tests.rs）

```rust
#[test]
fn flutter_pointer_event_extracts_pointer_id_and_buttons() {
    // @onPointerMove 的 handler 读 e.pointerId + e.buttons → 产物含
    // pointerId/buttons 提取（PointerMoveEvent 直取）与 localPosition 换算
    let out = compile_flutter(/* PointerMove handler 读 e.pointerId, e.x */);
    assert!(out.contains("pointerId"));
    assert!(out.contains("buttons"));
    assert!(out.contains("localPosition"), "x/y 由 localPosition 换算（元素局部坐标）");
}
```

- [ ] **Step 2: 运行确认失败 → Step 3: 最小实现**（flutter/view.rs 事件字段匹配补 arm）

- [ ] **Step 4: 验证**

Run: `cargo test -p omnilang --features codegen-flutter` + `dart analyze` 门禁（`scripts/verify-flutter.sh` 或 `generated_code_validation` 等价入口，按既有 CI 命令）
Expected: PASS + 0 error（warnings ≤ 673 基线）

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(codegen-flutter): 事件字段映射补齐——pointerId/buttons/localPosition/progress（Task 7）"
```

---

### Task 8: demo 迁移（learn-omniview 事件用点一步到位）

**Files:**
- Modify: `test/projects/learn-omniview/src/` 下含旧事件名的 .omni（grep 定位：@onClick 7 处、@onMouseEnter 3、@onMouseLeave 3、@onChange 13 处判别）
- Test: 三目标编译 + Playwright 全量

**Interfaces:**
- Consumes: Task 2 的硬诊断（迁移后必须零旧名）
- Produces: 零旧名语料

- [ ] **Step 1: 定位用点**

Run: `grep -rn "@onClick\|@onMouseEnter\|@onMouseLeave\|@onChange" test/projects/learn-omniview/src --include="*.omni"`

- [ ] **Step 2: 迁移**

- `@onClick` → `@onPress`（7 处）
- `@onMouseEnter` → `@onEnter`、`@onMouseLeave` → `@onLeave`
- `@onChange` 逐个判别：`<input>` 上 → `@onInput`；`<select>`/`<input type=checkbox|radio>` 上 → 保留 `@onChange`

- [ ] **Step 3: 验证零残留 + 三目标编译**

Run: `grep -rn "@onClick\|@onMouseEnter\|@onMouseLeave" test/projects/learn-omniview/src --include="*.omni"`（Expected: 0 命中）
Run: `cargo build --release -p omnic && omnic build --target react|flutter|omniview`（按项目既有 omnic 用法）
Expected: 三目标编译通过

- [ ] **Step 4: 浏览器全量**

Run: Playwright 全量套件（项目既有入口）
Expected: 0 failures（46+1 基线内，事件行为断言确认 press/enter/leave/input 工作）

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(demo): learn-omniview 事件用点一步到位迁移——onClick→onPress/onMouseEnter→onEnter/onChange 判别（Task 8）"
```

---

### Task 9: 登记册 + 文档 + spec 同步 + 注入语料

**Files:**
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 3 事件行重分类：14 语义化 + 40 透传 + 状态列）
- Modify: `docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md`（§3.1 schema 修正：PointerEvent 移除 dx/dy、+pointerId/buttons；§3.3 切换表更新）
- Modify: `docs/omniview/07-events-and-bindings.md`（语义事件表 + 字段 schema + 迁移对照表）
- Modify: `scripts/verify-semantic-universe.py`（若域 3 状态机需要新值「语义化/透传」）
- Test: verify 双脚本 + docs-gate

**Interfaces:**
- Consumes: Task 1 的 `SEMANTIC_EVENT_NAMES`（脚本断言锚点 = parser 常量，ADR-074 先例）
- Produces: 登记册零失实 + docs-gate 0 failed

- [ ] **Step 1: 域 3 登记册重分类**

semantic-universe 域 3 事件行：14 个语义事件标记「语义化」（状态 + schema 列），其余 40 个标记「透传」。verify-semantic-universe.py 断言锚点指向 `SEMANTIC_EVENT_NAMES`（`rust_str_array` 提取模式，ADR-074 先例）。

- [ ] **Step 2: spec §3.1/§3.3 核对**（dx/dy 移除 + pointerId/buttons 已随计划提交同步；核对切换表与 Task 2 实现一致）

- [ ] **Step 3: docs/omniview/07 更新**（语义事件表 + 迁移对照 + key 枚举节 + 坐标基准说明）

- [ ] **Step 4: 注入语料扩张**（注入面：新 schema 字段名/旧名切换消息——855 元组矩阵补插入点，模式缺失断言）

- [ ] **Step 5: 验证 + Commit**

Run: `python scripts/verify-semantic-universe.py`（0 problem）+ `python scripts/verify-docs.py --check`（248 blocks 0 failed）+ `cargo test --workspace`（全绿）+ 注入语料全绿

```bash
git commit -m "docs(universe): 域3 事件行重分类（14 语义化/40 透传）+ spec/demo 文档同步 + 注入语料扩张（Task 9）"
```

---

## 收口验证（全部任务完成后）

Run: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` + verify-react GATE + Flutter dart analyze + docs-gate + verify 双脚本 + Playwright 全量
Expected: 全绿；learn-omniview 零旧事件名；spec §10 验收标准第 1 条达成（阶段 1 交付）
