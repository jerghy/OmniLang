# 语义批次 5 长尾四归组 实施计划（2026-08-09-semantic-batch5-longtail.md）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实施本计划。任务用 checkbox（`- [ ]`）语法跟踪。

**Goal:** 实施语义批次 5 长尾四归组（交互/i18n/媒体/生命周期）12 项能力，V1-V17 诊断全落地，三目标 codegen + demo 迁移 + 五层门禁绿，semantic-universe 域 14-17 登记入册。

**Architecture:** 共享语法基建（T2 意图属性解析框架 + ElementIntentIR）先行；四归组子流（交互 T3-T7 / i18n T8-T10 / 媒体 T11-T12 / 生命周期 T13-T14）相互独立可并行；登记先行（T1）为全部语法任务的前置；T15 语料与 docs 终态、T16 五层门禁终验收口。

**Tech Stack:** Rust（parser/IR/semantic/codegen，omnilang-core + omnilang）、React 19 + dnd-kit/@use-gesture/react-intl、Flutter（ReorderableListView/GestureDetector/Shortcuts/Clipboard——全 SDK 内置，file_picker 注记）、OmniView、Playwright、verify-semantic-universe.py / verify-vocabulary.py。

## Global Constraints

**上游裁决（P1-P6 已定案，计划内直接执行）：** P1 @copy 无成功反馈 toast（YAGNI）；P2 @key 极简两层（键位→handler，Flutter 编译器内部映射 Intent，用户不写三层）；P3 @t 消息源 v0 内联字面量（ARB/键表提取为编译器职责）；P4 权限模型 = 组件级 `@permission` 声明（编译器隐式生成 + 静态清单注入属工程面注记）；P5 硬化范围仅 @onLongpress + @onCopy（cut/paste 等不动）；P6 canvas Flutter paint() 重放 + Playwright 像素级断言（toDataURL 像素非空）。

**Validation 消息 verbatim（spec §3，实现必须逐字）：**

| # | 级别 | 消息 |
|---|---|---|
| V1 | 硬 | 「@reorder 只能用于 @for 容器上」 |
| V2 | 硬 | 「@reorder 的 handler 必须是组件函数名（禁止内联箭头/调用）」 |
| V3 | 硬 | 「@reorder 与 @drag 不能在同一元素（重排与位移二选一）」 |
| V4 | 硬 | 「@swipe/@longPress/@pinch 的 handler 必须是组件函数名」 |
| V5 | 硬（迁移） | 「@onLongpress 已弃用——使用 @longPress 意图指令」 |
| V6 | 硬（迁移） | 「@onCopy 已弃用——使用 @copy 意图指令」 |
| V7 | 硬 | 「@key 键位串语法错误：修饰符须为 ctrl/alt/shift/meta，键名不在语义键名枚举」 |
| V8 | 硬 | 「@key 键位串必须含至少一个修饰符（裸键请用 @onKeydown）」 |
| V9 | 硬 | 「@t 消息语法错误：花括号未闭合/占位符名缺失」 |
| V10 | 硬 | 「@t 消息不支持 offset/#/date/time——参数面留生态」 |
| V11 | 警告 | 「@t 参数 {n} 在消息中未使用」 / 「消息占位符 {n} 缺少参数」 |
| V12 | 硬 | 「@locale 的值必须是裸信号引用」 |
| V13 | 硬 | 「timer/interval/subscribe 只能在 @effect 块内使用」 |
| V14 | 硬 | 「canvas ctx 方法不在白名单：{name}」 |
| V15 | 硬 | 「@capture 需要 @permission 声明」 |
| V16 | 硬 | 「@t 消息嵌套深度超过 3」 |
| V17 | 警告（Flutter） | 「@capture="audio" Flutter 端无官方实现——未映射」 |

**V3 实施定义（AST 形态定案）：** @for 是独立 ViewNode（`ViewNode::ForDirective`），@drag 挂 Element——「同元素」字面不存在。实施定义：**Element 带 @drag/@dragEnd 且其直接子节点含带 reorder= 的 ForDirective → V3 硬错误**。v0 不放开「reorder 列表内项级 @drag」（§7 风险注记，语料出现时按 ADR 流程重评）。

**登记先行机制（ADR-074，T1 定稿）：**
- `verify-semantic-universe.py`：STATUSES 闭集 {支持, 部分, 未支持}——域 14-17 新词条状态用「未支持」暂态（「待实施」不可用）；结构性循环 `range(1,10)` 硬编码**不可扩**（域 10-13 尚未登记会 fail）——域 14-17 断言用手写 `assert_set_*`/`assert_matrix_*` 函数 + ENABLED 布尔守卫（默认 False），随对应语法任务同 commit 置 True 启用（域 14→T4 末、域 15→T9 末、域 16→T12 末、域 17→T13 末）；白名单表 well-formed 占位（未支持 ∩ whitelist = ∅ 检查通过）。
- `verify-vocabulary.py`：DOMAIN_TITLES 现覆盖 1..6，`vocab(n)` 对缺失域返回 None → 断言优雅跳过 → 新域 7-10 惰性登记；借词检查循环（固定 (n,label) 列表）扩展排除词：swipeleft/swiperight、clipboard、keydown/keyup/keypress。
- universe 新域登记（spec §5）：域 14 交互意图（reorder/swipe/longPress/pinch/copy/key）、域 15 国际化（@t/@locale/formatDate）、域 16 媒体（canvas ctx 门类/@filePick/@capture/@permission）、域 17 生命周期（@mount/@unmount/@effect/@store + timer/interval/subscribe 子词汇，事实源 = 15-grammar-view §1.11 lifecycle_hook 产生式交叉断言）。

**测试模板（verbatim，全部集成测试用）：**
- parser 级：`use omnilang_core::error::ParseError; use omnilang_core::lexer::tokenize; use omnilang_core::parser::{self, ast::Program};` → `let (program, errors): (Program, Vec<ParseError>) = parser::parse(tokens, &source);` → `errors.iter().any(|e| e.message.contains("..."))`（正例断言 errors 为空 + `matches!(...)` 结构断言）。
- semantic 级（cross_file_const_test.rs 模板）：`use omnilang_core::analyse::analyse_multi; use rustc_hash::FxHashMap;` → `let mut sources = FxHashMap::default(); sources.insert("./x.omni".to_string(), r##"..."##.to_string());` → `let mut path_mapping = FxHashMap::default(); path_mapping.insert("./x.omni".to_string(), "./x.omni".to_string());` → `let result = analyse_multi(&sources, &path_mapping);` → `result.diagnostics.iter().filter(|d| d.severity == omnilang_core::error::Severity::Error).collect()` → `assert!(errors.is_empty(), "should have no errors, got: {:?}", errors)`；诊断断言 `errors.iter().any(|d| d.message.contains("V-n 消息"))`。
- codegen 级（injection.rs:565 compile_target helper）：`fn compile_target(source: &str, target: Target) -> crate::InternalCompileResult { let mut sources = FxHashMap::default(); sources.insert("input.omni".to_string(), source.to_string()); let pm = crate::path_mapping::derive_path_mapping(&sources); crate::compile_multi_internal(sources, target, pm, false, false, None) }`；内容断言 `result.files.values().any(|code| code.contains("..."))`；诊断断言 `result.warnings.iter().any(|w| w.message.contains("..."))` / `result.errors`。
- 新 codegen 测试文件须在 `omnilang/src/codegen/tests_regression/mod.rs` 加 `mod` 声明。

**门禁命令（每任务按需组合）：** `cargo test --workspace`｜`cargo clippy --workspace --all-targets -- -D warnings`｜IR 类型变更后 `cargo test export_bindings --lib -p omnilang`（ts-rs freshness）+ `git diff --exit-code poiesis/src/commands/__generated__/` 干净｜`python scripts/verify-semantic-universe.py` + `python scripts/verify-vocabulary.py`｜`python scripts/verify-docs.py --check`（先 `cargo build -p omnic`）｜tsc/dart analyze 经 `cargo test --test generated_code_validation`（fixtures 自动枚举，工具链缺失 SKIP；dart pub get 失败即红）｜Playwright：在 `test/projects/learn-omniview/build/react-test-app/` 目录 `npx playwright test tests/batch5-longtail.spec.ts`。

**键名单源：** @key key_name 复用 `crate::semantic::validate_semantic_keys::SEMANTIC_KEYS` pub const（域 3 keydown 语义键名枚举），T2 实施时确认无循环依赖，如有则抽到共享常量模块。

**编译产物依赖注入：** React 目标产物引用 dnd-kit/@use-gesture/react-intl 时，同步注入 `react-test-app/package.json` dependencies（T5/T10）；Flutter file_picker 为工程面注记（T12，不自动注入 pubspec——poiesis 产物工程承接）。

**V5/V6 硬化机制：** 既有 `legacy_spelling(s)` 与用户拼写 canonical 同值命中判定 + E0006 硬错误 + 迁移提示（validate_event_names.rs 测试先例 `legacy_event_names_hard_error_with_migration_hint` :251）。T4 将 longpress/copy 从透传态（event_names.rs :35/:79）迁入硬化——用户写 `@onLongpress`/`@onCopy` → V5/V6 硬错误 + 迁移提示「使用 @longPress/@copy 意图指令」。cut/paste 不动（P5）。

---

## 1. 背景与范围

批次 5 = 基线 §4 批次 5「长尾四归组」12 项能力（spec §1 范围表）：交互（@for reorder= / @swipe / @longPress / @pinch / @copy / @key）、i18n（@t ICU 子集 / @locale / formatDate）、媒体（canvas RefOp ctx 门类 / @filePick / @capture+@permission）、生命周期（@effect 强化 timer/interval/subscribe + @store/@mount/@effect 入册 1B 项）。语言边界项（§6）不实施仅记录。设计细节以 spec 为唯一详细级输入（本计划只引用章节号，不重复设计）。

## 2. 任务划分（16 任务）

### Task 1: 登记先行——semantic-universe 域 14-17 + vocabulary-registry 域 7-10 + verify 双脚本骨架

**Files:**
- Modify: `docs/superpowers/specs/semantic-universe.md`（新增域 14-17 章节）
- Modify: `docs/superpowers/specs/vocabulary-registry.md`（新增域 7-10 词条）
- Modify: `scripts/verify-semantic-universe.py`（域 14-17 手写断言函数 + ENABLED 守卫）
- Modify: `scripts/verify-vocabulary.py`（借词循环扩展）
- Create: `scripts/verify-batch5-registrations.py`（域 14-17 交叉断言骨架——仅在本任务建立占位与 ENABLED 机制，断言体随各域语法任务同 commit 填充；或并入既有脚本，实施者任选一，以不破坏既有脚本为约束）

**Interfaces:**
- Produces: `verify-semantic-universe.py` 中 `assert_set_d14`/`assert_matrix_d14`（ENABLED_D14 = False）等每域一对函数；`verify-vocabulary.py` 扩展后借词排除列表；universe 域 14-17 词条表 + vocabulary 域 7-10 行（状态=未支持）。

- [ ] **Step 1: 写失败测试（脚本断言占位）**

在 `verify-semantic-universe.py` 尾部追加（先不实现函数体）：

```python
def assert_d14(entries, whitelist):
    """域 14 交互意图：语义词汇存在性断言（随 T4 同 commit 启用）。"""
    assert ENABLED_D14, "域 14 断言未启用"
    # 词汇存在性断言（reorder 在 for_directive 解析、手势/@copy/@key 在 parse_event_prop 分派）
    raise NotImplementedError("T4 启用")

ENABLED_D14 = False
```

- [ ] **Step 2: 运行确认失败**

Run: `python scripts/verify-semantic-universe.py`
Expected: FAIL（`assert_d14` 未定义引用错误或 NotImplementedError 未触发——以「断言函数存在 + ENABLED=False 时跳过」为最终口径）

- [ ] **Step 3: 实现**

1. universe 文档追加域 14-17 章节（spec §5 条目表草案 verbatim，状态列=未支持）。
2. vocabulary 文档追加域 7-10 行（spec §5 vocabulary-registry 四字段 verbatim）。
3. 脚本实现 `assert_d14..d17` 函数骨架：`if not ENABLED: return` 早退 + 函数体占位（条目存在性断言随语法任务填充）；`ENABLED_D14..17 = False` 常量。
4. 借词循环扩展排除词：swipeleft/swiperight/clipboard/keydown/keyup/keypress。

- [ ] **Step 4: 运行确认通过**

Run: `python scripts/verify-semantic-universe.py && python scripts/verify-vocabulary.py`
Expected: PASS（新断言 ENABLED=False 跳过；既有断言不受影响）

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/semantic-universe.md docs/superpowers/specs/vocabulary-registry.md scripts/verify-semantic-universe.py scripts/verify-vocabulary.py
git commit -m "docs(registry): 批次 5 域 14-17 登记骨架 + verify 双脚本 ENABLED 守卫扩展"
```

**验证门禁：** `python scripts/verify-semantic-universe.py && python scripts/verify-vocabulary.py` 全绿；`cargo test --workspace` 不受影响。

**依赖：** 无（全批次先行任务）。

---

### Task 2: 共享意图属性解析框架——8 个元素意图 prop + ElementIntentIR

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/view.rs`（ElementProp 追加 8 变体）
- Modify: `crates/omnilang-core/src/parser/view/element.rs`（parse_event_prop 分派 + @key/@capture 双绑定 + merge 冲突检查）
- Modify: `crates/omnilang-core/src/ir/types.rs`（ElementIntentIR/KeymapBindingIR/CaptureBindingIR/CaptureModeIR + ElementIR.intents 字段）
- Modify: `crates/omnilang-core/src/ir/lower/*`（ElementIR 全量构造点补 `intents: ElementIntentIR::default()`）
- Create: `crates/omnilang-core/tests/batch5_intent_parser_test.rs`

**Interfaces:**
- Consumes: 既有 `parse_event_prop` 分派头（element.rs:665）+ @drag 特殊分支先例（:711-719）+ `ElementProp` 既有 15 变体（view.rs:166-247）。
- Produces: `ElementProp::{GestureIntent{kind, handler}, Clipboard{value}, Keymap{combo, handler}, Locale{signal}, FilePick{handler}, Capture{mode, handler}}`（GestureKind::{Swipe,LongPress,Pinch}、CaptureMode::{Camera,Audio}）；`ElementIntentIR` struct（swipe/long_press/pinch/copy/keymap/locale/file_pick/capture 字段）+ `ElementIR.intents` 字段；`@key`/`@capture` 双绑定解析（字符串字面量 + '=' + 花括号 ident）；V7/V8 键位串验证函数 `parse_key_combo(&str) -> Result<KeyCombo, String>`（修饰符 ctrl/alt/shift/meta + key_name ∈ SEMANTIC_KEYS，裸键 → V8 错误）。

- [ ] **Step 1: 写失败测试（parser 级）**

```rust
// crates/omnilang-core/tests/batch5_intent_parser_test.rs
use omnilang_core::error::ParseError;
use omnilang_core::lexer::tokenize;
use omnilang_core::parser::{self, ast::{Program, ViewNode, ElementProp}};

fn parse_view(source: &str) -> (Program, Vec<ParseError>) {
    let tokens = tokenize(source).unwrap();
    parser::parse(tokens, source)
}

#[test]
fn parses_gesture_clipboard_keymap_locale_filepick_capture_props() {
    let source = r##"
component App {
    view {
        <Card @swipe={handleSwipe} @longPress={handleLongPress} @pinch={handlePinch} />
        <Button @copy={shareLink}>复制</Button>
        <Page @key="ctrl+s"={onSave} />
        <div @locale={lang} />
        <input type="file" @filePick={handlePick} />
        <Button @capture="camera"={onShot} />
    }
}
"##;
    let (program, errors) = parse_view(source);
    assert!(errors.is_empty(), "parse errors: {:?}", errors);
    // 意图 prop 均挂到 Element 节点
    let view = &program.components[0].view;
    let props: Vec<&ElementProp> = view.iter().filter_map(|n| match n {
        ViewNode::Element(e) => Some(e.props.iter().collect::<Vec<_>>()),
        _ => None,
    }).flatten().collect();
    let names: Vec<&str> = props.iter().filter_map(|p| match p {
        ElementProp::GestureIntent { .. } => Some("gesture"),
        ElementProp::Clipboard { .. } => Some("clipboard"),
        ElementProp::Keymap { .. } => Some("keymap"),
        ElementProp::Locale { .. } => Some("locale"),
        ElementProp::FilePick { .. } => Some("filepick"),
        ElementProp::Capture { .. } => Some("capture"),
        _ => None,
    }).collect();
    assert!(names.contains(&"gesture") && names.contains(&"clipboard") && names.contains(&"keymap"));
    assert!(names.contains(&"locale") && names.contains(&"filepick") && names.contains(&"capture"));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_intent_parser_test`
Expected: FAIL（编译错误：`ElementProp::GestureIntent` 等变体不存在；`ViewNode::Element` 字段名以实际为准调整）

- [ ] **Step 3: 写第二个失败测试（@key 键位串验证）**

```rust
#[test]
fn rejects_bare_key_and_invalid_modifier_in_keymap() {
    // 裸键 → V8
    let source = r##"
component App {
    view { <Page @key="s"={onSave} /> }
}
"##;
    let (_program, errors) = parse_view(source);
    assert!(errors.iter().any(|e| e.message.contains("裸键请用 @onKeydown")),
        "expected V8 bare-key error, got: {:?}", errors);

    // 非法修饰符 → V7
    let source2 = r##"
component App {
    view { <Page @key="supper+s"={onSave} /> }
}
"##;
    let (_p2, errors2) = parse_view(source2);
    assert!(errors2.iter().any(|e| e.message.contains("修饰符须为 ctrl/alt/shift/meta")),
        "expected V7 modifier error, got: {:?}", errors2);
}
```

- [ ] **Step 4: 实现**

1. `ElementProp` 追加 8 变体（view.rs，挂既有 15 变体之后）。
2. `parse_event_prop`（element.rs:665）内、generic 事件路径之前加 @drag 式特殊分支：`@swipe/@longPress/@pinch`（ident handler）→ `GestureIntent`；`@copy=`（任意表达式，parse_expr）→ `Clipboard`；`@key=` + 键位串 + '=' + 花括号 ident → `Keymap`（combo 经 `parse_key_combo` 验证，V7/V8 消息）；`@locale=`（ident）→ `Locale`；`@filePick=` → `FilePick`；`@capture=` + 字符串字面量（"camera"/"audio"）+ '=' + 花括号 ident → `Capture`。
3. `parse_key_combo`：按 '+' 分段，首段为修饰符（须 ∈ {ctrl,alt,shift,meta}，无修饰符 → V8），末段为 key_name（须 ∈ SEMANTIC_KEYS，否则 V7）。
4. merge/冲突检查（element.rs:764-807）补新 prop 的去重逻辑（同元素同名 prop 报错，与既有 merge 机制一致）。
5. `ir/types.rs` 新增 `ElementIntentIR`/`KeymapBindingIR`/`CaptureBindingIR`/`CaptureModeIR`；`ElementIR` 加 `intents` 字段；lower/optimize 全量构造点补 `ElementIntentIR::default()`（后续任务填充）。
6. 关键：`ElementIntentIR` 及新枚举加 `#[derive(Clone, Debug, PartialEq)]` + ts-rs `#[derive(ts::TS)]`（与既有 IR 类型同规格），跑 export_bindings 保证 __generated__ 同步。

- [ ] **Step 5: 运行确认通过**

Run: `cargo test --workspace && cargo test export_bindings --lib -p omnilang && git diff --exit-code poiesis/src/commands/__generated__/`
Expected: PASS；`git status` 干净（__generated__ 无残留 diff）

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/view.rs crates/omnilang-core/src/parser/view/element.rs crates/omnilang-core/src/ir/types.rs crates/omnilang-core/tests/batch5_intent_parser_test.rs poiesis/src/commands/__generated__/
git commit -m "feat(parser): 批次 5 元素意图属性解析框架——8 prop + @key/@capture 双绑定 + V7/V8 键位串验证"
```

**验证门禁：** `cargo test --workspace`｜`cargo clippy --workspace --all-targets -- -D warnings`｜export_bindings + __generated__ 干净。

**依赖：** T1（新词汇已登记——ADT-074 登记先行）。

---

### Task 3: @for reorder 容器选项 + V1/V2/V3

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/view.rs`（ForDirective 加 `reorder: Option<String>` 字段）
- Modify: `crates/omnilang-core/src/parser/view/directives.rs`（parse_for_directive :111-188，key= :136-145 后挂 reorder= 解析）
- Modify: `crates/omnilang-core/src/semantic/validate_drag.rs`（或新建 `validate_reorder.rs`，V1/V2/V3）
- Modify: `crates/omnilang-core/src/ir/types.rs`（ListRenderingIntent :619-632 加 `reorder: Option<String>`）
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:1117` + `crates/omnilang-core/src/ir/optimize.rs:825`（构造点补 reorder 透传）
- Create: `crates/omnilang-core/tests/batch5_reorder_test.rs`

**Interfaces:**
- Consumes: `parse_for_directive` key= 解析先例（directives.rs:136-145）；`ListRenderingIntent` 既有字段；@drag 验证先例（validate_drag.rs）。
- Produces: `ForDirective.reorder: Option<String>`（BNF：`('reorder=' '{' <ident> '}')?`）；V1（普通元素上名为 reorder 的静态 prop → 硬错误）、V2（handler 非 fn 名 → 硬错误——parser 层 BNF 限 ident + semantic 层验证 ident ∈ 组件 fn 表）、V3（实施定义见 Global Constraints：Element 带 @drag/@dragEnd 且直接子节点含带 reorder= 的 ForDirective → 硬错误）；`ListRenderingIntent.reorder` 贯通 lower/optimize。

- [ ] **Step 1: 写失败测试（parser 正例 + V2 负例）**

```rust
// crates/omnilang-core/tests/batch5_reorder_test.rs
use omnilang_core::error::ParseError;
use omnilang_core::lexer::tokenize;
use omnilang_core::parser::{self, ast::ViewNode};

fn parse_view(source: &str) -> (Vec<ViewNode>, Vec<ParseError>) {
    let tokens = tokenize(source).unwrap();
    let (program, errors) = parser::parse(tokens, source);
    (program.components[0].view.clone(), errors)
}

#[test]
fn parses_reorder_option_on_for_directive() {
    let source = r##"
component App {
    view {
        @for item in visibleItems key={item.id} reorder={handleMove} {
            <TodoItem text={item.text} />
        }
    }
}
"##;
    let (view, errors) = parse_view(source);
    assert!(errors.is_empty(), "parse errors: {:?}", errors);
    match &view[0] {
        ViewNode::ForDirective(f) => {
            assert_eq!(f.reorder.as_deref(), Some("handleMove"));
        }
        other => panic!("expected ForDirective, got {:?}", other),
    }
}

#[test]
fn rejects_inline_arrow_or_call_as_reorder_handler() {
    // V2：reorder 值必须是 ident——内联箭头 → parse error
    let source = r##"
component App {
    view {
        @for item in items reorder={(a, b) => moveIt(a, b)} {
            <div>{item}</div>
        }
    }
}
"##;
    let (_view, errors) = parse_view(source);
    assert!(errors.iter().any(|e| e.message.contains("组件函数名")),
        "expected V2 parse error, got: {:?}", errors);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_reorder_test`
Expected: FAIL（`ForDirective` 无 reorder 字段；reorder= 按未知属性报错）

- [ ] **Step 3: 写第二个失败测试（semantic 级 V1/V2/V3）**

```rust
// 同文件追加，使用 analyse_multi 模板
use omnilang_core::analyse::analyse_multi;
use rustc_hash::FxHashMap;
use omnilang_core::error::Severity;

fn semantic_errors(source: &str) -> Vec<String> {
    let mut sources = FxHashMap::default();
    sources.insert("./x.omni".to_string(), source.to_string());
    let mut path_mapping = FxHashMap::default();
    path_mapping.insert("./x.omni".to_string(), "./x.omni".to_string());
    let result = analyse_multi(&sources, &path_mapping);
    result.diagnostics.iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn reorder_handler_must_be_component_fn() {
    // V2 semantic 层：ident 不在组件 fn 表
    let source = r##"
component App {
    view {
        @for item in items key={item.id} reorder={noSuchFn} {
            <div>{item}</div>
        }
    }
}
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("@reorder 的 handler 必须是组件函数名")),
        "expected V2, got: {:?}", errors);
}

#[test]
fn reorder_and_drag_conflict_on_same_container() {
    // V3：元素带 @drag 且直接子节点含带 reorder= 的 @for
    let source = r##"
component App {
    view {
        <div @drag={onDrag}>
            @for item in items key={item.id} reorder={handleMove} {
                <div>{item}</div>
            }
        </div>
    }
}
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("@reorder 与 @drag 不能在同一元素")),
        "expected V3, got: {:?}", errors);
}
```

- [ ] **Step 4: 实现**

1. `ForDirective.reorder` 字段 + directives.rs 解析（ident-only，箭头/调用 → V2 parse error；handler 名记录字符串）。
2. 新建 `semantic/validate_reorder.rs`（或并入 validate_drag.rs，实施者以文件规模 <800 行为准）：V1（`ElementProp::Static{name:"reorder"}` 或元素未知 prop "reorder" → V1）、V2（reorder ident ∉ 组件 fn 表 → V2）、V3（walk 元素子节点，元素带 drag_binding 且直接子节点含 reorder ForDirective → V3）；semantic/mod.rs 挂新验证器。
3. `ListRenderingIntent.reorder` + lower/optimize 构造点透传。

- [ ] **Step 5: 运行确认通过**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo test export_bindings --lib -p omnilang`
Expected: PASS；__generated__ 无残留 diff

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/view.rs crates/omnilang-core/src/parser/view/directives.rs crates/omnilang-core/src/semantic/validate_reorder.rs crates/omnilang-core/src/semantic/mod.rs crates/omnilang-core/src/ir/types.rs crates/omnilang-core/tests/batch5_reorder_test.rs
git commit -m "feat(semantic): @for reorder= 容器选项——V1/V2/V3 + ListRenderingIntent.reorder 贯通"
```

**验证门禁：** `cargo test --workspace`｜clippy -D warnings｜export_bindings 同步。

**依赖：** T2（意图属性框架——reorder 走 for_directive 独立解析，实际仅依赖 T1 登记；标注：T3 与 T8/T11/T13 无依赖，可与 T2 并行实施前提是 T1 完成）。

---

### Task 4: 交互语义验证——V4/V5/V6 + 域 14 断言启用

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_drag.rs`（或新 `validate_gestures.rs`，V4）
- Modify: `crates/omnilang-core/src/semantic/validate_event_names.rs`（V5/V6 硬化：longpress/copy 迁移硬诊断）
- Modify: `crates/omnilang-core/src/codegen/shared/event_names.rs`（:35 longpress / :79 copy 条目移除或改硬化标记）
- Modify: `crates/omnilang-core/src/semantic/mod.rs`（挂验证器）
- Modify: `scripts/verify-semantic-universe.py`（ENABLED_D14 = True + assert_d14 断言体实现）
- Create: `crates/omnilang-core/tests/batch5_intent_semantic_test.rs`

**Interfaces:**
- Consumes: `ElementProp::{GestureIntent, Clipboard, Keymap}`（T2）；legacy_spelling 硬化机制（validate_event_names.rs，E0006 先例 :251）。
- Produces: V4（手势 handler 非 fn 名 → 硬错误）、V5（`@onLongpress` → 硬错误 + 「使用 @longPress 意图指令」）、V6（`@onCopy` → 硬错误 + 「使用 @copy 意图指令」）；域 14 断言启用（universe 域 14 状态翻「部分」——reorder/swipe/longPress/pinch/copy/key 词汇存在性断言：reorder 在 for_directive 解析、手势/@copy/@key 在 parse_event_prop 分派——脚本内以 grep/解析既有源文件路径为事实源）。

- [ ] **Step 1: 写失败测试（semantic 级）**

```rust
// crates/omnilang-core/tests/batch5_intent_semantic_test.rs（semantic_errors helper 同 T3 模板）
#[test]
fn gesture_handlers_must_be_component_fns() {
    // V4
    let source = r##"
component App {
    view { <Card @swipe={noSuchFn} /> }
}
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("@swipe/@longPress/@pinch 的 handler 必须是组件函数名")),
        "expected V4, got: {:?}", errors);
}

#[test]
fn legacy_onLongpress_and_onCopy_are_hard_errors_with_migration_hint() {
    // V5/V6
    let source = r##"
component App {
    view { <Button @onLongpress={p} @onCopy={c} /> }
}
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("@onLongpress 已弃用——使用 @longPress 意图指令")),
        "expected V5, got: {:?}", errors);
    assert!(errors.iter().any(|m| m.contains("@onCopy 已弃用——使用 @copy 意图指令")),
        "expected V6, got: {:?}", errors);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_intent_semantic_test`
Expected: FAIL（V4 未实现；@onLongpress/@onCopy 仍透传无诊断）

- [ ] **Step 3: 实现**

1. V4：walk ElementProp::GestureIntent，handler ident ∉ 组件 fn 表 → V4。
2. V5/V6：validate_event_names.rs 加硬化表 `{ "longpress" → V5, "copy" → V6 }`（用户事件名拼写命中即硬错误，消息 verbatim + 迁移提示；不扰动既有 legacy_spelling 机制，cut/paste 不动）；event_names.rs 移除/标记 :35/:79 透传条目。
3. `assert_d14` 断言体实现：解析 `for_directive` 解析源（parser/view/directives.rs）含 "reorder"、parse_event_prop（element.rs）含 "swipe"/"longPress"/"pinch"/"copy"/"key"/"filePick"/"capture" 分派名——事实源存在性断言；ENABLED_D14 = True；universe 文档域 14 状态「部分」。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --workspace && python scripts/verify-semantic-universe.py`
Expected: PASS（域 14 断言启用后绿）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/ crates/omnilang-core/src/codegen/shared/event_names.rs crates/omnilang-core/tests/batch5_intent_semantic_test.rs docs/superpowers/specs/semantic-universe.md scripts/verify-semantic-universe.py
git commit -m "feat(semantic): 交互意图验证 V4/V5/V6——手势 handler 约束 + @onLongpress/@onCopy 硬化迁移 + 域 14 登记启用"
```

**验证门禁：** `cargo test --workspace`｜clippy｜verify-semantic-universe.py。

**依赖：** T3（手势/@copy/@key 解析来自 T2，T3 仅 reorder；V4/V5/V6 依赖 T2 的解析与 T1 登记）。注：本任务与 T3 可并行（不同文件域），但 V5/V6 硬化可能影响既有语料（learn-omniview 中若有 @onLongpress/@onCopy 需同步迁移——T7 处理）。

---

### Task 5: 交互 codegen——React + OmniView（reorder/swipe/longPress/pinch/copy/key）

**Files:**
- Modify: `crates/omnilang-core/src/codegen/react/`（ElementIntentIR → React 产物；@for reorder → dnd-kit DndContext + SortableContext + useSortable；手势 → @use-gesture onSwipe/onLongPress/onPinch；@copy → navigator.clipboard.writeText + 非 secure context execCommand 兜底；@key → window keydown 监听 + 修饰符检查）
- Modify: `crates/omnilang-core/src/codegen/omniview/`（HTML DnD 排序合成 + Pointer Events 手势判定 + clipboard + keydown）
- Modify: `test/projects/learn-omniview/build/react-test-app/package.json`（dnd-kit/@use-gesture 依赖注入）
- Modify: `omnilang/src/codegen/tests_regression/mod.rs`（+mod）
- Create: `omnilang/src/codegen/tests_regression/longtail_intents.rs`

**Interfaces:**
- Consumes: `ElementIntentIR`（T2）+ `ListRenderingIntent.reorder`（T3）+ V4/V5/V6 语义通过（T4）。
- Produces: React 产物断言锚点：reorder → `DndContext`/`SortableContext`/`onDragEnd`；swipe → `useGesture` + `onSwipe`；longPress → `onLongPress`；pinch → `onPinch`；copy → `navigator.clipboard.writeText`；key → `keydown` 监听 + `event.key`/`event.ctrlKey` 组合检查。OmniView 产物：`draggable` 属性 + drop 判定；Pointer Events `pointermove` 阈值；`clipboard.writeText`；`keydown` 监听。

- [ ] **Step 1: 写失败测试（codegen 内容断言）**

```rust
// omnilang/src/codegen/tests_regression/longtail_intents.rs
// compile_target helper 同 injection.rs:565（见 Global Constraints 模板）

#[test]
fn react_reorder_generates_dndkit_sortable_wiring() {
    let source = r##"
component App {
    signal items: string[] = ["a", "b"]
    view {
        @for item in items key={item} reorder={handleMove} {
            <div>{item}</div>
        }
    }
}
fn handleMove(from: i32, to: i32) {}
"##;
    let result = compile_target(source, Target::React);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(code.contains("DndContext"), "expected dnd-kit DndContext");
    assert!(code.contains("SortableContext"), "expected dnd-kit SortableContext");
    assert!(code.contains("onDragEnd"), "expected onDragEnd wiring");
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib -p omnilang longtail_intents`
Expected: FAIL（生成物无 DndContext——reorder 尚为透传/未接线）

- [ ] **Step 3: 写第二个失败测试（@copy/@key React 断言 + OmniView 手势）**

```rust
#[test]
fn react_copy_and_keymap_generate_clipboard_and_keydown() {
    let source = r##"
component App {
    signal link: string = "https://x.dev"
    view { <Button @copy={link}>复制</Button> <Page @key="ctrl+s"={onSave} /> }
}
fn onSave() {}
"##;
    let result = compile_target(source, Target::React);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(code.contains("navigator.clipboard.writeText"), "expected clipboard write");
    assert!(code.contains("keydown"), "expected keydown listener");
    assert!(code.contains("ctrlKey"), "expected ctrl modifier check");
}

#[test]
fn omniview_swipe_generates_pointer_threshold_judgement() {
    let source = r##"
component App {
    view { <Card @swipe={handleSwipe}>x</Card> }
}
fn handleSwipe(dir: string) {}
"##;
    let result = compile_target(source, Target::OmniView);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(code.contains("pointermove"), "expected pointermove listener");
}
```

- [ ] **Step 4: 实现**

按 §4 映射表 React 行 + OmniView 行：reorder → dnd-kit 三件套（onDragEnd → from/to 调 handler）；swipe/longPress/pinch → @use-gesture `useGesture` 配置；copy → click handler 内 clipboard.writeText + 非 secure context `document.execCommand('copy')` 兜底（编译器生成，注记注释）；key → window keydown 监听 + 修饰符组合检查（Flutter Intent 对齐为内部结构不要求用户三层）。OmniView：draggable + drop 判定排序合成、Pointer Events 位移阈值判定、clipboard、keydown。package.json 注入 dnd-kit/@use-gesture。

- [ ] **Step 5: 运行确认通过**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo test --test generated_code_validation react_output_compiles_with_tsc`
Expected: PASS（tsc 编译门禁绿——工具链缺失则 SKIP）

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/codegen/ omnilang/src/codegen/tests_regression/ test/projects/learn-omniview/build/react-test-app/package.json
git commit -m "feat(codegen-react): 交互意图三件套——reorder dnd-kit + 手势 use-gesture + clipboard/keydown（React+OmniView）"
```

**验证门禁：** `cargo test --workspace`｜clippy｜`cargo test --test generated_code_validation react_output_compiles_with_tsc`。

**依赖：** T2+T3+T4。

---

### Task 6: 交互 codegen——Flutter + 域 14 状态翻转

**Files:**
- Modify: `crates/omnilang-core/src/codegen/flutter/`（reorder → ReorderableListView + onReorder；手势 → GestureDetector onPan 判定/onLongPress/onScale；@copy → Clipboard.setData；@key → Shortcuts + Actions + Intent 内部映射）
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 14 状态 → 支持）
- Modify: `omnilang/src/codegen/tests_regression/longtail_intents.rs`（Flutter 断言追加）

**Interfaces:**
- Consumes: T5 同源 IR。
- Produces: Flutter 产物断言锚点：`ReorderableListView`/`onReorder`；`onLongPress`（SDK）；`Clipboard.setData`；`Shortcuts`/`Actions`/`Intent`（键位→意图→动作编译器内部映射，用户只写键位+handler）。

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn flutter_reorder_generates_reorderable_list_view() {
    let source = r##"
component App {
    signal items: string[] = ["a", "b"]
    view {
        @for item in items key={item} reorder={handleMove} {
            <div>{item}</div>
        }
    }
}
fn handleMove(from: i32, to: i32) {}
"##;
    let result = compile_target(source, Target::Flutter);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(code.contains("ReorderableListView"), "expected ReorderableListView");
    assert!(code.contains("onReorder"), "expected onReorder wiring");
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib -p omnilang longtail_intents`
Expected: FAIL

- [ ] **Step 3: 实现**

按 §4 映射表 Flutter 行：reorder → ReorderableListView + onReorder（from/to 调 handler）；swipe → Dismissible/GestureDetector onPan 判定；longPress → GestureDetector onLongPress；pinch → onScale；copy → Clipboard.setData；key → Shortcuts + Actions + Intent（编译器映射键位串为 Shortcuts 绑定，行为对等）。键盘可达性注记（dnd-kit 内置 / Flutter 无内置——§4 注记）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --workspace && cargo test --test generated_code_validation flutter_output_compiles_with_dart_analyze`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/flutter/ docs/superpowers/specs/semantic-universe.md omnilang/src/codegen/tests_regression/longtail_intents.rs
git commit -m "feat(codegen-flutter): 交互意图 Flutter 映射——ReorderableListView/GestureDetector/Clipboard/Shortcuts + 域 14 翻支持"
```

**验证门禁：** `cargo test --workspace`｜dart analyze 编译门禁。

**依赖：** T5（同源 IR 与语义；可并行但共享测试文件，实施者以先完成的为准追加）。

---

### Task 7: 交互 demo 迁移 + Playwright ①-④

**Files:**
- Modify: `test/projects/learn-omniview/src/components/TodoList.omni`（@for 加 `reorder={handleMove}` + handleMove 动作——store 顺序更新；若 TodoStore 无 move 方法则 action 内用 splice 惯用法）
- Modify: `test/projects/learn-omniview/src/main.omni`（页面 hub 接线新 demo）
- Create: `test/projects/learn-omniview/src/pages/SwipeDemo.omni`（列表项 @swipe 滑动删除——TodoItem 接线）
- Create: `test/projects/learn-omniview/src/pages/ClipboardDemo.omni`（分享链接 @copy 按钮）
- Create: `test/projects/learn-omniview/src/pages/ShortcutDemo.omni`（@key="ctrl+s" 保存意图——TodoStore 保存接线）
- Create: `test/projects/learn-omniview/build/react-test-app/tests/batch5-longtail.spec.ts`（断言 ①-④ 块，文件先建后各任务追加）

**Interfaces:**
- Consumes: T5/T6 codegen 可用（demo 需编译运行）。
- Produces: batch5-longtail.spec.ts 断言 ①（拖拽后数据源顺序变化 + 重渲染）、②（滑动触发 handler，dir 判定）、③（点击后 `navigator.clipboard.readText` 断言——Playwright 需 `context.grantPermissions(['clipboard-read','clipboard-write'])`）、④（ctrl+s 触发 handler 且输入框内 ctrl+s 不冲突——焦点上下文断言）。

- [ ] **Step 1: 写失败测试（Playwright 断言 ①）**

```typescript
// tests/batch5-longtail.spec.ts
import { test, expect } from '@playwright/test';

test.beforeEach(async ({ context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
});

test('① @for reorder: 拖拽后数据源顺序变化 + 重渲染', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('link', { name: /TodoList|待办/ }).click();
  // 拖拽第一项到第二项位置（handle 或整项 draggable）
  const first = page.locator('[data-dnd-handle]').first();
  const second = page.locator('[data-dnd-handle]').nth(1);
  await first.dragTo(second);
  await expect(page.locator('[data-todo-item]').nth(1)).toContainText(/第一项的新位置文本/);
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd test/projects/learn-omniview/build/react-test-app && npx playwright test tests/batch5-longtail.spec.ts`
Expected: FAIL（demo 未接线——TodoList 无 reorder）

- [ ] **Step 3: 实现**

1. TodoList.omni 加 `reorder={handleMove}`（spec §8 逐字：`@for item in visibleItems key={item.id} reorder={handleMove}`）+ `fn handleMove(from: i32, to: i32)`（store 顺序更新，splice 惯用法）。
2. 新 demo 三页（SwipeDemo/ClipboardDemo/ShortcutDemo，spec §8 语料）+ main.omni 接线。
3. 重新构建 react-test-app（`npm install` 新依赖 + build）。

- [ ] **Step 4: 运行确认通过（断言 ①-④ 全绿）**

Run: `npx playwright test tests/batch5-longtail.spec.ts`
Expected: PASS（①②③④ 四断言——本任务完成 ①④ 与 ③④ 对应 demo 接线；② SwipeDemo 滑动可用 Playwright mouse 拖拽模拟）

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/src/ test/projects/learn-omniview/build/react-test-app/tests/batch5-longtail.spec.ts
git commit -m "feat(demo): 交互 demo 迁移——TodoList reorder + Swipe/Clipboard/Shortcut 三新页 + Playwright ①-④"
```

**验证门禁：** `cargo test --workspace`（demo 编译不破坏）+ `npx playwright test tests/batch5-longtail.spec.ts` ①-④ 绿 + tsc 门禁（react-test-app build）。

**依赖：** T5+T6。

---

### Task 8: @t ICU 子集——表达式意图函数 + V9/V10/V16 + V11 警告

**Files:**
- Modify: `crates/omnilang-core/src/parser/expr/prefix.rs`（TokenKind::At 分支：`@t` 解析）
- Modify: `crates/omnilang-core/src/parser/ast/expr.rs`（`Expr::I18nExpr { message: IcuMessage, args: Vec<(String, Expr)> }` + `IcuMessage` 结构）
- Modify: `crates/omnilang-core/src/semantic/validate_i18n.rs`（新建，V11 警告）
- Modify: `crates/omnilang-core/src/semantic/mod.rs`
- Create: `crates/omnilang-core/tests/batch5_i18n_parser_test.rs`

**Interfaces:**
- Consumes: `parse_prefix` :30 分派点；expr 级 TokenKind::At 当前落 error（确认锚点）。
- Produces: ICU 子集解析器（spec §2.2① 子集定义 verbatim：`icu_message ::= (text | placeholder)*`；placeholder = 插值 `{arg}` | `{arg, number}` | `{arg, (plural|select), cases}`；cases 闭集 + `other` 结尾；**不含 offset/#/date/time/ordinal/嵌套>3**）；V9（花括号未闭合/占位符名缺失）、V10（子集外语法）、V16（嵌套深度 >3）——解析期硬错误；V11（参数与占位符不匹配）——semantic 层警告；`@t` 返回值语义 = String（不参与类型推断深化——IR type_ann 缺陷归 1F，见 Global Constraints 注记）。

- [ ] **Step 1: 写失败测试（parser 级）**

```rust
// crates/omnilang-core/tests/batch5_i18n_parser_test.rs（parse_view helper 同 T2 模板）
#[test]
fn parses_t_with_interpolation_plural_and_select() {
    let source = r##"
component App {
    view {
        <span>{@t("Hello {name}", {name: user.name})}</span>
        <span>{@t("{count, plural, =1 {1 item} other {{count} items}}", {count: cart.count})}</span>
        <label text: @t("Submit") />
    }
}
"##;
    let (_program, errors) = parse_view(source);
    assert!(errors.is_empty(), "parse errors: {:?}", errors);
}

#[test]
fn rejects_subset_outside_syntax_v10_and_depth_v16() {
    // V10：offset/#/date/time
    let source = r##"
component App {
    view { <span>{@t("{n, plural, offset:1 =1 {x} other {y}}", {n: 3})}</span> }
}
"##;
    let (_p, errors) = parse_view(source);
    assert!(errors.iter().any(|e| e.message.contains("不支持 offset")),
        "expected V10 offset rejection, got: {:?}", errors);

    // V16：嵌套深度超过 3
    let source2 = r##"
component App {
    view { <span>{@t("{a, select, x {{b, select, y {{c, select, z {deep} other {o2}} other {o3}}} other {o4}} other {o5}}", {a: 1, b: 2, c: 3})}</span> }
}
"##;
    let (_p2, errors2) = parse_view(source2);
    assert!(errors2.iter().any(|e| e.message.contains("嵌套深度超过 3")),
        "expected V16 depth rejection, got: {:?}", errors2);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_i18n_parser_test`
Expected: FAIL（@t 在表达式位置报错）

- [ ] **Step 3: 写第二个失败测试（V9 + V11）**

```rust
#[test]
fn rejects_unclosed_brace_v9_and_warns_unused_args_v11() {
    // V9：花括号未闭合
    let source = r##"
component App {
    view { <span>{@t("Hello {name", {name: "x"})}</span> }
}
"##;
    let (_p, errors) = parse_view(source);
    assert!(errors.iter().any(|e| e.message.contains("花括号未闭合")),
        "expected V9, got: {:?}", errors);

    // V11：参数未使用 → 警告（semantic 层，同文件用 analyse_multi 模板）
    let src2 = r##"
component App {
    signal name: string = "x"
    view { <span>{@t("Hello {name}", {name: name, unused: 1})}</span> }
}
"##;
    // 见 Step 5 semantic helper——警告断言 severity == Warning
}
```

- [ ] **Step 4: 实现**

1. prefix.rs `TokenKind::At` 分支：ident == "t" → `parse_i18n_expr`；否则维持 error。消息串解析：扫描花括号平衡（V9）、子集外语法判定（V10：offset/#/date/time/ordinal 词法）、嵌套深度计数（V16 >3 拒绝）、`{arg, number}`/`{arg, plural|select, cases}` 形态识别、cases 闭集 + other 结尾检查。
2. `IcuMessage` AST（placeholder 列表 + 文本段）；`Expr::I18nExpr`。
3. `validate_i18n.rs`：V11 警告两条消息（参数未使用 / 占位符缺参数）——比对 args 键集与消息占位符名集。

- [ ] **Step 5: 运行确认通过**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS（含 V11 警告断言——`d.severity == Severity::Warning` 过滤）

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/expr/prefix.rs crates/omnilang-core/src/parser/ast/expr.rs crates/omnilang-core/src/semantic/validate_i18n.rs crates/omnilang-core/tests/batch5_i18n_parser_test.rs
git commit -m "feat(parser): @t ICU 消息子集——插值/复数/选择 + V9/V10/V16 解析 + V11 参数警告"
```

**验证门禁：** `cargo test --workspace`｜clippy。

**依赖：** T1（@t 词汇登记）。注：本任务与 T2/T3/T11/T13 无依赖，可并行。

---

### Task 9: @locale 组件级指令 V12 + formatDate stdlib + 域 15 断言启用

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_i18n.rs`（V12：@locale 值须裸信号引用）
- Modify: `crates/omnilang-core/src/stdlib/mod.rs`（注册 i18n 模块）
- Create: `crates/omnilang-core/src/stdlib/i18n.rs`（`formatDate` 条目）
- Modify: `crates/omnilang-core/src/codegen/shared/`（formatDate 代码生成分派）
- Modify: `scripts/verify-semantic-universe.py`（ENABLED_D15 = True + assert_d15）
- Create: `crates/omnilang-core/tests/batch5_i18n_semantic_test.rs`

**Interfaces:**
- Consumes: `StdlibFn` struct（mod.rs :27-41）+ `wrong_arity_error` 助手（:45）；`ElementProp::Locale`（T2）；`Expr::I18nExpr`（T8）。
- Produces: V12（@locale 值非裸信号 → 硬错误）；stdlib `formatDate(date, locale?)`（module="i18n"，min_args=1，max_args=Some(2)——3 参（模式串）→ arity 硬诊断「参数面留生态」）；域 15 断言（@t 表达式解析 / locale_prop / stdlib formatDate 事实源存在性）；@locale IR：ElementIntentIR.locale（String 信号名）。

- [ ] **Step 1: 写失败测试（semantic 级）**

```rust
// crates/omnilang-core/tests/batch5_i18n_semantic_test.rs（analyse_multi 模板同 T3）
#[test]
fn locale_value_must_be_bare_signal_ref() {
    // V12
    let source = r##"
component App {
    view { <div @locale={getLang()}>x</div> }
}
fn getLang() -> string { "zh" }
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("@locale 的值必须是裸信号引用")),
        "expected V12, got: {:?}", errors);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_i18n_semantic_test`
Expected: FAIL（V12 未实现）

- [ ] **Step 3: 写第二个失败测试（formatDate stdlib 注册 + arity）**

```rust
#[test]
fn format_date_registered_with_arity_bounds() {
    use omnilang_core::stdlib::registry;
    let fns = registry();
    let fd = fns.iter().find(|f| f.name == "formatDate").expect("formatDate registered");
    assert_eq!(fd.module, "i18n");
    assert_eq!(fd.min_args, 1);
    assert_eq!(fd.max_args, Some(2));
}

#[test]
fn format_date_third_arg_is_arity_hard_error() {
    // 模式串参数 → arity 硬诊断（参数面留生态）
    let source = r##"
component App {
    view { <span>{formatDate(order.createdAt, "en-US", "yyyy-MM-dd")}</span> }
}
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("期望至多 2 个参数") || m.contains("参数")),
        "expected arity error, got: {:?}", errors);
}
```

- [ ] **Step 4: 实现**

1. V12：validate_i18n.rs 检查 ElementProp::Locale 值——仅裸信号 ident 引用放行（约束同 focus= 裸信号引用先例）。
2. `stdlib/i18n.rs`：`pub fn register() -> Vec<StdlibFn>` 含 formatDate 条目（module="i18n"、codegen 闭包按 Target 分派——`new Intl.DateTimeFormat(locale).format(date)` / `DateFormat` 映射留 T10 具体产物，本任务注册 + arity + 语义放行）；mod.rs `r.extend(i18n::register())`。
3. `assert_d15` 断言体 + ENABLED_D15 = True；universe 域 15 状态「部分」。

- [ ] **Step 5: 运行确认通过**

Run: `cargo test --workspace && python scripts/verify-semantic-universe.py`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_i18n.rs crates/omnilang-core/src/stdlib/ crates/omnilang-core/tests/batch5_i18n_semantic_test.rs docs/superpowers/specs/semantic-universe.md scripts/verify-semantic-universe.py
git commit -m "feat(i18n): @locale V12 裸信号约束 + formatDate stdlib 最简形态 + 域 15 登记启用"
```

**验证门禁：** `cargo test --workspace`｜clippy｜verify 脚本。

**依赖：** T2（Locale prop 解析）+ T8（@t 同域，语义验证同文件）。

---

### Task 10: i18n codegen 三目标 + LanguagePage 拆分 + Playwright ⑤ + 域 15 翻转

**Files:**
- Modify: `crates/omnilang-core/src/codegen/react/`（@t → react-intl `FormattedMessage`/`formatMessage` + `IntlProvider locale`；formatDate → `Intl.DateTimeFormat(locale).format`）
- Modify: `crates/omnilang-core/src/codegen/flutter/`（gen_l10n ARB 提取——编译器职责注记：v0 内联字面量直出 `AppLocalizations` 或 intl 包 `Intl.message` 风格；@locale → `Localizations.override` + supportedLocales；formatDate → `DateFormat`）
- Modify: `crates/omnilang-core/src/codegen/omniview/`（内部消息表 + `Intl.DateTimeFormat`）
- Modify: `test/projects/learn-omniview/build/react-test-app/package.json`（react-intl 依赖注入）
- Modify: `test/projects/learn-omniview/src/pages/LanguagePage.omni`（spec §8：@locale 切换 + @t 复数 + formatDate；复用现有 LanguagePage 页位）
- Modify: `omnilang/src/codegen/tests_regression/longtail_intents.rs`（i18n 断言追加）
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 15 → 支持）

**Interfaces:**
- Consumes: `Expr::I18nExpr`（T8）+ `ElementIntentIR.locale`（T9）+ stdlib formatDate（T9）。
- Produces: 断言锚点：React `IntlProvider` + `locale=` + `formatMessage`（或 FormattedMessage）；Flutter `Localizations.override` + `supportedLocales`；OmniView 消息表 + `Intl.DateTimeFormat`；formatDate 三目标产物。

- [ ] **Step 1: 写失败测试（codegen 内容断言）**

```rust
#[test]
fn react_locale_and_t_generate_intl_provider_and_format_message() {
    let source = r##"
component App {
    signal lang: string = "zh"
    view {
        <div @locale={lang}>
            <span>{@t("Hello {name}", {name: "x"})}</span>
            <span>{formatDate(order.createdAt)}</span>
        </div>
    }
}
"##;
    let result = compile_target(source, Target::React);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(code.contains("IntlProvider"), "expected IntlProvider");
    assert!(code.contains("locale="), "expected locale prop");
    assert!(code.contains("formatMessage"), "expected formatMessage");
    assert!(code.contains("Intl.DateTimeFormat"), "expected DateTimeFormat");
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib -p omnilang longtail_intents`
Expected: FAIL（@t 表达式尚未映射到 react-intl 产物——或 stdlib 未接线 codegen）

- [ ] **Step 3: 实现**

按 §4 i18n 行三目标：React IntlProvider/FormattedMessage；Flutter Localizations.override + supportedLocales + gen_l10n ARB（v0 内联字面量提取为编译器职责——P3）；OmniView 消息表。formatDate 三目标（Intl.DateTimeFormat / DateFormat / Intl.DateTimeFormat）。LanguagePage 拆分（@locale 切换按钮 + @t 复数 + formatDate，spec §8 语料）；react-intl 依赖注入；域 15 状态 → 支持。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --workspace && cargo test --test generated_code_validation react_output_compiles_with_tsc && cargo test --test generated_code_validation flutter_output_compiles_with_dart_analyze`
Expected: PASS（LanguagePage 生成物 tsc/dart analyze 编译绿）

- [ ] **Step 5: Playwright ⑤（追加断言块）**

```typescript
// tests/batch5-longtail.spec.ts 追加
test('⑤ @locale 切换后 @t 文案与 formatDate 输出同步变化', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('link', { name: /Language|语言/ }).click();
  const before = await page.locator('[data-i18n-greeting]').textContent();
  await page.getByRole('button', { name: /切换|Switch/ }).click();
  await expect(page.locator('[data-i18n-greeting]')).not.toHaveText(before!);
  // formatDate 输出 locale 同步变化（日期文案断言）
});
```

Run: `npx playwright test tests/batch5-longtail.spec.ts`
Expected: PASS（⑤ 绿 + 既有 ①-④ 不回归）

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/codegen/ omnilang/src/codegen/tests_regression/ test/projects/learn-omniview/src/pages/LanguagePage.omni test/projects/learn-omniview/build/react-test-app/ docs/superpowers/specs/semantic-universe.md
git commit -m "feat(codegen-i18n): @t/@locale/formatDate 三目标映射——react-intl/gen_l10n/OmniView 消息表 + LanguagePage 拆分 + Playwright ⑤"
```

**验证门禁：** `cargo test --workspace`｜tsc + dart analyze 编译门禁｜Playwright ⑤。

**依赖：** T8+T9。

---

### Task 11: canvas RefOp ctx 方法门类——V14 + 三目标 codegen + CanvasDrawDemo + Playwright ⑥ + 域 16 断言启用

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_ref_ops.rs`（ctx 方法白名单门类 + V14）
- Modify: `crates/omnilang-core/src/codegen/react/`（canvas + `getContext('2d')` 直写）
- Modify: `crates/omnilang-core/src/codegen/omniview/`（canvas + ctx 直写）
- Modify: `crates/omnilang-core/src/codegen/flutter/`（CustomPaint + CustomPainter 重放——paint() 内执行同一序列 + ValueNotifier repaint，§4 降级注记）
- Modify: `scripts/verify-semantic-universe.py`（ENABLED_D16 = True + assert_d16 骨架——本任务先断言 ctx 白名单事实源，@filePick/@capture 词条待 T12 后完整）
- Create: `test/projects/learn-omniview/src/pages/CanvasDrawDemo.omni`（spec §8：`<canvas @ref={c} width={400} height={200}/>` + fn draw() ctx 命令式序列 + @mount 调用）
- Modify: `test/projects/learn-omniview/src/main.omni`（接线）
- Modify: `omnilang/src/codegen/tests_regression/longtail_intents.rs`（canvas 断言追加）
- Create: `crates/omnilang-core/tests/batch5_media_semantic_test.rs`

**Interfaces:**
- Consumes: RefOp 既有机制（域 4，validate_ref_ops.rs 1693 行白名单先例）；`<canvas @ref={c}>` 既有解析。
- Produces: ctx 方法门类 v0 闭集（spec §2.3① verbatim：beginPath/moveTo/lineTo/closePath/stroke/fill/arc/rect/fillRect/strokeRect/clearRect/translate/rotate/scale/save/restore + 属性 fillStyle/strokeStyle/lineWidth + canvas 级 toDataURL()）；V14（ctx 方法不在白名单 → 硬错误含方法名）；React/OmniView `getContext('2d')` 直写；Flutter CustomPaint 重放 + ValueNotifier repaint（显式注记 ADR-054）。

- [ ] **Step 1: 写失败测试（semantic 级）**

```rust
// crates/omnilang-core/tests/batch5_media_semantic_test.rs（analyse_multi 模板）
#[test]
fn ctx_method_outside_whitelist_is_hard_error() {
    // V14
    let source = r##"
component App {
    view { <canvas @ref={c} width={100} height={100} /> }
    @mount {
        c.ctx.drawImage(img, 0, 0)
    }
}
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("canvas ctx 方法不在白名单") && m.contains("drawImage")),
        "expected V14 with method name, got: {:?}", errors);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_media_semantic_test`
Expected: FAIL（drawImage 当前按 RefOp 通用规则放行/未识别）

- [ ] **Step 3: 写第二个失败测试（codegen 内容断言）**

```rust
#[test]
fn react_canvas_generates_2d_context_direct_writes() {
    let source = r##"
component App {
    view { <canvas @ref={c} width={100} height={100} /> }
    @mount {
        c.ctx.moveTo(10, 10)
        c.ctx.lineTo(90, 90)
        c.ctx.stroke()
    }
}
"##;
    let result = compile_target(source, Target::React);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(code.contains("getContext('2d')"), "expected 2d context");
    assert!(code.contains("moveTo") && code.contains("stroke"), "expected draw sequence");
}
```

- [ ] **Step 4: 实现**

1. validate_ref_ops.rs 加 ctx 门类白名单（RefOp 分派先例——ctx. 前缀方法集 + 属性集 + canvas 级 toDataURL 例外）；V14 消息含方法名。
2. React/OmniView：canvas 元素 + getContext('2d') 直写（组件函数内调用序列原样生成，context 就绪保证）。
3. Flutter：CustomPaint + CustomPainter——paint() 内重放同一序列 + ValueNotifier 触发 repaint（诚实降级注记注释——ADR-054，非静默）。
4. CanvasDrawDemo + main.omni 接线 + Playwright ⑥（`toDataURL` 像素非空断言——P6 裁决）。

- [ ] **Step 5: 运行确认通过（含 Playwright ⑥）**

Run: `cargo test --workspace && cargo test --test generated_code_validation react_output_compiles_with_tsc && npx playwright test tests/batch5-longtail.spec.ts`
Expected: PASS（⑥：`page.evaluate(() => canvas.toDataURL())` 非空 + 长度断言）

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_ref_ops.rs crates/omnilang-core/src/codegen/ omnilang/src/codegen/tests_regression/ test/projects/learn-omniview/src/pages/CanvasDrawDemo.omni test/projects/learn-omniview/src/main.omni scripts/verify-semantic-universe.py
git commit -m "feat(canvas): RefOp ctx 方法门类 V14 + 三目标 codegen（Flutter CustomPaint 重放降级注记）+ CanvasDrawDemo + Playwright ⑥"
```

**验证门禁：** `cargo test --workspace`｜clippy｜tsc 门禁｜Playwright ⑥。

**依赖：** T1（域 16 登记骨架）。注：本任务与 T2/T3/T8/T13 无依赖，可并行。

---

### Task 12: @filePick/@capture/@permission——V15/V17 + codegen + FilePickDemo + Playwright ⑦ + 域 16 翻转

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/component.rs`（@permission 组件体声明，:182-183 生命周期同机制——`ComponentIR.permissions: Vec<PermissionDecl>`）
- Modify: `crates/omnilang-core/src/semantic/validate_media.rs`（新建，V15：@capture 无 @permission → 硬错误；V17：Flutter 目标 audio → 警告）
- Modify: `crates/omnilang-core/src/codegen/react/`（@filePick → 隐藏 input + click 代理 + FileReader；@capture → getUserMedia + MediaRecorder，secure context 降级硬诊断；@permission → 权限请求生成——P4 编译器隐式生成）
- Modify: `crates/omnilang-core/src/codegen/flutter/`（file_picker 插件调用 + 工程面注记注释；camera 插件；@capture="audio" → V17 警告 + 不接线保持可编译）
- Modify: `crates/omnilang-core/src/codegen/omniview/`（input[type=file] + File API；getUserMedia）
- Modify: `scripts/verify-semantic-universe.py`（assert_d16 完整化 + ENABLED_D16 = True）
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 16 → 支持）
- Create: `test/projects/learn-omniview/src/pages/FilePickDemo.omni`（spec §8 未列——**计划级最小补全**：filePick 回传元数据展示 + capture 按钮组；注记不推翻 spec）
- Modify: `test/projects/learn-omniview/src/main.omni`（接线）
- Modify: `omnilang/src/codegen/tests_regression/longtail_intents.rs`（filePick/capture 断言追加）

**Interfaces:**
- Consumes: `ElementProp::{FilePick, Capture}`（T2）+ `ComponentIR.permissions`（本任务）+ T11 域 16 骨架。
- Produces: V15（@capture 无 @permission 声明 → 硬错误）、V17（Flutter @capture="audio" → Warning 诊断 verbatim，生成物保持可编译——dart analyze 零告警不受影响，警告为编译期诊断不进生成物）；React 隐藏 input + click 代理 + FileReader；Flutter file_picker 插件（v0 限文本 content；二进制留生态）+ 工程面注记；getUserMedia 运行时询问 + secure context 硬诊断。

- [ ] **Step 1: 写失败测试（semantic 级）**

```rust
// crates/omnilang-core/tests/batch5_media_semantic_test.rs 追加
#[test]
fn capture_requires_permission_declaration() {
    // V15
    let source = r##"
component App {
    view { <Button @capture="camera"={onShot} /> }
}
fn onShot(data: string) {}
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("@capture 需要 @permission 声明")),
        "expected V15, got: {:?}", errors);
}

#[test]
fn capture_with_permission_declaration_compiles() {
    let source = r##"
component App {
    @permission="camera"
    view { <Button @capture="camera"={onShot} /> }
}
fn onShot(data: string) {}
"##;
    let errors = semantic_errors(source);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_media_semantic_test`
Expected: FAIL（V15 未实现；@permission 声明未解析）

- [ ] **Step 3: 写第二个失败测试（codegen 级 V17 警告断言）**

```rust
#[test]
fn flutter_audio_capture_emits_v17_warning_and_still_compiles() {
    let source = r##"
component App {
    @permission="microphone"
    view { <Button @capture="audio"={onRecorded} /> }
}
fn onRecorded(data: string) {}
"##;
    let result = compile_target(source, Target::Flutter);
    assert!(result.success, "compile failed: {:?}", result.errors);
    assert!(result.warnings.iter().any(|w| w.message.contains("@capture=\"audio\" Flutter 端无官方实现")),
        "expected V17 warning, got: {:?}", result.warnings);
    // 生成物保持可编译——无 capture 接线代码（正常元素渲染）
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(!code.contains("CameraController"), "audio capture should not wire camera plugin");
}
```

- [ ] **Step 4: 实现**

1. component.rs @permission 组件体声明解析（`@permission="camera"` / `"microphone"`，挂生命周期同级）；ComponentIR.permissions。
2. validate_media.rs：V15（walk Capture prop → 检查组件 permissions 含对应声明）；V17（Flutter 目标 + CaptureMode::Audio → Warning 诊断；诊断注入点 = codegen 阶段，编译期诊断不进生成物）。
3. codegen：React 隐藏 input + click 代理 + FileReader（v0 限文本 content）；getUserMedia + MediaRecorder + secure context 硬诊断（`window.isSecureContext` 检查生成）；Flutter file_picker/camera 插件调用 + 工程面注记注释（pubspec 注入属 poiesis 产物工程承接，不自动注入——P4）；OmniView input[type=file] + File API。
4. FilePickDemo + main.omni 接线 + Playwright ⑦（file chooser 回传元数据断言——`page.setInputFiles` 或 filechooser 事件）；assert_d16 完整化 + ENABLED_D16 = True；域 16 → 支持。

- [ ] **Step 5: 运行确认通过（含 Playwright ⑦）**

Run: `cargo test --workspace && cargo test --test generated_code_validation react_output_compiles_with_tsc && npx playwright test tests/batch5-longtail.spec.ts && python scripts/verify-semantic-universe.py`
Expected: PASS（⑦：setInputFiles 后元数据（name/size/type/content）展示断言）

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/top_level/component.rs crates/omnilang-core/src/semantic/validate_media.rs crates/omnilang-core/src/codegen/ omnilang/src/codegen/tests_regression/ test/projects/learn-omniview/src/pages/FilePickDemo.omni test/projects/learn-omniview/src/main.omni docs/superpowers/specs/semantic-universe.md scripts/verify-semantic-universe.py
git commit -m "feat(media): @filePick/@capture/@permission——V15/V17 + 三目标 codegen + FilePickDemo + Playwright ⑦ + 域 16 翻支持"
```

**验证门禁：** `cargo test --workspace`｜clippy｜tsc 门禁｜Playwright ⑦｜verify 脚本。

**依赖：** T2（FilePick/Capture 解析）+ T11（域 16 骨架同文件域）。

---

### Task 13: 生命周期托管函数——timer/interval/subscribe + V13 + 域 17 入册断言启用

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_effect_deps.rs`（V13：effect 体外调用 → 硬错误；体内豁免）
- Modify: `crates/omnilang-core/src/stdlib/mod.rs`（注册 timer 模块）
- Create: `crates/omnilang-core/src/stdlib/timer.rs`（`timer`/`interval`/`subscribe` 条目——module="timer"；codegen 闭包占位分派，T14 完成产物）
- Modify: `scripts/verify-semantic-universe.py`（ENABLED_D17 = True + assert_d17：lifecycle_hook 产生式交叉断言——15-grammar-view §1.11 存在性 + @mount/@effect 白名单交叉）
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 17 登记表——@store 语法位注记 grammar-view §1.7）
- Create: `crates/omnilang-core/tests/batch5_lifecycle_semantic_test.rs`

**Interfaces:**
- Consumes: `validate_effect_deps.rs` 既有 walk（effect 体遍历先例）；`StdlibFn` struct；域 17 登记骨架（T1）。
- Produces: V13（timer/interval/subscribe 在 @effect 块外 → 硬错误 verbatim；块内放行）；stdlib std/timer 三成员（不新增 IR 节点——保持 call 表达式 + codegen 名分派，spec §2.4①）；fn_ref 允许内联箭头（与事件 handler 不同——表达式上下文）。

- [ ] **Step 1: 写失败测试（semantic 级）**

```rust
// crates/omnilang-core/tests/batch5_lifecycle_semantic_test.rs（analyse_multi 模板）
#[test]
fn lifecycle_fns_outside_effect_are_hard_errors() {
    // V13
    let source = r##"
component App {
    signal n: i32 = 0
    view { <div>{n}</div> }
    @mount {
        interval(1000, () => tick())
    }
}
fn tick() { }
"##;
    let errors = semantic_errors(source);
    assert!(errors.iter().any(|m| m.contains("timer/interval/subscribe 只能在 @effect 块内使用")),
        "expected V13, got: {:?}", errors);
}

#[test]
fn lifecycle_fns_inside_effect_compile() {
    let source = r##"
component App {
    signal n: i32 = 0
    view { <div>{n}</div> }
    @effect {
        interval(1000, () => tick())
        timer(3000, () => autoClose())
        subscribe(TodoStore.count, (n) => onCount(n))
    }
}
fn tick() { }
fn autoClose() { }
fn onCount(n: i32) { }
"##;
    let errors = semantic_errors(source);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p omnilang-core --test batch5_lifecycle_semantic_test`
Expected: FAIL（interval 在块外按未定义函数报错而非 V13；或 stdlib 未注册）

- [ ] **Step 3: 实现**

1. stdlib/timer.rs 三条目注册（module="timer"，min_args=2，codegen 闭包 T14 填充——本任务先注册 + 语义放行，codegen 生成兜底注释防空闭包）。
2. validate_effect_deps.rs：walk 内识别 stdlib timer 模块调用——effect 体内豁免既有 unknown-call 检查；effect 体外 → V13 硬错误。
3. 域 17 登记表终稿（spec §5 verbatim，@store 语法位注记）+ assert_d17（lifecycle_hook 产生式交叉断言 + @mount/@effect 白名单交叉——域 7 既有断言 Task 2 模式复用）+ ENABLED_D17 = True。
4. grammar-view §1.11 若缺 timer/interval/subscribe 注记则补（docs-gate 同步——与 T15 终态一致，本任务可先只做断言，文档终稿 T15）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --workspace && python scripts/verify-semantic-universe.py`
Expected: PASS（域 17 断言启用后绿）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_effect_deps.rs crates/omnilang-core/src/stdlib/timer.rs crates/omnilang-core/src/stdlib/mod.rs crates/omnilang-core/tests/batch5_lifecycle_semantic_test.rs docs/superpowers/specs/semantic-universe.md scripts/verify-semantic-universe.py
git commit -m "feat(lifecycle): @effect 托管 timer/interval/subscribe + V13 + 域 17 入册断言启用（1B 项）"
```

**验证门禁：** `cargo test --workspace`｜clippy｜verify 脚本。

**依赖：** T1（域 17 骨架）。注：与 T2/T3/T8/T11 无依赖，可并行。

---

### Task 14: 生命周期 codegen 三目标 + ClockDemo + DataFetching 轮询检查 + Playwright ⑧ + 域 17 翻转

**Files:**
- Modify: `crates/omnilang-core/src/codegen/react/`（@effect 体内 timer/interval/subscribe → useEffect cleanup 收集：setInterval/setTimeout 句柄 + return 闭包清理；subscribe → 退订）
- Modify: `crates/omnilang-core/src/codegen/flutter/`（State.dispose 列表：Timer.periodic/Timer + StreamSubscription.cancel）
- Modify: `crates/omnilang-core/src/codegen/omniview/`（effect 生命周期 + 清理）
- Modify: `test/projects/learn-omniview/src/components/Timer.omni`（保留原 demo——显式清理合法形态；新增 ClockDemo 对照）
- Create: `test/projects/learn-omniview/src/components/ClockDemo.omni`（spec §8：`@effect { interval(1000, () => tick()) }` 自动托管形态）
- Modify: `test/projects/learn-omniview/src/pages/DataFetching.omni`（轮询手写检查——有则迁移 @effect + timer/interval，无则跳过记录）
- Modify: `test/projects/learn-omniview/src/main.omni`（接线）
- Modify: `omnilang/src/codegen/tests_regression/longtail_intents.rs`（lifecycle 断言追加）
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 17 → 支持）

**Interfaces:**
- Consumes: stdlib timer 注册 + V13（T13）。
- Produces: React 断言锚点：effect 体内 `interval(...)` → `setInterval` + cleanup `clearInterval`（useEffect return 闭包）；Flutter：`Timer.periodic` + dispose `cancel`；OmniView：清理通道。Playwright ⑧：interval 组件卸载后无泄漏（定时器不再触发，console 断言——卸载后等待 > 周期，断言 tick 计数不再增长）。

- [ ] **Step 1: 写失败测试（codegen 内容断言）**

```rust
#[test]
fn react_effect_interval_generates_cleanup() {
    let source = r##"
component App {
    signal n: i32 = 0
    view { <div>{n}</div> }
    @effect {
        interval(1000, () => tick())
    }
}
fn tick() { }
"##;
    let result = compile_target(source, Target::React);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let code = result.files.values().collect::<Vec<_>>().join("\n");
    assert!(code.contains("setInterval"), "expected setInterval");
    assert!(code.contains("clearInterval"), "expected cleanup clearInterval");
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib -p omnilang longtail_intents`
Expected: FAIL（interval 无 codegen 产物或兜底注释）

- [ ] **Step 3: 实现**

1. React：@effect 代码生成处收集 timer/interval/subscribe 调用 → setInterval/setTimeout/退订句柄 + useEffect return 清理闭包（spec §2.4① 编译器展开为各自清理通道）。
2. Flutter：dispose 列表收集 Timer.periodic/Timer/StreamSubscription + cancel。
3. OmniView：effect 生命周期清理。
4. ClockDemo（自动托管对照）+ Timer.omni 保留 + DataFetching.omni 轮询检查 + main.omni 接线 + Playwright ⑧ + 域 17 → 支持。

- [ ] **Step 4: 运行确认通过（含 Playwright ⑧）**

Run: `cargo test --workspace && cargo test --test generated_code_validation react_output_compiles_with_tsc && npx playwright test tests/batch5-longtail.spec.ts`
Expected: PASS（⑧：进入 ClockDemo → 计数增长 → 导航离开 → 等待 2.5s → 返回断言计数未跳变/console 无泄漏告警）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/ omnilang/src/codegen/tests_regression/ test/projects/learn-omniview/src/ docs/superpowers/specs/semantic-universe.md
git commit -m "feat(codegen-lifecycle): @effect 托管三函数三目标 cleanup + ClockDemo + DataFetching 轮询检查 + Playwright ⑧ + 域 17 翻支持"
```

**验证门禁：** `cargo test --workspace`｜tsc 门禁｜Playwright ⑧。

**依赖：** T13。

---

### Task 15: 注入语料 855 扩展 + docs-gate 终态

**Files:**
- Modify: `omnilang/tests/fixtures/`（新 fixture：reorder/手势/@key/@t 复数/formatDate/托管函数/RefOp ctx 正负例——generated_code_validation.rs 自动枚举）
- Modify: `omnilang/src/codegen/tests_regression/injection.rs`（injection_payload_corpus 追加元组——@t 插值/复数/选择（含子集外负例）、reorder schema（from/to）、手势 schema（dir/scale）、@key 键位串正负例、@effect 托管函数正负例、RefOp ctx 白名单正负例；V1-V17 每条至少 1 正 1 负）
- Modify: `omnilang/tests/llm_error_corpus.rs`（六类错误模式 A-F 追加新诊断模式）
- Modify: `docs/omniview/15-grammar-view.md`（§1.11 lifecycle 注记 timer/interval/subscribe；for_directive :495 加 reorder=；元素指令表加 8 意图 prop + @t 表达式 + @permission 组件声明）
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 14-17 状态终态核对）
- Modify: `docs/superpowers/specs/vocabulary-registry.md`（终态核对）

**Interfaces:**
- Consumes: T1-T14 全部实现。
- Produces: 855 元组扩展全量（spec §7.5）；docs-gate 绿（grammar-view/登记册/脚本同 commit 全生态迁移——ROADMAP 硬规则）；llm_error_corpus 六类模式与 V1-V17 诊断映射无冲突。

- [ ] **Step 1: 写失败测试（注入语料负例——先确认失败）**

在 injection.rs 追加：

```rust
#[test]
fn batch5_icu_subset_negative_cases_across_backends() {
    // V10：date 复杂格式拒绝
    let source = r##"
component App {
    view { <span>{@t("{d, date, long}", {d: 1})}</span> }
}
"##;
    for target in [Target::React, Target::Flutter, Target::OmniView] {
        let result = compile_target(source, target);
        assert!(result.errors.iter().any(|e| e.message.contains("不支持")),
            "target {:?}: expected V10-style error, got: {:?}", target, result.errors);
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --lib -p omnilang batch5_icu`
Expected: FAIL（若 T8 已实现则转 Step 3 直接全量填充——本任务以「全量正负例存在 + 断言绿」为验收，负例断言全部应在实施后通过）

- [ ] **Step 3: 实现**

1. fixtures 新文件（每 fixture 单一能力 + 正负例可编译形态；V 级负例 fixture 预期 errors 断言——llm_error_corpus 模式复用）。
2. injection.rs 语料元组按 spec §7.5 全量补齐（V1-V17 每条 ≥1 正 1 负）。
3. grammar-view 产生式更新（lifecycle 托管函数、reorder=、意图 prop、@t、@permission）；universe/vocabulary 终态核对（状态全「支持」/「部分」符合各任务翻转结果）。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test --workspace && cargo build -p omnic && python scripts/verify-docs.py --check && python scripts/verify-semantic-universe.py && python scripts/verify-vocabulary.py`
Expected: 全绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/ docs/omniview/15-grammar-view.md docs/superpowers/specs/
git commit -m "docs(corpus): 855 元组批次 5 扩展——V1-V17 正负例全量 + grammar-view 产生式同步 + 登记册终态"
```

**验证门禁：** `cargo test --workspace`｜verify-docs.py --check｜verify 双脚本。

**依赖：** T1-T14 全部。

---

### Task 16: 五层门禁终验 + 占位符自查

**Files:**
- Verify-only（不改代码，发现问题则开票/回修）：全仓库扫描 `TBD|TODO|FIXME|NotImplementedError|兜底注释` 在批次 5 新增/修改文件中清零；`git log --oneline` 核对 16 任务提交齐全。

- [ ] **Step 1: 五层门禁终验**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test export_bindings --lib -p omnilang && git diff --exit-code poiesis/src/commands/__generated__/
cargo build -p omnic && python scripts/verify-docs.py --check
python scripts/verify-semantic-universe.py && python scripts/verify-vocabulary.py
cargo test --test generated_code_validation react_output_compiles_with_tsc flutter_output_compiles_with_dart_analyze
cd test/projects/learn-omniview/build/react-test-app && npx playwright test tests/batch5-longtail.spec.ts
```

Expected: 全部绿（Playwright ①-⑧ + 既有 5 spec 全量；工具链缺失项 SKIP 记录）

- [ ] **Step 2: 占位符自查**

```bash
grep -rn "TBD\|TODO\|FIXME\|NotImplementedError" docs/superpowers/plans/2026-08-09-semantic-batch5-longtail.md crates/omnilang-core/src/ omnilang/src/ --include="*.rs" --include="*.md" | grep -i "batch5\|reorder\|swipe\|longPress\|pinch\|@copy\|@key\|@t\b\|@locale\|formatDate\|@filePick\|@capture\|@permission\|timer\|interval\|subscribe\|ctx" | head -20
```

Expected: 无批次 5 相关命中（T1 的 ENABLED 守卫 False 转为 True 后不得残留 NotImplementedError 路径——T4/T9/T12/T13 已替换为真实断言体）

- [ ] **Step 3: spec 覆盖核对（self-review 清单）**

按 spec §1 范围表 12 能力项 + §3 V1-V17 + §7 门禁 ①-⑧ + §8 demo 迁移 7 语料位逐项勾对（能力项→任务映射表见 §3 阶段顺序末），发现缺口立即补任务/回修。

- [ ] **Step 4: 报告终态**

在批次报告文件（`.superpowers/sdd/semantic-batch-specs-plans/batch5-plan-report.md`）补终验结果表（五层门禁每层 PASS/SKIP + 说明），提交：

```bash
git add docs/superpowers/plans/2026-08-09-semantic-batch5-longtail.md .superpowers/sdd/semantic-batch-specs-plans/batch5-plan-report.md
git commit -m "docs(plan): 批次 5 终验报告——五层门禁全绿 + 占位符清零自查"
```

**验证门禁：** 本任务即门禁本身。

**依赖：** T1-T15 全部。

---

## 3. 阶段顺序（任务依赖图）

```
T1（登记先行）──┬── T2（意图属性解析框架）──┬── T3（reorder V1-V3）──┬── T4（V4-V6 + 域14）──┬── T5（React+OV codegen）──┬── T6（Flutter + 域14翻）──┬── T7（demo + PW①-④）
                │                          └── T8（@t ICU 子集）──┬── T9（@locale V12 + formatDate + 域15）──┬── T10（i18n codegen + PW⑤ + 域15翻）
                ├── T11（canvas V14 + 域16骨架）──┬── T12（filePick/capture V15/V17 + PW⑦ + 域16翻）
                └── T13（托管函数 V13 + 域17）──┬── T14（lifecycle codegen + PW⑧ + 域17翻）
                                                        └──────┬──────┬──────┬──────┬──────┘
                                                               T15（语料 855 扩展 + docs-gate 终态）
                                                                        │
                                                                       T16（五层门禁终验）
```

**可并行标注：** T8（i18n 解析）/T11（canvas）/T13（托管语义）与 T2 无依赖、彼此无依赖——T1 完成后即可与 T2 并行启动四子流；交互子流（T3-T7）内部串行；i18n 子流（T8-T10）串行；媒体子流（T11-T12）串行；生命周期子流（T13-T14）串行；四子流彼此独立可并行（共享文件注意：universe 文档/verify 脚本由各子流任务分别追加不同域段落，实施者用 git 原子 commit 避免冲突；longtail_intents.rs 由 T5/T6/T10/T11/T12/T14 共享追加——按依赖顺序实施或同会话合并）。

**能力项 → 任务映射（spec §1 范围表）：** 拖拽重排→T3/T5/T6/T7；基本手势→T2/T4/T5/T6/T7；剪贴板→T2/T4/T5/T6/T7；快捷键→T2/T4/T5/T6/T7；@t→T8/T10；@locale→T2/T9/T10；formatDate→T9/T10；canvas→T11；文件上传→T2/T12；摄像头录音→T2/T12；@effect 强化→T13/T14；@store/@mount/@effect 入册→T1/T13。

## 4. 登记先行任务

T1（全部语法任务的硬前置——ADR-074 登记先行）；域 14 断言启用 = T4、域 15 = T9、域 16 = T12、域 17 = T13（各断言随对应语法实施同 commit 置 True——见 Global Constraints 登记先行机制）。vocabulary-registry 新词汇行（reorder/swipe/longPress/pinch/copy/key/t/locale/formatDate/filePick/capture/permission/timer/interval/subscribe）在 T1 一次性登记，语法实施不再改登记册本体（状态列由各任务翻转）。

## 5. 语义化即切换任务

- **V5/V6 硬化**（T4）：@longPress/@copy 意图落地时 @onLongpress/@onCopy 旧形态硬诊断 + 迁移提示；既有语料检查（learn-omniview 若有 @onLongpress/@onCopy 使用处 → T7 demo 迁移时同步改意图指令；cut/paste 不动——P5）。
- **demo 迁移**（逐语料位，spec §8 清单）：`components/TodoList.omni` reorder= 加装 → T7；`components/Timer.omni` 保留 + ClockDemo 对照 → T14；`pages/DataFetching.omni` 轮询检查 → T14；LanguagePage 拆分（i18n demo）→ T10；CanvasDrawDemo → T11；ShortcutDemo → T7；SwipeDemo → T7；ClipboardDemo → T7；FilePickDemo（计划级补全，Playwright ⑦ 载体）→ T12。
- **stdlib 归属**：sleep 类保留 std/async（非生命周期资源）；timer/interval/subscribe 新 std/timer 成员（T13）——既有语料不破坏。

## 6. 门禁终态（五层终验口径，T16 全量执行）

| 层 | 命令 | 验收 |
|---|---|---|
| 1 tsc | `cargo test --test generated_code_validation react_output_compiles_with_tsc` + react-test-app build | React 生成物（dnd-kit/use-gesture/react-intl 接线、@copy 剪贴板、canvas、filePick）类型检查绿 |
| 2 dart analyze | `cargo test --test generated_code_validation flutter_output_compiles_with_dart_analyze` | Flutter 生成物（ReorderableListView/Shortcuts/GestureDetector/CustomPaint 重放/gen_l10n）零告警；V17 降级路径含编译期诊断验证 |
| 3 Playwright | `npx playwright test tests/batch5-longtail.spec.ts`（①-⑧）+ 既有 5 spec | ①reorder 数据源顺序 ②swipe dir 判定 ③clipboard.readText ④ctrl+s 焦点上下文 ⑤locale 同步变化 ⑥canvas toDataURL 像素非空 ⑦filePick 元数据 ⑧interval 卸载无泄漏 |
| 4 docs-gate | `cargo build -p omnic && python scripts/verify-docs.py --check` + verify 双脚本 | grammar-view 产生式/域 14-17 登记/vocabulary 同步（同 commit 全生态迁移硬规则） |
| 5 注入语料 | `cargo test --workspace`（injection.rs + llm_error_corpus + fixtures） | 855 元组扩展：@t/复数/选择 + 子集外负例、reorder schema、手势 schema、@key 正负例、托管函数正负例、RefOp ctx 正负例；V1-V17 每条 ≥1 正 1 负 |

**代码质量终态：** `cargo clippy --workspace --all-targets -- -D warnings` 零告警；ts-rs 同步（`cargo test export_bindings --lib -p omnilang` + `git diff --exit-code poiesis/src/commands/__generated__/` 干净）；占位符自查（T16 Step 2）无批次 5 相关 TBD/TODO/FIXME/NotImplementedError。

## 7. 风险与降级注记（spec §6 语言边界落地）

- **V3 实施定义差异**：spec 文字「同元素」与 AST 实际形态（@for 独立节点）的适配已定案（Global Constraints）——实施时测试断言「Element 带 @drag 且直接子节点含 reorder @for」；项级 @drag 与 reorder 共存 v0 不放开（语料驱动重评）。
- **reorder 键盘可达性**：dnd-kit 内置键盘重排，Flutter ReorderableListView 无内置——编译器补足项记录不实施（§4 注记）。
- **@copy 非 secure context**：web 端 execCommand 兜底编译器生成（§4 注记）；Playwright 需 grantPermissions（T7 已含）。
- **canvas Flutter 重放保真度（P6 定案）**：paint() 重放 + 像素级断言（toDataURL 非空）——逐帧同步不做；动态绘制 ValueNotifier 触发 repaint（T11 注记注释）。
- **V17 降级**：Flutter @capture="audio" 仅编译期警告，生成物不接线保持可编译——dart analyze 门禁不受影响（T12）。
- **@t 类型推断**：IR type_ann 已知缺陷归 1F 检查点——@t 返回 String 但 v0 不深化类型推断（Global Constraints 注记，不阻塞本批次）。
- **file_picker/camera 插件注入**：属 poiesis 产物工程承接（P4 裁决）——本批次以注记注释 + demo 端 npm/工程面处理落地，不自动改 pubspec。
- **V17 及硬化可能导致既有语料红**：T7 迁移检查兜底；若 learn-omniview 既有 @onLongpress/@onCopy 使用位，同 commit 迁移。
- **T15 占位符清零纪律**：T1 的 ENABLED 守卫 NotImplementedError 路径必须在 T4/T9/T12/T13 真实断言体替换（T16 Step 2 自查兜底）。

---

（本计划无 TBD/TODO/FIXME 占位；能力项→任务映射见 §3 末表；上游 spec 引用为唯一详细级输入。）
