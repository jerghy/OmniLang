//! 批次 2 导航/路由：组件级导航语义验证（V3-V13 + W1）。
//!
//! 覆盖 spec 2026-08-09 §3 的全部路由/导航校验规则：
//! - V3      `@route` 与 `@layout` 互斥
//! - V4      `@route` 重复声明（同组件两条 → 消息点名两条路径）
//! - V6      路径参数段重名（`/a/:id/b/:id`）
//! - V7      注入参数名与显式 prop/signal/event/fn 同名且类型不兼容
//! - V8      `@query` 名与路径参数同名
//! - V9      guard fn 引用存在性 + 签名 `() -> Bool`
//! - V10     `@layout` 组件视图含 `<@slot/>`
//! - V11     `redirect:` 必须与 `guard:` 配对
//! - V12     navigate() 首参为字面量且非 `/` 开头（硬错误）
//! - V13     组件成员名与保留名 navigate/back 冲突（硬错误）
//! - W1      navigate() 首参为动态表达式（警告，不阻断编译）
//!
//! 边界与分工注记：
//! - V2（路径非 `/` 开头）/V5（参数段非法）在解析期报告
//!   （parser/top_level/route.rs），本模块不重复；
//! - V4 重复声明的**全部**路径由 lowering 侧 `RouteIntentIR.decl_paths`
//!   携带（IR 只保留第一条声明，spec §2.1 语义化即切换），本模块点名报错；
//! - spec §6 边界 6：`@if` 页面互斥与 `event onNavigate` 是合法旧形态，
//!   本模块不做硬诊断——只校验新增导航语义。

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::{walk_expr, walk_stmt, walk_view};
use crate::ir::*;
use rustc_hash::FxHashSet;

/// 本模块覆盖的校验规则编号（T7 登记册锚点）。
pub const ROUTE_VALIDATE_RULES: &[&str] = &[
    "V3", "V4", "V6", "V7", "V8", "V9", "V10", "V11", "V12", "V13", "W1",
];

/// 内建语义函数保留名（navigate/back——V13 冲突检测集合）。
const NAVIGATE_RESERVED: &[&str] = &["navigate", "back"];

/// 导航语义验证入口：遍历全部模块组件，产出 V3-V13 + W1 诊断。
///
/// 接入点与 `validate_feedback_modal` 同列（analyse.rs Phase 3f3），
/// 诊断直接追加到共享集合。
pub fn validate_route_navigation(ir: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    for module in ir.modules.values() {
        for component in module.components.values() {
            validate_component(component, diagnostics);
        }
    }
}

/// 单组件校验：V3/V4/V6/V7/V8/V9/V11（@route 声明族）+ V10（@layout）+
/// V13（保留名）+ V12/W1（navigate 调用扫描）。
fn validate_component(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    // V3：@route 与 @layout 互斥——一个组件要么是页面要么是布局。
    if component.route_intent.is_some() && component.layout_intent.is_some() {
        error_diag(
            component,
            format!(
                "组件 '{}' 同时声明了页面（@route）与布局（@layout）——两者互斥，请拆分为两个组件",
                component.name
            ),
            diagnostics,
        );
    }

    if let Some(route) = &component.route_intent {
        // V4：@route 重复声明——decl_paths 保留全部声明路径，点名前两条。
        if route.decl_paths.len() > 1 {
            error_diag(
                component,
                format!(
                    "组件 '{}' 重复声明 @route（'{}' 与 '{}'）——一个组件只对应一个页面，多页面请拆分为多组件",
                    component.name, route.decl_paths[0], route.decl_paths[1]
                ),
                diagnostics,
            );
        }

        // V6：路径参数段重名（`/a/:id/b/:id`）——参数名在一条路径内唯一。
        let mut seen_params: FxHashSet<&str> = FxHashSet::default();
        for param in &route.params {
            if !seen_params.insert(param.as_str()) {
                error_diag(
                    component,
                    format!(
                        "路径 '{}' 参数段 `:{}` 重复——参数名在一条路径内必须唯一",
                        route.path, param
                    ),
                    diagnostics,
                );
            }
        }

        // V7：注入参数名与显式 prop/signal/event/fn 同名冲突
        // （同名 String prop 兼容放行——路径参数注入为 String prop）。
        for param in &route.params {
            validate_param_conflict(component, param, diagnostics);
        }

        // V8：@query 名与路径参数同名——两者注入同名 prop。
        for query in &component.query_params {
            if route.params.iter().any(|p| p == &query.name) {
                error_diag(
                    component,
                    format!(
                        "查询参数 '{}' 与路径参数 `:{}` 同名冲突——两者注入同名 prop",
                        query.name, query.name
                    ),
                    diagnostics,
                );
            }
        }

        // V9：guard fn 引用存在性 + 签名 `() -> Bool`（查组件 fn 表）。
        if let Some(guard) = &route.guard {
            if !is_valid_guard(component, guard) {
                error_diag(
                    component,
                    format!(
                        "守卫 '{}' 未找到或签名不是 `() -> Bool`（v0 只支持组件级同步布尔 fn）",
                        guard
                    ),
                    diagnostics,
                );
            }
        }

        // V11：redirect 必须与 guard 配对（单独使用无意义）。
        if route.redirect.is_some() && route.guard.is_none() {
            error_diag(
                component,
                "`redirect:` 必须与 `guard:` 配对使用（单独使用无意义）".into(),
                diagnostics,
            );
        }
    }

    // V10：@layout 组件视图必须包含 `<@slot/>`（子页面渲染出口）。
    if component.layout_intent.is_some()
        && !component.view.as_ref().is_some_and(view_contains_default_slot)
    {
        error_diag(
            component,
            format!(
                "布局组件 '{}' 的视图未包含 `<@slot/>`——子页面没有渲染出口",
                component.name
            ),
            diagnostics,
        );
    }

    // V13：组件成员名（fn/event/prop/signal）与保留名 navigate/back 冲突。
    for reserved in NAVIGATE_RESERVED {
        if component.fns.iter().any(|f| f.name == *reserved)
            || component.events.iter().any(|e| e.name == *reserved)
            || component.props.contains_key(*reserved)
            || component.signals.iter().any(|s| s.name == *reserved)
        {
            error_diag(
                component,
                format!(
                    "'{}' 是内建语义函数保留名，组件 '{}' 不可声明同名成员",
                    reserved, component.name
                ),
                diagnostics,
            );
        }
    }

    // V12/W1：navigate() 首参静态校验（全部可执行位置扫描）。
    scan_navigate_calls(component, diagnostics);
}

/// V7：路径参数注入名与显式成员同名冲突检测。
///
/// 显式 `prop <param>: String` 兼容放行（路径参数即注入为 String prop）；
/// 显式 prop 类型非 String，或 signal/event/fn 同名——均硬错误。
fn validate_param_conflict(
    component: &ComponentIR,
    param: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(ty) = component.props.get(param) {
        if !matches!(ty.kind, TypeKind::String) {
            error_diag(
                component,
                format!(
                    "路径参数 `:{}` 注入为 String prop，与组件 '{}' 已声明的 '{}' 冲突（类型不兼容）——请改名或删除显式声明",
                    param, component.name, param
                ),
                diagnostics,
            );
        }
        return;
    }
    if component.signals.iter().any(|s| s.name == param)
        || component.events.iter().any(|e| e.name == param)
        || component.fns.iter().any(|f| f.name == param)
    {
        error_diag(
            component,
            format!(
                "路径参数 `:{}` 注入为 String prop，与组件 '{}' 已声明的 '{}' 冲突（类型不兼容）——请改名或删除显式声明",
                param, component.name, param
            ),
            diagnostics,
        );
    }
}

/// V9：guard 引用须命中组件内同步、零参、`-> Bool` 的 fn。
fn is_valid_guard(component: &ComponentIR, name: &str) -> bool {
    component.fns.iter().any(|f| {
        f.name == name
            && f.params.is_empty()
            && !f.is_async
            && matches!(f.return_type.as_ref().map(|t| &t.kind), Some(TypeKind::Bool))
    })
}

/// V10：视图树递归查找默认 `<@slot/>`（Slot{name: None}）。
///
/// 遍历覆盖：元素子节点、@if 分支体与 fallback、@for 循环体与 @empty 体、
/// Fragment、具名 @slot 的降级内容（fallback 里嵌默认槽同样合法）。
fn view_contains_default_slot(view: &ViewTree) -> bool {
    match view {
        ViewTree::Slot { name: None, .. } => true,
        ViewTree::Slot {
            name: Some(_),
            fallback,
        } => fallback.iter().any(view_contains_default_slot),
        ViewTree::Element(el) => el.children.iter().any(view_contains_default_slot),
        ViewTree::Conditional(c) => {
            c.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_contains_default_slot))
                || c.fallback.as_ref().is_some_and(view_contains_default_slot)
        }
        ViewTree::ListRendering(l) => {
            l.body.as_ref().is_some_and(view_contains_default_slot)
                || l.empty_body.as_ref().is_some_and(view_contains_default_slot)
        }
        ViewTree::Fragment(children) => children.iter().any(view_contains_default_slot),
        ViewTree::Interpolation(_) | ViewTree::Text(_) => false,
    }
}

/// V12/W1：扫描全部可执行位置中的 navigate() 调用。
///
/// 范围：组件 fn / @effect / 生命周期 / computed 体 / signal 初值 /
/// prop 默认值 / 视图属性表达式 / 内联事件体（与 batch-1 showToast 同域，
/// 另扩 signal/prop 默认值——导航目标可来自任意表达式位置）。
fn scan_navigate_calls(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let mut visitor = |expr: &ExprIR| check_navigate_call(component, expr, diagnostics);

    for f in &component.fns {
        for stmt in &f.body.statements {
            walk_stmt(stmt, &mut visitor);
        }
    }
    for effect in &component.effects {
        for stmt in &effect.body.statements {
            walk_stmt(stmt, &mut visitor);
        }
    }
    for lifecycle in &component.lifecycles {
        for stmt in &lifecycle.body.statements {
            walk_stmt(stmt, &mut visitor);
        }
    }
    for computed in &component.computeds {
        walk_expr(&computed.body, &mut visitor);
    }
    for signal in &component.signals {
        walk_expr(&signal.initial_value, &mut visitor);
    }
    for default in component.prop_defaults.values() {
        walk_expr(default, &mut visitor);
    }
    if let Some(view) = &component.view {
        walk_view(view, &mut |el: &ElementIR| {
            for prop in el.props.values() {
                walk_expr(prop, &mut visitor);
            }
            for event in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = event {
                    for stmt in &body.statements {
                        walk_stmt(stmt, &mut visitor);
                    }
                }
            }
        });
    }
}

/// V12（硬错误）/W1（警告）：navigate() 首参必须是 `/` 开头的字面量。
///
/// 字面量且非 `/` 开头 → V12 外部链接引导（`<a href>`）；非字面量表达式
/// → W1 动态路径警告（目标端运行时负责校验）。零参调用不诊断。
fn check_navigate_call(component: &ComponentIR, expr: &ExprIR, diagnostics: &mut Vec<Diagnostic>) {
    let ExprIR::CallExpr { name, args } = expr else {
        return;
    };
    if name != "navigate" || args.is_empty() {
        return;
    }
    match &args[0] {
        ExprIR::LiteralExpr { value } => {
            let path = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            if !path.starts_with('/') {
                error_diag(
                    component,
                    format!(
                        "navigate 只接受应用内路径（`/` 开头）：'{}'——外部链接请用 `<a href>`",
                        path
                    ),
                    diagnostics,
                );
            }
        }
        dynamic => warning_diag(
            component,
            format!(
                "navigate 路径 '{}' 为动态表达式，无法静态校验是否已注册——目标端运行时负责",
                expr_to_string(dynamic)
            ),
            diagnostics,
        ),
    }
}

/// 表达式 → 用户可读串（W1 消息用；仅需覆盖常见形态，未知形态回退 `?`）。
fn expr_to_string(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string()),
        ExprIR::IdentifierExpr { name, .. } => name.clone(),
        ExprIR::FieldAccessExpr { object, field, .. } => {
            format!("{}.{}", expr_to_string(object), field)
        }
        ExprIR::OptionalChainExpr { object, field, .. } => {
            format!("{}?.{}", expr_to_string(object), field)
        }
        ExprIR::MethodCallExpr { object, method, .. } => {
            format!("{}.{}()", expr_to_string(object), method)
        }
        ExprIR::CallExpr { name, .. } => format!("{}()", name),
        ExprIR::BinaryOpExpr { left, op, right, .. } => format!(
            "{} {} {}",
            expr_to_string(left),
            binary_op_glyph(op),
            expr_to_string(right)
        ),
        ExprIR::MapExpr { .. } => "{ ... }".to_string(),
        ExprIR::ArrayExpr { .. } => "[ ... ]".to_string(),
        _ => "?".to_string(),
    }
}

/// BinaryOpIR → 运算符号（Display 输出 snake_case 名，非符号字形）。
fn binary_op_glyph(op: &BinaryOpIR) -> &'static str {
    match op {
        BinaryOpIR::Add => "+",
        BinaryOpIR::Sub => "-",
        BinaryOpIR::Mul => "*",
        BinaryOpIR::Div => "/",
        BinaryOpIR::Mod => "%",
        BinaryOpIR::Eq => "==",
        BinaryOpIR::Neq => "!=",
        BinaryOpIR::Lt => "<",
        BinaryOpIR::LtEq => "<=",
        BinaryOpIR::Gt => ">",
        BinaryOpIR::GtEq => ">=",
        BinaryOpIR::And => "&&",
        BinaryOpIR::Or => "||",
        BinaryOpIR::NullishCoalesce => "??",
        BinaryOpIR::Assign => "=",
        _ => "?",
    }
}

/// 构建错误诊断（E0006，span 回退组件级——与 validate_form_load 的
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

/// 构建警告诊断（W1：Severity::Warning，不阻断编译）。
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
    use crate::error::{Diagnostic, Severity};

    /// 编译源码并收集全部 Error 诊断（错误类 V 的断言入口）。
    fn compile_diags(src: &str) -> Vec<Diagnostic> {
        crate::compile(src)
            .errors
            .into_iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .collect()
    }

    /// 编译源码并收集全量诊断（含 Warning）——W1 是警告，不过 Error 过滤器。
    fn compile_all_diags(src: &str) -> Vec<Diagnostic> {
        crate::compile(src).errors
    }

    /// V3：@route 与 @layout 互斥——同组件两者都声明 → 错误含「互斥」。
    #[test]
    fn test_v3_route_layout_exclusive() {
        let diags = compile_diags(
            r#"component App {
                @route "/a";
                @layout "/b";
                view { <div/> }
            }"#,
        );
        assert!(
            diags.iter().any(|d| d.message.contains("互斥")),
            "expected V3 mutual-exclusion error, got: {:?}",
            diags
        );
    }

    /// V4：@route 重复声明——两条路径都点名。
    #[test]
    fn test_v4_duplicate_route() {
        let diags = compile_diags(
            r#"component App {
                @route "/a";
                @route "/b";
                view { <div/> }
            }"#,
        );
        let hit = diags.iter().find(|d| d.message.contains("重复声明 @route"));
        assert!(hit.is_some(), "expected V4 duplicate error, got: {:?}", diags);
        let msg = hit.unwrap().message.clone();
        assert!(msg.contains("/a") && msg.contains("/b"), "V4 must name both paths: {msg}");
    }

    /// V6：路径参数段重名（`/a/:id/b/:id`）。
    #[test]
    fn test_v6_duplicate_param() {
        let diags = compile_diags(
            r#"component App {
                @route "/a/:id/b/:id";
                view { <div/> }
            }"#,
        );
        assert!(
            diags.iter().any(|d| d.message.contains("参数段 `:id` 重复")),
            "expected V6 duplicate param error, got: {:?}",
            diags
        );
    }

    /// V7：注入参数名与显式成员同名——非 String signal 冲突（负），
    /// 同名 String prop 兼容放行（正）。
    #[test]
    fn test_v7_param_name_conflict() {
        let neg = compile_diags(
            r#"component App {
                signal id: Bool = false;
                @route "/x/:id";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("类型不兼容")),
            "expected V7 type-conflict error, got: {:?}",
            neg
        );

        let pos = compile_diags(
            r#"component App {
                prop id: String;
                @route "/x/:id";
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("类型不兼容")),
            "String prop must be compatible, got: {:?}",
            pos
        );
    }

    /// V8：@query 名与路径参数同名。
    #[test]
    fn test_v8_query_param_conflict() {
        let diags = compile_diags(
            r#"component App {
                @route "/x/:filter";
                @query filter: String = "all";
                view { <div/> }
            }"#,
        );
        assert!(
            diags.iter().any(|d| d.message.contains("同名冲突")),
            "expected V8 query/path conflict, got: {:?}",
            diags
        );
    }

    /// V9：guard fn 引用——缺失（负）、合法签名（正）、带参签名（负）。
    #[test]
    fn test_v9_guard_fn_missing() {
        let missing = compile_diags(
            r#"component App {
                @route "/x" guard: isAdmin;
                view { <div/> }
            }"#,
        );
        assert!(
            missing.iter().any(|d| d.message.contains("未找到或签名不是")),
            "expected V9 missing-guard error, got: {:?}",
            missing
        );

        let valid = compile_diags(
            r#"component App {
                fn isAdmin() -> Bool { return true; }
                @route "/x" guard: isAdmin;
                view { <div/> }
            }"#,
        );
        assert!(
            !valid.iter().any(|d| d.message.contains("未找到或签名不是")),
            "valid guard must pass, got: {:?}",
            valid
        );

        let bad_sig = compile_diags(
            r#"component App {
                fn isAdmin(name: String) -> Bool { return true; }
                @route "/x" guard: isAdmin;
                view { <div/> }
            }"#,
        );
        assert!(
            bad_sig.iter().any(|d| d.message.contains("未找到或签名不是")),
            "param-taking guard must fail V9, got: {:?}",
            bad_sig
        );
    }

    /// V10：@layout 组件视图——含 `<@slot/>` 通过（正），缺失报错（负）。
    #[test]
    fn test_v10_layout_needs_slot() {
        let pos = compile_diags(
            r#"component App {
                @layout "/admin";
                view { @slot default }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("未包含 `<@slot/>`")),
            "layout with default slot must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @layout "/admin";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("未包含 `<@slot/>`")),
            "expected V10 missing-slot error, got: {:?}",
            neg
        );
    }

    /// V11：redirect 与 guard 配对——无 guard 报错（负），有 guard 通过（正），
    /// 仅 guard 无 redirect 通过（默认 `/`）。
    #[test]
    fn test_v11_redirect_requires_guard() {
        let neg = compile_diags(
            r#"component App {
                @route "/x" redirect: "/login";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("必须与 `guard:` 配对")),
            "expected V11 pairing error, got: {:?}",
            neg
        );

        let pos = compile_diags(
            r#"component App {
                fn isAdmin() -> Bool { return true; }
                @route "/x" guard: isAdmin redirect: "/login";
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("必须与 `guard:` 配对")),
            "guard + redirect must pass, got: {:?}",
            pos
        );

        let guard_only = compile_diags(
            r#"component App {
                fn isAdmin() -> Bool { return true; }
                @route "/x" guard: isAdmin;
                view { <div/> }
            }"#,
        );
        assert!(
            !guard_only.iter().any(|d| d.message.contains("必须与 `guard:` 配对")),
            "guard without redirect must pass, got: {:?}",
            guard_only
        );
    }

    /// V12：navigate() 首参为非 `/` 开头的字面量 → 硬错误（外部链接引导）。
    #[test]
    fn test_v12_navigate_external_path() {
        let diags = compile_diags(
            r#"component App {
                fn go() { navigate("https://x.com"); }
                view { <div/> }
            }"#,
        );
        let hit = diags.iter().find(|d| d.message.contains("只接受应用内路径"));
        assert!(hit.is_some(), "expected V12 external-path error, got: {:?}", diags);
        let msg = hit.unwrap().message.clone();
        assert!(msg.contains("<a href>"), "V12 must suggest <a href>: {msg}");
    }

    /// V13：组件成员名与保留名 navigate/back 冲突——fn（负）、signal（负）、
    /// 常规成员名（正）。
    #[test]
    fn test_v13_reserved_name() {
        let fn_hit = compile_diags(
            r#"component App {
                fn navigate() {}
                view { <div/> }
            }"#,
        );
        assert!(
            fn_hit.iter().any(|d| d.message.contains("保留名")),
            "expected V13 fn conflict, got: {:?}",
            fn_hit
        );

        let signal_hit = compile_diags(
            r#"component App {
                signal back: String = "";
                view { <div/> }
            }"#,
        );
        assert!(
            signal_hit.iter().any(|d| d.message.contains("保留名")),
            "expected V13 signal conflict, got: {:?}",
            signal_hit
        );

        let pass = compile_diags(
            r#"component App {
                signal page: String = "";
                fn go() {}
                view { <div/> }
            }"#,
        );
        assert!(
            !pass.iter().any(|d| d.message.contains("保留名")),
            "normal member names must pass, got: {:?}",
            pass
        );
    }

    /// W1：navigate() 首参为动态表达式 → 警告（不阻断编译）。
    #[test]
    fn test_w1_navigate_dynamic_path() {
        let diags = compile_all_diags(
            r#"component App {
                signal page: String = "/a";
                fn go() { navigate(page); }
                view { <div/> }
            }"#,
        );
        let hit = diags.iter().find(|d| d.message.contains("动态表达式"));
        assert!(hit.is_some(), "expected W1 dynamic-path warning, got: {:?}", diags);
        assert!(
            matches!(hit.unwrap().severity, Severity::Warning),
            "W1 must be a warning, got: {:?}",
            hit.unwrap()
        );
    }

    // ============================================================
    // 批次 2 注入语料（spec §7 层 5）：路由语法 12 元组，每元组一正一负。
    // 用例名为 compiler test corpus 风格（test_corpus_*），断言要点照
    // V1-V14 任务测试模式——正例断言无对应规则错误，负例断言点名消息。
    // ============================================================

    /// 元组 1：@route 静态路径——`/todos`（正）vs 非 `/` 开头（负，V2 解析错误）。
    #[test]
    fn test_corpus_route_static_path() {
        let pos = compile_diags(
            r#"component App {
                @route "/todos";
                view { <div/> }
            }"#,
        );
        assert!(
            pos.is_empty(),
            "static @route must compile clean, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @route "todos";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("必须以 `/` 开头且非空")),
            "expected V2 path-prefix error, got: {:?}",
            neg
        );
    }

    /// 元组 2：@route 路径参数段——`:id`（正）vs 非法段（负，V5 解析错误）。
    #[test]
    fn test_corpus_route_path_param() {
        let pos = compile_diags(
            r#"component App {
                @route "/todos/:id";
                view { <div/> }
            }"#,
        );
        assert!(
            pos.is_empty(),
            "param @route must compile clean, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @route "/todos/:1x";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("路径参数段 ':1x' 非法")),
            "expected V5 illegal-param error, got: {:?}",
            neg
        );
    }

    /// 元组 3：@route guard——合法签名（正）vs fn 缺失（负，V9）。
    #[test]
    fn test_corpus_route_guard() {
        let pos = compile_diags(
            r#"component App {
                fn isAdmin() -> Bool { return true; }
                @route "/admin" guard: isAdmin;
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("未找到或签名不是")),
            "valid guard must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @route "/admin" guard: isAdmin;
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("未找到或签名不是")),
            "expected V9 missing-guard error, got: {:?}",
            neg
        );
    }

    /// 元组 4：@route lazy——布尔字面量（正）vs 非布尔（负，解析错误）。
    #[test]
    fn test_corpus_route_lazy() {
        let pos = compile_diags(
            r#"component App {
                @route "/heavy" lazy: true;
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("lazy 属性必须是 true/false")),
            "lazy: true must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @route "/heavy" lazy: 42;
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("lazy 属性必须是 true/false")),
            "expected lazy type error, got: {:?}",
            neg
        );
    }

    /// 元组 5：@layout——含 `<@slot/>`（正）vs 缺失（负，V10）。
    #[test]
    fn test_corpus_layout_slot() {
        let pos = compile_diags(
            r#"component App {
                @layout "/admin";
                view { @slot default }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("未包含 `<@slot/>`")),
            "layout with slot must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @layout "/admin";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("未包含 `<@slot/>`")),
            "expected V10 missing-slot error, got: {:?}",
            neg
        );
    }

    /// 元组 6：@query——与路径参数不同名（正）vs 同名冲突（负，V8）。
    #[test]
    fn test_corpus_query_params() {
        let pos = compile_diags(
            r#"component App {
                @route "/todos";
                @query filter: String = "all";
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("同名冲突")),
            "distinct query param must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @route "/todos/:filter";
                @query filter: String = "all";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("同名冲突")),
            "expected V8 query/path name conflict, got: {:?}",
            neg
        );
    }

    /// 元组 7：navigate 字面量路径——应用内路径（正）vs 外部链接（负，V12）。
    #[test]
    fn test_corpus_navigate_literal() {
        let pos = compile_diags(
            r#"component App {
                fn go() { navigate("/todos"); }
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("只接受应用内路径")),
            "in-app literal path must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                fn go() { navigate("https://x.com"); }
                view { <div/> }
            }"#,
        );
        let hit = neg.iter().find(|d| d.message.contains("只接受应用内路径"));
        assert!(hit.is_some(), "expected V12 external-path error, got: {:?}", neg);
        assert!(hit.unwrap().message.contains("<a href>"), "V12 must suggest <a href>");
    }

    /// 元组 8：navigate + params 选项——字面量路径（正）vs 动态路径（负，W1 警告）。
    #[test]
    fn test_corpus_navigate_params() {
        let pos = compile_all_diags(
            r#"component App {
                fn go() { navigate("/todos/:id", { params: { id: "42" } }); }
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("动态表达式")),
            "literal path with params must pass, got: {:?}",
            pos
        );

        let neg = compile_all_diags(
            r#"component App {
                signal page: String = "/a";
                fn go() { navigate(page, { params: { id: "42" } }); }
                view { <div/> }
            }"#,
        );
        let hit = neg.iter().find(|d| d.message.contains("动态表达式"));
        assert!(hit.is_some(), "expected W1 dynamic-path warning, got: {:?}", neg);
        assert!(
            matches!(hit.unwrap().severity, Severity::Warning),
            "W1 must be a warning, got: {:?}",
            hit.unwrap()
        );
    }

    /// 元组 9：navigate + query 选项——字面量路径（正）vs 空/非 `/` 字面量（负，V12）。
    #[test]
    fn test_corpus_navigate_query() {
        let pos = compile_diags(
            r#"component App {
                fn go() { navigate("/todos", { query: { filter: "all" } }); }
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("只接受应用内路径")),
            "literal path with query must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                fn go() { navigate("", { query: { filter: "all" } }); }
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("只接受应用内路径")),
            "empty literal path must fail V12, got: {:?}",
            neg
        );
    }

    /// 元组 10：back 内建——调用（正）vs 成员名占用（负，V13）。
    #[test]
    fn test_corpus_back_builtin() {
        let pos = compile_diags(
            r#"component App {
                fn go() { back(); }
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("保留名")),
            "back() call must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                signal back: String = "";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("保留名")),
            "expected V13 reserved-name error, got: {:?}",
            neg
        );
    }

    /// 元组 11：路径参数注入引用——显式 String prop + 视图 `{id}` 插值（正）
    /// vs 非 String 同名成员（负，V7）。
    #[test]
    fn test_corpus_param_injection_reference() {
        let pos = compile_diags(
            r#"component App {
                prop id: String;
                @route "/todos/:id";
                view { <div>{id}</div> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("类型不兼容")),
            "String prop + injected param must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                signal id: Bool = false;
                @route "/todos/:id";
                view { <div>{id}</div> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("类型不兼容")),
            "expected V7 type-conflict error, got: {:?}",
            neg
        );
    }

    /// 元组 12：守卫失败重定向——guard + redirect 配对（正）vs 无 guard（负，V11）。
    #[test]
    fn test_corpus_guard_redirect() {
        let pos = compile_diags(
            r#"component App {
                fn isAdmin() -> Bool { return true; }
                @route "/admin" guard: isAdmin redirect: "/login";
                view { <div/> }
            }"#,
        );
        assert!(
            !pos.iter().any(|d| d.message.contains("必须与 `guard:` 配对")),
            "guard + redirect must pass, got: {:?}",
            pos
        );

        let neg = compile_diags(
            r#"component App {
                @route "/admin" redirect: "/login";
                view { <div/> }
            }"#,
        );
        assert!(
            neg.iter().any(|d| d.message.contains("必须与 `guard:` 配对")),
            "expected V11 redirect-without-guard error, got: {:?}",
            neg
        );
    }
}
