# 阶段 2 域 A：表单 + 数据三态实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 语言新增两个功能设计级语义域——**表单意图**（`@form` + 字段级 `validate:` + 提交校验 + 错误信号 + a11y）与**数据三态意图**（`@load` + 自动生成 loading/error/data 三态信号 + retry），三目标（React / Flutter / OmniView）地道 codegen 收口。

**Architecture:** 既有机制全部复用：`style: { }` 块（ElementProp::Style，element.rs:367）是 `validate: { }` 的解析先例；`@if/@for` 视图级分派（view/mod.rs:31-64，fallback-to-text 模式）是 `@load` 的解析先例；信号/事件/`bind:value` 机制已成熟（阶段 1 @onSubmit schema `{values}` 已落地）。新增：AST 3 个变体 + IR 2 个 intent + 1 个语义 pass（validate_form_load）+ 三目标 codegen 扩展。@load 是**视图级**组件指令（v0 每组件至多一个），@form 是 `<form>` 元素上的裸属性指令。

**Tech Stack:** Rust（omnilang-core / omnilang）、React 生成（tsc 门禁）、Flutter 生成（dart analyze 门禁 + Riverpod 3.0 既有依赖）、OmniView 生成、Playwright 浏览器验证。

## Global Constraints

- 诊断消息**中文 verbatim**（ADR-056 政策）
- **新语法先登记 semantic-universe**（ADR-074）——本计划 Task 7 在实现完成后登记域 8（表单）/域 9（数据三态）并接线双向断言
- `cargo fmt` + `cargo clippy --workspace --all-targets -- -D warnings` 零警告
- 门禁：`cargo test --workspace` 全绿；verify-react GATE PASSED（豁免不新增）；Flutter dart analyze ≤ 基线；docs-gate 0 failed；verify-semantic-universe/vocabulary 0 problems；注入语料（855 元组矩阵）插新点
- **IR 序列化变更 → ts-rs bindings 同步**（poiesis/src/commands/__generated__/，ci.yml:31-38 有 freshness 门禁——lowering 改 IR 后必须跑 `cargo test export_bindings --lib -p omnilang` 并提交产物）
- **@load 与既有 async action 的关系（spec §4.2 决策点）**：@load 是既有 `async action + await` 之上的**三态封装增量**，不替代既有原语——已有 async/await 代码零改动；@load 只接管「组件挂载即取数 + 三态信号 + retry」这一重复样板
- **@load v0 范围**：仅 Array 返回类型（data 零值 `[]`）；struct/其他返回 → 硬诊断「@load 仅支持 Array 返回（v0）」
- **@form v0 范围**：每组件至多一个 @form；@form 必须配 @onSubmit；`validate:` 必须配 `bind:value`/`bind:checked`；validate 规则白名单封闭（required/format/min/max/pattern/custom）
- 保留名（@load 存在时 `loading`/`error`/`data`；@form 存在时 `formErrors`/`formValid`/`formSubmitting`）与用户信号声明冲突 → 硬诊断
- **禁用提交态（spec「编译器生成…禁用提交态」）v0 形态**：编译器生成 `form.submitting` 信号，按钮禁用由用户显式写 `disabled={form.submitting}`——v0 不做自动按钮接线（侵入性：编译器不知哪个按钮）；用例模式写入 docs（Task 7 Step 4）
- 明确 defer（本计划不做）：V6 字段类型检查警告（spec §3.5 继续 defer）；@load 非 Array 返回；多表单/跨组件表单共享；FormData 预填；`form.submitting` 异步语义（Flutter 端同步行为注记）

---

### Task 1: 语法面——lexer 关键字 + AST 变体 + parser 解析

**Files:**
- Modify: `crates/omnilang-core/src/lexer/keywords.rs`（@form/@load 关键字映射，@if 先例 :76-78）
- Modify: `crates/omnilang-core/src/lexer/token.rs`（TokenKind::AtForm/AtLoad）
- Modify: `crates/omnilang-core/src/parser/ast/view.rs`（ElementProp::Form/Validate、ViewNode::LoadDirective）
- Modify: `crates/omnilang-core/src/parser/view/mod.rs`（parse_view_node @load 分派，@if 先例 :38-64）
- Modify: `crates/omnilang-core/src/parser/view/element.rs`（@ 属性分派 @form 臂 + validate: 块解析）

**Interfaces:**
- Consumes: 无
- Produces: `ElementProp::Form`（裸，无值）；`ElementProp::Validate { rules: Vec<ValidateRule> }`（`ValidateRule { name: String, value: Expr }`）；`ViewNode::LoadDirective { fn_name: String, span: Span }`——后续 Task 2 lowering 消费

- [ ] **Step 1: 写失败测试**（lexer/keywords 追加 + parser 测试）

```rust
// lexer/mod.rs tests 追加
#[test]
fn at_form_and_at_load_keywords() {
    assert_token("@form", TokenKind::AtForm);
    assert_token("@load", TokenKind::AtLoad);
}

// parser/view/mod.rs tests 追加（或 element.rs tests）
#[test]
fn parse_form_element_with_validate() {
    let src = r#"
component App {
    fn submit(e) {}
    view {
        <form @form @onSubmit={submit}>
            <input bind:value={name} validate: { required: true, min: 3 } />
        </form>
    }
}
"#;
    let comp = parse_component(src).unwrap();
    // form 元素含 ElementProp::Form（无值）；input 含 ElementProp::Validate { rules }
    // validate 规则解析为 ValidateRule::required(true) + ValidateRule::min(3)
}

#[test]
fn parse_load_directive_in_view() {
    let src = "component App { fn f() {} view { @load={f} <div /> } }";
    let comp = parse_component(src).unwrap();
    // view.children[0] == ViewNode::LoadDirective { fn_name: "f" }
}
```

- [ ] **Step 2: 运行确认失败**
Run: `cargo test -p omnilang-core at_form_and_at_load_keywords` + `cargo test -p omnilang-core parse_load_directive`
Expected: FAIL（TokenKind::AtForm/AtLoad 未定义）

- [ ] **Step 3: 最小实现**
1. `lexer/keywords.rs`：`"@form" => Some(TokenKind::AtForm), "@load" => Some(TokenKind::AtLoad)`；`lexer/token.rs` TokenKind 追加 `AtForm, AtLoad`（AtIf/AtFor 同处）
2. `ast/view.rs` ElementProp 追加（Style 变体同处）：
```rust
/// 表单意图：`@form` 裸属性——激活表单校验/错误信号/a11y 接线（spec 2026-08-08 §4.1）。
Form,
/// 字段校验声明：`validate: { required: true, min: 3 }`——镜像 style: 块形态。
Validate { rules: Vec<ValidateRule> },
```
`ValidateRule { pub name: String, pub value: Expr }`（带 span）放在 ElementProp 旁；ViewNode 追加：
```rust
/// 数据三态意图：`@load={fn}`——视图级指令，生成 loading/error/data 信号（v0 仅 Array 返回）。
LoadDirective { fn_name: String, span: Span },
```
3. `view/mod.rs` parse_view_node 追加 AtLoad 臂（镜像 AtIf :38-50——先试 parse_load_directive，失败回退 collect_at_text("@load")）；`parse_load_directive` 解析 `@load = { IDENTIFIER }`（严格：等号 + 花括号 + 单标识符——其余形态解析失败走 text 回退）
4. `element.rs` @ 属性分派：`@form` 裸属性 → `ElementProp::Form`（在 Ref/Drag 分派同处，element.rs:659/695 附近——注意 @form 无 `=`，分派须在等号消费前判定）；`validate:` 标识符属性（`if attr_name == "validate"`，bind: 先例 :348）→ 解析 `{ name: expr, ... }` 块（镜像 style: 块解析 :354-367——键为规则名标识符，值为表达式）

- [ ] **Step 4: 运行确认通过**
Run: `cargo test -p omnilang-core at_form_and_at_load` + `cargo test -p omnilang-core parse_load_directive` + `cargo test -p omnilang-core parse_form_element_with_validate` + 全量 `cargo test -p omnilang-core`
Expected: PASS（注意：@form/@load 成为文本关键字——既有 text 测试含 "@load" 字样的需检查 fallback 行为，AtLoad 走 fallback-to-text 与 @if 同）

- [ ] **Step 5: Commit**
```bash
git add crates/omnilang-core/src/lexer crates/omnilang-core/src/parser
git commit -m "feat(parser): @form/@load 关键字 + ElementProp::Form/Validate + ViewNode::LoadDirective（Task 1）"
```

---

### Task 2: IR + lowering——FormIntentIR / LoadIntentIR

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs`（新 intent 结构）
- Modify: `crates/omnilang-core/src/ir/lower/component.rs`（lower_component 接线）
- Modify: `crates/omnilang-core/src/ir/lower/mod.rs`（ViewTree 新变体走法）
- Test: 各文件 tests + `ir/mod.rs` re-export

**Interfaces:**
- Consumes: Task 1 的 ElementProp::Form/Validate、ViewNode::LoadDirective
- Produces:
  - `FormIntentIR { fields: Vec<FormFieldIR>, submit_handler: String }`；`FormFieldIR { name: String, rules: Vec<ValidateRuleIR> }`（name = bind: 信号路径的末段）；`ValidateRuleIR { name: String, value: ExprIR }`
  - `LoadIntentIR { fn_name: String }`
  - ComponentIR 追加 `form_intent: Option<FormIntentIR>`、`load_intent: Option<LoadIntentIR>`（序列化：serde derive 自动，ts-rs 重新导出）
  - 内建信号在 IR 层**不生成**信号声明——由 semantic pass 校验保留名冲突、codegen 生成对应实现（三目标各自惯用形态）

- [ ] **Step 1: 写失败测试**（lower/component.rs tests）

```rust
#[test]
fn lower_form_intent_collects_validated_fields() {
    // <form @form @onSubmit={submit}> + <input bind:value={name} validate:{required:true} />
    let ir = lower_component_src(src);
    let form = ir.form_intent.expect("form intent");
    assert_eq!(form.submit_handler, "submit");
    assert_eq!(form.fields.len(), 1);
    assert_eq!(form.fields[0].name, "name");
    assert_eq!(form.fields[0].rules.len(), 1);
}

#[test]
fn lower_load_directive_carries_fn_name() {
    let ir = lower_component_src("component App { fn f() {} view { @load={f} <div/> } }");
    assert_eq!(ir.load_intent.expect("load intent").fn_name, "f");
}
```

- [ ] **Step 2: 运行确认失败**
Run: `cargo test -p omnilang-core lower_form_intent` + `cargo test -p omnilang-core lower_load_directive`
Expected: FAIL（form_intent/load_intent 不存在）

- [ ] **Step 3: 最小实现**
1. `ir/types.rs`：
```rust
/// 表单意图（spec 2026-08-08 §4.1）——@form 元素上的字段校验规则集。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormIntentIR {
    pub fields: Vec<FormFieldIR>,
    pub submit_handler: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFieldIR {
    /// bind: 信号路径末段（errors 信号的键名）
    pub name: String,
    pub rules: Vec<ValidateRuleIR>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRuleIR { pub name: String, pub value: ExprIR }
/// 数据三态意图（spec 2026-08-08 §4.2）——@load 绑定的组件 fn。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadIntentIR { pub fn_name: String }
```
ComponentIR 追加 `pub form_intent: Option<FormIntentIR>, pub load_intent: Option<LoadIntentIR>`（fixtures_shared.rs 等既有 ComponentIR 构造点需补 `form_intent: None, load_intent: None`——grep `ComponentIR {` 全量补）
2. `ir/lower/component.rs`：遍历元素树收集——`ElementProp::Form` 所在元素 → 扫描其子树的 `ElementProp::Validate` + 对应 `ElementProp::Bind`（取 signal_path 末段为 field name）+ 同元素 `ElementProp::Event`（name == "onSubmit"）的 handler；`ViewNode::LoadDirective` → LoadIntentIR。收集遍历复用既有 walk 模式（lower 内 view 递归）
3. `ir/lower/mod.rs`：ViewNode::LoadDirective 分支（Element/If/For 同处）

- [ ] **Step 4: 验证 + ts-rs 同步**
Run: `cargo test -p omnilang-core lower_form_intent` + `cargo test -p omnilang-core`（全量回归）
Run: `cargo test export_bindings --lib -p omnilang`（ts-rs 重新生成）→ `git diff poiesis/src/commands/__generated__/` 确认无意外变更
Expected: PASS + 生成产物 diff 仅新字段

- [ ] **Step 5: Commit**
```bash
git add crates/omnilang-core/src/ir poiesis/src/commands/__generated__
git commit -m "feat(ir): FormIntentIR/LoadIntentIR + lowering 收集（Task 2）"
```

---

### Task 3: 语义验证——validate_form_load（规则白名单 + 约束 + 保留名冲突）

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_form_load.rs`
- Modify: `crates/omnilang-core/src/semantic/mod.rs`（pass 注册）

**Interfaces:**
- Consumes: Task 2 的 FormIntentIR/LoadIntentIR（pass 消费 IR）
- Produces: 诊断（Severity::Error，中文消息）——规则见下；`pub const VALIDATE_RULES: &[&str]`（白名单，Task 7 登记册锚点）

- [ ] **Step 1: 写失败测试**

```rust
/// V 清单（spec §4.1/§4.2 落地）：
/// V-F1 @form 只能在 <form> 元素上（tag 判定）→ error「@form 只能用于 form 元素」
/// V-F2 @form 必须配 @onSubmit → error「@form 元素必须绑定 @onSubmit」
/// V-F3 validate: 必须配 bind:value/bind:checked（表单意图外使用 = 孤儿规则）→ error「validate: 必须与 bind: 配对使用」
/// V-F4 validate 规则名 ∈ VALIDATE_RULES → error「未知校验规则 "@x"——合法规则: required/format/min/max/pattern/custom」
/// V-F5 规则值类型：required → Bool；min/max → Number；format → "email"|"url" 枚举；pattern → String；custom → 组件 fn 名（存在 + 单参 + 返回 String）
/// V-F6 每组件至多一个 @form / 一个 @load → error「组件内 @form/@load 至多一个」
/// V-F7 @load fn 必须存在、无参、返回 Array 类型 → error（含「@load 仅支持 Array 返回（v0）」）
/// V-F8 保留名冲突：@load 存在时用户信号 loading/error/data；@form 存在时 formErrors/formValid/formSubmitting → error「信号名 "@x" 与 @load 生成的保留名冲突」
#[test]
fn form_validate_rules_and_constraints() {
    let src = "component App { fn s(e) {} view { <form @form> <input bind:value={n} validate:{bogus: true} /> </form> } }";
    let diags = compile_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("必须绑定 @onSubmit")));
    assert!(diags.iter().any(|d| d.message.contains("未知校验规则")));
}

#[test]
fn load_fn_must_return_array() {
    let src = "component App { fn f() -> User {} view { @load={f} <div/> } }";
    let diags = compile_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("仅支持 Array 返回")));
}

#[test]
fn reserved_signal_conflict() {
    let src = "component App { signal loading: bool = false; fn f() {} view { @load={f} <div/> } }";
    let diags = compile_diags(src);
    assert!(diags.iter().any(|d| d.message.contains("保留名冲突")));
}
```

- [ ] **Step 2: 运行确认失败**
Run: `cargo test -p omnilang-core form_validate_rules_and_constraints`
Expected: FAIL（pass 未注册，无诊断）

- [ ] **Step 3: 最小实现**
`semantic/validate_form_load.rs`（镜像 validate_event_names.rs 结构：入口 fn + 组件循环 + walk_view_tree 递归）：
```rust
/// 校验规则白名单（spec 2026-08-08 §4.1 v0 封闭集；Task 7 登记册锚点，门禁双向断言）。
pub const VALIDATE_RULES: &[&str] = &["required", "format", "min", "max", "pattern", "custom"];
pub const FORMAT_VALUES: &[&str] = &["email", "url"];
```
判定要点：
- V-F1：form_intent 收集时记录元素 tag（lowering 时把 tag 一并带入 FormIntentIR？——不加字段，pass 直接从 ComponentIR 的 view 树重走：find `ElementProp::Form` 元素 → 检查 tag == "form"、同元素 Event "onSubmit"。**注意**：V-F1/F-F2 在 IR 上做需要元素定位——IR ElementIR 保留 tag/events，可行（element.rs IR 有 tag + events map）
- V-F3：validate: 所在元素须有 Bind 且 bind name ∈ {value, checked}——IR ElementIR 有 bindings
- V-F4/F-F5：规则名/值类型检查（ExprIR 类型信息：字面量类型判定——Bool/Number/String 字面量；custom → 查组件 fn 表（validate 既有 fn 解析模式——`fn_exists`/签名检查参考 validate_component.rs 的 fn 签名验证）
- V-F7：fn 返回类型检查——ComponentIR fns 的 return_type；Array 类型判定参考既有类型系统（Array<T> 的 TypeAnnotation 匹配）
- V-F8：组件信号名集合 vs 保留名（@form 存在：formErrors/formValid/formSubmitting；@load 存在：loading/error/data）
- pass 注册：semantic/mod.rs 既有 pass 列表追加 `validate_form_load`（run 时机与 validate_event_names 同批——post-IR-merge）

- [ ] **Step 4: 运行确认通过**
Run: `cargo test -p omnilang-core validate_form_load` 全绿 + `cargo test -p omnilang-core`（全量回归——注意既有 fixture 无 @form/@load，预期无破坏）
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add crates/omnilang-core/src/semantic
git commit -m "feat(semantic): 表单/数据三态验证 pass——规则白名单 + 约束 + 保留名冲突（Task 3）"
```

---

### Task 4: React codegen——受控表单 + 校验执行器 + @load 三态

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`（form 元素发射 + validate 接线）
- Modify: `omnilang/src/codegen/react/component.rs`（或既有组件脚手架文件——信号/useEffect 发射处）
- Test: `omnilang/src/codegen/react/tests.rs` 产物断言 + verify-react 门禁

**Interfaces:**
- Consumes: Task 2/3 的 FormIntentIR/LoadIntentIR + 校验后 IR
- Produces: React 产物——form 组件：受控输入（bind:value 既有机制）+ `validateField`/`submitForm` 逻辑 + `formErrors` state（`{ [k: string]: string }`）+ `formValid`/`formSubmitting` state + aria-invalid/aria-describedby + 提交拦截；@load 组件：`useEffect` 一次调用 + `loading/error/data` state + `retry` 回调

- [ ] **Step 1: 写失败测试**（react/tests.rs 产物断言）

```rust
#[test]
fn react_form_emits_validation_and_a11y() {
    let out = compile_react(FORM_SRC); // @form + 2 字段（required/email）
    assert!(out.contains("formErrors"));
    assert!(out.contains("aria-invalid"));
    assert!(out.contains("aria-describedby"));
    assert!(out.contains("e.preventDefault()"), "提交拦截");
    assert!(out.contains("required"), "required 规则落 DOM");
}

#[test]
fn react_load_emits_three_state_effect() {
    let out = compile_react(LOAD_SRC); // @load={fetchTodos} + @if 渲染三态
    assert!(out.contains("useEffect"));
    assert!(out.contains("loading"));
    assert!(out.contains("error"));
    assert!(out.contains("retry"));
}
```

- [ ] **Step 2: 运行确认失败**
Run: `cargo test -p omnilang --features codegen-react react_form_emits`
Expected: FAIL（无 form 相关发射）

- [ ] **Step 3: 最小实现**
- **@form**（react/view.rs form 元素分支 + 组件级 state 注入）：生成
  - `const [formErrors, setFormErrors] = useState<Record<string, string>>({});` + `const [formSubmitting, setFormSubmitting] = useState(false);`（formValid 由 `Object.keys(formErrors).length === 0` 推导——不生成冗余 state，产物 `const formValid = Object.keys(formErrors).length === 0;`）
  - 校验执行器：`function validateForm(values) { const errs = {}; ...规则映射...; return errs; }`——规则映射：required→`!v`；format:email→正则；min/max→`v.length < n`；pattern→`new RegExp(p).test(v)`；custom→调用组件 fn
  - submit handler 包装：原 @onSubmit handler 前插入 `const errs = validateForm(formValues); if (Object.keys(errs).length > 0) { setFormErrors(errs); return; }`——校验失败不发事件（「提交拦截」= handler 不调用，DOM submit 需 preventDefault——form 元素发射 `onSubmit={(e) => { e.preventDefault(); ... }}`）
  - 字段 aria：`aria-invalid={formErrors["name"] ? true : undefined}` + `aria-describedby={formErrors["name"] ? "err-name" : undefined}`；错误容器 id="err-{field}"（用户显式渲染 @if={form.errors.name}——FieldAccess 在 Map 上 → JS `formErrors["name"]`）
  - `form.errors` → `formErrors`；`form.valid` → `formValid`；`form.submitting` → `formSubmitting`（FieldAccess on 内建 form 对象——React 端 `formErrors`/`formValid`/`formSubmitting` 直接作为 JS 标识符可见性：信号名映射既有机制（信号 → state 名），form.* 访问映射为 state 标识符）
- **@load**（component.rs 组件脚手架）：生成
  - `const [loading, setLoading] = useState(true);` + `const [error, setError] = useState("");` + `const [data, setData] = useState<T>([]);`（T = fn 返回数组类型）
  - `const load = useCallback(() => { setLoading(true); setError(""); <fn>(<args>).then((r) => { setData(r); setLoading(false); }).catch((e) => { setError(String(e)); setLoading(false); }); }, []);`——fn 无参（V-F7 保证）；同步 fn 包装 `Promise.resolve().then(() => <fn>())`
  - `useEffect(() => { load(); }, [load]);` + `retry` = `load` 的别名导出
  - 信号名 → state 映射沿用既有信号发射机制（loading/error/data 是普通组件信号，发射逻辑既有；@load 只补「useEffect 触发 + 赋值」部分——data 初始值 `[]` 由信号声明默认值发射，@load 场景 data 信号默认 [] 已由语义保证）

- [ ] **Step 4: 验证**
Run: `cargo test -p omnilang --features codegen-react`（产物断言全绿）+ verify-react 门禁（`bash scripts/verify-react.sh`——生成产物过 tsc + eslint，GATE PASSED 豁免不新增）
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add omnilang/src/codegen/react
git commit -m "feat(codegen-react): @form 受控校验 + a11y + @load 三态 useEffect（Task 4）"
```

---

### Task 5: Flutter codegen——Form + TextFormField.validator + FutureProvider

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（form 元素发射）
- Modify: `omnilang/src/codegen/flutter/component.rs`（或既有脚手架文件）
- Test: `omnilang/src/codegen/flutter/tests.rs` + dart analyze 门禁

**Interfaces:**
- Consumes: Task 2/3 的 intent IR
- Produces: Flutter 产物——`Form` + `TextFormField(validator: ...)`（错误走 `errorText`——Flutter 端**自动**错误展示，与 React/OmniView 显式渲染不同：Flutter TextFormField 原生形态，注记于 docs）+ `AutovalidateMode`；@load → Riverpod 3.0 `FutureProvider` + `AsyncValue` 三态（loading/error/data 信号 → `AsyncValue.when` 接线）+ retry（`ref.invalidate`）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn flutter_form_emits_form_and_validators() {
    let out = compile_flutter(FORM_SRC);
    assert!(out.contains("Form("));
    assert!(out.contains("TextFormField"));
    assert!(out.contains("validator:"));
    assert!(out.contains("errorText"));
}

#[test]
fn flutter_load_emits_future_provider() {
    let out = compile_flutter(LOAD_SRC);
    assert!(out.contains("FutureProvider"));
    assert!(out.contains("AsyncValue"));
}
```

- [ ] **Step 2: 运行确认失败 → Step 3: 最小实现**
- **@form**：form 元素发射 `Form(child: ...)` + `GlobalKey<FormState>`；绑定字段发射 `TextFormField(validator: (v) => <规则链>)`——validator 返回错误消息字符串或 null（Flutter 原生形态）；校验规则映射：required→`v.isEmpty ? '此项必填' : null`、min/max→长度、pattern→RegExp、custom→组件 fn 调用；`form.errors` 信号在 Flutter 端**不生成**（errorText 替代——诚实降级注记 + 语言面信号仍可读？v0：Flutter 端 form.errors 信号发射为常量空 map + 注记「Flutter 错误经 errorText 展示」——或直接诊断？**决策：语言面保留（跨目标一致），Flutter 端发射空 map + 注释**；form.valid/form.submitting 同理——submitting 同步行为注记）。submit 拦截：`onSubmitted`/按钮 `onPressed` 前 `if (!_formKey.currentState!.validate()) return;`——handler 包装
- **@load**：组件脚手架生成 `final todosProvider = FutureProvider<List<Todo>>((ref) async => <fn>());`（fn 名映射）；三态信号 loading/error/data → `ref.watch(todosProvider).when(loading: ..., error: ..., data: ...)`——@if/@for 在数据信号上的渲染走 AsyncValue 分支；retry → `ref.invalidate(todosProvider)`

- [ ] **Step 4: 验证**
Run: `cargo test -p omnilang --features codegen-flutter` + dart analyze 门禁（`scripts/verify-flutter.sh` 等价入口；warnings ≤ 基线）
Expected: PASS + 0 error

- [ ] **Step 5: Commit**
```bash
git add omnilang/src/codegen/flutter
git commit -m "feat(codegen-flutter): @form Form+validator + @load FutureProvider/AsyncValue（Task 5）"
```

---

### Task 6: OmniView codegen——HTML5 校验 + 自绘三态

**Files:**
- Modify: `omnilang/src/codegen/omniview/view.rs`（form 元素发射）
- Test: 既有 omniview codegen tests + 浏览器断言（Task 8 并入）

**Interfaces:**
- Consumes: Task 2/3 的 intent IR
- Produces: OmniView 产物——`<form onsubmit>` 拦截 + HTML5 `required`/`pattern` 属性落 DOM + 自绘错误（`formErrors` 信号 + 用户显式渲染）+ aria 接线；@load → JS 手写三态（loading/error/data 信号 + 挂载调用 + retry fn）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn omniview_form_emits_html5_and_errors() {
    let out = compile_omniview(FORM_SRC);
    assert!(out.contains("onsubmit"));
    assert!(out.contains("required"), "required 规则落 HTML5 属性");
    assert!(out.contains("aria-invalid"));
}

#[test]
fn omniview_load_emits_three_state_signals() {
    let out = compile_omniview(LOAD_SRC);
    assert!(out.contains("loading"));
    assert!(out.contains("retry"));
}
```

- [ ] **Step 2: 运行确认失败 → Step 3: 最小实现**
- **@form**：`<form onsubmit="...">` 拦截（preventDefault + 校验）；required/pattern 规则 → HTML5 属性（pattern 规则映射 `pattern` 属性）；自绘错误：formErrors 信号（组件信号机制既有）+ 字段 `aria-invalid`/`aria-describedby` 动态绑定；提交拦截：校验失败不发事件
- **@load**：组件脚本生成三态信号（loading=true 初始）+ 挂载调用（`__omni_el` 挂载后调用或既有生命周期机制——复用 React 端 useCallback 同构逻辑的 JS 形态）+ retry 函数；data 默认 []

- [ ] **Step 4: 验证**
Run: `cargo test -p omnilang --features codegen-omniview`（产物断言全绿）+ `cargo test --workspace` 全量回归
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add omnilang/src/codegen/omniview
git commit -m "feat(codegen-omniview): @form HTML5 校验自绘错误 + @load 三态信号（Task 6）"
```

---

### Task 7: 登记册 + universe 域 8/9 + spec 收口 + docs/omniview + 注入语料

**Files:**
- Modify: `docs/superpowers/specs/semantic-universe.md`（新增域 8 表单 / 域 9 数据三态）
- Modify: `docs/superpowers/specs/vocabulary-registry.md`（如涉及新语义词汇——validate 规则名/内建信号名判定）
- Modify: `docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md`（§4.1 形态收口：字段级 validate/错误信号显式渲染/@load 视图级——记录本计划设计决策）
- Modify: `docs/omniview/`（新节：表单 + 数据三态——跟随既有文档结构，参考 07-events-and-bindings.md 的表格风格）
- Modify: `scripts/verify-semantic-universe.py`（域 8/9 断言：语义集条目 ↔ 实现状态 ↔ 诊断行为——锚点 = `validate_form_load.rs` 的 `VALIDATE_RULES` 常量，rust_str_array 提取）
- Modify: 注入语料（855 元组矩阵——新语法插入点：validate 规则名/保留名冲突消息）

**Interfaces:**
- Consumes: Task 3 的 VALIDATE_RULES 常量
- Produces: 登记册零失实 + docs-gate 0 failed + verify 双脚本 0 problems

- [ ] **Step 1: universe 域 8/9 登记**
域 8（表单）：语义集条目 = @form / validate 规则（required/format/min/max/pattern/custom 各条目——语义列注明规则意图与内建错误消息）；实现状态列（支持/部分——Flutter errorText 形态、OmniView HTML5 形态差异注记于诊断行为列）；域 9（数据三态）：@load/loading/error/data/retry 条目 + 状态 + v0 边界（仅 Array 返回）
- [ ] **Step 2: verify-semantic-universe.py 断言接线**
域 8 锚点 `VALIDATE_RULES`（rust_str_array 提取 validate_form_load.rs）+ 域 9 锚点（@load 相关常量或存在性断言——按 ADR-074 先例）
- [ ] **Step 3: spec §4.1 收口**（形态决策记录：字段级 validate 属性/错误信号显式渲染/@load 视图级指令/v0 范围边界）
- [ ] **Step 4: docs/omniview 新节**（表单三件套 + 数据三态 + 迁移/用例；Flutter errorText 差异注记）
- [ ] **Step 5: 注入语料扩张**（新点：未知校验规则消息/保留名冲突/仅 Array 返回）
- [ ] **Step 6: 验证**
Run: `python scripts/verify-semantic-universe.py --check`（0 problems）+ `python scripts/verify-vocabulary.py --check`（0）+ `python scripts/verify-docs.py --check`（0 failed，新鲜 binary——先 `cargo build -p omnic` 若源码有变）+ 注入语料全绿
- [ ] **Step 7: Commit**
```bash
git commit -m "docs(universe): 域 8 表单/域 9 数据三态登记 + spec 收口 + docs 新节 + 注入语料（Task 7）"
```

---

### Task 8: learn-omniview 表单 demo 迁移 + 三目标编译 + Playwright 断言

**Files:**
- Modify: `test/projects/learn-omniview/src/`（表单相关 .omni——grep 定位：`<form`/`bind:value`/`@onSubmit`/`@onInput` 用点 + ValidationInput 组件）
- Test: Playwright 全量（新增表单行为断言：必填拦截/错误消息展示/校验通过提交）

**Interfaces:**
- Consumes: Task 1-6 全部（真机验证载体）
- Produces: 零手写表单样板语料 + 缺漏清单（重构中写不出来的界面 = 缺漏 → 追加设计或登记语言边界）

- [ ] **Step 1: 定位用点**
Run: `grep -rn "<form\|bind:value\|@onSubmit\|@onInput" test/projects/learn-omniview/src --include="*.omni"`
- [ ] **Step 2: 迁移**
- 手写校验组件（ValidationInput 等）→ `@form` + `validate:`（逐步——校验逻辑替换为规则声明；若某校验写不出来（如自定义跨字段校验）→ 登记语言边界缺漏）
- 文本输入 `@onInput` 手动三态样板 → 判别的 `@load`（异步获取场景）
- **注意**：@form 是新增语义——demo 迁移遵循「语义化即切换」纪律：迁移后旧手写形态不留并存
- [ ] **Step 3: 三目标编译 + 浏览器全量**
Run: `omnic build --target react|flutter|omniview`（既有 omnic 用法）+ Playwright 全量套件
Expected: 0 failures + 新断言（表单校验行为：空提交拦截 + 错误消息 + 合法提交通过）
- [ ] **Step 4: Commit**
```bash
git commit -m "refactor(demo): learn-omniview 表单用点迁移 @form/@load + Playwright 校验行为断言（Task 8）"
```

---

## 收口验证（全部任务完成后）

Run: `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test export_bindings --lib -p omnilang`（ts-rs freshness）+ verify-react GATE + Flutter dart analyze + `python scripts/verify-docs.py --check`（新鲜 binary）+ verify 双脚本 + 注入语料 + Playwright 全量 + `git diff --exit-code poiesis/src/commands/__generated__/` 干净
Expected: 全绿；spec §4.1 验收第 2 条达成（表单 + 数据三态意图契约实现，各带 Validation 清单 + 三目标门禁）

**明确 defer（登记）**：V6 字段类型检查警告（spec §3.5）；@load 非 Array 返回（struct 等）；多表单/跨组件表单共享；FormData 预填；form.submitting Flutter 异步语义（同步注记）；阶段 2 其余三域（反馈+模态 / 导航 / 响应式——各自独立计划）。
