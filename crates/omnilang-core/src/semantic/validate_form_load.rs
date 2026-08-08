//! 表单/数据三态语义验证 pass（spec 2026-08-08 §4.1/§4.2）——消费 Task 2 的
//! FormIntentIR/LoadIntentIR，校验约束与保留名冲突（V-F2/V-F3/V-F4/V-F5/V-F7/V-F8）。
//!
//! V-F1（@form 仅限 form 元素）与 V-F6（每组件至多一个 @form/@load）在
//! lowering 扫描处报错（ir/lower/component.rs）——因为 @form/@load 标记
//! 经 lowering 后从 IR 视图树消失（view.rs 将其从 props 丢弃），本 pass
//! 无法定位具体元素，只能消费已收集的意图 IR（诊断 span 回退组件级）。

/// 校验规则白名单（spec 2026-08-08 §4.1 v0 封闭集；Task 7 登记册锚点，门禁双向断言）。
pub const VALIDATE_RULES: &[&str] = &["required", "format", "min", "max", "pattern", "custom"];
/// `format` 规则的合法取值（v0 封闭集）。
pub const FORMAT_VALUES: &[&str] = &["email", "url"];

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ComponentIR, ExprIR, FormIntentIR, IRProgram, LoadIntentIR, TypeKind};

/// 校验 @form/@load 语义约束（spec 2026-08-08 §4.1/§4.2 的 V-F2/F3/F4/F5/F7/F8）。
///
/// V-F1（@form 仅限 form 元素）与 V-F6（每组件至多一个 @form/@load）在
/// lowering 扫描处报错（ir/lower/component.rs）——本 pass 消费已收集的
/// FormIntentIR/LoadIntentIR，诊断 span 回退组件级（意图 IR 无元素 span）。
pub fn validate_form_load(ir: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    for module in ir.modules.values() {
        for component in module.components.values() {
            if let Some(form) = &component.form_intent {
                validate_form(component, form, diagnostics);
            }
            if let Some(load) = &component.load_intent {
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
                "custom" => rule_is_custom_fn(component, &rule.value),
                _ => true, // 白名单已过滤，不可达
            };
            if !ok {
                error_diag(
                    component,
                    format!("校验规则 \"@{}\" 的值类型不合法（v0）", rule.name),
                    diagnostics,
                );
            }
        }
    }
    // V-F8（@form 侧）保留名冲突：formErrors/formValid/formSubmitting
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
}

/// V-F7/V-F8（@load 侧）。
fn validate_load(component: &ComponentIR, load: &LoadIntentIR, diagnostics: &mut Vec<Diagnostic>) {
    // V-F7 @load fn 必须存在、无参、返回 Array 类型
    let Some(f) = component.fns.iter().find(|f| f.name == load.fn_name) else {
        error_diag(
            component,
            format!("@load 引用的函数 \"@{}\" 不存在", load.fn_name),
            diagnostics,
        );
        return; // 函数不存在，其余检查无意义
    };
    if !f.params.is_empty() {
        error_diag(
            component,
            format!("@load 引用的函数 \"@{}\" 必须无参数", load.fn_name),
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
    // V-F8（@load 侧）保留名冲突：loading/error/data/retry（retry 为
    // @load 回调别名，语义 pass 已播种内建符号）
    for signal in &component.signals {
        if matches!(signal.name.as_str(), "loading" | "error" | "data" | "retry") {
            error_diag(
                component,
                format!("信号名 \"@{}\" 与 @load 生成的保留名冲突", signal.name),
                diagnostics,
            );
        }
    }
}

/// V-F5 custom 规则：组件 fn 名（存在 + 单参 + 返回 String）。
/// 非 IdentifierExpr 形态（表达式计算值）无法静态判定，跳过放行。
fn rule_is_custom_fn(component: &ComponentIR, value: &ExprIR) -> bool {
    let ExprIR::IdentifierExpr { name, .. } = value else {
        return true;
    };
    component.fns.iter().any(|f| {
        f.name == *name
            && f.params.len() == 1
            && f.return_type
                .as_ref()
                .is_some_and(|t| matches!(t.kind, TypeKind::String))
    })
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
}
