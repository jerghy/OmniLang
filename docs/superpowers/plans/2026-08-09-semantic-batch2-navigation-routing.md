# 批次 2 域 C 导航路由 实施计划（2026-08-09-semantic-batch2-navigation-routing.md）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**日期**: 2026-08-09
**上游 spec**: `docs/superpowers/specs/2026-08-09-semantic-batch2-navigation-routing-design.md`（唯一详细级输入——本计划只引用章节号，不重复设计）
**上游基线**: `docs/superpowers/specs/2026-08-09-semantic-capability-baseline-design.md`（§4 批次 2 方向 / §5 路线图 / §6 验收 / §8 治理）
**执行方式**: subagent-driven + 逐任务审查（每任务一个 subagent 会话，完成后主 agent 审查再放行）

**Goal**: 将导航路由 8 项能力（C1-C8）进语言——`@route`/`@layout`/`@query` 声明 + `navigate`/`back` 内建函数，三目标（React Router v7 data router / Flutter GoRouter / OmniView hash 路由）地道映射，五层门禁全绿。

**Architecture**: 声明侧（组件级指令 @route/@layout/@query + 隐式参数 prop）→ 语义验证（V1-V14 逐组件 + V14/W2 跨文件注册表）→ IR（route_intent/layout_intent/query_params 挂 ComponentIR）→ 三目标 codegen（各目标新 router.rs）。导航调用侧 navigate/back 走既有函数调用通道（内建语义函数登记），codegen 按目标翻译。登记先行（ADR-074）：域 11 导航 + 域 7 词汇在语法实施前登记。

**Tech Stack**: Rust（omnilang-core parser/semantic/ir/lower + omnilang codegen）/ React Router v7（createBrowserRouter 数据路由）/ GoRouter（redirect/ShellRoute）/ OmniView hash 路由（零依赖）/ ts-rs 绑定同步 / Python verify 脚本 / Playwright。

## Global Constraints

- 五层门禁终态（spec §7）：① tsc——`test/projects/learn-omniview/build/react-test-app` 全量 `npx tsc --noEmit` 零错误；② dart analyze——`build/flutter-test-app` `dart analyze` 零错误；③ Playwright——`build/react-test-app/tests/behavioral-parity.spec.ts` 扩展 10 条路由断言（spec §7 清单）；④ docs-gate——`python scripts/verify-semantic-universe.py` 域 11 全绿 + `python scripts/verify-vocabulary.py` 域 7 全绿 + docs/omniview 导航文档与登记册同步；⑤ 注入语料——路由语法元组 12+（spec §7 层 5）+ NavDemo 三目标编译全绿
- 错误消息逐字采用 spec §3 V1-V14/W1/W2 表（中文），前后缀区分解析错误/语义硬错误/警告——不得改写
- 登记先行（ADR-074）：semantic-universe 域 11 + vocabulary-registry 域 7 必须先于 parser 实施（Task 1）
- 语法变更 = 同 commit 全生态迁移（编译器 + registry + 脚本 + demo + 文档）——ROADMAP 硬规则
- 语义化即切换（ADR-075 原则 8）：`@if` 页面互斥/`event onNavigate` 是合法语法，**不做硬诊断**（spec §6 边界 6）；收口 = 文档引导 + demo 迁移（Task 10）
- 诚实降级（ADR-054）：Flutter 移动端懒加载 no-op 编译注记、栈空 back no-op 注记、深链平台清单只注记不生成（spec §6 边界 4/5/7）——绝不静默
- 提交消息：中文 conventional commits（`feat:`/`fix:`/`docs:`/`test:`），同任务门禁绿才 commit
- IR 结构变更必须同步 ts-rs 绑定（`cargo test export_bindings --lib -p omnilang` → `poiesis/src/commands/__generated__/`）
- React Router v7 精确版本与包形态：实施期以官方文档为准（spec §9 未决 4）；依赖注入沿 Zustand 先例（`omnilang/src/codegen/react/design.rs:556`——编译器发射 import，依赖项由模板维护）
- 语言边界执行口径（spec §6）：tab 面板不自动路由化（§9 未决 1 已定：9 标签页保留 tab UI 态）；守卫无 redirect 默认重定向 `/`（§9 未决 2 已定）；Flutter 保挂载 v0 用标准 ShellRoute + 差异注记（§9 未决 3 已定）

---

## 1. 背景与范围

本批次把导航路由 8 项裁决全量进 floor（spec §1 表 C1-C8）：

| # | 能力项 | 语言面形态（spec §2） | 实施任务 |
|---|---|---|---|
| C1 | 页面切换 | `@route "/path"` 组件级指令 | T2/T3/T5 |
| C2 | 路由参数 | `:id` 段 → 隐式 String prop 注入 | T2/T3/T5-T8 |
| C3 | 返回/前进 | `back()` 内建语义函数 | T2/T3/T5-T8 |
| C4 | 深链/URL 同步 | 无新语法——`@route` 路径即 URL 语义；web URL 同步 + Flutter 移动端平台清单注记 | T6-T8 |
| C5 | 守卫/权限 | `guard: fnName` + `redirect: "/path"`（默认 `/`） | T2/T3/T6-T8 |
| C6 | 嵌套路由 | `@layout "/prefix"` 独立语法 + `<@slot/>` 出口 | T2/T3/T6-T8 |
| C7 | URL 查询参数 | `@query name: Type = default` | T2/T3/T5-T8 |
| C8 | 懒加载 | `lazy: true` 属性扩展点 | T2/T3/T6-T8 |

Validation 清单 V1-V14/W1/W2 全量落地（spec §3）：V1/V2/V5 解析错误（T2）、V3-V13 + W1 语义硬错误/警告（T4）、V14/W2 跨文件注册表（T5）。

登记册：域 11 导航 8 条目（spec §5.1）+ 域 7 导航 5 词汇（spec §5.2）——T1 登记先行，T9 回填实施后锚点。

demo 迁移 7 项（spec §8）——T10 逐语料位执行。

## 2. 任务划分

### Task 1: 登记先行（ADR-074）——semantic-universe 域 11 + vocabulary-registry 域 7 + verify 脚本骨架

**内容**（引用 spec §5.1/§5.2/§5.3；基线 §8 治理）：
- `docs/superpowers/specs/semantic-universe.md` 追加「域 11: 导航」：语义域定义 + 语义集条目表（8 条目，照 spec §5.1 表格，**实现状态列填「未支持」并注记「2026-08-09 批次 2 实施」**——universe 状态值域无「🔜 本批次」值）+ 已实现白名单表（先列 spec §5.1 预期锚点，状态标注「实施后回填」，锚点文件建成后由 T9 回填「支持」）
- `docs/superpowers/specs/vocabulary-registry.md` 追加「域 7: 导航」：五词汇四字段（来源域/语义词汇/目标映射/排除词汇），照 spec §5.2 表格 + 维护者注记（路径参数注入 prop 名为编译器保留名判定，非词汇条目）
- `scripts/verify-semantic-universe.py`：`DOMAIN_TITLES[11] = "导航"`（:110-120 处）+ 域循环 `range(1, 12)` + 域 11 结构断言（8 语义集条目存在性 + 四子节）——**本任务只加结构断言，白名单锚点断言在 T9 加入**（锚点文件届时才存在）
- `scripts/verify-vocabulary.py`：`DOMAIN_TITLES[7] = "导航"`（:78-87 处）+ 域循环改 `range(1, 8)`（:226）+ 域 7 四字段断言（五词汇存在性 + 排除词汇禁借断言）——词汇锚点断言（对照 parser 常量）在 T9 加入

**涉及文件**:
- Modify: `D:/space/OmniLang/docs/superpowers/specs/semantic-universe.md`
- Modify: `D:/space/OmniLang/docs/superpowers/specs/vocabulary-registry.md`
- Modify: `D:/space/OmniLang/scripts/verify-semantic-universe.py`
- Modify: `D:/space/OmniLang/scripts/verify-vocabulary.py`

**TDD 测试点**（RED 先写脚本断言）:
- `verify-semantic-universe.py` 跑挂：域 11 缺失（`domain 11 missing (expected '## 域 11: …导航…')`）→ 写登记 → 跑绿
- `verify-vocabulary.py` 跑挂：域 7 缺失（同形态）→ 写登记 → 跑绿
- 断言要点：域标题含「导航」关键词；域 11 语义集条目表含 8 条目名（@route 页面声明/路径参数 prop 注入/@query 查询参数/guard 守卫/@layout 嵌套布局/navigate 内建函数/back 内建函数/lazy 懒加载）；域 7 语义词汇表含 5 词汇（@route/@layout/@query/navigate/back）且四子节齐全

**验证门禁**:
```bash
python scripts/verify-semantic-universe.py
python scripts/verify-vocabulary.py
cargo test -p omnilang-core 2>&1 | tail -3   # 既有测试不受影响
```
预期：双脚本绿（其余域断言不回归）。

**依赖**: 无（批次 1 若已合并域 10 则域循环含 10，正常处理）

### Task 2: 词法 + 解析——@route/@layout/@query 组件级指令 + navigate/back 内建调用登记

**内容**（引用 spec §2.1-2.8 语法形态 + §3 V1/V2/V5 解析错误 + BNF）:
- 词法：`crates/omnilang-core/src/lexer/mod.rs:159` `read_at_keyword()` 增加 `@route`/`@layout`/`@query` 三关键词 → `lexer/token.rs` 新增 `AtRoute`/`AtLayout`/`AtQuery` TokenKind 变体（照 AtStore/AtForm/AtLoad 先例 :66-76）
- 解析：`crates/omnilang-core/src/parser/top_level/component.rs` 组件顶层 dispatch 循环（:43 AtStore / :182 AtMount 先例）新增三分支；新文件 `crates/omnilang-core/src/parser/top_level/route.rs`：
  - `parse_route_decl`：`@route STRING (guard: IDENT | lazy: BOOL)*`——`redirect:` 属性与 guard 配对（属性扩展点照 spec 2.5）
  - `parse_layout_decl`：`@layout STRING`
  - `parse_query_decl`：`@query IDENT : TYPE (= expr)?`
  - 解析错误 V1（位置域：仅组件顶层块内）、V2（路径空/非 `/` 开头）、V5（参数段非法：`:123`/`::id`/`:id-2`）——错误消息逐字照 spec §3 表
  - 路径解析出参数段列表 `path_param ::= ':' IDENT`（spec §2.2）
- AST：`parser/ast/` 组件块成员枚举新增 `RouteDecl`/`LayoutDecl`/`QueryDecl` 变体（含路径/属性/参数段/类型/默认值字段）
- 内建调用登记：`parser/expr/` 表达式解析处登记 `navigate`/`back` 为内建语义函数名（照 stdlib 函数调用通道——`navigate(path, {params, query})`/`back()` 以 CallExpr 形态进入 AST，参数照 spec §2.3 BNF；本任务只登记不校验——校验在 T4）
- 白名单同步：本任务完成后 BNF 文档（`docs/` 下 grammar 文档若存在对应章节）同 commit 更新

**涉及文件**:
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/lexer/mod.rs`
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/lexer/token.rs`
- Create: `D:/space/OmniLang/crates/omnilang-core/src/parser/top_level/route.rs`
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/top_level/mod.rs`（模块注册）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/top_level/component.rs`（dispatch 接线）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/ast/`（AST 节点 + 组件块成员枚举）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/expr/`（navigate/back 调用登记）

**TDD 测试点**（RED 先写测试，`parser` 测试模块风格——内联 `#[cfg(test)]` 或 tests/）:
- `test_parse_route_decl_static_path`：`@route "/todos";` → 组件块含 RouteDecl{path:"/todos", params:[]}，断言字段值
- `test_parse_route_decl_params_and_attrs`：`@route "/todos/:id" guard: isAdmin lazy: true redirect: "/login";` → 参数段 `["id"]` + 属性齐全
- `test_parse_layout_decl`：`@layout "/admin";` → LayoutDecl{prefix:"/admin"}
- `test_parse_query_decl_with_default`：`@query filter: String = "all";` → QueryDecl{name:"filter", ty:String, default:Some("all")}
- `test_parse_route_outside_component`（V1 负例）：视图内/文件顶层写 `@route` → 解析错误消息含「只能声明在组件顶层块内」
- `test_parse_route_bad_path`（V2 负例）：`@route "todos";`/`@route "";` → 错误含「必须以 `/` 开头且非空」
- `test_parse_route_bad_param_segment`（V5 负例）：`:123`/`::id`/`:id-2` → 错误含「参数段」与「非法」
- `test_parse_navigate_call`：`navigate("/todos/:id", { params: { id: item } })` → CallExpr{callee:"navigate"} + options 结构保持
- `test_parse_back_call`：`back()` → CallExpr{callee:"back"}
- 断言要点：路径/属性/参数段/查询声明的 AST 字段逐项相等；错误消息含 spec §3 表的关键子串

**验证门禁**:
```bash
cargo test -p omnilang-core parser 2>&1 | tail -5
cargo clippy -p omnilang-core 2>&1 | tail -3
```
预期：新测试全绿 + 既有 2046 tests 不回归 + clippy 零新增告警。

**依赖**: Task 1（登记先行——parser 白名单进 BNF 前域已登记）

### Task 3: IR + lowering——RouteIntentIR/LayoutIntentIR/QueryParamIR + navigate/back 调用 IR + ts-rs 同步

**内容**（引用 spec §5.1 白名单锚点「ir/lower/route.rs」+ 基线 §8 登记纪律）:
- `crates/omnilang-core/src/ir/types.rs`：`ComponentIR`（:955-997）新增字段（照 `form_intent`/`load_intent` 先例，`#[serde(default)]`）：
  - `route_intent: Option<RouteIntentIR>`（path/params 列表/guard_fn 名/lazy 标志/redirect 目标）
  - `layout_intent: Option<LayoutIntentIR>`（prefix）
  - `query_params: Vec<QueryParamIR>`（name/ty/default 表达式）
  - 新结构体 RouteIntentIR/LayoutIntentIR/QueryParamIR（序列化 + Clone）
- `crates/omnilang-core/src/ir/logic.rs:97` ModuleIR：无新增（路由信息挂组件级；跨文件注册表在 AST 层 T5）
- lower：新文件 `crates/omnilang-core/src/ir/lower/route.rs`（`collect_route_intent`/`collect_layout_intent`/`collect_query_params`，照 `collect_form_intent`/`collect_load_intent` 先例——`ir/lower/component.rs:417` 同列接线）；`ir/lower/mod.rs` 注册
- navigate/back 调用：表达式 IR 无需新变体——CallExpr 携带 callee 名，codegen 按内建名分派（stdlib 函数先例：`expr_uses_stdlib_fns` `omnilang/src/codegen/flutter/mod.rs:2823`）；本任务确认 IR 层 round-trip 不丢 navigate/back 调用信息（测试点）
- ts-rs 绑定同步：`cargo test export_bindings --lib -p omnilang` 重新生成 `poiesis/src/commands/__generated__/`（IR 结构变化必须同步）

**涉及文件**:
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/types.rs`
- Create: `D:/space/OmniLang/crates/omnilang-core/src/ir/lower/route.rs`
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/lower/component.rs`（collect 接线）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/ir/lower/mod.rs`
- Modify: `D:/space/OmniLang/poiesis/src/commands/__generated__/`（export_bindings 重生成）

**TDD 测试点**:
- `test_lower_route_intent`：带 `@route "/todos/:id" guard: isAdmin lazy: true;` 的组件 → lower 后 ComponentIR.route_intent 为 Some，字段逐项相等（path/params/guard/lazy/redirect）
- `test_lower_layout_intent`：`@layout "/admin";` → layout_intent 为 Some{prefix:"/admin"}
- `test_lower_query_params`：`@query filter: String = "all";` → query_params 含 {name:"filter", ty:String, default:Some(...)}
- `test_lower_navigate_back_roundtrip`：视图事件绑定 `@onPress={() => { navigate("/todos/:id", { params: { id: item } }); back() }}` → IR 表达式树保留 navigate/back 调用（callee 名 + 参数结构不丢失）
- `test_export_bindings_sync`：`cargo test export_bindings --lib -p omnilang` 后 git status 干净（绑定已同步，无漂移）
- 断言要点：IR 字段逐项相等；Option 形态正确；ts-rs 生成无 diff

**验证门禁**:
```bash
cargo test -p omnilang-core lower 2>&1 | tail -5
cargo test export_bindings --lib -p omnilang 2>&1 | tail -3
cargo clippy -p omnilang-core 2>&1 | tail -3
```

**依赖**: Task 2

### Task 4: 语义验证 pass——validate_route_navigation（V3-V13 + W1 逐组件）

**内容**（引用 spec §3 V3-V13/W1 表——错误消息逐字）:
- 新文件 `crates/omnilang-core/src/semantic/validate_route_navigation.rs`（照 `validate_form_load.rs` 先例：VALIDATE_RULES 常量风格 + 校验函数 + 错误/警告收集）：
  - V3：@route 与 @layout 互斥
  - V4：@route 重复声明（同组件两条 → 错误消息含两条路径）
  - V6：路径参数段重名（`/a/:id/b/:id`）
  - V7：注入参数名与显式 prop/signal/event/fn 同名且类型不兼容（同名 String prop 允许——路径参数注入为 String prop，显式 `prop id: String` 兼容放行）
  - V8：@query 名与路径参数同名
  - V9：guard fn 引用存在性 + 签名 `() -> Bool`（查组件 fn 表）
  - V10：@layout 组件视图含 `<@slot/>`（视图树遍历）
  - V11：redirect 未与 guard 配对
  - V12：navigate() 首参为字面量且非 `/` 开头 → 硬错误（外部链接引导 `<a href>`）
  - V13：组件成员名（fn/event/prop/signal）与保留名 navigate/back 冲突 → 硬错误
  - W1：navigate() 首参为非常量表达式 → 警告（动态路径无法静态校验）
- 接线：`semantic/mod.rs` 校验链注册（与 validate_form_load 同层调用——需确认 semantic 入口函数在 analyse/lower 管线中的调用点，照 form-load 先例接线）
- 语义化即切换：本任务**不做**旧形态硬诊断（`@if` 互斥/onNavigate 事件合法——spec §6 边界 6）

**涉及文件**:
- Create: `D:/space/OmniLang/crates/omnilang-core/src/semantic/validate_route_navigation.rs`
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/semantic/mod.rs`（校验链接线）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/ast/`（如需暴露视图树遍历接口——查 `@slot` 既有语义校验先例）

**TDD 测试点**（每规则一正一负，错误消息断言含 spec §3 关键子串）:
- `test_v3_route_layout_exclusive`：组件同声明 @route 与 @layout → 错误含「互斥」
- `test_v4_duplicate_route`：`@route "/a"; @route "/b";` → 错误含「重复声明 @route」与两路径
- `test_v6_duplicate_param`：`@route "/a/:id/b/:id"` → 错误含「参数段 `:id` 重复」
- `test_v7_param_name_conflict`：`@route "/x/:id"; prop id: String;` → 放行（String 兼容）；`@route "/x/:id"; signal id: Bool;` → 错误含「类型不兼容」
- `test_v8_query_param_conflict`：`@route "/x/:filter"; @query filter: String;` → 错误含「同名冲突」
- `test_v9_guard_fn_missing`：`guard: isAdmin` 无此 fn → 错误含「未找到或签名不是」；`fn isAdmin() -> Bool` 存在 → 通过；`fn isAdmin(name: String) -> Bool` → 错误（签名非法）
- `test_v10_layout_needs_slot`：@layout 视图无 `<@slot/>` → 错误含「未包含 `<@slot/>`」；含 → 通过
- `test_v11_redirect_requires_guard`：`redirect: "/login"` 无 guard → 错误含「必须与 `guard:` 配对」；`guard: isAdmin redirect: "/login"` → 通过；`guard: isAdmin` 无 redirect → 通过（默认 `/`）
- `test_v12_navigate_external_path`：`navigate("https://x.com")` → 错误含「只接受应用内路径」与「`<a href>`」
- `test_v13_reserved_name`：`fn navigate() {}` → 错误含「保留名」；`signal back: String` → 错误；正常成员名 → 通过
- `test_w1_navigate_dynamic_path`：`navigate(pageVar)` → 警告含「动态表达式」（不阻塞编译）
- 断言要点：severity/kind 正确（硬错误 = Error、警告 = Warning）；消息与 spec §3 表逐字一致（至少关键子串）

**验证门禁**:
```bash
cargo test -p omnilang-core semantic 2>&1 | tail -5
cargo clippy -p omnilang-core 2>&1 | tail -3
```

**依赖**: Task 2（AST 层有 route/navigate 信息）

### Task 5: 全工程 @route 注册表——V14 硬错误 + W2 跨文件警告

**内容**（引用 spec §3 V14/W2 + §5.1 白名单「全工程 @route 注册表——编译器 import 图遍历」+ §6 边界 8）:
- `crates/omnilang-core/src/validate.rs`：`validate_multi_ref`（:73）新增 **Phase 5: 路由注册表校验**（`validate_route_registry` 独立函数，照 Phase 2/3/4 先例追加）：
  - 遍历所有已解析 program 的组件块，收集 `@route` 路径全集 + `@layout` 前缀全集（AST 级——无需 IR/lowering）
  - V14 硬错误：两组件声明相同 @route 路径，或 @route 路径与 @layout 前缀相同 → 消息含「路径 '{}' 已被组件 '{}' 声明」（错误消息逐字照 spec §3 表）
  - W2 警告：遍历所有组件体内 navigate 调用（字面量路径），未匹配任何已声明 @route/@layout → 警告「未匹配到任何 @route/@layout 声明」（动态路径跳过——W1 已覆盖）
  - 注册表作用域：编译入口的源文件集（`analyse_multi`/`analyse_multi_cached` 传入的 sources——`omnilang/src/lib.rs` :297/:322 两个入口都经 `validate_multi_ref` :433/:624，注册表随 validate_multi_ref 自动覆盖两入口）
- 诊断构造：照 validate.rs 既有 `make_diag`/Diagnostic 风格（file 级 line=0 或定位到声明处——V14 报第二个声明组件所在文件）
- 接线验证：`analyse_multi`/`analyse_multi_cached` 两入口都过 Phase 5（测试点覆盖）

**涉及文件**:
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/validate.rs`
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/parser/ast/`（如需要组件块遍历辅助——只读）

**TDD 测试点**:
- `test_v14_duplicate_route_path_cross_file`：文件 A 组件 X `@route "/todos"` + 文件 B 组件 Y `@route "/todos"` → `validate_multi_ref` 返回错误含「已被组件」与路径
- `test_v14_route_layout_prefix_conflict`：组件 X `@route "/admin"` + 组件 Y `@layout "/admin"` → 硬错误（同路径/前缀不得由多个组件声明）
- `test_v14_same_file_duplicate`：单文件内两组件同路径 → 同样报（注册表不分文件）
- `test_w2_navigate_unregistered_path`：组件 navigate("/nonexistent") 且无任何 @route "/nonexistent" → 警告含「未匹配到任何 @route/@layout」
- `test_w2_navigate_registered_path`：`@route "/todos"` + `navigate("/todos")` → 零警告（注册表命中）
- `test_registry_scope_both_entries`：`analyse_multi` 与 `analyse_multi_cached`（lib.rs 两入口）各跑一遍 V14 用例 → 都报错（注册表覆盖两入口）
- 断言要点：V14 为 Error severity；W2 为 Warning severity 且不阻塞编译（errors 为空、warnings 含该项——照 MultiFileResult 结构确认 warnings 承载位，若 MultiFileResult 仅 errors 则 W2 走 compiler warnings 通道并在测试断言处确认）

**验证门禁**:
```bash
cargo test -p omnilang-core validate 2>&1 | tail -5
cargo test -p omnilang-core 2>&1 | tail -3
cargo clippy -p omnilang-core 2>&1 | tail -3
```

**依赖**: Task 2（AST 含 route/layout/navigate 信息）；建议 Task 4 后（语义 pass 先就位，注册表复用 navigate 调用识别——若 navigate 调用识别逻辑在 T4 已有，T5 直接复用；否则 T5 独立实现 AST 遍历）

### Task 6: React codegen——react/router.rs（data router 路由表 + 守卫 loader + 懒加载 + 参数注入 + navigate/back 翻译）

**内容**（引用 spec §4.1/§4.2 React 形态 + §4.3 floor 契约 C1-C8 + §2.7 深链）:
- 新文件 `omnilang/src/codegen/react/router.rs`：
  - 收集 IR 中全部 `route_intent`/`layout_intent`/`query_params`（`IRProgram.all_components()` 遍历）
  - 发射 `App.tsx`：`createBrowserRouter` 路由表 + `RouterProvider`（spec §4.2 代码形态——路由表/嵌套 children/loader 守卫 redirect/路由级 lazy + Suspense）
  - 页面组件文件内接线：navigate → `const navigate = useNavigate();` + 调用点翻译（模板路径 + params 拼接、query 拼入 URL 字符串）；back → `navigate(-1)`
  - 参数注入：路由表 `element={<TodoDetailPage id={params.id}/>}`（loader 内取 params 透传——spec §4.2）
  - 查询参数：`useSearchParams` 读取注入 prop（`@query` 声明了的才注入）/ navigate 时拼 URL
  - 依赖注入沿 Zustand 先例（design.rs:556）：发射 `import { createBrowserRouter, RouterProvider, redirect } from 'react-router';`；依赖项由 react-test-app 模板维护——`test/projects/learn-omniview/build/react-test-app/package.json` 添加 react-router（精确版本号实施期按官方最新 v7 确定——spec §9 未决 4；tsc 门禁验证）
  - 深链：URL 天然事实源（createBrowserRouter 默认 history 形态）——无需额外接线；降级注记发射（如适用）
  - 懒加载：`lazy: () => import(...)` 路由级形态（spec §2.8/§4.2）
  - 编译器职责降级路径（spec §4.1 注）：若 data router 迁移代价过大，降级 BrowserRouter + 条件渲染 `<Navigate>`——实施期先试 data router，tsc 门禁裁决，降级需在计划审查中记录（诚实降级注记，语言面不变）
- 接线：`react/mod.rs:39` generate 内调用 router 发射（路由信息存在时输出 App.tsx；无 @route 组件时不输出 router 文件——既有工程不回归）

**涉及文件**:
- Create: `D:/space/OmniLang/omnilang/src/codegen/react/router.rs`
- Modify: `D:/space/OmniLang/omnilang/src/codegen/react/mod.rs`（generate 接线 + 模块注册）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/react/component/`（navigate/back 调用翻译接入组件 fn 生成）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/build/react-test-app/package.json`（react-router 依赖注入）

**TDD 测试点**（Rust 单元断言风格——react/tests.rs 先例；编译后 tsc 为最终裁决）:
- `test_react_router_table_emission`：IR 含 @route 组件 ×2（静态 + 参数路径）→ 生成 App.tsx 含 `createBrowserRouter` + 两条路由表项（`path: '/todos'`/`path: '/todos/:id'`）+ `RouterProvider`
- `test_react_guard_loader_redirect`：`guard: isAdmin redirect: "/login"` → 路由项含 `loader: () => (isAuthorized() ? null : redirect('/login'))` 形态（守卫 fn 名按组件 fn 翻译）
- `test_react_nested_layout_children`：@layout + 子 @route → 嵌套 children 路由项 + `<Outlet/>` 于布局组件视图出口（@slot 位置）
- `test_react_param_injection`：`@route "/todos/:id"` → 路由表 `element={<TodoDetailPage id={params.id}/>}` 注入形态
- `test_react_lazy_route`：`lazy: true` → `lazy: () => import(...)` + Suspense fallback 包装
- `test_react_navigate_translation`：组件事件内 `navigate("/todos/:id", { params: { id: item } })` → `navigate('/todos/' + item)` 形态；`navigate("/todos", { query: { filter: "all" } })` → URL 拼接形态；`back()` → `navigate(-1)`
- `test_react_query_param_prop`：`@query filter: String = "all"` → useSearchParams 读取注入 prop 形态
- `test_react_no_route_no_router_file`：IR 无任何 @route/@layout → 不输出 App.tsx/router 文件（既有工程零回归）
- 断言要点：生成字符串含关键代码形态子串（`createBrowserRouter`/`RouterProvider`/`redirect(`/`useNavigate`/`Outlet`/`Suspense`/`params.`/`useSearchParams`）

**验证门禁**:
```bash
cargo test -p omnilang codegen::react 2>&1 | tail -5
cd test/projects/learn-omniview && omnic build --target react 2>&1 | tail -5   # 重生成产物
cd build/react-test-app && npx tsc --noEmit 2>&1 | tail -5   # 层 1 门禁
```

**依赖**: Task 3（IR 有 route_intent）、Task 5（V14/W2 保证注册表路径合法——codegen 不重复校验）

### Task 7: Flutter codegen——flutter/router.rs（GoRouter 配置 + redirect/ShellRoute + pathParameters 注入 + context.push/pop）

**内容**（引用 spec §4.1/§4.2 Flutter 形态 + §4.3 floor 契约 + §2.7 深链/§2.8 懒加载降级）:
- 新文件 `omnilang/src/codegen/flutter/router.rs`：
  - 收集 route/layout intent → 发射 `router.dart`：`GoRouter(initialLocation: '/', routes: [...])` + `MaterialApp.router(routerConfig: router)` 接线（spec §4.2 代码形态）
  - GoRoute 表：静态/参数路径；`builder: (_, s) => XPage(id: s.pathParameters['id']!)` 参数注入形态
  - 守卫：`redirect: (_, __) => isAuthorized() ? null : '/login'`（组件 fn 翻译）
  - 嵌套：`ShellRoute(builder: (_, __, child) => AdminLayout(child: child), routes: [...])`——布局组件接收 child prop（@slot 出口）
  - navigate → `context.push(path)`（栈语义）；back → `context.pop()`（栈空 no-op 降级注记注释发射）
  - 查询参数：`uri.queryParameters` 读取注入 prop / navigate 时拼
  - 懒加载：**移动端 no-op + 编译注记**（spec §6 边界 5——注释「deferred import（web）// 移动端单 bundle 无按需加载」）；web deferred 形态以注释注记先行
  - 深链：GoRouter web URL 同步天然；移动端平台清单**只出注记注释不生成清单**（spec §6 边界 4）
- 接线：`flutter/mod.rs:47` generate 内调用（路由信息存在时输出 router.dart + main.dart 接线）
- 依赖：flutter-test-app 模板 `pubspec.yaml` 添加 go_router 依赖（版本实施期按 pub.dev 官方最新；dart analyze 门禁验证）

**涉及文件**:
- Create: `D:/space/OmniLang/omnilang/src/codegen/flutter/router.rs`
- Modify: `D:/space/OmniLang/omnilang/src/codegen/flutter/mod.rs`（generate 接线 + 模块注册）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/flutter/`（navigate/back 语句翻译接入）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/build/flutter-test-app/pubspec.yaml`（go_router 依赖）

**TDD 测试点**:
- `test_flutter_gorouter_table_emission`：静态 + 参数路径组件 → router.dart 含 `GoRouter(` + `GoRoute(path: '/todos'` + `GoRoute(path: '/todos/:id'` + `MaterialApp.router`
- `test_flutter_guard_redirect`：guard → `redirect: (_, __) => isAuthorized() ? null : '/login'` 形态
- `test_flutter_shell_route_nested`：@layout + 子 @route → `ShellRoute(` + 布局组件 child 注入 + 子 GoRoute
- `test_flutter_param_injection`：`:id` → `s.pathParameters['id']!` 注入形态
- `test_flutter_query_param`：@query → `uri.queryParameters['filter']` 读取注入
- `test_flutter_navigate_back_translation`：`navigate(...)` → `context.push(...)`；`back()` → `context.pop()`；栈空 no-op 注记注释存在
- `test_flutter_lazy_mobile_noop_note`：lazy: true → 生成注释含「移动端」与 no-op 语义（诚实降级注记，绝不静默）
- `test_flutter_no_route_no_router`：无 @route → 不输出 router.dart（零回归）
- 断言要点：生成字符串含关键形态子串；降级注记注释存在

**验证门禁**:
```bash
cargo test -p omnilang codegen::flutter 2>&1 | tail -5
cd test/projects/learn-omniview && omnic build --target flutter 2>&1 | tail -5   # 重生成产物
cd build/flutter-test-app && dart analyze 2>&1 | tail -5   # 层 2 门禁
```

**依赖**: Task 3（IR）；Task 5（注册表合法）

### Task 8: OmniView codegen——omniview/router.rs（hash 路由运行时 + 守卫 + 嵌套渲染 + 参数/查询解析 + navigate/back）

**内容**（引用 spec §4.2 OmniView 形态 + §4.3 floor 契约 + §2.7/§2.8）:
- 新文件 `omnilang/src/codegen/omniview/router.rs`：
  - 路由运行时发射（照 spec §4.2 伪代码形态——`runtime.rs`「No external dependencies」零依赖约束）：
    - `hashchange` 监听 + 路由表匹配（前缀/参数段/查询参数解析）
    - 页面切换渲染：参数段 → prop 注入（`{Comp}` 实例化传入）；查询参数 → 注入 prop
    - 守卫 fn：路由命中后调用组件守卫 fn，false → `location.hash` 改重定向目标（默认 `/`）
    - 嵌套渲染：布局组件 + 子页面渲染进 `<@slot/>` 出口位置
    - `navigate()` → `location.hash = '#/...'`（模板路径 + params 替换、query 拼）；`back()` → `history.back()`
    - 懒加载：动态 import 页面组件（OmniView 端形态；若运行环境不支持动态 import，以同文件内联 + 编译注记降级——实施期按 runtime 环境实测裁决，诚实降级）
  - 深链：hash 可直达可分享；URL 形态 `#/path` 降级注记（非 history pushState——运行环境限制，spec §4.2 注）
- 接线：`omniview/mod.rs:149` generate 内调用 + 输出组装（OmniViewOutput js/html/css 结构——路由 js 注入 js 输出，入口 html 接路由挂载）

**涉及文件**:
- Create: `D:/space/OmniLang/omnilang/src/codegen/omniview/router.rs`
- Modify: `D:/space/OmniLang/omnilang/src/codegen/omniview/mod.rs`（generate 接线 + 模块注册）
- Modify: `D:/space/OmniLang/omnilang/src/codegen/omniview/`（navigate/back 语句翻译 + 组件实例化参数注入接入）

**TDD 测试点**:
- `test_omniview_hash_router_emission`：@route 组件集 → js 含 `hashchange` 监听 + 路由表（路径/参数段模式匹配）
- `test_omniview_param_and_query_injection`：`:id` + @query → 路由命中后注入 prop 形态（参数段解析 + URLSearchParams 读取）
- `test_omniview_guard_redirect`：guard → 守卫 fn 调用 + 失败改 hash 到重定向目标（默认 `/`）
- `test_omniview_nested_layout_render`：@layout + 子 @route → 布局渲染 + 子页面进插槽（@slot 出口）
- `test_omniview_navigate_back`：`navigate("/todos/42")` → `location.hash = '#/todos/42'` 形态；query 拼接；`back()` → `history.back()`
- `test_omniview_no_route_regression`：无 @route → 现有输出零变化（既有 103 文件工程不回归）
- 断言要点：js 输出含关键形态子串；零依赖（无外部 import）

**验证门禁**:
```bash
cargo test -p omnilang codegen::omniview 2>&1 | tail -5
cd test/projects/learn-omniview && omnic build --target omniview 2>&1 | tail -5
cargo test -p omnilang 2>&1 | tail -3   # 全量 codegen 回归
```

**依赖**: Task 3（IR）；Task 5

### Task 9: 登记册收尾 + 注入语料 + 导航文档——白名单锚点回填 + verify 脚本锚点断言 + 855 元组扩展 + docs/omniview 章节

**内容**（引用 spec §5.1 白名单回填 + §5.3 锚点方案 + §7 层 4/层 5 + §8 #7）:
- semantic-universe 域 11：实现状态列「未支持」→「支持」回填（8 条目）；已实现白名单 6 锚点回填（parser route.rs/lower route.rs/expr 内建登记/react router.rs/flutter router.rs/omniview router.rs——文件已存在，补行号/常量名）
- `verify-semantic-universe.py`：域 11 白名单锚点断言加入（存在性检查——照域 8/9 断言模式：锚点文件路径 + 关键符号 grep）；保留名 navigate/back 与既有保留名表无重断言
- `verify-vocabulary.py`：域 7 词汇锚点断言（对照 parser route.rs 关键词/BNF 生产式词——照 `bnf_doc_subset` 先例 :304）+ 排除词汇禁借断言（目标端词 path/route/Routes/GoRoute/Outlet/ShellRoute/query/searchParams/push/go/useNavigate/Navigate/pop 不得成为语义词汇）
- 注入语料（spec §7 层 5）：路由语法元组 12+ 进 Rust 测试语料（compiler test corpus——@route 静态路径/@route :param/@route guard/@route lazy/@layout/@query/navigate 字面量/navigate+params/navigate+query/back/参数注入引用/守卫失败重定向——12 元组正负例进 omnilang-core tests；用例名与断言要点照 V1-V14 任务测试风格）
- 导航文档：`docs/omniview/` 导航相关章节（13-project-structure 等）——`@route`/`navigate`/`back` 用法 + floor 差异注记（深链/懒加载/栈空 back）+ tab 与路由边界（spec §8 #7 + §6 边界 6 文档引导）

**涉及文件**:
- Modify: `D:/space/OmniLang/docs/superpowers/specs/semantic-universe.md`（域 11 状态回填 + 白名单锚点）
- Modify: `D:/space/OmniLang/scripts/verify-semantic-universe.py`（锚点断言）
- Modify: `D:/space/OmniLang/scripts/verify-vocabulary.py`（词汇锚点 + 排除词汇断言）
- Modify: `D:/space/OmniLang/crates/omnilang-core/src/`（注入语料测试——tests 目录或 parser/semantic 测试模块）
- Modify: `D:/space/OmniLang/docs/omniview/`（导航章节）

**TDD 测试点**:
- 注入语料 12+ 元组：每个元组一正一负（RED 先写测试再补实现——若 T2-T5 已实现则本任务直接断言绿；重点覆盖负例：非法路径/重复路径/保留名冲突在语料层再确认）
- verify 脚本跑挂→回填→跑绿：先加锚点断言（锚点文件缺失 → 挂）→ 回填 universe/registry → 绿
- `test_registry_no_reserved_name_collision`：universe 保留名表与 navigate/back 无重（脚本断言）
- 断言要点：脚本全绿 + cargo 全量绿

**验证门禁**:
```bash
python scripts/verify-semantic-universe.py
python scripts/verify-vocabulary.py
cargo test 2>&1 | tail -5   # 全量（含注入语料 12+ 元组）
cargo clippy --all-targets 2>&1 | tail -3
```

**依赖**: Task 6/7/8（锚点文件存在后回填）

### Task 10: demo 迁移 + Playwright——NavDemo 全要素 + main.omni/Overview 接线 + 10 条路由断言

**内容**（引用 spec §8 迁移清单 7 项 + §7 Playwright 断言清单 10 条 + §9 裁决 1-3 执行口径）:
- §8 #1/#2：`test/projects/learn-omniview/src/components/PageSwitcherDemo.omni` 迁移为**导航演示**——HomePage `@route "/home"`、AboutPage `@route "/about"`、按钮改 `navigate("/about")`/`navigate("/home")`、删除 PageSwitcherDemo 外壳与 onSwitch 事件；Flutter 保挂载差异注记注释（边界 7——标准 ShellRoute 落地）；重生成三目标构建产物 + `build/flutter-test-app/test/page_switch_test.dart` 断言改路由形态（切换断言 → URL 变化 + 页面渲染）
- §8 #5：新文件 `test/projects/learn-omniview/src/pages/NavigationDemo.omni`——C1-C8 全要素：页面切换 + `/todos/:id` 参数详情 + `@query filter` + guard 登录示例（admin 页面 + 重定向 `/login`）+ `@layout` 嵌套（AdminLayout）+ `lazy: true` 页面 + back 按钮（spec §8 #5 清单）
- §8 #6：main.omni 引入 NavigationDemo + TabBar/Overview 挂入口（9 标签页保留 tab UI 态——§9 未决 1 已定：不路由化）
- §8 #4：Overview.omni CapabilityCard 事件模式保留（合法组件事件）；「跳页面」意图改 `navigate()`（NavDemo 入口卡片用 navigate("/nav")——新页面 @route 路径）
- Playwright：`build/react-test-app/tests/behavioral-parity.spec.ts` 扩展 10 条路由断言（spec §7 清单逐条）：默认路由渲染/点击切换 + URL 同步/直接访问 `#/todos/42` 参数注入/`?filter=all` 注入/返回按钮/浏览器后退/未授权 /admin 重定向 /admin/users 嵌套布局/懒加载 Suspense fallback/未注册路径空路由兜底（React + OmniView 编译产物各跑）
- 文档注记（§8 #7 在 T9 已做，本任务补 demo 迁移注记）

**涉及文件**:
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/components/PageSwitcherDemo.omni`
- Create: `D:/space/OmniLang/test/projects/learn-omniview/src/pages/NavigationDemo.omni`
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/main.omni`（NavDemo 入口接线）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/src/pages/Overview.omni`（navigate 意图）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/build/react-test-app/tests/behavioral-parity.spec.ts`（10 条断言）
- Modify: `D:/space/OmniLang/test/projects/learn-omniview/build/flutter-test-app/test/page_switch_test.dart`（路由形态断言）

**TDD 测试点**（Playwright 断言先写 RED——新断言跑挂 → 迁移 demo → 跑绿）:
- `route default renders`：启动 → `/` 页面渲染（断言 1）
- `route navigate updates url and view`：点击导航按钮 → 页面切换 + 地址栏 URL 同步（断言 2，React history / OmniView hash 两形态）
- `route param injection direct access`：直接访问 `#/todos/42` → 渲染「待办 #42」（断言 3）
- `route query param injection`：`#/todos?filter=all` → filter prop 生效（断言 4）
- `route back button`：点击返回 → 上一页（断言 5）
- `route browser back`：浏览器后退 → 页面返回（断言 6）
- `route guard redirects unauth`：未授权访问 /admin → 重定向 /login 且 admin 内容不渲染（断言 7）
- `route nested layout`：访问 /admin/users → AdminLayout 外壳（header）渲染 + AdminUsers 内嵌 @slot 出口（断言 8）
- `route lazy suspense fallback`：懒加载页面 → Suspense fallback 出现后内容渲染（断言 9）
- `route unregistered path fallback`：navigate 未注册路径 → 空路由兜底（断言 10——W2 已警告，行为由目标端决定）
- 断言要点：URL 断言 + 文本断言 + 元素存在性断言（spec §7 清单逐条对应）

**验证门禁**（五层终态全量）:
```bash
# 三目标重生成
cd test/projects/learn-omniview && omnic build --target react && omnic build --target flutter && omnic build --target omniview
# 层 1 tsc
cd build/react-test-app && npx tsc --noEmit
# 层 2 dart analyze
cd build/flutter-test-app && dart analyze
# 层 3 Playwright
cd build/react-test-app && npx playwright test tests/behavioral-parity.spec.ts
# 层 4 docs-gate
python scripts/verify-semantic-universe.py && python scripts/verify-vocabulary.py
# 层 5 注入语料
cargo test 2>&1 | tail -5
```
预期：五层全绿。

**依赖**: Task 6/7/8（codegen 可用）、Task 9（文档/语料就位）

## 3. 阶段顺序

```
T1 登记先行 ──→ T2 parser ──→ T3 IR+lower+ts-rs ──→ T6 React codegen ──→ T9 登记收尾+语料+文档
                （严格串行）    （串行）              T7 Flutter codegen  ──→ T10 demo+Playwright
                                                     T8 OmniView codegen ──┘
                                     └─→ T4 语义 pass（V3-V13+W1）──→ T5 注册表（V14+W2）
```

- **串行主线**：T1 → T2 → T3 →（T4/T5 语义层）→（T6/T7/T8 三目标并行）→ T9 → T10
- **可并行**：T4 与 T3 可并行（都依赖 T2；T4 纯 AST 级）；T6/T7/T8 三目标互不依赖可并行；T9 依赖 T6/T7/T8（锚点回填）
- **依赖摘要**：T1(无) → T2(T1) → T3(T2)；T4(T2)；T5(T2, 建议 T4 后) → T6/T7/T8(T3, T5) → T9(T6/T7/T8) → T10(T6/T7/T8, T9)

## 4. 登记先行任务

- **Task 1** 完成 semantic-universe 域 11（8 条目，状态「未支持」+ 注记）+ vocabulary-registry 域 7（5 词汇四字段）+ 双 verify 脚本域存在性断言——**严格先于 Task 2 parser 实施**（ADR-074：先登记 → 才进 BNF/白名单 → 才实施）
- **Task 9** 回填：域 11 状态「未支持」→「支持」+ 已实现白名单 6 锚点 + verify 脚本锚点断言（存在性检查——机制级条目照 ADR-074 例外）+ 保留名无重断言
- 若批次 1（域 10 反馈模态）在 T1 执行时已合并入 main：域循环含 10 正常处理；未合并则域 11 按 11 登记，域 10 归批次 1 收尾

## 5. 语义化即切换任务（逐语料位）

无旧语法硬诊断（ADR-075 原则 8——`@if` 互斥/`event onNavigate` 是合法语法）；收口 = 文档引导 + demo 迁移：

| 语料位 | 动作 | 任务 |
|---|---|---|
| PageSwitcherDemo.omni（HomePage/AboutPage 手写 @if 互斥 + onSwitch 事件） | 迁移为 @route + navigate（spec §8 #1/#2） | T10 |
| build 产物 12 件 + page_switch_test.dart | 同步重生成 + 断言改路由形态 | T10 |
| main.omni 9 标签页 @if 链 | **保留 tab UI 态**（§9 未决 1 已定：tab ≠ 路由）——仅挂 NavDemo 入口 | T10 |
| Overview.omni onNavigate 事件 | 事件模式保留（合法）；「跳页面」意图改 navigate() | T10 |
| 新增 NavigationDemo.omni | C1-C8 全要素新 demo | T10 |
| docs/omniview 导航章节 | @route/navigate/back 用法 + floor 差异注记 + tab 与路由边界（spec §8 #7） | T9 |

## 6. 门禁终态（五层）

| 层 | 命令 | 验收口径 |
|---|---|---|
| 1 tsc | `cd test/projects/learn-omniview/build/react-test-app && npx tsc --noEmit` | 零错误——createBrowserRouter/RouterProvider/loader/useNavigate/useSearchParams/Suspense 生成代码过类型检查 |
| 2 dart analyze | `cd test/projects/learn-omniview/build/flutter-test-app && dart analyze` | 零告警——GoRoute/redirect/ShellRoute/pathParameters/uri.queryParameters/context.push/pop 零 analyzer 问题 |
| 3 Playwright | `cd build/react-test-app && npx playwright test tests/behavioral-parity.spec.ts` | 10 条路由断言全绿（React + OmniView 编译产物） |
| 4 docs-gate | `python scripts/verify-semantic-universe.py && python scripts/verify-vocabulary.py` | 域 11 全绿 + 域 7 全绿 + docs/omniview 与登记册同步（T9 锚点断言 + T10 文档） |
| 5 注入语料 | `cargo test`（全量） | 路由语法元组 12+ 全绿；NavDemo 三目标编译全绿（T9/T10） |

## 7. 风险与降级注记（spec §6 边界落地方式）

| 边界（spec §6） | 落地任务 | 落地方式 |
|---|---|---|
| 1 动态守卫目标 | T4 | v0 `redirect:` 限静态字面量（V11 校验）；语料出现多角色权限时重评——semantic-universe 域 11 注记「留生态」触发条件 |
| 2 通配符 404 fallback | T10 | 留生态；未注册路径空路由兜底行为由目标端决定（Playwright 断言 10 记录行为） |
| 3 表达式守卫 | T4 | v0 限组件级 fn（V9）；注记于域 11 |
| 4 深链平台清单 | T7 | 编译器只出注记注释不生成清单（诚实降级）；web 端对等 |
| 5 懒加载移动端 | T7 | GoRouter 移动端 no-op + 编译注记；React 路由级 lazy；OmniView 动态 import（环境不支持则内联 + 注记） |
| 6 手写页面切换 | T9/T10 | 不可硬诊断——文档引导 + demo 迁移；tab 面板不路由化（§9 未决 1 已定） |
| 7 页面保挂载 | T7/T10 | v0 标准 ShellRoute + 差异注记（§9 未决 3 已定）；indexedStack 变体随语料 |
| 8 navigate 跨文件校验 | T5 | W2 注册表校验在 T5（validate_multi_ref Phase 5）；动态表达式校验属 1F，不阻塞 |
| 9 滚动恢复/预取 | T9 | 留生态注记（域 11） |
| 10 外部链接 | T4 | V12 强制 navigate 只收应用内路径；外链 `<a href>` |

其他风险：
- **React Router v7 API 细节**（spec §9 未决 4）：网络封锁下无法核实——实施期以官方文档为准，tsc 门禁裁决；data router 迁移代价过大则降级 BrowserRouter + `<Navigate>`（诚实降级注记，语言面不变——T6 审查点）
- **W2 警告通道**：validate_multi_ref 的 MultiFileResult 仅 errors 字段——W2 若无法经该结构承载，走 compiler warnings 通道（`omnilang/src/lib.rs:446-462` 既有 warnings 去重机制）并在 T5 测试断言处确认
- **learn-omniview 现有工程回归**：T6-T8 均含「无 @route 不输出 router 文件」测试点——learn-omniview 现有 103 文件在 NavDemo 引入前零变化（react-test-app/flutter-test-app 门禁在 T6/T7 即跑，T10 前不迁移 demo 不触发路由生成）

## 自查结论

- 占位符（TBD/TODO）：清零
- spec 覆盖：C1-C8 全量（T2-T8 表 §1 映射）；V1-V14/W1/W2 全量（T2/V1,V2,V5；T4/V3-V13,W1；T5/V14,W2）；§5 登记（T1 先行 + T9 回填）；§7 五层门禁（各任务门禁 + §6 终态表）；§8 迁移 7 项（T10 六项 + T9 一项）；§9 未决 1-3 执行口径（Global Constraints + T10 注记）、未决 4（T6 注记）
- 与基线 §5/§6/§8 一致：批次 2 独立走 spec → plan → subagent-driven；检查点不跳（可验证产物 + 门禁绿）
