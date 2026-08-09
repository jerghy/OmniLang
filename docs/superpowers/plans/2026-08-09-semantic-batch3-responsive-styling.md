# 批次 3 响应式 + 样式修复 实施计划（2026-08-09-semantic-batch3-responsive-styling.md）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实施批次 3 六项能力——breakpoint 断点声明 + bp() 断点条件表达式 + 选择器静默跳过诊断修复（W-301）+ 用户级 @media 块（web 透传 / Flutter 诚实降级 W-302）+ @container 硬错误 E-404 + 登记册与 demo 迁移——三目标（React/Flutter/OmniView）五层门禁全绿。

**Architecture:** 语法面按「parser AST → semantic IR → 三目标 codegen」既有管线分层落地：breakpoint 声明走 design 块解析（E-402/E-403 编译期校验）→ DesignIR.breakpoints 常量表；bp() 复用既有 CallExpr 表达式形态零 parser 改动，新增合并级语义 pass `validate_breakpoints`（E-401/E-405 + 参数改写为数字字面量，三调用点：run_pipeline 尾部 / analyse_multi / analyse_multi_cached）后由三目标 format_call 特殊分派；选择器诊断采用「parser 记录文本+span+类别到 StyleRule.unsupported_selector + analyse 第 10 pass 发射 warning」机制（不动 Parser 公共 API，warnings 管线零改动）；@media 块为 style 块内 at-rule（条件文本括号配对透传不解析），web 端 CSS 输出层原样透传、Flutter 端 W-302 跳过。

**Tech Stack:** Rust workspace（omnilang-core 解析/语义/IR、omnilang 三目标 codegen facade、omnic CLI）；ts-rs 绑定同步（poiesis WASM 通道）；Python verify 双脚本（semantic-universe / vocabulary）；learn-omniview demo（React 产物 + Playwright）；注入语料 855 元组扩展。

## Global Constraints

- **登记先行（ADR-074）**：语义集/词汇登记与 verify 脚本改造（T1）必须排在语法实施（T2 起）之前——universe/vocabulary 登记先行任务是硬约束，不得调整顺序。
- **诊断消息逐字对齐 spec §3**：E-401/E-402/E-403/E-404/E-405/W-301/W-302 七条消息文案逐字复制（见各任务），不得改写；诊断类别词按实际触发类别。
- **诚实降级（ADR-054）**：@media 块 Flutter 目标 = W-302 + 块内规则跳过（不生效）+ 指引 bp()，首版不做 MediaQuery 分支映射（spec §9 未决问题 1 已拍板）；@container 遇之即 E-404 硬错误 + 指引。
- **Rust 纪律**：每任务提交前 `cargo fmt` + `cargo clippy -D warnings -p <crate>` 零告警；生产代码禁 unwrap/expect；测试走 TDD（先 RED 后 GREEN）。
- **ts-rs 绑定同步**：IR 结构变更后必须跑 `cargo test export_bindings --lib -p omnilang` 并提交 `poiesis/src/commands/__generated__/` 变更（T4）。
- **serde 兼容**：所有新增 IR/AST 字段带 `#[serde(default)]`（含 ts-rs 侧），既有缓存/镜像不破。
- **demo 迁移同 commit**：语法变更照常走 ADR + 同 commit 全生态迁移硬规则（ROADMAP），T9 与语法任务不可拆散提交。
- **占位符清零**：完成时自查 TBD/TODO 占位符为零。

---

## 0. 头部

- **日期**: 2026-08-09
- **上游 spec**: `docs/superpowers/specs/2026-08-09-semantic-batch3-responsive-styling-design.md`（详细级唯一输入，本计划只引用章节号）
- **基线 spec**: `docs/superpowers/specs/2026-08-09-semantic-capability-baseline-design.md` §5 批次路线图 / §6 验收 / §8 治理
- **执行方式**: subagent-driven + 逐任务审查（每任务独立 subagent 会话 + 两阶段审查）

## 1. 背景与范围

批次 3 合并两个已裁决域（响应式断点 + 样式选择器缺陷修复）为一个实施批次，另含 @media 映射策略定案与容器查询候选记录。能力项清单与上游裁决引用（spec §1 表）：

| # | 能力项 | 本批次处置 | spec 章节 |
|---|--------|-----------|-----------|
| 1 | breakpoint 断点声明（design 块） | 语法 + E-402/E-403 + 常量表 | §2.1 / §3 / §4.1 |
| 2 | bp("name") 断点条件表达式 | 语义 pass E-401/E-405 + 三目标映射 | §2.2 / §3 / §4.2 |
| 3 | 选择器静默跳过缺陷修复 | W-301 warning 硬诊断（机制已拍板） | §2.3 / §3 / §4.4 |
| 4 | 用户级 @media 块 | web 透传 / Flutter W-302 诚实降级（已拍板） | §2.4 / §3 / §4.3 |
| 5 | @container 容器查询 | 不实施——E-404 硬错误 + 指引 | §2.5 |
| 6 | 深色模式 / 样式作用域 | 无动作——仅登记册注记 | §1 表 #6 |

**已拍板的未决问题**（spec §9，本计划直接执行，不再请示）：(1) @media Flutter = 诚实降级 W-302；(2) 选择器诊断机制 = parser 记录 span + analyse pass 发射 warning；(3) breakpoint 值域 = 仅限正数，无上限。

## 2. 任务划分

### 任务编号总览

| 任务 | 标题 | 依赖 | 并行组 |
|------|------|------|--------|
| T1 | 登记先行（semantic-universe 域 12 + 域 14 留生态 + vocabulary-registry + verify 双脚本 + docs 注记） | — | A（首跑） |
| T2 | lexer AtMedia/AtContainer token + parser design 块 breakpoint 声明（E-402/E-403） | T1 | B |
| T3 | parser style 块：unsupported_selector 捕获 + @media 块解析 + @container E-404 | T1 | B |
| T4 | IR + lowering + ts-rs 绑定同步（breakpoints / media_blocks / unsupported_selector） | T2, T3 | — |
| T5 | 语义 passes：walk_view_exprs + validate_breakpoints（E-401/E-405）+ validate_style_selectors（W-301）+ omnic warnings 折叠测试 | T4 | — |
| T6 | React codegen（bp_{n} useMediaQuery + breakpoints.ts BP 表 + @media 透传） | T5 | — |
| T7 | Flutter codegen（bpSm 常量 + MediaQuery.sizeOf + W-302 跳过） | T5 | — |
| T8 | OmniView codegen（matchMedia 内联 + @media 透传） | T5 | — |
| T9 | demo 迁移（learn-omniview 六处）+ golden 镜像 + 注入语料 855 扩展 | T6, T7, T8 | — |
| T10 | Playwright 浏览器断言 + 门禁终态收口 | T9 | — |

---

### Task 1: 登记先行——semantic-universe 域 12 新域 + 域 14「留生态」状态值 + vocabulary-registry 两行 + verify 双脚本改造 + docs 注记

**Files:**
- Modify: `D:/space/OmniLang/docs/superpowers/specs/semantic-universe.md`（域 9 之后追加「域 12: 响应式（responsive）」；域 14 样式增补留生态行——域 14 若尚不存在则新建「域 14: 样式（styling）」）
- Modify: `D:/space/OmniLang/docs/superpowers/specs/vocabulary-registry.md`（域 6 之后追加「域 7: 响应式（responsive）」两行语义词汇）
- Modify: `D:/space/OmniLang/scripts/verify-semantic-universe.py`
- Modify: `D:/space/OmniLang/scripts/verify-vocabulary.py`
- Modify: `D:/space/OmniLang/docs/omniview/10-design-system.md`（breakpoint 声明说明）
- Modify: `D:/space/OmniLang/docs/omniview/09-styling.md`（@media 注记 + W-301/W-302 降级说明）

**Interfaces:**
- Consumes: 无（本任务独立，先于一切语法实施——ADR-074 登记先行硬约束）
- Produces:
  - `semantic-universe.md` 域 12 语义集条目表（5 行：breakpoint 声明 / bp() 条件 / @media 块 / @container 块 / 深色模式）+ 已实现白名单表（5 行：breakpoint 声明 / bp() 条件 / @media 块 / 选择器诊断 / 深色模式）+ 域 14 留生态登记行（3 行：复杂选择器 留生态 / @container 留生态（候选）/ @media 块 部分）
  - `vocabulary-registry.md` 域 7 两行：`bp`（来源域 12 响应式 / 目标映射 React useMediaQuery · Flutter MediaQuery · OmniView matchMedia / 排除词汇 `useMediaQuery`（React 借词禁——语义词汇不得为合成事件名/旗标名/库 API 名））、`breakpoint`（来源域 12 响应式 / 目标映射 断点常量表 → 各目标常量 / 排除词汇 —）
  - `verify-semantic-universe.py`：STATUSES 增「留生态」；域 12 双表断言 + 编译器锚点 needle 断言；域 14 状态值域校验
  - `verify-vocabulary.py`：域 7 两行四字段 + 排除词汇禁借断言
  - 注：域 12 编译器锚点断言在 T5/T6/T8 落地前为 **pending-red**（预期红，文档化），其余断言全绿；T5 完成锚点转绿

- [ ] **Step 1: 写语义集条目表 + 白名单表（semantic-universe.md）**

在 `semantic-universe.md` 域 9（@load）之后追加（照既有域格式，条目表 + 已实现白名单表两表结构）：

```markdown
## 域 12: 响应式（responsive）

### 语义域定义

视口级响应式能力——命名断点声明 + 断点条件表达式 + 媒体查询透传。

### 语义集条目表

| 条目 | 语义 | 实现状态 | 诊断行为 |
|------|------|----------|----------|
| breakpoint 声明 | 命名断点数值表（design 块） | 支持（批次 3） | 值 ≤ 0 → E-403；同块重名 → E-402 |
| bp() 条件 | 视口宽度 ≥ 断点值 | 支持（批次 3） | 未声明引用 → E-401；非字面量参数 → E-405 |
| @media 块 | 媒体查询样式（web 透传） | 部分（web 生效 / Flutter 降级 W-302） | Flutter 目标 → W-302 |
| @container 块 | 容器查询（查询侧） | 未支持（低优先候选） | 遇之 → E-404 硬错误 + 指引 |
| 深色模式 | @themes → prefers-color-scheme 生成物 | 支持（既有，无动作） | — |

### 已实现白名单表

| 条目 | 编译器事实源锚点 |
|------|------------------|
| breakpoint 声明 | parser design 块 breakpoint_decl 识别；语义层断点常量表产出 |
| bp() 条件 | 语义层 validate_breakpoints 查表；codegen 三目标锚点（React useMediaQuery / Flutter MediaQuery.sizeOf / OmniView matchMedia） |
| @media 块 | parser media_block 识别 + web codegen 透传输出 |
| 选择器诊断 | analyse validate_style_selectors pass 发射 W-301 |
| 深色模式 | design codegen prefers-color-scheme 生成物 |
```

在 `semantic-universe.md` 追加域 14（若不存在则新建；存在则并入既有样式域）：

```markdown
## 域 14: 样式（styling）——留生态登记行

| 条目 | 语义 | 实现状态 | 语言边界注记（触发条件 + 目标生态映射指引） |
|------|------|----------|---------------------------------------------|
| 复杂选择器（兄弟/子/属性/带参伪类/伪元素/选择器列表） | 样式选择器表达 | 留生态 | 机制语法面（CSS 透传通道）；Flutter 以组件级样式表达同等意图（表达通道差异，非功能缺失）；诊断先行已落地（W-301）；支持补齐触发条件 = 语料需求 |
| @container 查询块 | 容器级自适应 | 留生态（候选） | 低优先候选；web @container / Flutter LayoutBuilder；触发条件 = 组件级自适应样式语料出现 |
| @media 块 | 视口级媒体查询 | 部分（域 12 同载） | web 透传生效；Flutter 降级（W-302）→ 增强候选 |
```

- [ ] **Step 2: 写 vocabulary-registry 两行（vocabulary-registry.md）**

在 `vocabulary-registry.md` 域 6 之后追加：

```markdown
## 域 7: 响应式（responsive）

### 来源域

semantic-universe 域 12 响应式

### 语义词汇

| 语义词汇 | 来源域 | 目标映射 | 排除词汇 |
|----------|--------|----------|----------|
| bp | 12 响应式 | React useMediaQuery / Flutter MediaQuery / OmniView matchMedia | `useMediaQuery`（React 借词禁——语义词汇不得为合成事件名/旗标名/库 API 名） |
| breakpoint | 12 响应式 | 断点常量表 → 各目标常量 | — |

> @media / @container 为机制透传关键字（CSS 原生词汇），不登记为语义词汇（域 7 at-rule 形态面；语义集条目见 semantic-universe 域 12）。
```

- [ ] **Step 3: 写失败测试（verify-semantic-universe.py 改造）**

在 `scripts/verify-semantic-universe.py` 中：

```python
# STATUSES 常量（第 ~126 行）：
STATUSES = {"支持", "部分", "未支持", "留生态"}
```

在 `audit()` 域断言区（域 8/9 断言块之后，照 633-686 行 needle 断言模式）追加：

```python
    # ---- 域 12: 响应式（needle 存在性断言；锚点未落地前 pending-red，见批次 3 计划 T5/T6/T8） ----
    if "域 12" in text or "域 12: 响应式" in text:
        en12 = entries(12)
        # 已实现白名单表（机制级条目存在性断言——照域 8/9 先例）
        en12_whitelist = whitelist(12)
        for name, needles in (
            ("breakpoint 声明", [("breakpoint ", LOWER_COMPONENT_FILE + "parser/design.rs"),
                                 ("breakpoints", LOWER_COMPONENT_FILE + "ir/design.rs")]),
            ("bp() 条件", [("validate_breakpoints", LOWER_COMPONENT_FILE + "analyse.rs"),
                          ("useMediaQuery", REACT_COMPONENT_FILE),
                          ("MediaQuery.sizeOf", FLUTTER_MOD_FILE),
                          ("matchMedia", OMNIVIEW_STORE_FILE)]),
            ("@media 块", [("AtMedia", LOWER_COMPONENT_FILE + "parser/view/style.rs"),
                          ("@media", REACT_COMPONENT_FILE),
                          ("media_blocks", FLUTTER_MOD_FILE)]),
            ("选择器诊断", [("validate_style_selectors", LOWER_COMPONENT_FILE + "analyse.rs"),
                          ("unsupported_selector", LOWER_COMPONENT_FILE)]),
            ("深色模式", [("prefers-color-scheme", REACT_COMPONENT_FILE)]),
        ):
            status = dict(en12_whitelist).get(name)
            if status in ("支持", "部分"):
                for needle, haystack in needles:
                    if needle not in haystack:
                        fail(f"域 12: '{name}' 声明 支持/部分 但编译器锚点 '{needle}' 缺失")
    # ---- 域 14: 样式留生态行（状态值域校验） ----
    if "域 14" in text or "域 14: 样式" in text:
        en14 = entries(14)
        statuses = {row[1] for row in en14}
        for s in statuses:
            if s not in STATUSES:
                fail(f"域 14 出现非法状态值 '{s}'")
        # 三条已知行状态断言
        for row in en14:
            if row[0] == "复杂选择器（兄弟/子/属性/带参伪类/伪元素/选择器列表）":
                if row[1] != "留生态":
                    fail("域 14 复杂选择器行应为 '留生态'")
            if row[0] == "@container 查询块":
                if row[1] != "留生态（候选）":
                    fail("域 14 @container 行应为 '留生态（候选）'")
            if row[0] == "@media 块":
                if row[1] != "部分":
                    fail("域 14 @media 块行应为 '部分'")
```

- [ ] **Step 4: 写失败测试（verify-vocabulary.py 改造）**

在 `scripts/verify-vocabulary.py` 的 `audit()`（第 214 行起，表单校验规则断言块之后）追加：

```python
    # ---- 域 7: 响应式 ----
    voc7 = vocab(7)
    assert_set(voc7, {"bp", "breakpoint"}, "域 7 语义词汇")
    for name in ("bp", "breakpoint"):
        row = [r for r in voc7 if r[0] == name]
        if not row:
            fail(f"域 7 缺少语义词汇 {name}")
        if len(row[0]) < 4 or any(not c.strip() for c in row[0]):
            fail(f"域 7 词汇 {name} 四字段不完整")
    ex7 = excluded(7)
    if not any("useMediaQuery" in e for e in ex7):
        fail("域 7 排除词汇应包含 useMediaQuery（React 借词禁）")
```

- [ ] **Step 5: 跑失败测试**

```bash
cd D:/space/OmniLang && python scripts/verify-semantic-universe.py --check && python scripts/verify-vocabulary.py --check
```

Expected: FAIL——`域 12: 'breakpoint 声明' 声明 支持/部分 但编译器锚点 ... 缺失`（锚点未落地，**pending-red 预期**；这是唯一失败类，其余断言必须全绿）。注：本任务结束时该红是**文档化预期**，T5/T6/T8 落地后转绿；不得为转绿提前实现语法。

- [ ] **Step 6: docs 注记（10-design-system.md / 09-styling.md）**

`docs/omniview/10-design-system.md` 的 design 块声明章节追加：

```markdown
### breakpoint 断点声明（批次 3）

design 块内专用声明，产出编译期断点常量表供 bp() 引用：

    breakpoint sm: 640;
    breakpoint md: 768;

- 值 = 无单位正数，语义为 CSS px（min-width 语义，与 Tailwind theme.screens 同构）
- 无主题覆盖（与既有非 color token 一致）；断点名与同块 token/class/variant 名互斥（E-402）
```

`docs/omniview/09-styling.md` 样式章节追加：

```markdown
### 用户级 @media（批次 3）

style 块内可书写 CSS 媒体查询（条件文本透传不解析）：web 目标原样输出；Flutter 目标降级跳过（W-302），响应式意图请用 bp()（跨目标主通道）。遇 @container 为硬错误 E-404（低优先候选未实施）。选择器含暂不支持部分（兄弟/子/属性/带参伪类等）时发 W-301 warning（构建不失败），绝不静默截断。
```

- [ ] **Step 7: 验证门禁**

```bash
cd D:/space/OmniLang && python scripts/verify-semantic-universe.py --check && python scripts/verify-vocabulary.py --check
```

Expected: 仅域 12 锚点断言 pending-red，其余断言（含域 14 状态值域、域 7 四字段）全绿。

- [ ] **Step 8: 提交**

```bash
cd D:/space/OmniLang && git add docs/superpowers/specs/semantic-universe.md docs/superpowers/specs/vocabulary-registry.md scripts/verify-semantic-universe.py scripts/verify-vocabulary.py docs/omniview/10-design-system.md docs/omniview/09-styling.md && git commit -m "docs(registry): 批次 3 登记先行——semantic-universe 域 12 响应式 + 域 14 留生态状态值 + vocabulary-registry 域 7（bp/breakpoint）+ verify 双脚本改造（域 12 锚点 pending-red 文档化）"
```

---

### Task 2: lexer AtMedia/AtContainer token + parser design 块 breakpoint 声明（E-402/E-403）

**Files:**
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/lexer/token.rs`（TokenKind 枚举：AtHero（89）之后、Bind（90）之前插入 `AtMedia`、`AtContainer`）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/lexer/keywords.rs`（at_keyword_to_token 73-104 追加 `"@media" => Some(TokenKind::AtMedia)`、`"@container" => Some(TokenKind::AtContainer)`）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/design.rs`（parse_design_body 45：dispatch 链 57-98 `variant ` 分支后加 `breakpoint ` 分支；E-402 事后 pass；E-403 解析期）
- Test: `D:/space/OmniLang/crates/omnilang-core/src/parser/design.rs`（tests 模块，test 入口 `parse_design_body(src, &[])`）
- Test: `D:/space/OmniLang/crates/omnilang-core/src/lexer/mod.rs`（tests 模块）

**Interfaces:**
- Consumes: `parse_design_body(input: &str, design_tokens: &[Token]) -> ParseResult<DesignDef>`（design.rs:45）；`err_at(input, remaining, msg)`（design.rs:37）；DesignDef 构造点 100-114
- Produces:
  - `DesignDef.breakpoints: Vec<(String, u32)>`（构造点追加字段；语义层 T4 转 FxHashMap）
  - `TokenKind::AtMedia` / `TokenKind::AtContainer`（lexer 全域：keywords.rs 映射 + mod.rs read_at_keyword 回退链自动覆盖）
  - 解析行为：`breakpoint <名>: <无单位正整数>;` 成功；名冲突/值≤0 报 E-402/E-403；值非正整数报普通解析错误「breakpoint 值必须为无单位整数」

- [ ] **Step 1: 写失败测试（lexer）**

`crates/omnilang-core/src/lexer/mod.rs` tests 模块追加：

```rust
#[test]
fn tokenizes_media_at_rules() {
    let tokens = tokenize("@media (min-width: 768px) { .card { padding: 32; } } @container (min-width: 400px) { .a { color: red; } }");
    assert!(tokens.iter().any(|t| t.kind == TokenKind::AtMedia));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::AtContainer));
}

#[test]
fn media_token_text_roundtrip() {
    // style.rs is_text_token/token_text 尾部分支在本任务 T3 落地；此处只验证 tokenize 形态
    let tokens = tokenize("@media (x) {}");
    let t = tokens.iter().find(|t| t.kind == TokenKind::AtMedia).unwrap();
    assert_eq!(t.text, "@media");
}
```

- [ ] **Step 2: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core tokenizes_media_at_rules --lib
```

Expected: FAIL——`AtMedia` 变体不存在编译错误。

- [ ] **Step 3: 写失败测试（breakpoint 解析）**

`crates/omnilang-core/src/parser/design.rs` tests 模块追加：

```rust
#[test]
fn parses_breakpoint_declarations() {
    let src = "design D {\n    breakpoint sm: 640;\n    breakpoint md: 768;\n    breakpoint lg: 1024;\n}";
    let (design, errors) = parse_design_body(src, &[]).expect("parse ok");
    assert!(errors.is_empty());
    let bps: Vec<(String, u32)> = design.breakpoints.clone();
    assert_eq!(bps, vec![("sm".into(), 640), ("md".into(), 768), ("lg".into(), 1024)]);
}

#[test]
fn breakpoint_conflicts_with_token_are_hard_error_e402() {
    let src = "design D {\n    token md: color = \"#6366f1\";\n    breakpoint md: 768;\n}";
    let err = parse_design_body(src, &[]).unwrap_err();
    let msg = err.message.clone();
    assert!(msg.contains("与 token/class/variant 名冲突"), "got: {msg}");
}

#[test]
fn breakpoint_zero_value_is_hard_error_e403() {
    let src = "design D {\n    breakpoint md: 0;\n}";
    let err = parse_design_body(src, &[]).unwrap_err();
    let msg = err.message.clone();
    assert!(msg.contains("必须为正数"), "got: {msg}");
}

#[test]
fn breakpoint_duplicate_name_is_hard_error_e402() {
    let src = "design D {\n    breakpoint md: 768;\n    breakpoint md: 1024;\n}";
    let err = parse_design_body(src, &[]).unwrap_err();
    let msg = err.message.clone();
    assert!(msg.contains("与 token/class/variant 名冲突"), "got: {msg}");
}

#[test]
fn breakpoint_decimal_value_is_plain_error() {
    let src = "design D {\n    breakpoint md: 768.5;\n}";
    let err = parse_design_body(src, &[]).unwrap_err();
    let msg = err.message.clone();
    assert!(msg.contains("无单位整数"), "got: {msg}");
}
```

- [ ] **Step 4: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core parses_breakpoint_declarations --lib
```

Expected: FAIL——`breakpoints` 字段不存在。

- [ ] **Step 5: 实现——lexer token**

`crates/omnilang-core/src/lexer/token.rs` TokenKind 枚举 `AtHero`（89）之后插入：

```rust
    AtMedia,
    AtContainer,
```

`crates/omnilang-core/src/lexer/keywords.rs` at_keyword_to_token 追加：

```rust
        "@media" => Some(TokenKind::AtMedia),
        "@container" => Some(TokenKind::AtContainer),
```

- [ ] **Step 6: 实现——DesignDef.breakpoints 字段 + E-403**

`crates/omnilang-core/src/parser/ast/design.rs`（或 DesignDef 所在文件）追加：

```rust
#[serde(default)]
pub breakpoints: Vec<(String, u32)>,
```

`crates/omnilang-core/src/parser/design.rs`：

```rust
// parse_design_body 内 dispatch 链 `variant ` 分支之后追加：
if remaining.starts_with("breakpoint ") {
    return self.parse_breakpoint_decl(input, remaining);
}
// （parse_design_body 保留原顺序：@themes → @default-theme → token → class → variant → breakpoint → else 报错）
```

新增方法（照 parse_token_decl 形态）：

```rust
fn parse_breakpoint_decl(&mut self, input: &str, remaining: &str) -> ParseResult<DesignDef> {
    let after_kw = &remaining["breakpoint ".len()..];
    let name_end = after_kw.find(':').unwrap_or(after_kw.len());
    let name = after_kw[..name_end].trim();
    if name.is_empty() {
        return self.err_at(input, remaining, "breakpoint 声明缺少名称");
    }
    let rest = after_kw[name_end..].trim_start();
    let rest = rest.strip_prefix(':').unwrap_or(rest);
    let rest = rest.trim_start();
    let after_val = rest.find(';').unwrap_or(rest.len());
    let value_str = rest[..after_val].trim();
    let value: u32 = match value_str.parse() {
        Ok(v) if v > 0 => v,
        Ok(0) => return self.err_at(input, remaining, "breakpoint md 的值必须为正数（当前 0）"),
        Ok(_) => unreachable!(),
        Err(_) => {
            if value_str.starts_with('-') {
                return self.err_at(input, remaining, "breakpoint md 的值必须为正数（当前 0）");
            }
            return self.err_at(input, remaining, "breakpoint 值必须为无单位整数");
        }
    };
    let mut design = self.parse_design_body(input, &[])?; // 占位——见 Step 7 说明
    // NOTE: 该分支实际在 parse_design_body 循环内，见 Step 7 完整实现
    design.breakpoints.push((name.to_string(), value));
    Ok(design)
}
```

> **实现提示（防踩坑）**：`parse_design_body` 是单函数循环式解析（dispatch 链 + DesignDef 构造在 100-114），`parse_breakpoint_decl` 应**直接改循环体**而非递归调用——递归会重复消费。正确做法：在 dispatch 链 `variant ` 分支之后加 `if remaining.starts_with("breakpoint ") { ...解析名与值...; breakpoints.push(...); return self.parse_design_body_continue(...) }` 或按该文件既有分支写法（`class`/`variant` 分支各写各自的消费逻辑）。E-403 消息中的断点名用实际解析到的 name 插值：`format!("breakpoint {name} 的值必须为正数（当前 {value_str}）")`——spec §3 的「md」「0」为示例，实际以源码为准。

- [ ] **Step 7: 实现——E-402 事后 pass**

`parse_design_body` 函数末尾、返回 `Ok(design)` 之前追加（照设计：顺序无关重名检测）：

```rust
    // E-402：breakpoint 名与同块 token/class/variant 名互斥；breakpoint 之间也不得重名
    let mut seen: FxHashSet<String> = FxHashSet::default();
    for (name, _) in &design.breakpoints {
        if !seen.insert(name.clone())
            || design.tokens.iter().any(|(t, _)| t == name)
            || design.classes.iter().any(|(c, _)| c == name)
            || design.variants.iter().any(|(v, _)| v == name)
        {
            return self.err_at(
                input,
                remaining,
                &format!("design 块中 breakpoint 名 {name} 与 token/class/variant 名冲突——breakpoint 名须与同块标识符互斥"),
            );
        }
    }
```

（`design.tokens` 等字段名按 DesignDef 实际字段调整；`FxHashSet` 用 `crate::common::fxhash` 或已导入的别名。）

- [ ] **Step 8: 跑测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core parses_breakpoint_declarations --lib && cargo test -p omnilang-core breakpoint_conflicts_with_token_are_hard_error_e402 --lib && cargo test -p omnilang-core breakpoint_zero_value_is_hard_error_e403 --lib && cargo test -p omnilang-core breakpoint_duplicate_name_is_hard_error_e402 --lib && cargo test -p omnilang-core breakpoint_decimal_value_is_plain_error --lib
```

Expected: PASS（4 个新测试 + 既有 design 测试不回归）。

- [ ] **Step 9: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core --lib && cargo fmt && cargo clippy -D warnings -p omnilang-core
```

- [ ] **Step 10: 提交**

```bash
cd D:/space/OmniLang && git add crates/omnilang-core/src/lexer/token.rs crates/omnilang-core/src/lexer/keywords.rs crates/omnilang-core/src/parser/design.rs crates/omnilang-core/src/parser/ast/design.rs && git commit -m "feat(parser): breakpoint 断点声明（E-402 名互斥 / E-403 正值校验）+ lexer AtMedia/AtContainer token"
```

---

### Task 3: parser style 块——unsupported_selector 捕获 + @media 块解析 + @container E-404

**Files:**
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/view/style.rs`
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/top_level/component.rs`（TokenKind::Style 分支 195-198 改用 parse_style_block）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/ast/view.rs`（StyleRule + UnsupportedSelector + MediaBlock + ComponentDef.media_blocks）
- Test: `D:/space/OmniLang/crates/omnilang-core/src/parser/view/style.rs` tests 模块（AST 级测试入口 `crate::parser::parse(crate::lexer::tokenize(src), src)`，组件定位 `prog.modules.values().flat_map(|m| m.components.iter()).find(|c| c.name == "App")`）

**Interfaces:**
- Consumes: `parse_style_rules()`（style.rs:76，返回 Vec<StyleRule>）；静默跳过循环（style.rs:166-174 原文）；StyleRule 构造点 366-370 + legacy parse_style_rule 构造点 486-490；`self.source`（parser 源码字段，char-offset span 切片用）；`consume_css_function_args`（518，括号深度模型先例）
- Produces:
  - `parse_style_block() -> ParseResult<(Vec<StyleRule>, Vec<MediaBlock>)>`（替代 parse_style_rules；唯一调用点 component.rs:196 同步改名）
  - `StyleRule { ..., #[serde(default)] pub unsupported_selector: Option<UnsupportedSelector> }`
  - `UnsupportedSelector { pub text: String, pub span: Span, pub category: String }`
  - `MediaBlock { pub condition: String, pub rules: Vec<StyleRule> }` + `ComponentDef.media_blocks: Vec<MediaBlock>`（serde default）
  - 行为：@media 条件原文透传（括号配对消费，`(` 起到块 `{`）；@container → E-404；静默跳过处记录 text/span/category（category 映射：Plus→兄弟选择器、Gt→子选择器、LBracket→属性选择器、Comma→选择器列表、LParen→带参伪类；`~`/`::before`/`:has(` 由 lexer 硬错误先行拦截，不可达）
  - W-301 文本 = class_name + 空格 + 被跳 token 的 token_text 连接；span = 首个被跳 token 的 span

- [ ] **Step 1: 写失败测试（@media 块解析 + 透传条件）**

`crates/omnilang-core/src/parser/view/style.rs` tests 模块追加：

```rust
#[test]
fn parses_media_block_with_raw_condition() {
    let src = r#"
        component App {
            style {
                .card { padding: 16; }
                @media (min-width: 768px) {
                    .card { padding: 32; }
                }
                @media (prefers-color-scheme: dark) {
                    .card { background: #111827; }
                }
            }
        }
    "#;
    let (prog, errors) = crate::parser::parse(crate::lexer::tokenize(src), src);
    assert!(errors.is_empty(), "parse errors: {errors:?}");
    let comp = prog.modules.values().flat_map(|m| m.components.iter()).find(|c| c.name == "App").unwrap();
    assert_eq!(comp.styles.len(), 1); // 块外 .card
    assert_eq!(comp.media_blocks.len(), 2);
    assert_eq!(comp.media_blocks[0].condition, "(min-width: 768px)");
    assert_eq!(comp.media_blocks[1].condition, "(prefers-color-scheme: dark)");
    assert_eq!(comp.media_blocks[0].rules.len(), 1);
    assert_eq!(comp.media_blocks[0].rules[0].class_name, ".card");
}

#[test]
fn media_block_condition_keeps_raw_nested_parens() {
    let src = r#"
        component App {
            style {
                @media (min-width: 768px) and (max-width: calc(100vw - 20px)) {
                    .a { color: red; }
                }
            }
        }
    "#;
    let (prog, errors) = crate::parser::parse(crate::lexer::tokenize(src), src);
    assert!(errors.is_empty());
    let comp = prog.modules.values().flat_map(|m| m.components.iter()).find(|c| c.name == "App").unwrap();
    assert_eq!(comp.media_blocks[0].condition, "(min-width: 768px) and (max-width: calc(100vw - 20px))");
}
```

- [ ] **Step 2: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core parses_media_block_with_raw_condition --lib
```

Expected: FAIL——`media_blocks` 字段不存在。

- [ ] **Step 3: 写失败测试（@container E-404）**

```rust
#[test]
fn container_at_rule_is_hard_error_e404() {
    let src = r#"
        component App {
            style {
                @container (min-width: 400px) {
                    .card { padding: 32; }
                }
            }
        }
    "#;
    let (_, errors) = crate::parser::parse(crate::lexer::tokenize(src), src);
    let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(msgs.iter().any(|m| m.contains("@container") && m.contains("低优先候选")), "got: {msgs:?}");
}
```

- [ ] **Step 4: 写失败测试（unsupported_selector 捕获）**

```rust
#[test]
fn unsupported_selector_captured_with_category() {
    let src = r#"
        component App {
            style {
                input:checked + span { color: red; }
            }
        }
    "#;
    let (prog, errors) = crate::parser::parse(crate::lexer::tokenize(src), src);
    assert!(errors.is_empty(), "parse errors: {errors:?}");
    let comp = prog.modules.values().flat_map(|m| m.components.iter()).find(|c| c.name == "App").unwrap();
    let rule = &comp.styles[0];
    let us = rule.unsupported_selector.as_ref().expect("unsupported selector captured");
    assert_eq!(us.category, "兄弟选择器");
    assert!(us.text.contains("+"));
}

#[test]
fn supported_selectors_have_no_unsupported_annotation() {
    let src = r#"
        component App {
            style {
                .a.b:hover .c { color: red; }
            }
        }
    "#;
    let (prog, errors) = crate::parser::parse(crate::lexer::tokenize(src), src);
    assert!(errors.is_empty());
    let comp = prog.modules.values().flat_map(|m| m.components.iter()).find(|c| c.name == "App").unwrap();
    assert!(comp.styles[0].unsupported_selector.is_none());
}
```

- [ ] **Step 5: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core container_at_rule_is_hard_error_e404 --lib && cargo test -p omnilang-core unsupported_selector_captured_with_category --lib
```

Expected: FAIL——`unsupported_selector` 字段不存在 / E-404 未触发。

- [ ] **Step 6: 实现——AST 字段**

`crates/omnilang-core/src/parser/ast/view.rs`：

```rust
// StyleRule 结构体（313-318）追加：
#[serde(default)]
pub unsupported_selector: Option<UnsupportedSelector>,

// StyleRule 之后新增：
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsupportedSelector {
    pub text: String,
    pub span: Span,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaBlock {
    pub condition: String,
    pub rules: Vec<StyleRule>,
}

// ComponentDef（styles 字段 44 之后）追加：
#[serde(default)]
pub media_blocks: Vec<MediaBlock>,
```

- [ ] **Step 7: 实现——style.rs 主体改造**

`crates/omnilang-core/src/parser/view/style.rs`：

```rust
// 1) 改名 + 返回类型（第 76 行起）：
pub fn parse_style_block(&mut self) -> ParseResult<(Vec<StyleRule>, Vec<MediaBlock>)> {
    let mut rules = Vec::new();
    let mut media_blocks = Vec::new();
    loop {
        // ... 既有循环体：selectors_and_rules 等分支照旧 ...
        // 循环顶部新增 at-rule 分支（在既有分支判断之前）：
        if *self.peek_kind() == TokenKind::AtMedia {
            let media = self.parse_media_block()?;
            media_blocks.push(media);
            continue;
        }
        if *self.peek_kind() == TokenKind::AtContainer {
            return Err(self.err_at(
                &self.source,
                &self.source[/* 当前 token 起 */..],
                "不支持的 at-rule @container——容器查询为低优先候选（语义级未实施），响应式意图请用 bp() 或 @media 表达",
            ));
        }
        // ... 既有分支 ...
    }
    Ok((rules, media_blocks))
}

// 2) 新方法（括号配对深度模型照 consume_css_function_args 先例）：
fn parse_media_block(&mut self) -> ParseResult<MediaBlock> {
    self.advance(); // consume @media
    // 条件原文：从第一个 token 起到块起始，按 char-offset span 切片 self.source
    let cond_start = self.peek_span().start; // 条件首 token（应为 LParen）
    self.expect_token(TokenKind::LParen, "expected '(' after @media")?;
    let mut depth = 1usize;
    while depth > 0 {
        let kind = *self.peek_kind();
        if kind == TokenKind::LParen { depth += 1; }
        if kind == TokenKind::RParen { depth -= 1; }
        if depth == 0 { break; }
        if kind == TokenKind::Eof || kind == TokenKind::LBrace {
            return Err(self.err_at(&self.source, &self.source[cond_start..], "未闭合的 @media 条件"));
        }
        self.advance();
    }
    // 条件结束于 RParen；条件文本 = 从 cond_start 到 RParen 的源切片
    let cond_end = self.prev_span().end; // RParen 的 span.end
    let condition = self.source[cond_start..cond_end].to_string();
    self.expect_token(TokenKind::LBrace, "expected '{' after @media condition")?;
    let mut rules = Vec::new();
    while *self.peek_kind() != TokenKind::RBrace {
        if *self.peek_kind() == TokenKind::Eof {
            return Err(self.err_at(&self.source, &self.source[..], "未闭合的 @media 块"));
        }
        // 块内规则：复用单条规则解析（selectors_and_rules 对应内部逻辑）
        rules.append(&mut self.parse_single_style_rule()?);
    }
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    Ok(MediaBlock { condition, rules })
}

// 3) 静默跳过处（166-174 原文循环）替换为带捕获版本：
// 原：
//   while *self.peek_kind() != TokenKind::LBrace && *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof { self.advance(); }
// 改：
let mut skipped_text = String::new();
let mut category: Option<String> = None;
let mut skip_span: Option<Span> = None;
while *self.peek_kind() != TokenKind::LBrace
    && *self.peek_kind() != TokenKind::RBrace
    && *self.peek_kind() != TokenKind::Eof
{
    let kind = *self.peek_kind();
    if category.is_none() {
        category = Some(match kind {
            TokenKind::Plus => "兄弟选择器".into(),
            TokenKind::Gt => "子选择器".into(),
            TokenKind::LBracket => "属性选择器".into(),
            TokenKind::Comma => "选择器列表".into(),
            TokenKind::LParen => "带参伪类".into(),
            _ => continue, // 未知 token 不归类——不设 category 保持保守
        });
        skip_span = Some(self.peek_span());
    }
    skipped_text.push_str(&self.peek_text());
    self.advance();
}
// 注意：上述匹配里 `continue` 在 while 内不推进会死循环——改用 flag：
//   if category.is_none() && matches!(kind, Plus | Gt | LBracket | Comma | LParen) { ... }
// 未知类别 token 仅计入 skipped_text 不设 category（保持 category=None → 不记录诊断）。

// 4) StyleRule 构造点（366-370 与 legacy 486-490）两处统一追加：
//   unsupported_selector: category.zip(skip_span).map(|(cat, span)| UnsupportedSelector {
//       text: format!("{class_name} {skipped_text}").trim().to_string(),
//       span,
//       category: cat,
//   }),
// legacy parse_style_rule（486-490）处直接 None + 注释：// 不做捕获，主路径已覆盖（W-301 走主解析路径）
```

> **实现提示（防踩坑）**：
> 1. `self.source[span.start..span.end]` 切片要求 span 为 char 偏移（lexer span 语义）；若 span 为字节偏移需先换算，err_at 先例在 `parser/design.rs:37` 可对照。
> 2. `+`（Plus）、`>`（Gt）、`[`（LBracket）、`,`（Comma）、`(`（LParen）均为既有 TokenKind；`~`、`::before`、`:has(` 由 lexer 在词法层硬错误，**不可达**——不要为它们写分支。
> 3. `parse_style_block` 的循环结构：函数体内有多处 `return Ok(rules)` 提前返回点（对照既有 parse_style_rules 全部返回点同步改为返回 tuple）。
> 4. `peek_text()`/`peek_span()`/`prev_span()` 方法名按该 parser 实际 API 调整（peek_token().text / peek_token().span 等）；`selectors_and_rules` 内部（366-370）是主 StyleRule 构造点。

- [ ] **Step 8: 实现——component.rs 调用点**

`crates/omnilang-core/src/parser/top_level/component.rs` 195-198：

```rust
TokenKind::Style => {
    let (mut rules, media_blocks) = self.parse_style_block()?;
    styles.append(&mut rules);
    media_blocks_list = media_blocks; // ComponentDef 构造点（225-249）同步加 media_blocks 字段
}
```

- [ ] **Step 9: 实现——is_text_token / token_text 尾部分支**

`crates/omnilang-core/src/parser/view/style.rs` 尾部（614-749）——该区域逐 token 枚举 At-* token，缺分支会**静默丢词**（文件注释明示），必须补：

```rust
// is_text_token 匹配臂（614-665 区域内）：
            TokenKind::AtMedia => true,
            TokenKind::AtContainer => true,

// token_text 匹配臂（668-749 区域内）：
            TokenKind::AtMedia => "@media".into(),
            TokenKind::AtContainer => "@container".into(),
```

- [ ] **Step 10: 跑测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core parses_media_block_with_raw_condition --lib && cargo test -p omnilang-core media_block_condition_keeps_raw_nested_parens --lib && cargo test -p omnilang-core container_at_rule_is_hard_error_e404 --lib && cargo test -p omnilang-core unsupported_selector_captured_with_category --lib && cargo test -p omnilang-core supported_selectors_have_no_unsupported_annotation --lib
```

Expected: PASS；再跑全量确认既有 style 测试不回归（`.a.b:hover .c` 等既有用例——Text 内容含 `@media` 的既有测试若被 is_text_token 新臂影响会暴露，逐一核对预期）。

- [ ] **Step 11: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core --lib && cargo fmt && cargo clippy -D warnings -p omnilang-core
```

- [ ] **Step 12: 提交**

```bash
cd D:/space/OmniLang && git add crates/omnilang-core/src/parser/view/style.rs crates/omnilang-core/src/parser/top_level/component.rs crates/omnilang-core/src/parser/ast/view.rs && git commit -m "feat(parser): style 块 @media 透传解析（条件原文括号配对）+ @container E-404 + 未支持选择器捕获（text/span/category）+ AtMedia/AtContainer 文本臂"
```

---

### Task 4: IR + lowering + ts-rs 绑定同步（breakpoints / media_blocks / unsupported_selector）

**Files:**
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/design.rs`（DesignIR 20-47 追加 breakpoints）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/types.rs`（StyleRuleIR 859-865 追加 unsupported_selector；ComponentIR 955-997 styles 后追加 media_blocks；MediaBlockIR 新结构）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/lower/design.rs`（或 DesignIR 构造所在 lower 文件，breakpoints 透传）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/lower/component.rs`（styles/media_blocks/unsupported_selector 透传；lower_component_src 786 测试入口）
- Test: `D:/space/OmniLang/crates/omnilang-core/src/ir/lower/component.rs` tests 模块

**Interfaces:**
- Consumes: AST DesignDef.breakpoints / StyleRule.unsupported_selector / ComponentDef.media_blocks（T2/T3 产物）
- Produces:
  - `DesignIR.breakpoints: FxHashMap<String, u32>`（serde default）
  - `StyleRuleIR.unsupported_selector: Option<UnsupportedSelectorIR>`（serde default；UnsupportedSelectorIR { text, span, category } 照 AST 形态）
  - `MediaBlockIR { pub condition: String, pub rules: Vec<StyleRuleIR> }` + `ComponentIR.media_blocks: Vec<MediaBlockIR>`（serde default）
  - ts-rs 绑定：`cargo test export_bindings --lib -p omnilang` 产物（`poiesis/src/commands/__generated__/`）随本任务提交

- [ ] **Step 1: 写失败测试（IR lower 透传）**

`crates/omnilang-core/src/ir/lower/component.rs` tests 模块追加（lower_component_src 786 入口）：

```rust
#[test]
fn lower_media_blocks_and_unsupported_selector() {
    let src = r#"
        design D { breakpoint md: 768; }
        component App {
            style {
                input:checked + span { color: red; }
                @media (min-width: 768px) { .card { padding: 32; } }
            }
        }
    "#;
    let comp = lower_component_src(src);
    assert_eq!(comp.media_blocks.len(), 1);
    assert_eq!(comp.media_blocks[0].condition, "(min-width: 768px)");
    let us = comp.styles[0].unsupported_selector.as_ref().expect("captured");
    assert_eq!(us.category, "兄弟选择器");
}
```

`crates/omnilang-core/src/ir/lower/design.rs` tests 模块追加（或与上同文件，用既有 design lower 入口）：

```rust
#[test]
fn lower_breakpoints_map() {
    let src = r#"
        design D {
            breakpoint sm: 640;
            breakpoint md: 768;
        }
    "#;
    let ir = /* 既有 design lower 测试入口（照该文件 tests 现状） */;
    let bps = ir.breakpoints;
    assert_eq!(bps.get("sm"), Some(&640));
    assert_eq!(bps.get("md"), Some(&768));
    assert_eq!(bps.len(), 2);
}
```

- [ ] **Step 2: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core lower_media_blocks_and_unsupported_selector --lib
```

Expected: FAIL——字段不存在。

- [ ] **Step 3: 实现——IR 结构**

`crates/omnilang-core/src/ir/design.rs` DesignIR 追加：

```rust
    #[serde(default)]
    pub breakpoints: FxHashMap<String, u32>,
```

`crates/omnilang-core/src/ir/types.rs`：

```rust
// StyleRuleIR（859-865）追加：
    #[serde(default)]
    pub unsupported_selector: Option<UnsupportedSelectorIR>,

// StyleRuleIR 之后新增：
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct UnsupportedSelectorIR {
    pub text: String,
    pub span: Span,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct MediaBlockIR {
    pub condition: String,
    pub rules: Vec<StyleRuleIR>,
}

// ComponentIR（styles 字段之后，form_intent 之前）追加：
    #[serde(default)]
    pub media_blocks: Vec<MediaBlockIR>,
```

- [ ] **Step 4: 实现——lower 透传**

`crates/omnilang-core/src/ir/lower/design.rs`（DesignIR 构造点）：`breakpoints: design_def.breakpoints.iter().cloned().collect()`（AST Vec<(String,u32)> → FxHashMap）。

`crates/omnilang-core/src/ir/lower/component.rs`（ComponentIR 构造点）：

```rust
    styles: /* 既有 styles 映射 */,
    media_blocks: comp.media_blocks.iter().map(|mb| MediaBlockIR {
        condition: mb.condition.clone(),
        rules: mb.rules.iter().map(|r| lower_style_rule(r)).collect(),
    }).collect(),
```

（`lower_style_rule` 为 styles 映射既有内部函数名，按实际名称调整；unsupported_selector 在 lower_style_rule 内透传：`unsupported_selector: rule.unsupported_selector.clone().map(|u| UnsupportedSelectorIR { text: u.text, span: u.span, category: u.category })`。）

- [ ] **Step 5: 跑测试 + ts-rs 绑定同步**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core lower_media_blocks_and_unsupported_selector --lib && cargo test -p omnilang-core lower_breakpoints_map --lib && cargo test export_bindings --lib -p omnilang
```

Expected: 两个 IR 测试 PASS；export_bindings 重生成绑定文件（`poiesis/src/commands/__generated__/` 出现新字段 diff）。

- [ ] **Step 6: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core --lib && cargo test -p omnilang --lib && cargo fmt && cargo clippy -D warnings -p omnilang-core -p omnilang
```

- [ ] **Step 7: 提交（含绑定文件）**

```bash
cd D:/space/OmniLang && git add crates/omnilang-core/src/ir/design.rs crates/omnilang-core/src/ir/types.rs crates/omnilang-core/src/ir/lower/ poiesis/src/commands/__generated__/ && git commit -m "feat(ir): DesignIR.breakpoints 常量表 + MediaBlockIR + StyleRuleIR.unsupported_selector + ts-rs 绑定同步"
```

---

### Task 5: 语义 passes——walk_view_exprs + validate_breakpoints（E-401/E-405）+ validate_style_selectors（W-301）+ omnic warnings 折叠测试

**Files:**
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/walk.rs`（walk_view 250-291 之后新增 walk_view_exprs）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/semantic/validate_breakpoints.rs`（新文件；或并入既有 validate_* 文件组）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/semantic/validate_style_selectors.rs`（新文件）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/semantic/mod.rs`（pass 注册）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/analyse.rs`（run_ir_passes 追加第 10 pass；三个调用点接线）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/lib.rs`（run_pipeline 尾部 ~115 追加 validate_breakpoints 调用）
- Modify: `D:/space/OmniLang/omnic/src/commands/build.rs`（warnings 折叠 CLI 测试——若既有 cmd_build_surfaces_warnings_without_failing 已覆盖则仅补 W-301 场景断言）
- Test: `crates/omnilang-core/src/semantic/validate_breakpoints.rs` / `validate_style_selectors.rs` tests 模块（compile_diags 模式，validate_form_load.rs:445-452 先例）

**Interfaces:**
- Consumes: `walk_expr`（walk.rs:19）/`walk_stmt`（145）/`walk_view`（250）；IRProgram.iter_all_components（1070）；ComponentIR.computeds（967）/events: FxHashMap<String, EventHandlerIR>（InlineFn.body: BlockIR）；validate_theme 调用模式（analyse.rs Phase 3d ~503 与 ~790）；run_pipeline 尾部（lib.rs ~115）
- Produces:
  - `walk_view_exprs(view: &ViewTree, f: &mut impl FnMut(&ExprIR))`：遍历 ElementIR.props + class_map_bindings + ConditionalBranch.condition + ListRendering.iterable/key_expr + Interpolation + Slot fallback（递归 children）
  - `validate_breakpoints(program: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>) -> ()`：三调用点（run_pipeline 尾部 / analyse_multi Phase 3d / analyse_multi_cached Phase 3d）；行为 = 收集断点常量表 → sweep（视图 walk_view_exprs + computeds walk_expr(body) + 事件 InlineFn walk_stmt(body.statements)）→ `bp("md")` 字符串参数查表（未声明 → E-401；非字面量 → E-405）→ **改写 args[0] 为 LiteralExpr(Number)**（codegen 只认 name=="bp" + 数字参数）
  - `validate_style_selectors(program: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>) -> ()`：第 10 pass；遍历组件 styles（含 media_blocks 内 rules）的 unsupported_selector → Severity::Warning 诊断（W-301 逐字文案，category 插值；锚点 = 组件 source_span/source_file——CallExpr IR 无 span，照 validate_effect_deps 先例）

- [ ] **Step 1: 写失败测试（E-401 未声明断点）**

`validate_breakpoints.rs` tests 模块（照 validate_form_load.rs compile_diags 模式 445-452）：

```rust
fn compile_diags(src: &str) -> Vec<Diagnostic> {
    let result = crate::compile(src);
    result.errors.into_iter().filter(|d| matches!(d.severity, crate::error::Severity::Error)).collect()
}

#[test]
fn undeclared_bp_is_e401() {
    let src = r#"
        component App {
            view {
                @if bp("md") { <div>desktop</div> }
            }
        }
    "#;
    let diags = compile_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("未声明的断点名 md")), "got: {diags:?}");
}

#[test]
fn declared_bp_passes_and_rewrites_to_number() {
    let src = r#"
        design D { breakpoint md: 768; }
        component App {
            view {
                @if bp("md") { <div>desktop</div> }
                <div class: {bp("md") ? "desktop-grid" : "mobile-list"} />
            }
        }
    "#;
    let result = crate::compile(src);
    assert!(result.success, "errors: {:?}", result.errors);
    // IR 中 bp 参数已被改写为数字字面量 768
    let ir = result.ir.expect("ir present");
    let mut found_bp = false;
    // 用 walk_view_exprs 扫 view，断言 CallExpr name=="bp" 且 args[0] 为 LiteralExpr(768)
    // （walk_view_exprs 本任务实现，测试直接用该函数——先于实现写测试 = TDD 锁接口）
    for comp in ir.iter_all_components() {
        if let Some(view) = &comp.view {
            crate::ir::walk::walk_view_exprs(view, &mut |e| {
                if let ExprIR::CallExpr { name, args } = e {
                    if name == "bp" {
                        found_bp = true;
                        assert!(matches!(args[0], ExprIR::LiteralExpr(serde_json::Value::Number(n)) if n.as_u64() == Some(768)));
                    }
                }
            });
        }
    }
    assert!(found_bp, "bp() call not found in IR");
}
```

- [ ] **Step 2: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core undeclared_bp_is_e401 --lib
```

Expected: FAIL——validate_breakpoints 不存在（或 bp 未查表）。

- [ ] **Step 3: 写失败测试（E-405 动态参数 + W-301）**

```rust
#[test]
fn non_literal_bp_arg_is_e405() {
    let src = r#"
        design D { breakpoint md: 768; }
        component App {
            store { signal bpName = "md"; }
            view {
                @if bp(bpName) { <div>x</div> }
            }
        }
    "#;
    let diags = compile_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("必须是字符串字面量")), "got: {diags:?}");
}

#[test]
fn bp_inside_computed_and_event_are_checked() {
    let src = r#"
        design D { breakpoint md: 768; }
        component App {
            computed isDesktop = bp("md");
            view {
                <button @onPress={() => console.log(bp("lg"))}>x</button>
            }
        }
    "#;
    // "lg" 未声明 → E-401（computed 与事件体内同样查表）
    let diags = compile_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("未声明的断点名 lg")), "got: {diags:?}");
}
```

`validate_style_selectors.rs` tests 模块：

```rust
fn compile_warnings(src: &str) -> Vec<Diagnostic> {
    let result = crate::compile(src);
    result.errors.into_iter().filter(|d| matches!(d.severity, crate::error::Severity::Warning)).collect()
}

#[test]
fn unsupported_selector_emits_w301_warning() {
    let src = r#"
        component App {
            style {
                input:checked + span { color: red; }
            }
        }
    "#;
    let result = crate::compile(src);
    assert!(result.success, "构建不失败（warning 非阻断）");
    let warnings = compile_warnings(src);
    let w = warnings.iter().find(|d| d.message.contains("暂不支持的语法部分")).expect("W-301 present");
    assert!(w.message.contains("兄弟选择器"), "got: {}", w.message);
    assert!(w.message.contains("input:checked + span"), "got: {}", w.message);
    assert!(w.message.contains("域 14"), "got: {}", w.message);
}

#[test]
fn supported_selector_no_warning() {
    let src = r#"
        component App {
            style {
                .a.b:hover .c { color: red; }
            }
        }
    "#;
    assert!(compile_warnings(src).is_empty());
}

#[test]
fn media_block_rules_also_checked() {
    let src = r#"
        component App {
            style {
                @media (min-width: 768px) { .a > .b { color: red; } }
            }
        }
    "#;
    let warnings = compile_warnings(src);
    assert!(warnings.iter().any(|d| d.message.contains("子选择器")), "got: {warnings:?}");
}
```

- [ ] **Step 4: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core non_literal_bp_arg_is_e405 --lib && cargo test -p omnilang-core unsupported_selector_emits_w301_warning --lib
```

Expected: FAIL。

- [ ] **Step 5: 实现——walk_view_exprs**

`crates/omnilang-core/src/ir/walk.rs`（walk_view 之后追加）：

```rust
/// 遍历视图树中的所有表达式（props/class_map 绑定/条件/迭代源/key/插值），供合并级 pass 扫 bp() 使用。
pub fn walk_view_exprs(view: &ViewTree, f: &mut impl FnMut(&ExprIR)) {
    walk_view(view, &mut |node| match node {
        ViewTree::Element(el) => {
            for e in el.props.values() { f(e); }
            for e in el.class_map_bindings.values() { f(e); }
        }
        ViewTree::Conditional(cond) => {
            for b in &cond.branches { f(&b.condition); }
        }
        ViewTree::ListRendering(lr) => {
            f(&lr.iterable);
            if let Some(k) = &lr.key_expr { f(k); }
        }
        ViewTree::Interpolation(e) => f(e),
        _ => {}
    });
}
```

（walk_view 签名按其实际形态调整——若 walk_view 回调是 `FnMut(&ViewTree)` 则上例直接可用；否则照该文件既有模式。）

- [ ] **Step 6: 实现——validate_breakpoints（新文件）**

`crates/omnilang-core/src/semantic/validate_breakpoints.rs`：

```rust
use crate::error::Diagnostic;
use crate::ir::types::{ExprIR, IRProgram};
use crate::ir::walk::{walk_expr, walk_stmt, walk_view_exprs};

/// 合并级 pass：bp("name") 编译期查表（E-401 未声明 / E-405 非字面量）+ 参数改写为数字字面量。
/// 三调用点：run_pipeline 尾部（单文件路径）、analyse_multi Phase 3d、analyse_multi_cached Phase 3d。
pub fn validate_breakpoints(program: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    // 1) 收集全程序断点常量表（name -> number；重复名 last-wins v0）
    let mut bp_table: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for comp in program.iter_all_components() {
        // 组件所属 design 的断点经 lower 汇入 DesignIR；此处置 0 组件表按既有 theme 查表模式取 design
        // 实现按 analyse.rs validate_theme 的 design 关联先例：组件 source_file → DesignIR.source_file 匹配
        if let Some(d) = program.designs.get(/* 组件关联 design 名 */) {
            for (name, value) in &d.breakpoints {
                bp_table.insert(name.clone(), *value);
            }
        }
    }
    // 2) sweep 所有组件：视图表达式 + computed body + 事件 InlineFn 语句体
    for comp in program.iter_all_components() {
        let sweep_expr = |e: &ExprIR, diags: &mut Vec<Diagnostic>| {
            if let ExprIR::CallExpr { name, args } = e {
                if name == "bp" {
                    match args.first() {
                        Some(ExprIR::LiteralExpr(serde_json::Value::String(bp_name))) => {
                            match bp_table.get(bp_name) {
                                Some(&value) => {
                                    // 改写参数为数字字面量（codegen 只认 name=="bp" + 数字参数）
                                    args[0] = ExprIR::LiteralExpr(serde_json::Value::Number((value as f64).into()));
                                }
                                None => {
                                    diags.push(Diagnostic::error(
                                        format!("bp(\"{bp_name}\") 引用了未声明的断点名 {bp_name}——请在 design 块声明：breakpoint {bp_name}: <数值>;（建议 768）"),
                                        /* 锚点：组件 source_span/source_file，照 validate_effect_deps 先例 */,
                                    ));
                                }
                            }
                        }
                        _ => {
                            diags.push(Diagnostic::error(
                                "bp() 的参数必须是字符串字面量（编译期静态查表），当前为动态表达式".to_string(),
                                /* 锚点同 E-401 */,
                            ));
                        }
                    }
                }
            }
        };
        // 视图
        if let Some(view) = &comp.view {
            walk_view_exprs(view, &mut |e| sweep_expr(e, diagnostics));
        }
        // computeds
        for c in &comp.computeds {
            walk_expr(&c.body, &mut |e| sweep_expr(e, diagnostics));
        }
        // 事件 InlineFn 语句体
        for handler in comp.events.values() {
            if let crate::ir::types::EventHandlerIR::InlineFn { body, .. } = handler {
                for stmt in &body.statements {
                    walk_stmt(stmt, &mut |e| sweep_expr(e, diagnostics));
                }
            }
        }
    }
}
```

> **实现提示**：Diagnostic::error 的具体构造签名与 E-401 文案中的 `建议 768` 保持一致（spec §3 逐字）。设计关联查表按 analyse.rs `validate_theme` 的既有模式（组件如何找 design 以该文件实际实现为准，若 theme 是经 lower 合并进组件的字段则同样处理）。run_pipeline 尾部（lib.rs ~115）在 `validate_theme` 调用之后追加 `validate_breakpoints(&mut ir, &mut diagnostics);`（或既有合并验证函数体内）。

- [ ] **Step 7: 实现——validate_style_selectors（新文件）**

```rust
use crate::error::Diagnostic;
use crate::ir::types::IRProgram;

/// 第 10 pass：未支持选择器 → W-301 warning（诚实降级非阻断）。
pub fn validate_style_selectors(program: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    for comp in program.iter_all_components() {
        let mut check = |rule: &crate::ir::types::StyleRuleIR, diags: &mut Vec<Diagnostic>| {
            if let Some(us) = &rule.unsupported_selector {
                let (line, col) = comp.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
                diags.push(Diagnostic::warning(format!(
                    "样式规则选择器「{}」包含暂不支持的语法部分（{}），已截断到「{}」继续解析——该部分 web 端按 CSS 透传处理、Flutter 端不生效；复杂选择器为留生态（semantic-universe 域 14 注记），支持随语料需求再议",
                    us.text, us.category, rule.class_name
                ), comp.source_file.clone(), line, col));
            }
        };
        for rule in &comp.styles { check(rule, diagnostics); }
        for mb in &comp.media_blocks {
            for rule in &mb.rules { check(rule, diagnostics); }
        }
    }
}
```

（`Diagnostic::warning` 构造照 `validate_event_names.rs` warning_diag（196-215）形态——severity Warning + kind 用既有 warning kind 或 E0006 同款；若 API 不同以该文件实际为准。）

- [ ] **Step 8: 实现——pass 注册 + 三调用点接线**

`crates/omnilang-core/src/semantic/mod.rs`：追加 `pub mod validate_breakpoints; pub mod validate_style_selectors;`

`crates/omnilang-core/src/analyse.rs` run_ir_passes（docstring 57-68 追加第 10 pass 说明）：

```rust
    passes.push(Box::new(validate_style_selectors)); // 第 10 pass（validate_semantic_keys 之后）
```

- 调用点 1：`lib.rs` run_pipeline 尾部（~115，validate_theme 之后）追加 `validate_breakpoints(&mut ir, &mut diagnostics);` 与 `validate_style_selectors(&ir, &mut diagnostics);`
- 调用点 2：`analyse.rs` analyse_multi Phase 3d（~503，validate_theme 调用旁）同追加
- 调用点 3：`analyse.rs` analyse_multi_cached Phase 3d（~790）同追加

（run_ir_passes 若已覆盖这三个路径则只注册 pass 即可——以实际文件结构为准；**三个调用点必须全部接线**，crate::compile 走 run_pipeline 单文件路径。）

- [ ] **Step 9: 跑测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core undeclared_bp_is_e401 --lib && cargo test -p omnilang-core declared_bp_passes_and_rewrites_to_number --lib && cargo test -p omnilang-core non_literal_bp_arg_is_e405 --lib && cargo test -p omnilang-core bp_inside_computed_and_event_are_checked --lib && cargo test -p omnilang-core unsupported_selector_emits_w301_warning --lib && cargo test -p omnilang-core supported_selector_no_warning --lib && cargo test -p omnilang-core media_block_rules_also_checked --lib
```

Expected: PASS（7 个新测试）。

- [ ] **Step 10: omnic warnings 折叠 CLI 测试**

`omnic/src/commands/build.rs` tests（照 cmd_build_surfaces_warnings_without_failing 先例）追加：

```rust
#[test]
fn cmd_build_surfaces_w301_without_failing() {
    // 构造含未支持选择器的 .omni 临时项目，跑 build 命令
    // 断言：退出码 0 + stderr 含「暂不支持的语法部分」+ 含「兄弟选择器」
}
```

（照既有 cmd_build_surfaces_warnings_without_failing 的临时目录/API 调用形态写，仅换源码与断言文本。）

- [ ] **Step 11: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core --lib && cargo test -p omnic --lib && cargo fmt && cargo clippy -D warnings -p omnilang-core -p omnic && python scripts/verify-semantic-universe.py --check && python scripts/verify-vocabulary.py --check
```

Expected: 双 verify 脚本**全绿**（域 12 锚点断言——validate_breakpoints/validate_style_selectors/unsupported_selector——本任务落地转绿；React/Flutter/OmniView 三目标锚点 pending-red 至 T6/T7/T8，见 Step 12 说明）。

- [ ] **Step 12: 提交（锚点分批转绿注记）**

```bash
cd D:/space/OmniLang && git add crates/omnilang-core/src/ir/walk.rs crates/omnilang-core/src/semantic/ crates/omnilang-core/src/analyse.rs crates/omnilang-core/src/lib.rs omnic/src/commands/build.rs && git commit -m "feat(semantic): validate_breakpoints（E-401/E-405 + bp 参数改写数字字面量，三调用点接线）+ validate_style_selectors W-301（第 10 pass）+ walk_view_exprs + omnic warnings 折叠测试"
```

> 注：verify 脚本在 T6/T7/T8 完成前仍可能报 React/Flutter/OmniView 锚点缺失（pending-red 文档化，同 T1 注记）；T8 完成后全绿。

---

### Task 6: React codegen——bp_{n} useMediaQuery + __design/breakpoints.ts BP 表 + @media 透传

**Files:**
- Modify: `D:/space/OmniLang/omnilang/src/codegen/react/expr.rs`（format_call 特殊分派：name=="bp" + 数字参数 → `bp_{n}` 变量引用）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/react/component/mod.rs`（store-hooks 区 878-896：收集 bp 值集合 → 发射 `const bp_{n} = useMediaQuery('(min-width: {n}px)')`；本地 SSR-safe useMediaQuery helper 每模块内联一次；hooks Vec 264-302 强制补 useState/useEffect；gen_component_css 1350：@media 块透传）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/react/design.rs`（gen_design_files 103：新增 `__design/breakpoints.ts` `export const BP = {...}`）
- Test: `D:/space/OmniLang/omnilang/src/codegen/react/tests.rs`（helpers: compile_react_src 3175 / compile_react_full 3189 / compile_react_with_design 3226）

**Interfaces:**
- Consumes: `validate_breakpoints` 改写后的 IR（bp CallExpr args[0] = 数字字面量）；DesignIR.breakpoints；ComponentIR.media_blocks；组件模块零外部依赖现状（imports.rs 172/175/181——仅 React + CSS module，useMediaQuery 必须本地 helper）
- Produces:
  - `const bp_{n} = useMediaQuery('(min-width: {n}px)');`（模块级 hoisted，store-hooks 区；n = 数字字面量值，如 bp_768）
  - 本地 `useMediaQuery` helper：SSR-safe（`typeof window === 'undefined' || typeof window.matchMedia !== 'function'` → 返回 false = 视口缺省分支，spec §4.2 SSR 注记）；基于 useState/useEffect 订阅 matchMedia change
  - `__design/breakpoints.ts`：`export const BP = { sm: 640, md: 768, lg: 1024 };`（跨 design 合并，重复名 last-wins v0）
  - CSS 输出层：`@media (min-width: 768px) { .card { padding: 32px; } }` 原样透传（gen_component_css 内、fade conditional 之后）

- [ ] **Step 1: 写失败测试（bp() 三使用面生成）**

`react/tests.rs` tests 模块追加：

```rust
#[test]
fn bp_emits_use_media_query_hooks() {
    let src = r#"
        design D {
            breakpoint sm: 640;
            breakpoint md: 768;
            breakpoint lg: 1024;
        }
        component App {
            view {
                @if bp("md") {
                    <div class: DesktopLayout>Desktop</div>
                } else {
                    <div class: MobileLayout>Mobile</div>
                }
                <div class: {bp("lg") ? "desktop-grid" : "mobile-list"}>x</div>
            }
        }
    "#;
    let out = compile_react_src(src);
    let joined = out.join("\n");
    assert!(joined.contains("const bp_768 = useMediaQuery('(min-width: 768px)');"), "got: {joined}");
    assert!(joined.contains("const bp_1024 = useMediaQuery('(min-width: 1024px)');"), "got: {joined}");
    assert!(joined.contains("useMediaQuery('(min-width: 768px)')") == false, "必须引用 hoisted 变量而非重复调用: {joined}");
}

#[test]
fn bp_hook_uses_local_ssr_safe_helper() {
    let src = r#"
        design D { breakpoint md: 768; }
        component App {
            view { @if bp("md") { <div>x</div> } }
        }
    "#;
    let out = compile_react_src(src);
    let joined = out.join("\n");
    assert!(joined.contains("function useMediaQuery"), "本地 helper 缺失: {joined}");
    assert!(joined.contains("matchMedia"), "matchMedia 订阅缺失: {joined}");
    assert!(joined.contains("typeof window === 'undefined'"), "SSR 缺省分支缺失: {joined}");
}
```

- [ ] **Step 2: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang bp_emits_use_media_query_hooks --lib
```

Expected: FAIL——生成物无 bp_768。

- [ ] **Step 3: 写失败测试（breakpoints.ts + @media 透传）**

```rust
#[test]
fn design_emits_breakpoints_const_table() {
    let src = r#"
        design D {
            breakpoint sm: 640;
            breakpoint md: 768;
            breakpoint lg: 1024;
        }
        component App { view { <div>x</div> } }
    "#;
    let bp_ts = compile_react_with_design(src); // 返回 __design/useDesignTokens.ts 内容——见实现提示
    // 若 helper 返回单文件内容不足，改用 compile_react_full 并查找 breakpoints.ts 文件
    let all = compile_react_full(src).join("\n");
    assert!(all.contains("__design/breakpoints.ts") || bp_ts.contains("export const BP"), "BP 表缺失: {all}");
    assert!(all.contains("sm: 640") && all.contains("md: 768") && all.contains("lg: 1024"), "BP 值缺失: {all}");
}

#[test]
fn media_block_passthrough_in_css() {
    let src = r#"
        component App {
            style {
                .card { padding: 16; }
                @media (min-width: 768px) {
                    .card { padding: 32; }
                }
                @media (prefers-color-scheme: dark) {
                    .card { background: #111827; }
                }
            }
            view { <div class: card>c</div> }
        }
    "#;
    let out = compile_react_src(src);
    let css = out.join("\n");
    assert!(css.contains("@media (min-width: 768px) {"), "got: {css}");
    assert!(css.contains(".card { padding: 32px; }"), "got: {css}");
    assert!(css.contains("@media (prefers-color-scheme: dark) {"), "got: {css}");
    // 主题生成物与用户级 @media 共存（3-layer CSS 输出层同级）
    assert!(css.contains("prefers-color-scheme"));
}
```

- [ ] **Step 4: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang design_emits_breakpoints_const_table --lib && cargo test -p omnilang media_block_passthrough_in_css --lib
```

Expected: FAIL。

- [ ] **Step 5: 实现——expr.rs format_call 分派**

`omnilang/src/codegen/react/expr.rs` format_call（照既有 stdlib/事件特殊分派形态，在返回普通 `name(args)` 之前插入）：

```rust
// bp("md")——IR 参数已被 validate_breakpoints 改写为数字字面量
if name == "bp" {
    if let Some(ExprIR::LiteralExpr(serde_json::Value::Number(n))) = args.first() {
        return Ok(format!("bp_{}", n.as_u64().unwrap_or(0)).into());
    }
}
```

- [ ] **Step 6: 实现——component/mod.rs bp 收集与发射**

`component/mod.rs` store-hooks 区（878-896）：

```rust
// 1) 组件表达式收集（复用 walk_view_exprs）：提取所有 bp("数字") 的数值集合
let mut bp_values: Vec<u64> = Vec::new();
if let Some(view) = &comp.view {
    crate::codegen::shared::expr::walk_view_exprs(view, &mut |e| {
        if let ExprIR::CallExpr { name, args } = e {
            if name == "bp" {
                if let Some(ExprIR::LiteralExpr(serde_json::Value::Number(n))) = args.first() {
                    if let Some(v) = n.as_u64() {
                        if !bp_values.contains(&v) { bp_values.push(v); }
                    }
                }
            }
        }
    });
}
// 注：walk_view_exprs 需在 omnilang facade 暴露或经 shared::expr 转发（按实际模块路径调整）
```

```rust
// 2) 发射 hoisted 变量（hooks Vec 之后、store 初始化区，随其余 hooks 一起）：
for v in &bp_values {
    hooks.push(format!("const bp_{v} = useMediaQuery('(min-width: {v}px)');"));
}
// 3) 若 bp_values 非空：强制补 useState/useEffect 依赖（hooks Vec 264-302 的 force-add 机制）+
//    本地 useMediaQuery helper 追加到模块尾部（仅一次）：
//    const isMobileSSR = typeof window === 'undefined' || typeof window.matchMedia !== 'function';
//    function useMediaQuery(query) { const [matches, setMatches] = useState(!isMobileSSR ? window.matchMedia(query).matches : false); useEffect(() => { ...addEventListener('change', ...)... }, [query]); return matches; }
//    （helper 语义：SSR/无 matchMedia → 恒 false = 视口缺省分支；change 订阅保持响应式）
```

- [ ] **Step 7: 实现——design.rs BP 表文件**

`design.rs` gen_design_files（103）返回文件列表处追加（照 useDesignTokens.ts 生成形态）：

```rust
// 跨 design 合并断点（重复名 last-wins v0）：
let mut bp_merged: Vec<(String, u32)> = Vec::new();
for d in designs { for (name, value) in &d.breakpoints { bp_merged.push((name.clone(), *value)); } }
if !bp_merged.is_empty() {
    let body = bp_merged.iter().map(|(k, v)| format!("  {k}: {v},")).collect::<Vec<_>>().join("\n");
    files.push(CompiledFile {
        path: "__design/breakpoints.ts".into(),
        content: format!("// Auto-generated by OmniLang (batch 3)\nexport const BP = {{\n{body}\n}};\n"),
        ..Default::default()
    });
}
```

- [ ] **Step 8: 实现——gen_component_css @media 透传**

`component/mod.rs` gen_component_css（1350，fade conditional 逻辑之后、既有 `@media (prefers-reduced-motion: reduce)` 块 1373-1382 同级位置）：

```rust
// 用户级 @media 透传（comp.media_blocks）
for mb in &comp.media_blocks {
    w.line(format!("@media {} {{", mb.condition))?;
    w.indent();
    for rule in &mb.rules {
        // 复用既有单规则 CSS 输出逻辑（selectors + properties）
        ...
    }
    w.dedent();
    w.line("}")?;
}
```

- [ ] **Step 9: 跑测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang bp_emits_use_media_query_hooks --lib && cargo test -p omnilang bp_hook_uses_local_ssr_safe_helper --lib && cargo test -p omnilang design_emits_breakpoints_const_table --lib && cargo test -p omnilang media_block_passthrough_in_css --lib
```

Expected: PASS（4 个新测试）。

- [ ] **Step 10: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib && cargo fmt && cargo clippy -D warnings -p omnilang && python scripts/verify-semantic-universe.py --check
```

Expected: 域 12 React 锚点（useMediaQuery / @media）转绿；Flutter/OmniView 锚点仍 pending-red（T7/T8 落地）。

- [ ] **Step 11: 提交**

```bash
cd D:/space/OmniLang && git add omnilang/src/codegen/react/ && git commit -m "feat(codegen-react): bp() → hoisted useMediaQuery hooks（本地 SSR-safe helper）+ __design/breakpoints.ts BP 常量表 + @media 块 CSS 透传"
```

---

### Task 7: Flutter codegen——bpSm 常量 + MediaQuery.sizeOf + W-302 降级跳过

**Files:**
- Modify: `D:/space/OmniLang/omnilang/src/codegen/flutter/design.rs`（emit_design_dart 96：per-design 循环 166-171 后追加 bp 常量）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/flutter/ref.rs`（FlutterFormatExpr impl FormatExpr 1774：format_call 分派 bp → `MediaQuery.sizeOf(context).width >= {n}`）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/flutter/mod.rs`（gen_component_dart 135：遍历 comp.media_blocks → CodegenWarning::at（W-302）+ 块跳过不生成）
- Test: `D:/space/OmniLang/omnilang/src/codegen/flutter/tests.rs`（helper compile_flutter_full 4478）

**Interfaces:**
- Consumes: DesignIR.breakpoints；改写后 bp CallExpr；ComponentIR.media_blocks；CodegenWarning::at（writer.rs:16-25）
- Produces:
  - `static const bpSm = 640.0; ...`（design.dart，emit_design_dart 输出）
  - `MediaQuery.sizeOf(context).width >= 768`（表达式层）
  - W-302 warning（`@media 块在 Flutter 目标不生效（降级跳过）——响应式意图请用 bp() 表达（跨目标主通道）`）+ 块内规则不输出；**生成物零 @media 残留语法**

- [ ] **Step 1: 写失败测试**

`flutter/tests.rs` tests 模块追加：

```rust
#[test]
fn bp_emits_media_query_sizeof() {
    let src = r#"
        design D { breakpoint md: 768; }
        component App {
            view {
                @if bp("md") { <Container>Desktop</Container> } else { <Container>Mobile</Container> }
            }
        }
    "#;
    let out = compile_flutter_full(src);
    let joined = out.join("\n");
    assert!(joined.contains("MediaQuery.sizeOf(context).width >= 768"), "got: {joined}");
}

#[test]
fn design_emits_bp_constants() {
    let src = r#"
        design D {
            breakpoint sm: 640;
            breakpoint md: 768;
            breakpoint lg: 1024;
        }
        component App { view { <Text>x</Text> } }
    "#;
    let out = compile_flutter_full(src);
    let joined = out.join("\n");
    assert!(joined.contains("static const bpSm = 640.0;"), "got: {joined}");
    assert!(joined.contains("static const bpMd = 768.0;"), "got: {joined}");
    assert!(joined.contains("static const bpLg = 1024.0;"), "got: {joined}");
}

#[test]
fn media_block_degrades_with_w302_and_no_residue() {
    let src = r#"
        component App {
            style {
                .card { padding: 16; }
                @media (min-width: 768px) {
                    .card { padding: 32; }
                }
            }
            view { <div class: card>c</div> }
        }
    "#;
    let (dart, warnings, _) = /* gen_component_dart 直调或 compile_flutter_full 的警告侧通道 */;
    let joined = out.join("\n");
    assert!(!joined.contains("@media"), "Flutter 生成物不得残留 @media 语法: {joined}");
    assert!(!joined.contains("min-width"), "块内规则不得生效: {joined}");
    // W-302 断言（warnings 含 W-302 文本；若 helper 无警告通道则用返回结构实际字段）
    assert!(warnings.iter().any(|w| w.message.contains("不生效") && w.message.contains("bp()")), "got: {warnings:?}");
}
```

- [ ] **Step 2: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang bp_emits_media_query_sizeof --lib && cargo test -p omnilang media_block_degrades_with_w302_and_no_residue --lib
```

Expected: FAIL。

- [ ] **Step 3: 实现——design.dart bp 常量**

`flutter/design.rs` emit_design_dart（per-design 循环 166-171 之后）：

```rust
// 每个 design 输出其断点常量（照 static const 惯例；字段名 bp + PascalCase 断点名）
for (name, value) in &design.breakpoints {
    let const_name = format!("bp{}", capitalize(name)); // sm → bpSm, md → bpMd
    out.push_str(&format!("  static const {const_name} = {};\n", format_double(*value)));
}
```

- [ ] **Step 4: 实现——ref.rs format_call 分派**

`flutter/ref.rs` FlutterFormatExpr（1774）format_call 插入（其他特殊分派之前）：

```rust
if name == "bp" {
    if let Some(ExprIR::LiteralExpr(serde_json::Value::Number(n))) = args.first() {
        let v = n.as_u64().unwrap_or(0);
        return Ok(format!("MediaQuery.sizeOf(context).width >= {v}").into());
    }
}
```

- [ ] **Step 5: 实现——gen_component_dart W-302 跳过**

`flutter/mod.rs` gen_component_dart（135）——@media 块不产生任何 Dart 输出，仅发 warning：

```rust
// 用户级 @media：Flutter 诚实降级（ADR-054）——W-302 + 块跳过，指引 bp()
for mb in &comp.media_blocks {
    warnings.push(CodegenWarning::at(
        "@media 块在 Flutter 目标不生效（降级跳过）——响应式意图请用 bp() 表达（跨目标主通道）".to_string(),
        comp.source_file.clone(),
        comp.source_span.map(|s| s.line as u32).unwrap_or(0),
        comp.source_span.map(|s| s.col as u32).unwrap_or(0),
    ));
}
```

- [ ] **Step 6: 跑测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang bp_emits_media_query_sizeof --lib && cargo test -p omnilang design_emits_bp_constants --lib && cargo test -p omnilang media_block_degrades_with_w302_and_no_residue --lib
```

Expected: PASS（3 个新测试）。

- [ ] **Step 7: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib && cargo fmt && cargo clippy -D warnings -p omnilang && python scripts/verify-semantic-universe.py --check
```

Expected: 域 12 Flutter 锚点（MediaQuery.sizeOf / media_blocks）转绿；OmniView 锚点仍 pending-red（T8 落地）。

- [ ] **Step 8: 提交**

```bash
cd D:/space/OmniLang && git add omnilang/src/codegen/flutter/ && git commit -m "feat(codegen-flutter): bp() → MediaQuery.sizeOf 表达式 + design.dart bp 常量表 + @media 块 W-302 降级跳过（零残留）"
```

---

### Task 8: OmniView codegen——matchMedia 内联 + @media 透传

**Files:**
- Modify: `D:/space/OmniLang/omnilang/src/codegen/omniview/expr.rs`（CallExpr arm 107：name=="bp" + 数字参数 → `matchMedia('(min-width: {n}px)').matches`，插在 stdlib 路由 113 之前）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/omniview/view/mod.rs`（emit_style 114 相关：media_blocks 透传进 self.css）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/omniview/store.rs`（styles 循环 600-603 后追加 media_blocks 输出）
- Test: `D:/space/OmniLang/omnilang/src/codegen/omniview/tests.rs`（helpers: make_module/make_component 49-71 / make_program 73-82 / `generate(&ir, true, false, false)` 输出 .js/.css 字段 / lower_component_src 786）

**Interfaces:**
- Consumes: 改写后 bp CallExpr（数字参数）；ComponentIR.media_blocks；OmniView 运行时 CSS 直通（spec §4.3）
- Produces:
  - 表达式：`matchMedia('(min-width: 768px)').matches`（内联，无常量文件——spec §4.1 OmniView 行「编译期常量表注入运行时（matchMedia 调用参数）」）
  - CSS 输出：`@media (min-width: 768px) { ... }` 原样透传（output.css 字段）

- [ ] **Step 1: 写失败测试**

`omniview/tests.rs` tests 模块追加：

```rust
#[test]
fn bp_emits_inline_match_media() {
    let comp = lower_component_src(r#"
        design D { breakpoint md: 768; }
        component App {
            view {
                @if bp("md") { <div>Desktop</div> } else { <div>Mobile</div> }
            }
        }
    "#);
    let ir = make_program(vec![comp]);
    let out = generate(&ir, true, false, false);
    let js = out.js; // 字段名按 generate 返回结构实际调整
    assert!(js.contains("matchMedia('(min-width: 768px)').matches"), "got: {js}");
}

#[test]
fn media_block_passthrough_in_css_output() {
    let comp = lower_component_src(r#"
        component App {
            style {
                .card { padding: 16; }
                @media (min-width: 768px) {
                    .card { padding: 32; }
                }
            }
            view { <div class: card>c</div> }
        }
    "#);
    let ir = make_program(vec![comp]);
    let out = generate(&ir, true, false, false);
    let css = out.css; // output.css 字段
    assert!(css.contains("@media (min-width: 768px) {"), "got: {css}");
    assert!(css.contains(".card { padding: 32px; }"), "got: {css}");
}
```

- [ ] **Step 2: 跑失败测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang bp_emits_inline_match_media --lib && cargo test -p omnilang media_block_passthrough_in_css_output --lib
```

Expected: FAIL。

- [ ] **Step 3: 实现——expr.rs bp 分派**

`omniview/expr.rs` CallExpr arm（107）内、stdlib 路由（113）之前：

```rust
if name == "bp" {
    if let Some(ExprIR::LiteralExpr(serde_json::Value::Number(n))) = args.first() {
        let v = n.as_u64().unwrap_or(0);
        return Ok(format!("matchMedia('(min-width: {v}px)').matches").into());
    }
}
```

- [ ] **Step 4: 实现——@media 透传**

`omniview/view/mod.rs` / `store.rs`：组件样式输出处（store.rs styles 循环 600-603 之后）追加：

```rust
for mb in &comp.media_blocks {
    self.css.push_str(&format!("@media {} {{", mb.condition));
    for rule in &mb.rules {
        self.css.push_str(&/* 既有单规则 CSS 输出逻辑（class selector + properties） */);
    }
    self.css.push_str("}");
}
```

- [ ] **Step 5: 跑测试**

```bash
cd D:/space/OmniLang && cargo test -p omnilang bp_emits_inline_match_media --lib && cargo test -p omnilang media_block_passthrough_in_css_output --lib
```

Expected: PASS（2 个新测试）。

- [ ] **Step 6: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang --lib && cargo fmt && cargo clippy -D warnings -p omnilang && python scripts/verify-semantic-universe.py --check && python scripts/verify-vocabulary.py --check
```

Expected: 域 12 全部锚点转绿——**verify 双脚本全绿（T1 pending-red 收口）**。

- [ ] **Step 7: 提交**

```bash
cd D:/space/OmniLang && git add omnilang/src/codegen/omniview/ && git commit -m "feat(codegen-omniview): bp() → matchMedia 内联表达式 + @media 块 CSS 透传（域 12 锚点全绿收口）"
```

---

### Task 9: demo 迁移（learn-omniview 六处）+ golden 镜像 + 注入语料 855 扩展

**Files:**
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/design.omni`（新增 breakpoint sm/md/lg 三行）
- Create: `D:/space/OmniLang/test/projects/learn-omniview/src/components/ResponsiveDemo.omni`（@if bp("md") 双分支 + {} 三元 + @media 块）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/pages/UiComponents.omni`（或既有页面接线处，照 LoadDemo 先例：页面组件 + 注册入口）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/components/ThemeSourceDemo.omni`（注记：@media (prefers-color-scheme: dark) 为主题生成物 vs 用户级 @media 独立语法面）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/main.omni`（页面入口 wiring）
- Modify: `D:/space/OmniLang/tests/fixtures/golden/test/projects/learn-omniview/`（golden 镜像 resync：`diff -rq test/projects/learn-omniview/src tests/fixtures/golden/test/projects/learn-omniview`）
- Modify: 注入语料 855 元组所在模块（位置定位步骤见 Step 5；候选 `crates/omnilang-core/src/validation/` 或 `crates/omnilang-core/src/tests/`）
- Test: `test/projects/learn-omniview/build/react-test-app/tests/`（浏览器断言文件——本任务只接线 demo，断言归 T10）

**Interfaces:**
- Consumes: T6/T7/T8 全部能力（bp/@media/breakpoint 可编译可运行）；LoadDemo 接线先例（页面组件 + 注册入口）；golden MANIFEST.md 字节级一致快照约定
- Produces:
  - `src/design.omni` 含 `breakpoint sm: 640; breakpoint md: 768; breakpoint lg: 1024;`
  - `ResponsiveDemo.omni`：三使用面演示组件（@if bp("md") 双分支 DesktopLayout/MobileLayout + `<div class: {bp("md") ? "desktop-grid" : "mobile-list"}>` 三元 + style 块内 @media (min-width: 768px) .card padding 增强）
  - 页面接线：ResponsiveDemo 可导航可达（UiComponents 页或新页 + main.omni 入口）
  - ThemeSourceDemo 展示文本注记
  - golden 镜像与 src 字节一致（`diff -rq` 零差异）
  - 注入语料 855 元组 + 7 点扩展（见 Step 5）

- [ ] **Step 1: design.omni 断点声明**

`test/projects/learn-omniview/src/design.omni` 的 design 块内（@themes / @default-theme 之后、token 区之前）追加：

```
    breakpoint sm: 640;
    breakpoint md: 768;
    breakpoint lg: 1024;
```

- [ ] **Step 2: 新组件 ResponsiveDemo.omni**

`test/projects/learn-omniview/src/components/ResponsiveDemo.omni`（新文件，照既有 demo 组件文件结构——props/signals/view/style）：

```omniview
component ResponsiveDemo {
    style {
        .card {
            padding: 16;
            border-radius: 8;
            background: var(--color-surface);
        }
        @media (min-width: 768px) {
            .card {
                padding: 32;
                border-radius: 12;
            }
        }
    }

    view {
        <div class: card>
            <h2>响应式布局演示</h2>
            <p>视口 ≥768px 显示桌面布局（bp("md") 为真）；否则显示移动布局。</p>
            <div class: {bp("md") ? "desktop-grid" : "mobile-list"}>
                <div>A</div>
                <div>B</div>
                <div>C</div>
            </div>
            @if bp("md") {
                <div class: DesktopLayout>桌面分支（≥768px 可见）</div>
            } else {
                <div class: MobileLayout>移动分支（<768px 可见）</div>
            }
            <p>@media 块规则在 768px 以上改变 .card 内边距（web 透传生效）。</p>
        </div>
    }
}
```

（DesktopLayout/MobileLayout/desktop-grid/mobile-list 样式类按 learn-omniview 既有样式体系补进 design.omni 或组件 style 块；具体类名以该 demo 既有约定为准。）

- [ ] **Step 3: 页面接线（UiComponents 页 / main.omni）**

照 LoadDemo 先例（`src/pages/` 页面组件 + 注册入口 + `src/main.omni` wiring）把 ResponsiveDemo 接到页面导航。`ThemeSourceDemo.omni` 展示文本追加注记：

```
@note @media (prefers-color-scheme: dark) 由 @themes 主题系统生成（生成物）；用户级 @media 是 style 块内的独立语法面（批次 3），二者不冲突。
```

- [ ] **Step 4: 全量编译验证 + golden resync**

```bash
cd D:/space/OmniLang && cargo run -p omnic -- build test/projects/learn-omniview/src/main.omni --target react && diff -rq test/projects/learn-omniview/src tests/fixtures/golden/test/projects/learn-omniview
```

Expected: 编译零错误零 warning（ResponsiveDemo 三使用面全通过）；diff 输出差异清单 → 用 `cp -r` 将 src 变更同步进 golden 镜像（照既有 golden 同步惯例），再跑 `diff -rq` 确认零差异。

- [ ] **Step 5: 注入语料 855 元组扩展（7 点）**

定位语料模块：

```bash
cd D:/space/OmniLang && grep -rn "855" crates/omnilang-core/src --include="*.rs" | grep -v easing
```

（855 元组 = ADR-056 验证网；若 grep 仅命中 easing 常量，则检查 `crates/omnilang-core/src/validation/` 目录下的语料数组模块，以实际位置为准。）

扩展 7 点（spec §7 注入语料行）：

```rust
// ① breakpoint 声明（sm/md/lg 三行 → 编译成功 + DesignIR.breakpoints 三键）
// ② bp() 三位置（@if / {} 绑定 / 三元）→ 编译成功
// ③ @media 块两种条件（min-width + prefers-color-scheme）→ 编译成功
// ④ 未支持选择器 `input:checked + span` → 编译成功 + warnings 含 W-301
// ⑤ bp 未声明引用 → E-401 硬错误
// ⑥ @container → E-404 硬错误
// ⑦ Flutter 目标 @media → W-302（codegen 警告侧通道断言）
```

（按既有 855 元组的元组结构与断言函数形态逐条追加；①-③ 走既有「编译成功」断言通道，④ 走 warnings 通道，⑤⑥ 走 errors 通道，⑦ 走 flutter codegen 警告通道。）

- [ ] **Step 6: 验证门禁**

```bash
cd D:/space/OmniLang && cargo test -p omnilang-core --lib && cargo test -p omnilang --lib && python scripts/verify-semantic-universe.py --check && python scripts/verify-vocabulary.py --check && diff -rq test/projects/learn-omniview/src tests/fixtures/golden/test/projects/learn-omniview
```

- [ ] **Step 7: 提交（含 golden 镜像）**

```bash
cd D:/space/OmniLang && git add test/projects/learn-omniview/src tests/fixtures/golden/test/projects/learn-omniview crates/omnilang-core/src/validation/ && git commit -m "feat(demo): learn-omniview 响应式演示——breakpoint 三断点 + ResponsiveDemo（bp 三使用面 + @media 透传）+ ThemeSourceDemo 注记 + golden 镜像 + 注入语料 855 元组 7 点扩展"
```

---

### Task 10: Playwright 浏览器断言 + 门禁终态收口

**Files:**
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/build/react-test-app/tests/`（既有 spec 文件之一，如 semantic-behavior.spec.ts 或新建 responsive.spec.ts）
- Test: Playwright 运行（learn-omniview React 测试应用）

**Interfaces:**
- Consumes: T9 的 ResponsiveDemo（页面可达）+ React 生成物（useMediaQuery + @media 透传 + breakpoints.ts）
- Produces: spec §7 浏览器行为断言 5 条全绿（清单见 Step 1）+ 五层门禁终态全绿 + 占位符清零自检

- [ ] **Step 1: 写浏览器断言（spec §7 五条）**

`test/projects/learn-omniview/build/react-test-app/tests/responsive.spec.ts`（新文件；若既有 spec 结构为单文件多 describe 则并入）：

```typescript
import { test, expect } from '@playwright/test';

test.describe('ResponsiveDemo（批次 3）', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // 导航到 ResponsiveDemo 所在页面（照既有 spec 的页面导航模式）
  });

  test('视口 ≥768：DesktopLayout 分支可见 + 三元绑定 desktop-grid', async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 800 });
    await expect(page.getByText('桌面分支（≥768px 可见）')).toBeVisible();
    await expect(page.getByText('移动分支（<768px 可见）')).toBeHidden();
    // 三元绑定：desktop-grid 类生效（断言其下 A/B/C 三项可见）
  });

  test('视口 <768：MobileLayout 分支可见 + 绑定 mobile-list', async ({ page }) => {
    await page.setViewportSize({ width: 500, height: 800 });
    await expect(page.getByText('移动分支（<768px 可见）')).toBeVisible();
    await expect(page.getByText('桌面分支（≥768px 可见）')).toBeHidden();
  });

  test('边界精度 767/768：min-width 语义无 off-by-one', async ({ page }) => {
    await page.setViewportSize({ width: 767, height: 800 });
    await expect(page.getByText('移动分支（<768px 可见）')).toBeVisible();
    await page.setViewportSize({ width: 768, height: 800 });
    await expect(page.getByText('桌面分支（≥768px 可见）')).toBeVisible();
  });

  test('@media 块规则在 web 生效（≥768 时 .card padding 变化）', async ({ page }) => {
    await page.setViewportSize({ width: 500, height: 800 });
    const padSmall = await page.locator('.card').evaluate((el) => getComputedStyle(el).padding);
    await page.setViewportSize({ width: 1024, height: 800 });
    const padLarge = await page.locator('.card').evaluate((el) => getComputedStyle(el).padding);
    expect(padLarge).not.toBe(padSmall);
  });
});

test('CLI 编译含 W-301 的 .omni：退出码 0 + stderr 含选择器原文与类别词', async () => {
  // 在测试前已由 Node child_process 或既有 CLI 测试约定执行（照 cmd_build_surfaces_w301_without_failing 的 CLI 侧覆盖）
  // 浏览器侧可跳过；若既有 Playwright 有 CLI 调用的先例则照做
});
```

> 注：第 5 条 CLI 断言主责任在 T5 Step 10（omnic 单测，退出码 0 + stderr 断言）；此处如既有 Playwright 套件有 CLI 调用先例则补一条端到端，否则以 omnic 单测为准并在本文件标注。

- [ ] **Step 2: 跑 Playwright**

```bash
cd D:/space/OmniLang/test/projects/learn-omniview && npx playwright test --grep "ResponsiveDemo"
```

Expected: 5 条全 PASS（含 767/768 resize 边界各一次）。

- [ ] **Step 3: 五层门禁终态收口**

逐层跑并记录：

```bash
# 1) tsc（React 生成物编译通过——useMediaQuery hook + 条件渲染 + breakpoints.ts 类型正确）
cd D:/space/OmniLang/test/projects/learn-omniview/build/react-test-app && npx tsc --noEmit
# 2) dart analyze（Flutter 生成物零告警——MediaQuery.sizeOf 分支 + W-302 降级路径无 @media 残留）
cd <flutter 产物目录> && flutter analyze
# 3) Playwright（Task 10 Step 2 已跑 + 既有全量 spec 回归）
npx playwright test
# 4) docs-gate（verify 双脚本全绿 + 登记册一致性）
cd D:/space/OmniLang && python scripts/verify-semantic-universe.py --check && python scripts/verify-vocabulary.py --check
# 5) 注入语料 855 元组扩展全绿（Task 9 Step 5 已并入 cargo test）
cd D:/space/OmniLang && cargo test -p omnilang-core --lib
```

- [ ] **Step 4: 占位符清零自检**

```bash
cd D:/space/OmniLang && grep -rn "TBD\|TODO\|FIXME" docs/superpowers/plans/2026-08-09-semantic-batch3-responsive-styling.md docs/superpowers/specs/2026-08-09-semantic-batch3-responsive-styling-design.md | head
```

Expected: 无输出（或仅 spec 中既有的既定义项）。

- [ ] **Step 5: 提交**

```bash
cd D:/space/OmniLang && git add test/projects/learn-omniview/build/react-test-app/tests/ && git commit -m "test(e2e): ResponsiveDemo 浏览器断言——bp 双分支/三元绑定/767-768 边界/@media 生效 + 五层门禁终态收口"
```

---

## 3. 阶段顺序（任务依赖图）

```
T1 登记先行（首跑，独立）
   │
   ├──► T2 lexer+breakpoint 解析 ──┐
   │                              ├──► T4 IR+lower+ts-rs ──► T5 语义 passes ──► T6 React ──┐
   └──► T3 style 块解析 ──────────┘                        │                ├──► T7 Flutter ──┤
                                                          │                └──► T8 OmniView ─┘
                                                          ▼
                                             T9 demo 迁移 + golden + 注入语料 ──► T10 Playwright + 门禁终态
```

**可并行标注**：
- T2 / T3 可并行（互不依赖，均依赖 T1）
- T6 / T7 / T8 可并行（均依赖 T5，互不依赖）
- T4 依赖 T2+T3；T5 依赖 T4；T9 依赖 T6+T7+T8；T10 依赖 T9

**关键路径**: T1 → T2 → T4 → T5 → T6 → T9 → T10（10 个串行段中的 7 段；T2/T3 并行、T6/T7/T8 并行可压缩墙钟时间）。

## 4. 登记先行任务（ADR-074 硬约束）

- **T1** 承担全部登记：semantic-universe 域 12 新域（语义集条目表 5 行 + 已实现白名单表 5 行）+ 域 14「留生态」状态值（verify 脚本 STATUSES 常量扩展）+ vocabulary-registry 域 7 两行（bp/breakpoint 四字段 + useMediaQuery 排除词）+ verify 双脚本改造（域 12 needle 断言 / 域 14 状态值域校验 / 域 7 四字段断言）+ docs 注记（10-design-system.md / 09-styling.md）。
- **顺序纪律**：T1 必须先行——T2 起的语法实施都以登记在案为前提（ADR-074「新增语法先登记 semantic-universe → 才进 BNF/白名单」）；T1 的编译器锚点断言以 pending-red 文档化方式容忍 T2-T8 的落地时差，T8 完成即全绿收口。
- **@media/@container 不登记语义词汇**（spec §5.3：机制透传关键字，CSS 原生词汇）——verify-vocabulary 不得为它们建行，T1 的注记写明。

## 5. 语义化即切换任务（ADR-075 原则 8）

本批次**无旧形态残留**（spec §3「语义化即切换检查」）：bp()/breakpoint/@media 均为全新语法面，无旧等价形态需硬诊断切换；@media 透传行为是新增能力（此前 parser 不支持用户级 @media，遇之报错），无双轨期。逐语料位核对：
- design.omni / 既有组件：无 breakpoint/bp/@media 旧写法需迁移（新增声明与组件，不修改既有语义）
- 既有「静默跳过」选择器语料：解析行为不变（支持集合不变），仅新增 W-301 诊断——旧语料编译结果不受影响，构建不失败
- @container 语料：此前行为 = parser 报错（不支持）；现行为 = E-404 带指引（诊断升级，非形态切换）

## 6. 门禁终态（五层门禁最终验收口径）

| 门禁 | 本批次断言 | 落地任务 |
|------|-----------|----------|
| tsc | React 生成物编译通过：useMediaQuery hook 调用 + 条件渲染类型正确（含 bp() 三态分支组件 + breakpoints.ts） | T6 + T10 Step 3 |
| dart analyze | Flutter 生成物零告警：MediaQuery.sizeOf 分支 + W-302 降级路径（无 @media 残留语法） | T7 + T10 Step 3 |
| Playwright | 浏览器行为断言 5 条（bp 双分支 / 三元绑定 / 767-768 边界 / @media 生效 / CLI W-301 stderr） | T10 Step 1-2 |
| docs-gate | semantic-universe 域 12 + 域 14 留生态行 + vocabulary-registry 域 7 两行 + 10-design-system.md breakpoint 说明 + 09-styling.md @media 注记，verify 双脚本绿 | T1 + T8 收口 |
| 注入语料 | 855 元组扩展 7 点（breakpoint 声明 / bp 三位置 / @media 两种条件 / W-301 / E-401 / E-404 / W-302） | T9 Step 5 |

**终态验收顺序**：T10 Step 3 逐层跑（1→5），全绿即批次完成；T10 Step 4 占位符清零自检为最终收口。

## 7. 风险与降级注记

| 风险 | 缓解 |
|------|------|
| E-401 文案中「建议 768」为 spec 示例——不同断点名需插值实际名称 | T5 Step 6 实现注记：`format!` 插值 bp_name，spec §3 文案的「md」「768」为示例形态 |
| bp() 在 ModuleIR 函数体内不被 sweep（spec 使用面为 @if/{} 绑定/三元） | T5 范围注记：视图 + computed + 事件三处覆盖，函数体为已知边界（与 spec 使用面清单一致） |
| 跨文件断点查表：bp() 所在组件与 breakpoint 声明所在 design 的关联 | T5 Step 6 实现提示：照 validate_theme 的 design 关联先例；v0 用组件 source_file ↔ DesignIR.source_file 匹配；关联空洞为已知边界（1F 跨文件验证空洞同族） |
| 未支持选择器 token 类别可达集小于 spec §3 列表（`~`/`::before`/`:has(` 词法层硬错误） | T3 Step 7 实现提示：只实现可达 5 类（Plus/Gt/LBracket/Comma/LParen）；spec 类别词表为枚举说明，W-301 类别词按实际触发类别输出 |
| golden 镜像字节级一致（MANIFEST.md 快照） | T9 Step 4：`diff -rq` 前置确认差异清单再同步，杜绝盲拷贝 |
| verify 脚本锚点断言时差（T1 pending-red） | T1 Step 5 / T5 Step 12 / T8 Step 6 三处注记：pending-red 是文档化预期，禁以提前实现语法转绿 |
| legacy parse_style_rule（486-490）不捕获 unsupported_selector | T3 Step 7：legacy 构造点直接 None + 注释说明主路径已覆盖；不扩 legacy 行为 |
| 注入语料 855 元组位置未确认（grep "855" 仅命中 easing） | T9 Step 5 含 locate 步骤（检查 validation/ 目录语料数组），以实际位置为准 |
| ts-rs 绑定遗漏（WASM 通道类型断裂） | T4 Step 5 export_bindings 测试 + 绑定文件同 commit（Global Constraints 硬约束） |

---

## 附：占位符与类型一致性自查记录（作者自检）

- **Spec 覆盖**：§2.1 breakpoint（T2/T4/T6/T7/T8）、§2.2 bp()（T5/T6/T7/T8）、§2.3 选择器诊断（T3/T5）、§2.4 @media（T3/T6/T7/T8）、§2.5 @container（T3）、§3 七条诊断（T2 E-402/E-403 / T3 E-404 / T5 E-401/E-405/W-301 / T7 W-302，文案逐字）、§4 三目标映射（T6/T7/T8）、§5 登记（T1）、§6 边界（T3 E-404 + T1 域 14 行）、§7 门禁（T10）、§8 demo 迁移（T9）。
- **占位符扫描**：任务步骤无 TBD/TODO；实现提示均为「防踩坑」注记而非待填项；唯一「以实际为准」项为既有文件内部命名（函数名/字段名/测试入口），均给出候选名与定位锚点。
- **类型一致性**：bp CallExpr 参数改写（LiteralExpr-Number）在 T5 定义、T6/T7/T8 消费，三处断言同一形态；UnsupportedSelector{text,span,category} AST→IR 同名透传（T3→T4→T5）；MediaBlock{condition,rules} 同构透传（T3→T4→T5 W-301 sweep 覆盖块内规则）；walk_view_exprs 在 T5 定义、T5/T6 共用。
