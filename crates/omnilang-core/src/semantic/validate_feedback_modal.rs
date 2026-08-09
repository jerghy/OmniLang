//! 批次 1 域 10：反馈模态语义验证（V1-V14）。
//!
//! 覆盖 spec 2026-08-09 §3 的全部校验规则：
//! - V1-V5  `@modal` 绑定约束（布尔信号 / 仅 dialog / 必填 / 唯一 / 禁 @for）
//! - V6-V7  `showToast` 选项合法性（kind 白名单 / duration 非负）
//! - V8     `showToast` 内建语义函数保留名
//! - V9-V10 `@skeleton` 放置约束（仅 @load loading 分支 / shape 白名单）
//! - V11-V14 `@errorBoundary` 约束（唯一绑定 / fallback 槽 / 降级内容唯一）
//!
//! 实现注记：
//! - V4（重复 `@modal`）/V11（重复 `@errorBoundary`）在语义层不可见
//!   （lower/view.rs prop 循环 last-wins / 布尔折叠），由 lowering 侧报告，
//!   本模块负责其余 12 条；
//! - V12/V13 为 Severity::Warning，不阻断编译（success 只看 Error）。

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::{walk_expr, walk_stmt, walk_view};
use crate::ir::*;

/// 反馈模态语义验证入口：遍历全部模块组件，产出 V1-V14 诊断。
///
/// 接入点与 `validate_form_load` 同列（lib.rs run_pipeline 与
/// analyse.rs Phase 3f 各调用一次），诊断直接追加到共享集合。
pub fn validate_feedback_modal(ir: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    for module in ir.modules.values() {
        for component in module.components.values() {
            validate_component(component, diagnostics);
        }
    }
}

/// 单组件校验：V8 声明名保留 + V6/V7 toast 调用扫描 + 视图树 walker。
fn validate_component(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    // V8：showToast 是内建语义函数，不能用作组件声明名（信号 / 计算 /
    // 函数 / store / 组件五处声明位，消息一致）。
    let reserved = "showToast";
    if component.name == reserved {
        error_diag(component, RESERVED_NAME_MESSAGE.into(), diagnostics);
    }
    for sig in &component.signals {
        if sig.name == reserved {
            error_diag(component, RESERVED_NAME_MESSAGE.into(), diagnostics);
        }
    }
    for c in &component.computeds {
        if c.name == reserved {
            error_diag(component, RESERVED_NAME_MESSAGE.into(), diagnostics);
        }
    }
    for f in &component.fns {
        if f.name == reserved {
            error_diag(component, RESERVED_NAME_MESSAGE.into(), diagnostics);
        }
    }
    for s in &component.store_refs {
        if s == reserved {
            error_diag(component, RESERVED_NAME_MESSAGE.into(), diagnostics);
        }
    }
    for s in &component.inline_stores {
        if s == reserved {
            error_diag(component, RESERVED_NAME_MESSAGE.into(), diagnostics);
        }
    }

    // V6/V7：showToast 调用扫描——函数体 / effect / lifecycle / computed
    // 均为合法调用位（执行体），逐一 walk 收集 BuiltinToastCall。
    for f in &component.fns {
        for stmt in &f.body.statements {
            walk_stmt(stmt, &mut |e| check_toast_call(e, component, diagnostics));
        }
    }
    for e in &component.effects {
        for stmt in &e.body.statements {
            walk_stmt(stmt, &mut |e| check_toast_call(e, component, diagnostics));
        }
    }
    for l in &component.lifecycles {
        for stmt in &l.body.statements {
            walk_stmt(stmt, &mut |e| check_toast_call(e, component, diagnostics));
        }
    }
    for c in &component.computeds {
        walk_expr(&c.body, &mut |e| {
            check_toast_call(e, component, diagnostics)
        });
    }

    // 视图树校验（V1/V2/V3/V5/V9/V10/V12/V13/V14 + 内联事件处理器 toast）。
    if let Some(view) = &component.view {
        walk_view_checked(component, view, false, false, diagnostics);
    }
}

/// V6/V7：单条 BuiltinToastCall 校验（kind 白名单 / duration 非负）。
fn check_toast_call(expr: &ExprIR, component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let ExprIR::BuiltinToastCall { kind, duration, .. } = expr else {
        return;
    };
    // V6：Unknown 承载原始串——字面量形态（"bogus"）与动态形态
    // （String::new()，lowering 对非字面量 kind 置空）均非法。
    if let Some(ToastKindIR::Unknown(raw)) = kind {
        error_diag(
            component,
            format!("未知 toast 类型 \"{}\"——合法类型: info/success/error", raw),
            diagnostics,
        );
    }
    // V7：duration 负数非法（0 合法——spec §4.2 长时展示）。
    if let Some(d) = duration {
        if *d < 0 {
            error_diag(
                component,
                "\"showToast\" 的 duration 不能为负数".into(),
                diagnostics,
            );
        }
    }
}

/// 视图树递归校验：携带 `in_for`（V5）与 `in_loading`（V9）两个继承标志。
///
/// 遍历覆盖：元素子节点、条件分支（@if loading 分支判定见
/// `expr_refs_loading`，fallback 分支重置标志）、@for 循环体 / @empty
/// 体（置 in_for）、Fragment 与 @slot 降级内容。
fn walk_view_checked(
    component: &ComponentIR,
    tree: &ViewTree,
    in_for: bool,
    in_loading: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match tree {
        ViewTree::Element(el) => {
            // V1：@modal 的值必须是布尔信号（open_signal 须命中 bool 信号声明）。
            if let Some(modal) = &el.modal {
                let is_bool_signal = component.signals.iter().any(|s| {
                    s.name == modal.open_signal && matches!(s.r#type.kind, TypeKind::Bool)
                });
                if !is_bool_signal {
                    error_diag(
                        component,
                        "\"@modal\" 的值必须是布尔信号".into(),
                        diagnostics,
                    );
                }
            }
            // V2：@modal 只能用于 dialog 元素。
            if el.modal.is_some() && el.tag != "dialog" {
                error_diag(
                    component,
                    "\"@modal\" 只能用于 dialog 元素".into(),
                    diagnostics,
                );
            }
            // V3：dialog 元素必须绑定 @modal。
            if el.tag == "dialog" && el.modal.is_none() {
                error_diag(
                    component,
                    "dialog 元素必须绑定 \"@modal\"".into(),
                    diagnostics,
                );
            }
            // V5：@modal 不能用在 @for 循环内。
            if el.modal.is_some() && in_for {
                error_diag(
                    component,
                    "\"@modal\" 不能用在 @for 循环内".into(),
                    diagnostics,
                );
            }
            // V9：@skeleton 只能用在 @load 的 loading 分支内（@if loading）。
            if el.skeleton.is_some() && !in_loading {
                error_diag(
                    component,
                    "\"@skeleton\" 只能用在 @load 的 loading 分支内（@if loading）".into(),
                    diagnostics,
                );
            }
            // V10：未知骨架形状（shape 白名单 text/line/circle/card；
            // None = 默认 text，合法）。
            if let Some(skel) = &el.skeleton {
                if let Some(shape) = &skel.shape {
                    if !matches!(shape.as_str(), "text" | "line" | "circle" | "card") {
                        error_diag(
                            component,
                            format!(
                                "未知骨架形状 \"{}\"——合法形状: text/line/circle/card",
                                shape
                            ),
                            diagnostics,
                        );
                    }
                }
            }
            // V12（Warning）：@errorBoundary 没有 fallback 槽——降级内容
            // 缺失时仅告警，不阻断编译（success 只看 Error）。
            if let Some(eb) = &el.error_boundary {
                if eb.fallback.is_none() {
                    warning_diag(
                        component,
                        "\"@errorBoundary\" 没有提供 fallback 槽——渲染失败时将显示空内容，建议添加 @slot fallback 降级内容".into(),
                        diagnostics,
                    );
                }
            }
            // V14：@errorBoundary 的 fallback 槽重复声明——children 中的
            // @slot fallback 指令保留为 ViewTree::Slot（lowering 仅取首个
            // 进 ErrorBoundaryBindingIR.fallback），此处全量计数。
            if el.error_boundary.is_some() {
                let fallback_slots = el
                    .children
                    .iter()
                    .filter(
                        |c| matches!(c, ViewTree::Slot { name: Some(n), .. } if n == "fallback"),
                    )
                    .count();
                if fallback_slots > 1 {
                    error_diag(
                        component,
                        "\"fallback\" 槽重复声明——降级内容只能有一个".into(),
                        diagnostics,
                    );
                }
            }
            // V13（Warning）：@modal dialog 内容没有标题（h1-h6）——
            // 生成的 aria-labelledby 将被省略，仅告警不阻断编译。
            if el.modal.is_some() && el.tag == "dialog" && !subtree_has_heading(&el.children) {
                warning_diag(
                    component,
                    "\"@modal\" 内容没有标题——生成的 aria-labelledby 将被省略，建议在 dialog 内放置标题元素".into(),
                    diagnostics,
                );
            }
            // V6/V7 补充：内联事件处理器（InlineFn）体是执行体，
            // showToast 调用同样合法——扫描其语句。
            for handler in el.events.values() {
                if let EventHandlerIR::InlineFn { body, .. } = handler {
                    for stmt in &body.statements {
                        walk_stmt(stmt, &mut |e| check_toast_call(e, component, diagnostics));
                    }
                }
            }
            // 递归子节点（标志原样传播）。
            for child in &el.children {
                walk_view_checked(component, child, in_for, in_loading, diagnostics);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                // @if loading 分支判定：@load 存在且条件引用 loading 信号。
                let loading = in_loading
                    || (component.load_intent.is_some() && expr_refs_loading(&branch.condition));
                if let Some(body) = &branch.body {
                    walk_view_checked(component, body, in_for, loading, diagnostics);
                }
            }
            // fallback（@else）不继承本层 loading 细化——恢复继承标志。
            // 顶层 `@if loading { } @else { }` 的 @else 确实不是 loading 分支；
            // 但嵌套子树内（@if loading { @if x { } @else { } }）的 @else
            // 仍属 loading 子树（spec §2.3 规则 2），不得重置为 false。
            if let Some(fallback) = &cond.fallback {
                walk_view_checked(component, fallback, in_for, in_loading, diagnostics);
            }
        }
        ViewTree::ListRendering(lr) => {
            // @for 循环体与 @empty 体均置 in_for（循环内不得有 @modal）。
            if let Some(body) = &lr.body {
                walk_view_checked(component, body, true, in_loading, diagnostics);
            }
            if let Some(empty) = &lr.empty_body {
                walk_view_checked(component, empty, true, in_loading, diagnostics);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_checked(component, child, in_for, in_loading, diagnostics);
            }
        }
        ViewTree::Slot { fallback, .. } => {
            // 槽降级内容是真实视图内容——继续校验。
            for child in fallback {
                walk_view_checked(component, child, in_for, in_loading, diagnostics);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}

/// 条件分支是否为 `@if loading`：表达式树中存在 IdentifierExpr "loading"。
fn expr_refs_loading(expr: &ExprIR) -> bool {
    let mut found = false;
    walk_expr(expr, &mut |e| {
        if matches!(e, ExprIR::IdentifierExpr { name, .. } if name == "loading") {
            found = true;
        }
    });
    found
}

/// 子树内是否存在标题元素（Heading/Title 组件或 h1-h6 原生元素）——
/// V13 判据。spec §2.1 规则 5：Heading 类元素包括 Heading/Title
/// 组件调用（lowering 保留 PascalCase 原始 tag，别名展开在 codegen
/// 层 shared/html.rs 完成）与 h1-h6 元素。
fn subtree_has_heading(children: &[ViewTree]) -> bool {
    for child in children {
        let mut found = false;
        walk_view(child, &mut |el| {
            if matches!(
                el.tag.as_str(),
                "Heading" | "Title" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            ) {
                found = true;
            }
        });
        if found {
            return true;
        }
    }
    false
}

/// V8 保留名错误消息（五处声明位共用）。
const RESERVED_NAME_MESSAGE: &str = "\"showToast\" 是内建语义函数，不能用作组件声明名";

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

/// 构建警告诊断（V12/V13：Severity::Warning，不阻断编译）。
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

    /// 编译源码并收集全部 Error 诊断（错误类 V 的断言入口）。
    fn compile_diags(src: &str) -> Vec<Diagnostic> {
        crate::compile(src)
            .errors
            .into_iter()
            .filter(|d| matches!(d.severity, crate::error::Severity::Error))
            .collect()
    }

    /// 编译源码并收集全量诊断（含 Warning）——V12/V13 是警告，不过 Error 过滤器。
    fn compile_all_diags(src: &str) -> Vec<Diagnostic> {
        crate::compile(src).errors
    }

    /// V1：`@modal` 的值必须是布尔信号
    ///     → error「"@modal" 的值必须是布尔信号」
    #[test]
    fn modal_value_must_be_bool_signal() {
        let src =
            "component App { signal open: String = \"x\"; view { <dialog @modal={open} /> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "\"@modal\" 的值必须是布尔信号"),
            "V1 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V2：`@modal` 只能用于 dialog 元素
    ///     → error「"@modal" 只能用于 dialog 元素」
    #[test]
    fn modal_only_on_dialog_element() {
        let src = "component App { signal open: bool = false; view { <div @modal={open} /> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "\"@modal\" 只能用于 dialog 元素"),
            "V2 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V3：dialog 元素必须绑定 @modal
    ///     → error「dialog 元素必须绑定 "@modal"」
    #[test]
    fn dialog_requires_modal() {
        let src = "component App { view { <dialog>x</dialog> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "dialog 元素必须绑定 \"@modal\""),
            "V3 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V4：元素不能重复绑定 @modal（lowering 侧报告）
    ///     → error「元素不能重复绑定 "@modal"」
    #[test]
    fn duplicate_modal_binding() {
        let src = "component App { signal open: bool = false; view { <dialog @modal={open} @modal={open} /> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "元素不能重复绑定 \"@modal\""),
            "V4 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V5：@modal 不能用在 @for 循环内
    ///     → error「"@modal" 不能用在 @for 循环内」
    #[test]
    fn modal_forbidden_inside_for_loop() {
        let src = "component App { signal open: bool = false; signal items: Array<Int> = []; view { <div> @for item in items { <dialog @modal={open} /> } </div> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "\"@modal\" 不能用在 @for 循环内"),
            "V5 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 正向对照：@modal 不在 @for 内 → 不报 V5
    #[test]
    fn modal_outside_for_loop_is_legal() {
        let src = "component App { signal open: bool = false; view { <dialog @modal={open} /> } }";
        let diags = compile_all_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.message == "\"@modal\" 不能用在 @for 循环内"),
            "V5 false positive, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V6：未知 toast 类型（字面量形态）
    ///     → error「未知 toast 类型 "bogus"——合法类型: info/success/error」
    #[test]
    fn unknown_toast_kind_literal() {
        let src =
            "component App { fn doToast() { showToast(\"x\", { kind: \"bogus\" }) } view { <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "未知 toast 类型 \"bogus\"——合法类型: info/success/error"),
            "V6 literal missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V6 变体：kind 为动态表达式（非字面量）同样非法
    ///     → error 前缀「未知 toast 类型」+ 合法类型后缀
    #[test]
    fn unknown_toast_kind_dynamic() {
        let src =
            "component App { signal k: String = \"warn\"; fn doToast() { showToast(\"x\", { kind: k }) } view { <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags.iter().any(|d| {
                d.message.starts_with("未知 toast 类型")
                    && d.message.ends_with("——合法类型: info/success/error")
            }),
            "V6 dynamic missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V7：showToast duration 不能为负数
    ///     → error「"showToast" 的 duration 不能为负数」
    #[test]
    fn negative_toast_duration() {
        let src =
            "component App { fn doToast() { showToast(\"x\", { duration: -5 }) } view { <div/> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "\"showToast\" 的 duration 不能为负数"),
            "V7 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 正向对照：duration: 0 合法（spec §4.2 Flutter 映射长时展示）
    #[test]
    fn zero_duration_is_legal() {
        let src =
            "component App { fn doToast() { showToast(\"x\", { duration: 0 }) } view { <div/> } }";
        let diags = compile_all_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.message == "\"showToast\" 的 duration 不能为负数"),
            "V7 false positive, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V8：showToast 是内建语义函数，不能用作组件声明名（信号 / 函数 / 组件三处声明位）
    ///     → error「"showToast" 是内建语义函数，不能用作组件声明名」
    #[test]
    fn show_toast_reserved_declaration_name() {
        let verbatim = "\"showToast\" 是内建语义函数，不能用作组件声明名";
        // 信号声明位
        let sig = "component App { signal showToast: bool = false; view { <div/> } }";
        let diags = compile_diags(sig);
        assert!(
            diags.iter().any(|d| d.message == verbatim),
            "V8 signal-site missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // 函数声明位
        let fun = "component App { fn showToast() {} view { <div/> } }";
        let diags = compile_diags(fun);
        assert!(
            diags.iter().any(|d| d.message == verbatim),
            "V8 fn-site missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // 组件声明位
        let comp = "component showToast { view { <div/> } }";
        let diags = compile_diags(comp);
        assert!(
            diags.iter().any(|d| d.message == verbatim),
            "V8 component-site missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V9：@skeleton 只能用在 @load 的 loading 分支内（含无 @load 场景）
    ///     → error「"@skeleton" 只能用在 @load 的 loading 分支内（@if loading）」
    #[test]
    fn skeleton_outside_loading_branch() {
        let verbatim = "\"@skeleton\" 只能用在 @load 的 loading 分支内（@if loading）";
        // @load 存在但 @skeleton 不在 @if loading 内
        let with_load =
            "component App { fn f() -> Array<Int> {} view { @load={f} <div @skeleton /> } }";
        let diags = compile_diags(with_load);
        assert!(
            diags.iter().any(|d| d.message == verbatim),
            "V9 with-load missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        // 无 @load 时 @skeleton 一律非法
        let no_load = "component App { view { <div @skeleton /> } }";
        let diags = compile_diags(no_load);
        assert!(
            diags.iter().any(|d| d.message == verbatim),
            "V9 no-load missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 正向对照：@skeleton 在 @load 的 @if loading 分支内 → 不报 V9
    #[test]
    fn skeleton_inside_loading_branch_is_legal() {
        let src = "component App { fn f() -> Array<Int> {} view { @load={f} @if loading { <div @skeleton /> } } }";
        let diags = compile_all_diags(src);
        assert!(
            !diags.iter().any(|d| {
                d.message == "\"@skeleton\" 只能用在 @load 的 loading 分支内（@if loading）"
            }),
            "V9 false positive, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 正向对照：嵌套 @else 属于 loading 子树（spec §2.3 规则 2）
    ///     `@if loading { @if x { A } @else { <div @skeleton/> } } @else { B }`
    ///     → 内层 @else 的 @skeleton 合法，不报 V9；
    ///       外层 @else（B）仍非 loading 分支（in_loading 恢复而非清零）
    #[test]
    fn skeleton_in_nested_else_of_loading_is_legal() {
        let verbatim = "\"@skeleton\" 只能用在 @load 的 loading 分支内（@if loading）";
        let src = "component App { signal x: bool = false; fn f() -> Array<Int> {} view { @load={f} @if loading { @if x { <div>A</div> } @else { <div @skeleton /> } } @else { <div>B</div> } } }";
        assert!(
            crate::compile(src).success,
            "nested else skeleton must compile"
        );
        let diags = compile_all_diags(src);
        assert!(
            !diags.iter().any(|d| d.message == verbatim),
            "V9 false positive on nested else, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V10：未知骨架形状
    ///     → error「未知骨架形状 "bogus"——合法形状: text/line/circle/card」
    #[test]
    fn unknown_skeleton_shape() {
        let src = "component App { fn f() -> Array<Int> {} view { @load={f} @if loading { <div @skeleton=\"bogus\" /> } } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "未知骨架形状 \"bogus\"——合法形状: text/line/circle/card"),
            "V10 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V11：元素不能重复绑定 @errorBoundary（lowering 侧报告）
    ///     → error「元素不能重复绑定 "@errorBoundary"」
    #[test]
    fn duplicate_error_boundary_binding() {
        let src = "component App { view { <div @errorBoundary @errorBoundary /> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "元素不能重复绑定 \"@errorBoundary\""),
            "V11 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V12（Warning）：@errorBoundary 没有 fallback 槽
    ///     → warning「"@errorBoundary" 没有提供 fallback 槽——渲染失败时将显示空内容，
    ///       建议添加 @slot fallback 降级内容」；severity 必须是 Warning 且不阻断编译
    #[test]
    fn error_boundary_missing_fallback_warns() {
        let verbatim = "\"@errorBoundary\" 没有提供 fallback 槽——渲染失败时将显示空内容，建议添加 @slot fallback 降级内容";
        let src = "component App { view { <div @errorBoundary /> } }";
        let diags = compile_all_diags(src);
        let hit = diags
            .iter()
            .find(|d| d.message == verbatim)
            .expect("V12 missing");
        assert!(
            matches!(hit.severity, crate::error::Severity::Warning),
            "V12 severity must be Warning"
        );
        // 警告不阻断编译：compile().success 为真
        assert!(crate::compile(src).success, "V12 must not block compile");
    }

    /// 正向对照：@errorBoundary 提供 @slot fallback → 不报 V12
    #[test]
    fn error_boundary_with_fallback_is_legal() {
        let src = "component App { view { <div @errorBoundary> @slot fallback { <Text>加载失败</Text> } </div> } }";
        let diags = compile_all_diags(src);
        assert!(
            !diags.iter().any(|d| {
                d.message
                    .starts_with("\"@errorBoundary\" 没有提供 fallback 槽")
            }),
            "V12 false positive, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V13（Warning）：@modal dialog 内容没有标题（h1-h6）
    ///     → warning「"@modal" 内容没有标题——生成的 aria-labelledby 将被省略，
    ///       建议在 dialog 内放置标题元素」；severity 必须是 Warning 且不阻断编译
    #[test]
    fn modal_missing_heading_warns() {
        let verbatim = "\"@modal\" 内容没有标题——生成的 aria-labelledby 将被省略，建议在 dialog 内放置标题元素";
        let src =
            "component App { signal open: bool = false; view { <dialog @modal={open}><div>x</div></dialog> } }";
        let diags = compile_all_diags(src);
        let hit = diags
            .iter()
            .find(|d| d.message == verbatim)
            .expect("V13 missing");
        assert!(
            matches!(hit.severity, crate::error::Severity::Warning),
            "V13 severity must be Warning"
        );
        assert!(crate::compile(src).success, "V13 must not block compile");
    }

    /// 正向对照：dialog 内含标题元素（h1-h6）→ 不报 V13
    #[test]
    fn modal_with_heading_is_legal() {
        let src =
            "component App { signal open: bool = false; view { <dialog @modal={open}><h1>标题</h1></dialog> } }";
        let diags = compile_all_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.message.starts_with("\"@modal\" 内容没有标题")),
            "V13 false positive, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 正向对照：dialog 内含 Heading 组件（spec §2.1 示例形态
    /// `<Heading>确认删除</Heading>`，lowering 保留 PascalCase tag，
    /// 别名展开在 codegen 层）→ 不报 V13
    #[test]
    fn modal_with_heading_component_is_legal() {
        let src =
            "component App { signal open: bool = false; view { <dialog @modal={open}><Heading>确认删除</Heading></dialog> } }";
        assert!(
            crate::compile(src).success,
            "Heading dialog must compile cleanly"
        );
        let diags = compile_all_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.message.starts_with("\"@modal\" 内容没有标题")),
            "V13 false positive on Heading component, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 正向对照：dialog 内含 Title 组件 → 不报 V13
    #[test]
    fn modal_with_title_component_is_legal() {
        let src =
            "component App { signal open: bool = false; view { <dialog @modal={open}><Title>确认删除</Title></dialog> } }";
        assert!(
            crate::compile(src).success,
            "Title dialog must compile cleanly"
        );
        let diags = compile_all_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.message.starts_with("\"@modal\" 内容没有标题")),
            "V13 false positive on Title component, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// V14：@errorBoundary 的 fallback 槽重复声明
    ///     → error「"fallback" 槽重复声明——降级内容只能有一个」
    #[test]
    fn duplicate_fallback_slot() {
        let src = "component App { view { <div @errorBoundary> @slot fallback { <Text>A</Text> } @slot fallback { <Text>B</Text> } </div> } }";
        let diags = compile_diags(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message == "\"fallback\" 槽重复声明——降级内容只能有一个"),
            "V14 missing, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// 正向对照：单一 fallback 槽 → 不报 V14
    #[test]
    fn single_fallback_slot_is_legal() {
        let src = "component App { view { <div @errorBoundary> @slot fallback { <Text>加载失败</Text> } </div> } }";
        let diags = compile_all_diags(src);
        assert!(
            !diags
                .iter()
                .any(|d| d.message == "\"fallback\" 槽重复声明——降级内容只能有一个"),
            "V14 false positive, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
