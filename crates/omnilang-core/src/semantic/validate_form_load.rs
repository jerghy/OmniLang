//! 表单/数据三态语义验证 pass（spec 2026-08-08 §4.1/§4.2、2026-08-09 §2.1-§2.4）
//! ——消费 Task 2 的 FormIntentIR/LoadIntentIR，校验约束与保留名冲突
//! （V-F2/V-F3/V-F4/V-F5/V-F7/V-F8；批次 4 V6 语义：匿名 @load 至多一个、
//! 命名 @load 数量不限）。
//!
//! V-F1（@form 仅限 form 元素）与 V-F6（@form 至多一个）/V6（匿名 @load
//! 至多一个）在 lowering 扫描处报错（ir/lower/component.rs）——因为
//! @form/@load 标记经 lowering 后从 IR 视图树消失（view.rs 将其从 props
//! 丢弃），本 pass 无法定位具体元素，只能消费已收集的意图 IR（诊断 span
//! 回退组件级）。

/// 校验规则白名单（spec 2026-08-08 §4.1 v0 封闭集；Task 7 登记册锚点，门禁双向断言）。
pub const VALIDATE_RULES: &[&str] = &["required", "format", "min", "max", "pattern", "custom"];
/// `format` 规则的合法取值（v0 封闭集）。
pub const FORMAT_VALUES: &[&str] = &["email", "url"];

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::{walk_expr, walk_stmt, walk_view};
use crate::ir::{
    BlockIR, ComponentIR, ElementIR, EventHandlerIR, ExprIR, FormIntentIR, IRProgram, LoadIntentIR,
    LoaderIR, TriggerKind, TypeKind, ViewTree,
};
use crate::semantic::load_contract::{
    LOAD_PAGINATED_SUFFIXES, LOAD_RESERVED_NAMES, PAGE_TYPE_NAME, TIMING_FUNCS,
};

/// 命名 @load 实例生成的派生信号基础 5 后缀（spec 2026-08-09 §2.2/§3 V4）——
/// 分页 4 后缀在 load_contract 的 LOAD_PAGINATED_SUFFIXES。
pub const LOAD_BASE_SUFFIXES: &[&str] = &["Loading", "Error", "Data", "Retry", "Reset"];

/// 校验 @form/@load 语义约束（spec 2026-08-08 §4.1/§4.2、2026-08-09 §2.1-§2.4）。
///
/// V-F1（@form 仅限 form 元素）与 V-F6（@form 至多一个）/V6（匿名 @load
/// 至多一个）在 lowering 扫描处报错（ir/lower/component.rs）——本 pass
/// 消费已收集的 FormIntentIR/LoadIntentIR，诊断 span 回退组件级（意图 IR
/// 无元素 span）。
pub fn validate_form_load(ir: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    for module in ir.modules.values() {
        for component in module.components.values() {
            if let Some(form) = &component.form_intent {
                validate_form(component, form, diagnostics);
            }
            // V9（spec §3）：std/timing debounce/throttle 参数形态——与
            // @load 实例是否在场无关（搜索防抖等可在无 @load 组件中独立
            // 使用），故置于 load_intent 门之外无条件执行。
            validate_timing_call_args(component, diagnostics);
            // 组件级 @load 网（spec §3 V2-V11）——每组件恰跑一次：
            // R5 retry 实参 / V-F8 保留名（匿名实例在场才有生成面）/
            // V3 命名重名 / V4 派生信号冲突。
            if !component.load_intent.is_empty() {
                validate_load_component(component, diagnostics);
            }
            // 多例化（Task 3）：命名 @load 数量不限——逐例校验
            // （V2 实例名冲突 / V7 分页须命名 / V8 scroll 组合 /
            //  V11 分页签名 / V-F7 装载函数形态）。
            for load in &component.load_intent {
                validate_load(component, load, diagnostics);
            }
        }
    }
}

/// V-F2/V-F3/V-F4/V-F5/V-F8（@form 侧）。
fn validate_form(component: &ComponentIR, form: &FormIntentIR, diagnostics: &mut Vec<Diagnostic>) {
    // V-F2 @form 必须配 @onSubmit（lowering 缺省为空串）
    if form.submit_handler.is_empty() {
        error_diag(
            component,
            "@form 元素必须绑定 @onSubmit".to_string(),
            diagnostics,
        );
    }
    for field in &form.fields {
        // V-F3 validate: 必须配 bind:value/bind:checked（孤儿规则，缺省为空串）
        if field.name.is_empty() {
            error_diag(
                component,
                "validate: 必须与 bind: 配对使用".to_string(),
                diagnostics,
            );
            continue; // 无绑定目标，后续规则检查无意义
        }
        for rule in &field.rules {
            // V-F4 规则名 ∈ VALIDATE_RULES
            if !VALIDATE_RULES.contains(&rule.name.as_str()) {
                error_diag(
                    component,
                    format!(
                        "未知校验规则 \"@{}\"——合法规则: {}",
                        rule.name,
                        VALIDATE_RULES.join("/")
                    ),
                    diagnostics,
                );
                continue; // 未知规则不再做值类型检查
            }
            // V-F5 规则值类型（仅检查可静态判定的形态，表达式计算值跳过）
            // R3：custom 三态拆分——fn 缺失/签名不符给出针对性消息，
            // 其余（含非标识符表达式）保持值类型消息。
            let custom_status = if rule.name.as_str() == "custom" {
                Some(rule_is_custom_fn(component, &rule.value))
            } else {
                None
            };
            let ok = match rule.name.as_str() {
                "required" => {
                    matches!(&rule.value, ExprIR::LiteralExpr { value } if value.is_boolean())
                }
                "min" | "max" => {
                    matches!(&rule.value, ExprIR::LiteralExpr { value } if value.is_number())
                }
                "format" => matches!(
                    &rule.value,
                    ExprIR::LiteralExpr { value } if value.as_str().is_some_and(|s| FORMAT_VALUES.contains(&s))
                ),
                "pattern" => {
                    matches!(&rule.value, ExprIR::LiteralExpr { value } if value.as_str().is_some())
                }
                "custom" => matches!(custom_status, Some(CustomRuleStatus::Valid)),
                _ => true, // 白名单已过滤，不可达
            };
            if !ok {
                let message = match custom_status {
                    Some(CustomRuleStatus::Missing) => {
                        format!("自定义校验函数 \"@{}\" 不存在", rule.name)
                    }
                    Some(CustomRuleStatus::BadSignature) => {
                        format!("自定义校验函数 \"@{}\" 签名应为单参返回 String", rule.name)
                    }
                    _ => format!("校验规则 \"@{}\" 的值类型不合法（v0）", rule.name),
                };
                error_diag(component, message, diagnostics);
            }
        }
    }
    // V-F8（@form 侧）保留名冲突：formErrors/formValid/formSubmitting
    // （R4：信号与计算属性同查——二者都会遮蔽 @form 生成表面）
    for signal in &component.signals {
        if matches!(
            signal.name.as_str(),
            "formErrors" | "formValid" | "formSubmitting"
        ) {
            error_diag(
                component,
                format!("信号名 \"@{}\" 与 @form 生成的保留名冲突", signal.name),
                diagnostics,
            );
        }
    }
    for computed in &component.computeds {
        if matches!(
            computed.name.as_str(),
            "formErrors" | "formValid" | "formSubmitting"
        ) {
            error_diag(
                component,
                format!(
                    "计算属性名 \"@{}\" 与 @form 生成的保留名冲突",
                    computed.name
                ),
                diagnostics,
            );
        }
    }
}

/// 组件级 @load 验证网（spec 2026-08-09 §3 V2-V11 组件维度）——每组件
/// 恰跑一次，与实例数无关：R5 retry 实参 / V-F8 保留名（仅匿名实例在场
/// 才有生成面）/ V3 命名重名 / V4 派生信号冲突 / V9 debounce-throttle
/// 参数形态。
fn validate_load_component(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    // R5：@load 内建别名 retry 仅支持无参调用（v0）——Flutter codegen 对
    // retry(42) 静默丢弃实参，React 端靠 TS2554 兜底报错，此处补跨目标
    // 一致的语义层硬诊断。仅 @load 存在时触发；R4 已保证此时用户不得
    // 声明同名 fn，任何 retry(...) 调用必指内建别名（无误报面）。
    validate_retry_call_arity(component, diagnostics);
    // V-F8（@load 侧）保留名冲突：loading/error/data/retry/reset 专供匿名
    // 实例（命名实例生成 `{name}Loading` 等前缀名，不与裸名相撞），故仅当
    // 组件存在匿名 @load 时查信号/计算属性/组件 fn。R4：信号与计算属性
    // 同查；组件 fn 名不得为 retry/load（硬诊断）——retry 冲突遮蔽内建
    // 别名，load 冲突与 codegen 生成的 load 标识符（React useCallback 名）
    // 相撞。独立于 V-F7 先行，@load fn 缺失时不吞掉这些冲突。
    if component.load_intent.iter().any(|l| l.name.is_none()) {
        for signal in &component.signals {
            if LOAD_RESERVED_NAMES.contains(&signal.name.as_str()) {
                error_diag(
                    component,
                    format!("信号名 \"@{}\" 与 @load 生成的保留名冲突", signal.name),
                    diagnostics,
                );
            }
        }
        for computed in &component.computeds {
            if LOAD_RESERVED_NAMES.contains(&computed.name.as_str()) {
                error_diag(
                    component,
                    format!(
                        "计算属性名 \"@{}\" 与 @load 生成的保留名冲突",
                        computed.name
                    ),
                    diagnostics,
                );
            }
        }
        for f in &component.fns {
            if matches!(f.name.as_str(), "retry" | "load") {
                error_diag(
                    component,
                    format!("函数名 \"@{}\" 与 @load 内建名冲突", f.name),
                    diagnostics,
                );
            }
        }
    }
    // V3（spec §3）：命名实例重名——每个重复名恰好 1 条（先到先记，
    // 后续同名只报一次）。
    let mut seen: Vec<&str> = Vec::new();
    let mut reported: Vec<&str> = Vec::new();
    for load in &component.load_intent {
        let Some(name) = load.name.as_deref() else {
            continue;
        };
        if seen.contains(&name) && !reported.contains(&name) {
            error_diag(
                component,
                format!("组件内 @load 实例名称重复：`{name}`——每个实例须唯一命名"),
                diagnostics,
            );
            reported.push(name);
        }
        seen.push(name);
    }
    // V4（spec §3）：命名实例派生信号与既有信号/函数冲突——基础 5 后缀
    // （Loading/Error/Data/Retry/Reset）恒查，分页 4 后缀（Page/HasMore/
    // LoadingMore/LoadMore）仅 paginated 实例查。派生名与信号/计算属性/
    // 函数任一相撞即报（消息含派生名，逐冲突报更有信息量）。
    for load in &component.load_intent {
        let Some(name) = load.name.as_deref() else {
            continue;
        };
        let suffixes = if load.mode.paginated {
            LOAD_BASE_SUFFIXES
                .iter()
                .chain(LOAD_PAGINATED_SUFFIXES.iter())
                .collect::<Vec<_>>()
        } else {
            LOAD_BASE_SUFFIXES.iter().collect::<Vec<_>>()
        };
        for suffix in suffixes {
            let derived = format!("{name}{suffix}");
            let conflict = component.signals.iter().any(|s| s.name == derived)
                || component.computeds.iter().any(|c| c.name == derived)
                || component.fns.iter().any(|f| f.name == derived);
            if conflict {
                error_diag(
                    component,
                    format!(
                        "@load 实例 `{name}` 生成的信号 `{derived}` 与组件信号/函数冲突，请更换实例名"
                    ),
                    diagnostics,
                );
            }
        }
    }
}

/// V2/V7/V8/V11/V-F7（@load 实例侧）——逐例校验，多例化下每实例独立。
fn validate_load(component: &ComponentIR, load: &LoadIntentIR, diagnostics: &mut Vec<Diagnostic>) {
    // V2（spec §3）：实例名与保留名/既有信号/函数冲突。实例-实例同名属
    // V3（组件级网已报），此处只查 保留名 + 组件信号/计算属性/函数 相撞。
    if let Some(name) = &load.name {
        let existing = component
            .signals
            .iter()
            .map(|s| s.name.as_str())
            .chain(component.computeds.iter().map(|c| c.name.as_str()))
            .chain(component.fns.iter().map(|f| f.name.as_str()))
            .find(|n| *n == name.as_str());
        if LOAD_RESERVED_NAMES.contains(&name.as_str()) || existing.is_some() {
            let conflict = existing.unwrap_or(name.as_str());
            error_diag(
                component,
                format!(
                    "@load 实例名称 `{name}` 与保留名/信号 `{conflict}` 冲突——保留名 {} 专供匿名实例，请更换实例名",
                    LOAD_RESERVED_NAMES.join("/")
                ),
                diagnostics,
            );
        }
    }
    // V7（spec §3）：分页实例必须命名——分页信号 page/hasMore/loadingMore
    // 需要实例归属（匿名实例的派生面是 loading/error/data 裸名，与分页
    // 信号面不兼容）。
    if load.mode.paginated && load.name.is_none() {
        error_diag(
            component,
            "分页 @load 必须命名（@load name={fn} paginated）——分页信号 page/hasMore/loadingMore 需要实例归属".to_string(),
            diagnostics,
        );
    }
    // V8（spec §3）：scroll 须与 paginated 组合——无限滚动是分页加载的
    // 滚动触发形态，脱离 paginated 的 scroll 语义悬空。
    if load.mode.scroll && !load.mode.paginated {
        error_diag(
            component,
            "scroll 触发须与 paginated 组合使用（@load name={fn} paginated scroll）".to_string(),
            diagnostics,
        );
    }
    // V-F7 装载函数存在性 + V11 分页签名（spec §3）：函数缺失时 V-F7 报错
    // 即返回，不叠加后续检查。
    let Some(f) = component
        .fns
        .iter()
        .find(|f| f.name == load.loader.fn_name())
    else {
        error_diag(
            component,
            format!("@load 引用的函数 \"@{}\" 不存在", load.loader.fn_name()),
            diagnostics,
        );
        return; // 函数不存在，其余检查无意义
    };
    if load.mode.paginated {
        // V11（spec §3，warning）：分页签名静态确认——仅 Fn 裸名形态可
        // 静态确认（fn {name}(page: i32) -> Page<T> 恰匹配时静默）；调用
        // 形态（Call 表达式的实参绑定占用 page 槽位，无法静态确认）一律
        // 警告，运行时以约定信封兜底（诚实降级）。
        let signature_ok = matches!(load.loader, LoaderIR::Fn(_))
            && f.params.len() == 1
            && f.params[0].name == "page"
            && f.params[0]
                .r#type
                .as_ref()
                .is_some_and(|t| matches!(t.kind, TypeKind::Int))
            && f.return_type.as_ref().is_some_and(|t| {
                matches!(&t.kind, TypeKind::Named { name } if name == PAGE_TYPE_NAME)
                    && t.type_args.len() == 1
            });
        if !signature_ok {
            warning_diag(
                component,
                format!(
                    "分页装载函数签名建议为 `fn {}(page: i32) -> Page<T>`——当前无法静态确认，运行时以约定信封兜底（诚实降级）",
                    f.name
                ),
                diagnostics,
            );
        }
    } else {
        // V-F7 @load fn 必须无参（仅 Fn 裸名形态检查——Call 形态的实参即
        // 绑定参数，属合法形态；Task 3 修复：不再对 Call 装载器误报）、
        // 返回 Array 类型（两种形态同查）。
        if matches!(load.loader, LoaderIR::Fn(_)) && !f.params.is_empty() {
            error_diag(
                component,
                format!("@load 引用的函数 \"@{}\" 必须无参数", f.name),
                diagnostics,
            );
        }
        let is_array = f.return_type.as_ref().is_some_and(|t| {
            matches!(&t.kind, TypeKind::Named { name } if name == "Array") && t.type_args.len() == 1
        });
        if !is_array {
            error_diag(
                component,
                "@load 仅支持 Array 返回（v0）".to_string(),
                diagnostics,
            );
        }
    }
}

/// R5：扫描组件全部表达式可达点，检出 `retry(实参)` 调用并报 E0006
/// （扫描骨架见 scan_component_exprs——R5/V9 共用）。
fn validate_retry_call_arity(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let mut check = |e: &ExprIR| {
        if let ExprIR::CallExpr { name, args } = e {
            if name == "retry" && !args.is_empty() {
                error_diag(
                    component,
                    "@load 内建别名 \"retry\" 仅支持无参调用（v0）".to_string(),
                    diagnostics,
                );
            }
        }
    };
    scan_component_exprs(component, &mut check);
}

/// V9（spec §3）：std/timing debounce/throttle 调用参数形态——`{fn}(callable,
/// ms)` 且 ms 须为正整数字面量（毫秒）、首参须为可调用表达式（标识符或
/// 调用形态；字面量/方法链/复杂表达式无法静态确认即报）。纯名字匹配
/// （validate_array_methods 同款先例）：未知符号的 E0001 由符号解析报，
/// 本检查只管参数形态。
fn validate_timing_call_args(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let mut check = |e: &ExprIR| {
        if let ExprIR::CallExpr { name, args } = e {
            if TIMING_FUNCS.contains(&name.as_str()) {
                let invalid = args.len() != 2
                    || !matches!(
                        &args[0],
                        ExprIR::IdentifierExpr { .. } | ExprIR::CallExpr { .. }
                    )
                    || !matches!(&args[1], ExprIR::LiteralExpr { value }
                        if value.as_u64().is_some_and(|v| v > 0));
                if invalid {
                    error_diag(
                        component,
                        format!("std/timing `{name}` 的 ms 参数须为正整数毫秒；fn 须为可调用函数"),
                        diagnostics,
                    );
                }
            }
        }
    };
    scan_component_exprs(component, &mut check);
}

/// 组件全部表达式可达点统一扫描（R5/V9 共用骨架）：fn 体/effect 体/
/// lifecycle 体（BlockIR）、computed 体、信号初值、prop 缺省值（ExprIR）、
/// 视图树（元素槽位 + 非元素表达式槽位）。
fn scan_component_exprs(component: &ComponentIR, check: &mut impl FnMut(&ExprIR)) {
    for f in &component.fns {
        walk_block_exprs(&f.body, check);
    }
    for effect in &component.effects {
        walk_block_exprs(&effect.body, check);
    }
    for lc in &component.lifecycles {
        walk_block_exprs(&lc.body, check);
    }
    for c in &component.computeds {
        walk_expr(&c.body, check);
    }
    for s in &component.signals {
        walk_expr(&s.initial_value, check);
    }
    for v in component.prop_defaults.values() {
        walk_expr(v, check);
    }
    if let Some(ref view) = component.view {
        walk_view_exprs(view, check);
    }
}

/// 本地块遍历——walk::walk_block 为私有，语义 pass 不可见（R5 自建等价物，
/// 与 walk.rs 的 walk_block 逐语句一致）。泛型 F 保持与 walk_expr/walk_stmt
/// 的 `impl FnMut` 签名（Sized）兼容。
fn walk_block_exprs<F: FnMut(&ExprIR)>(block: &BlockIR, f: &mut F) {
    for stmt in &block.statements {
        walk_stmt(stmt, f);
    }
}

/// 遍历元素上全部携带 ExprIR 的槽位。inline_style/style_classes 是
/// serde_json::Value，不在表达式遍历范围内（R7 边界：JSON 仅含原始值
/// 与 $标识符，无调用表达式，无需扫描）。
fn walk_element_exprs<F: FnMut(&ExprIR)>(el: &ElementIR, f: &mut F) {
    for prop in el.props.values() {
        walk_expr(prop, f);
    }
    for expr in el.class_map_bindings.values() {
        walk_expr(expr, f);
    }
    for expr in el.semantic_bindings.values() {
        walk_expr(expr, f);
    }
    for handler in el.events.values() {
        if let EventHandlerIR::InlineFn { body, .. } = handler {
            walk_block_exprs(body, f);
        }
    }
    if let Some(ref a11y) = el.a11y {
        if let Some(ref expr) = a11y.hidden {
            walk_expr(expr, f);
        }
        if let Some(ref expr) = a11y.alt {
            walk_expr(expr, f);
        }
    }
    if let Some(ref motion) = el.motion {
        for expr in [
            motion.in_view_once.as_ref(),
            motion.in_view_repeat.as_ref(),
            motion.in_view_threshold.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            walk_expr(expr, f);
        }
        for trigger in &motion.triggers {
            if let Some(ref body) = trigger.inline_motion {
                walk_expr(&body.over, f);
                for stop in &body.stops {
                    for expr in stop.properties.values() {
                        walk_expr(expr, f);
                    }
                }
                if let Some(ref delay) = body.playback.delay {
                    walk_expr(delay, f);
                }
            }
            match &trigger.kind {
                TriggerKind::Scroll {
                    start, end, scrub, ..
                } => {
                    for expr in [start.as_ref(), end.as_ref(), scrub.as_ref()]
                        .into_iter()
                        .flatten()
                    {
                        walk_expr(expr, f);
                    }
                }
                TriggerKind::DragHorizontal {
                    constraints,
                    elastic,
                    threshold,
                    ..
                }
                | TriggerKind::DragVertical {
                    constraints,
                    elastic,
                    threshold,
                    ..
                } => {
                    if let Some((ref min, ref max)) = constraints {
                        walk_expr(min, f);
                        walk_expr(max, f);
                    }
                    for expr in [elastic.as_ref(), threshold.as_ref()].into_iter().flatten() {
                        walk_expr(expr, f);
                    }
                }
                _ => {}
            }
        }
    }
}

/// 遍历整个视图树。walk_view 只访问 ElementIR 节点（含 Conditional
/// 分支体/ListRendering 体/Slot 兜底），非元素表达式槽位——Interpolation、
/// ListRendering iterable+key_expr、Conditional 条件——由本递归补齐
/// （覆盖模型同 optimize.rs 的 collect_idents_from_view_exprs）。元素
/// 子节点经 walk_view 回调访问其槽位、经本递归访问其子树的非元素节点，
/// 任一表达式槽位恰好被访问一次。
pub(crate) fn walk_view_exprs<F: FnMut(&ExprIR)>(tree: &ViewTree, f: &mut F) {
    walk_view(tree, &mut |el| walk_element_exprs(el, f));
    match tree {
        ViewTree::Interpolation(expr) => walk_expr(expr, f),
        ViewTree::ListRendering(lr) => {
            walk_expr(&lr.iterable, f);
            if let Some(ref key) = lr.key_expr {
                walk_expr(key, f);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                walk_expr(&branch.condition, f);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_exprs(child, f);
            }
        }
        ViewTree::Element(el) => {
            for child in &el.children {
                walk_view_exprs(child, f);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            for child in fallback {
                walk_view_exprs(child, f);
            }
        }
        ViewTree::Text(_) => {}
    }
}

/// V-F5 custom 规则的静态判定结果。
enum CustomRuleStatus {
    /// 组件中存在匹配 fn（单参、返回 String）
    Valid,
    /// 非 IdentifierExpr 形态（表达式计算值）无法静态判定，跳过放行
    Unverifiable,
    /// 标识符，但组件中没有该名字的 fn
    Missing,
    /// fn 存在但签名不匹配（必须单参返回 String）
    BadSignature,
}

/// V-F5 custom 规则判定：组件 fn 名（存在 + 单参 + 返回 String）。
fn rule_is_custom_fn(component: &ComponentIR, value: &ExprIR) -> CustomRuleStatus {
    let ExprIR::IdentifierExpr { name, .. } = value else {
        return CustomRuleStatus::Unverifiable;
    };
    match component.fns.iter().find(|f| f.name == *name) {
        None => CustomRuleStatus::Missing,
        Some(f)
            if f.params.len() == 1
                && f.return_type
                    .as_ref()
                    .is_some_and(|t| matches!(t.kind, TypeKind::String)) =>
        {
            CustomRuleStatus::Valid
        }
        Some(_) => CustomRuleStatus::BadSignature,
    }
}

/// 构建错误诊断（E0006，span 回退组件级——与 validate_event_names 的
/// `error_diag` 同构，仅省去元素级 span 参数）。
fn error_diag(component: &ComponentIR, message: String, diagnostics: &mut Vec<Diagnostic>) {
    let (line, col) = component
        .source_span
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        kind: DiagnosticKind::E0006,
        message,
        file: component.source_file.clone(),
        line,
        col,
        end_line: None,
        end_col: None,
        suggestion: None,
    });
}

/// 构建警告诊断（E0006 + Severity::Warning，span 回退组件级——与
/// `error_diag` 同构，severity 不同；V11 分页签名走此通道）。
fn warning_diag(component: &ComponentIR, message: String, diagnostics: &mut Vec<Diagnostic>) {
    let (line, col) = component
        .source_span
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    diagnostics.push(Diagnostic {
        severity: Severity::Warning,
        kind: DiagnosticKind::E0006,
        message,
        file: component.source_file.clone(),
        line,
        col,
        end_line: None,
        end_col: None,
        suggestion: None,
    });
}

#[cfg(test)]
mod tests {
    use crate::error::Diagnostic;

    /// 编译源码并收集全部 Severity::Error 诊断（pass 断言统一入口）。
    fn compile_diags(src: &str) -> Vec<Diagnostic> {
        let result = crate::compile(src);
        result
            .errors
            .into_iter()
            .filter(|d| matches!(d.severity, crate::error::Severity::Error))
            .collect()
    }

    /// V 清单（spec §4.1/§4.2 落地）：
    /// V-F2 @form 必须配 @onSubmit → error「@form 元素必须绑定 @onSubmit」
    /// V-F3 validate: 必须配 bind:value/bind:checked（表单意图外使用 = 孤儿规则）
    ///     → error「validate: 必须与 bind: 配对使用」
    /// V-F4 validate 规则名 ∈ VALIDATE_RULES
    ///     → error「未知校验规则 "@x"——合法规则: required/format/min/max/pattern/custom」
    #[test]
    fn form_validate_rules_and_constraints() {
        let src = "component App { fn s(e) {} view { <form @form> <input bind:value={n} validate:{bogus: true} /> </form> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("必须绑定 @onSubmit")),
            "V-F2 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            diags.iter().any(|d| d.message.contains("未知校验规则")),
            "V-F4 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// R3：V-F5 custom 三态——fn 不存在 → 「自定义校验函数 "@x" 不存在」
    ///     （区别于通用值类型消息；checkName 未声明仍会另有 E0001，断言只看本消息）
    #[test]
    fn custom_rule_fn_missing_message() {
        let src = "component App { fn s(e) {} view { <form @form @onSubmit={s}> <input bind:value={n} validate:{custom: checkName} /> </form> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("自定义校验函数") && d.message.contains("不存在")),
            "V-F5 missing-fn message missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// R3：V-F5 custom 三态——fn 存在但签名不符 → 「签名应为单参返回 String」
    #[test]
    fn custom_rule_fn_bad_signature_message() {
        let src = "component App { fn checkName() {} fn s(e) {} view { <form @form @onSubmit={s}> <input bind:value={n} validate:{custom: checkName} /> </form> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("签名应为单参返回 String")),
            "V-F5 bad-signature message missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V-F7 @load fn 必须存在、无参、返回 Array 类型
    ///     → error「@load 仅支持 Array 返回（v0）」
    #[test]
    fn load_fn_must_return_array() {
        let src = "component App { fn f() -> User {} view { @load={f} <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("仅支持 Array 返回")),
            "V-F7 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V-F8 保留名冲突：@load 存在时用户信号 loading/error/data
    ///     → error「信号名 "@x" 与 @load 生成的保留名冲突」
    #[test]
    fn reserved_signal_conflict() {
        let src =
            "component App { signal loading: bool = false; fn f() {} view { @load={f} <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags.iter().any(|d| d.message.contains("保留名冲突")),
            "V-F8 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// R4：V-F8 同查计算属性——@load 存在时用户计算属性 data
    ///     → error「计算属性名 "@data" 与 @load 生成的保留名冲突」
    #[test]
    fn reserved_computed_conflict() {
        let src = "component App { computed data: Array<String> = []; fn f() -> Array<String> {} view { @load={f} <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("计算属性名") && d.message.contains("保留名冲突")),
            "R4 computed conflict missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// R4：V-F8 组件 fn 名不得为 retry/load——@load 存在时 `fn retry`
    ///     → error「函数名 "@retry" 与 @load 内建名冲突」
    #[test]
    fn reserved_fn_conflict() {
        let src =
            "component App { fn retry() {} fn f() -> Array<String> {} view { @load={f} <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("函数名") && d.message.contains("内建名冲突")),
            "R4 fn conflict missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// R5：@load 内建别名 retry 带实参调用（Flutter codegen 静默丢参，
    /// React 仅靠 TS2554 兜底）→ error「@load 内建别名 "retry" 仅支持无参调用（v0）」
    #[test]
    fn retry_call_with_args_reports() {
        let src = "component App { fn f() -> Array<String> {} view { @load={f} <button @onClick={retry(42)} /> } }";
        let diags = compile_diags(src);
        assert!(
            diags.iter().any(|d| d.message.contains("仅支持无参调用")),
            "R5 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// R5 反向：retry() 空参调用（@load 刷新重试的标准用法）不报该错
    #[test]
    fn retry_call_without_args_ok() {
        let src = "component App { fn f() -> Array<String> {} view { @load={f} <button @onClick={retry()} /> } }";
        let diags = compile_diags(src);
        assert!(
            !diags.iter().any(|d| d.message.contains("仅支持无参调用")),
            "R5 false positive, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// R5 覆盖非视图槽位：computed 体内 retry(42)（用户计算属性调用内建
    /// 别名）同样报错。computed 须在视图中被引用——未被引用的计算属性会
    /// 在 optimize 阶段（先于语义 pass）被 DCE 消除，本检查只覆盖会抵达
    /// codegen 的代码。
    #[test]
    fn retry_call_with_args_in_computed_reports() {
        let src = "component App { computed canRetry: bool = retry(42) != \"\"; fn f() -> Array<String> {} view { @load={f} @if canRetry { <div/> } } }";
        let diags = compile_diags(src);
        assert!(
            diags.iter().any(|d| d.message.contains("仅支持无参调用")),
            "R5 computed-site missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// H1（reviewer BLOCK）：@form 生成的内建符号 `form` 必须在语义 pass 可见——
    /// 用户源码 `form.errors.name`（错误显式渲染，brief 第 44 行）不得报 E0001。
    /// V-F8 保留名证明 formErrors/formValid/formSubmitting 是编译器生成符号，
    /// 语义 pass 必须种入 `form` 供 FieldAccess 解析对象。
    #[test]
    fn form_builtin_symbol_resolves() {
        let src = "component App { signal name: string = \"\"; fn submit(e) {} view { <form @form @onSubmit={submit}> <input bind:value={name} validate:{required: true} /> @if form.errors.name { <span id=\"err-name\">{form.errors.name}</span> } </form> } }";
        let diags = compile_diags(src);
        assert!(
            diags.is_empty(),
            "form builtin symbol should resolve, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// H1（reviewer BLOCK）：@load 生成的内建符号 loading/error/data 必须在语义
    /// pass 可见——用户源码三态渲染（`@if loading` / `@if error != ""` /
    /// `@for item in data`，brief 第 27 行）不得报 E0001。
    #[test]
    fn load_builtin_symbols_resolve() {
        let src = "component App { fn fetchTodos() -> Array<String> {} view { @load={fetchTodos} @if loading { <div>loading</div> } @if error != \"\" { <div>{error}</div> } @for item in data { <div>{item}</div> } } }";
        let diags = compile_diags(src);
        assert!(
            diags.is_empty(),
            "load builtin symbols should resolve, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 编译源码并收集全部 Severity::Warning 诊断（V10/V11 断言专用——
    /// compile_diags 只收 Error，warning 断言需单独通道）。
    fn compile_warnings(src: &str) -> Vec<Diagnostic> {
        let result = crate::compile(src);
        result
            .errors
            .into_iter()
            .filter(|d| matches!(d.severity, crate::error::Severity::Warning))
            .collect()
    }

    // ───────────── 批次 4（spec §3 数据能力 V1-V11）─────────────
    // V1 实例名合法标识符（解析层拦截 parse_load_directive——语义兜底为
    //     Option<String> 结构约束，无可重检形态，测试随 V2 覆盖）
    // V2 实例名 vs 保留名/既有信号/函数冲突 → error「@load 实例名称 `{name}`
    //    与保留名/信号 `{conflict}` 冲突——保留名 loading/error/data/retry/reset
    //    专供匿名实例，请更换实例名」
    // V3 命名实例重名 → error「组件内 @load 实例名称重复：`{name}`——
    //    每个实例须唯一命名」（每重复名恰好 1 条）
    // V4 命名实例派生信号 vs 既有信号/函数冲突 → error「@load 实例 `{name}`
    //    生成的信号 `{derived}` 与组件信号/函数冲突，请更换实例名」
    // V5 interval 正整数（解析层拦截——语义侧 u64 不可能非正）
    // V6 匿名至多一个（lowering 扫描处报——覆盖断言）
    // V7 分页须命名 → error「分页 @load 必须命名（@load name={fn} paginated）
    //    ——分页信号 page/hasMore/loadingMore 需要实例归属」
    // V8 scroll 须配 paginated → error「scroll 触发须与 paginated 组合使用
    //    （@load name={fn} paginated scroll）」
    // V9 debounce/throttle 参数 → error「std/timing `{fn}` 的 ms 参数须为正
    //    整数毫秒；fn 须为可调用函数」
    // V10 scroll 容器可滚动性（lowering 扫描处报）→ warning「无限滚动 @load
    //    所在容器可能不可滚动——滚动触发可能永不生效，请确认容器有滚动能力」
    // V11 分页签名静态确认 → warning「分页装载函数签名建议为 `fn {name}
    //    (page: i32) -> Page<T>`——当前无法静态确认，运行时以约定信封兜底
    //    （诚实降级）」

    /// V2（spec §3）：命名实例名与保留名/既有信号冲突
    #[test]
    fn named_load_conflicts_with_reserved() {
        // 保留名：实例名 loading（保留名专供匿名实例）
        let src = "component App { fn f() -> Array<String> {} view { @load loading={f} <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("与保留名/信号 `loading` 冲突")
                    && d.message.contains("专供匿名实例")),
            "V2 reserved missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // 既有信号：@load items={f} 且 signal items
        let src2 =
            "component App { signal items: Array<String> = []; fn f() -> Array<String> {} view { @load items={f} <div/> } }";
        let diags2 = compile_diags(src2);
        assert!(
            diags2
                .iter()
                .any(|d| d.message.contains("与保留名/信号 `items` 冲突")),
            "V2 signal missing, got: {:?}",
            diags2.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V3（spec §3）：两个命名 @load 同名（每重复名恰好 1 条）
    #[test]
    fn duplicate_named_load_rejected() {
        let src = "component App { fn f() -> Array<String> {} view { <div> @load items={f} @load items={f} </div> } }";
        let diags = compile_diags(src);
        let v3: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("实例名称重复"))
            .collect();
        assert_eq!(
            v3.len(),
            1,
            "V3 should report once, got: {:?}",
            v3.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            v3[0].message.contains("`items`——每个实例须唯一命名"),
            "V3 message mismatch, got: {}",
            v3[0].message
        );
    }

    /// V4（spec §3）：命名实例派生信号与既有信号冲突——基础 5 后缀
    /// （Loading/Error/Data/Retry/Reset）恒查；分页 4 后缀
    /// （Page/HasMore/LoadingMore/LoadMore）仅 paginated 实例。
    #[test]
    fn derived_signal_conflict_detected() {
        // 基础后缀：signal itemsLoading 与 @load items={f} 的派生名冲突
        let src = "component App { signal itemsLoading: bool = false; fn f() -> Array<String> {} view { @load items={f} <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("`items` 生成的信号 `itemsLoading`")
                    && d.message.contains("与组件信号/函数冲突")),
            "V4 base missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // 分页后缀：signal itemsPage 与 paginated 实例派生名冲突
        let src2 = "component App { signal itemsPage: bool = false; fn f() -> Array<String> {} view { @load items={f} paginated <div/> } }";
        let diags2 = compile_diags(src2);
        assert!(
            diags2
                .iter()
                .any(|d| d.message.contains("`items` 生成的信号 `itemsPage`")),
            "V4 paginated missing, got: {:?}",
            diags2.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V5（spec §3）：interval 非正整数（解析层拦截——load_interval_error，
    /// 消息逐字锁定解析诊断）
    #[test]
    fn interval_must_be_positive() {
        let src =
            "component App { fn f() -> Array<String> {} view { @load={f} interval: 0 <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("interval 参数须为正整数毫秒")
                    && d.message.contains("当前：`0`")),
            "V5 zero missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        let src2 =
            "component App { fn f() -> Array<String> {} view { @load={f} interval: -100 <div/> } }";
        let diags2 = compile_diags(src2);
        assert!(
            diags2
                .iter()
                .any(|d| d.message.contains("interval 参数须为正整数毫秒")
                    && d.message.contains("当前：`-100`")),
            "V5 negative missing, got: {:?}",
            diags2.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V6（spec §3）：匿名 @load 至多一个（lowering 扫描处报——覆盖断言）
    #[test]
    fn multiple_anonymous_rejected() {
        let src = "component App { fn f() -> Array<String> {} fn g() -> Array<String> {} view { @load={f} @load={g} <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("组件内至多一个匿名 @load")
                    && d.message.contains("当前匿名 2 个")),
            "V6 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V7（spec §3）：分页实例匿名
    #[test]
    fn anonymous_paginated_rejected() {
        let src =
            "component App { fn f() -> Array<String> {} view { @load={f} paginated <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("分页 @load 必须命名")
                    && d.message.contains("需要实例归属")),
            "V7 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V8（spec §3）：scroll 未与 paginated 组合
    #[test]
    fn scroll_requires_paginated() {
        let src =
            "component App { fn f() -> Array<String> {} view { @load items={f} scroll <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("scroll 触发须与 paginated 组合使用")),
            "V8 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V9（spec §3）：debounce/throttle 参数校验——ms 须为正整数字面量、
    /// 首参须为可调用表达式
    #[test]
    fn debounce_throttle_args_validated() {
        // 负例：ms=0 / ms=-5 / 首参非函数
        for handler in [
            "debounce(doSearch, 0)",
            "debounce(doSearch, -5)",
            "debounce(123, 300)",
        ] {
            let src = format!(
                "component App {{ fn doSearch(q: String) {{}} view {{ <button @onClick={{{handler}}} /> }} }}"
            );
            let diags = compile_diags(&src);
            assert!(
                diags.iter().any(|d| d
                    .message
                    .contains("std/timing `debounce` 的 ms 参数须为正整数毫秒")),
                "V9 missing for {handler}, got: {:?}",
                diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            );
        }
        // 正例：debounce(fn, 300) / throttle(fn, 1000) → 无 V9
        let ok_src = "component App { fn doSearch(q: String) {} fn handleRefresh() {} view { <button @onClick={debounce(doSearch, 300)} /> <button @onClick={throttle(handleRefresh, 1000)} /> } }";
        let diags_ok = compile_diags(ok_src);
        assert!(
            !diags_ok.iter().any(|d| d.message.contains("std/timing")),
            "V9 false positive, got: {:?}",
            diags_ok.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V10（spec §3，warning）：scroll 容器无滚动能力（静态启发式——
    /// style 缺 height/max-height/overflow 键；lowering 扫描处报，容器
    /// 元素 span 在扫描时可用）
    #[test]
    fn non_scrollable_container_warns() {
        let src = "component App { fn fetchPage(page: i32) -> Page<String> {} view { <div> @load feed={fetchPage} paginated scroll @for item in feedData { <div>{item}</div> } </div> } }";
        let warns = compile_warnings(src);
        assert!(
            warns.iter().any(
                |d| d.message.contains("无限滚动 @load 所在容器可能不可滚动")
                    && d.message.contains("请确认容器有滚动能力")
            ),
            "V10 missing, got: {:?}",
            warns.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // 正向：容器带高度约束 → 不报
        let ok_src = "component App { fn fetchPage(page: i32) -> Page<String> {} view { <div style: { height: '400px' }> @load feed={fetchPage} paginated scroll @for item in feedData { <div>{item}</div> } </div> } }";
        let warns_ok = compile_warnings(ok_src);
        assert!(
            !warns_ok.iter().any(|d| d.message.contains("无限滚动")),
            "V10 false positive, got: {:?}",
            warns_ok.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V11（spec §3，warning）：分页装载函数签名无法静态确认——调用形态
    /// （实参绑定占用 page 槽位）恒 warning；Fn 形态签名不符 → warning；
    /// 签名恰为 fn {name}(page: i32) -> Page<T> → 静默。
    #[test]
    fn pagination_signature_unconfirmable_warns() {
        // 调用形态：@load feed={fetchPage(1)} paginated
        let src = "component App { fn fetchPage(page: i32) -> Page<String> {} view { @load feed={fetchPage(1)} paginated <div/> } }";
        let warns = compile_warnings(src);
        assert!(
            warns.iter().any(|d| d
                .message
                .contains("分页装载函数签名建议为 `fn fetchPage(page: i32) -> Page<T>`")
                && d.message.contains("诚实降级")),
            "V11 call missing, got: {:?}",
            warns.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // Fn 形态签名不符：fn f(x: String) -> Array<String>
        let src2 = "component App { fn f(x: String) -> Array<String> {} view { @load feed={f} paginated <div/> } }";
        let warns2 = compile_warnings(src2);
        assert!(
            warns2.iter().any(|d| d
                .message
                .contains("分页装载函数签名建议为 `fn f(page: i32) -> Page<T>`")),
            "V11 fn-sig missing, got: {:?}",
            warns2.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // 正向：签名恰为 fn fetchPage(page: i32) -> Page<String> → 无 V11
        let ok_src = "component App { fn fetchPage(page: i32) -> Page<String> {} view { @load feed={fetchPage} paginated <div/> } }";
        let warns_ok = compile_warnings(ok_src);
        assert!(
            !warns_ok
                .iter()
                .any(|d| d.message.contains("分页装载函数签名建议")),
            "V11 false positive, got: {:?}",
            warns_ok.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
