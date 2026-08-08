//! KeyEvent `e.key` 语义键名枚举验证（D2，Task 5）——`e.key` 字符串本身跨平台
//! 一致（lib.dom KeyboardEvent.key 规范），价值在编译期验证与近邻建议：
//! KeyEvent handler（onKeyDown/onKeyUp）中 `e.key == "X"` 的字符串字面量
//! 必须是语义键名（Enter/Escape/... 或单字母 A-Z / 单数字 0-9），否则
//! E0006 硬错误 + 最近合法建议。非字面量比较（变量）v0 范围跳过（V6 警告项）。
//!
//! Runs after component lowering via `run_ir_passes`（pass 9），与
//! `validate_drag` 同构：`walk_view` 遍历元素，逐 handler 检查。

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::{walk_stmt, walk_view};
use crate::ir::{
    BinaryOpIR, BlockIR, ComponentIR, EventDataKind, EventHandlerIR, ExprIR, FunctionIR,
};
use crate::lexer::Span;

/// 语义键名枚举（D2）——e.key 字符串本身跨平台一致，价值在验证与建议。
/// v0 表（覆盖 demo 用点 + 导航/表单高频键；扩展按 lib.dom KeyboardEvent.key 规范）：
const SEMANTIC_KEYS: &[&str] = &[
    "Enter",
    "Escape",
    "Backspace",
    "Delete",
    "Tab",
    "CapsLock",
    "Shift",
    "Control",
    "Alt",
    "Meta",
    "Space",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    // 单字母 A-Z / 单数字 0-9 经谓词接受（keys.len() == 1 && is_alphanumeric）
];

/// 键名是否语义合法：表内枚举值，或单字母 A-Z / 单数字 0-9（大小写敏感）。
#[must_use]
pub fn is_valid_semantic_key(k: &str) -> bool {
    SEMANTIC_KEYS.contains(&k)
        || (k.len() == 1 && k.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()))
}

/// 对 SEMANTIC_KEYS 求最近合法键名——复用 `nearest_event_suggestion` 的前缀权重
/// score 模式（`common_prefix_len` 在 event_names.rs 为私有，此处同逻辑实现）：
/// `score = len(bad) + len(k) − 2·lcp`，并列取表序第一个（min_by_key 语义）。
#[must_use]
pub fn nearest_key_suggestion(bad: &str) -> Option<&'static str> {
    let lowered = bad.to_ascii_lowercase();
    SEMANTIC_KEYS.iter().copied().min_by_key(|k| {
        let lcp = common_prefix_len(&lowered, &k.to_ascii_lowercase());
        bad.len() + k.len() - 2 * lcp
    })
}

/// Longest common prefix length of two strings (case already folded).
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.as_bytes()
        .iter()
        .zip(b.as_bytes())
        .take_while(|(x, y)| x == y)
        .count()
}

/// 遍历组件 view 树，检查每个 KeyEvent handler 中 `e.key == "X"` 字面量比较。
pub fn validate_semantic_keys(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(ref view) = component.view {
        walk_view(view, &mut |el| {
            for handler in el.events.values() {
                if *handler.event_data() != Some(EventDataKind::KeyEvent) {
                    continue;
                }
                // 事件参数名与 handler body：InlineFn 直接携带 body；
                // FnRef 在组件 fns 中查（解析规则同 lower/view.rs）。
                let (block, param_name): (&BlockIR, &str) = match handler {
                    EventHandlerIR::InlineFn { body, params, .. } => {
                        let p = params.first().map(|p| p.as_str()).unwrap_or("e");
                        (body, p)
                    }
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        match find_component_fn(component, r#ref) {
                            Some(f) => (&f.body, component_fn_param(f).unwrap_or("e")),
                            None => continue, // 未解析的 fn 引用——其他 pass 负责
                        }
                    }
                    EventHandlerIR::StoreAction { .. } => continue,
                };
                check_block(block, param_name, component, el.source_span, diagnostics);
            }
        });
    }
}

/// 组件函数查找——与 lower/view.rs 的 FnRef 解析同规则（含 `_{name}` 后缀）。
fn find_component_fn<'c>(component: &'c ComponentIR, ref_name: &str) -> Option<&'c FunctionIR> {
    component
        .fns
        .iter()
        .find(|f| f.name == ref_name)
        .or_else(|| {
            component
                .fns
                .iter()
                .find(|f| f.name == ref_name || f.name.ends_with(&format!("_{ref_name}")))
        })
}

fn component_fn_param(f: &FunctionIR) -> Option<&str> {
    f.params.first().map(|p| p.name.as_str())
}

/// 扫描 handler body：`e.key == "X"` 且字面量非语义键名 → E0006 硬错误 + 建议。
fn check_block(
    block: &BlockIR,
    param_name: &str,
    component: &ComponentIR,
    el_span: Option<Span>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for stmt in &block.statements {
        walk_stmt(stmt, &mut |expr| {
            let ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Eq,
                left,
                right,
            } = expr
            else {
                return;
            };
            let ExprIR::LiteralExpr { value } = right.as_ref() else {
                return; // 非字面量（变量/表达式）——v0 范围跳过
            };
            let serde_json::Value::String(lit) = value else {
                return;
            };
            let ExprIR::FieldAccessExpr { object, field } = left.as_ref() else {
                return;
            };
            if field != "key" {
                return;
            }
            let ExprIR::IdentifierExpr { name, .. } = object.as_ref() else {
                return;
            };
            if name != param_name {
                return;
            }
            if is_valid_semantic_key(lit) {
                return;
            }
            diagnostics.push(error_diag(
                component,
                el_span,
                match nearest_key_suggestion(lit) {
                    Some(s) => format!("e.key 值 \"{lit}\" 不是语义键名——最近合法建议 \"{s}\""),
                    None => format!("e.key 值 \"{lit}\" 不是语义键名"),
                },
            ));
        });
    }
}

/// E0006 硬错误诊断（同 validate_event_names 的 error_diag 通道），定位到元素源 span。
fn error_diag(component: &ComponentIR, el_span: Option<Span>, message: String) -> Diagnostic {
    let (line, col) = el_span
        .or(component.source_span)
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    Diagnostic {
        severity: Severity::Error,
        kind: DiagnosticKind::E0006,
        message,
        file: component.source_file.clone(),
        line,
        col,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DiagnosticKind;
    use crate::error::Severity;

    fn compile_src(source: &str) -> crate::CompileResult {
        crate::compile(source)
    }

    #[test]
    fn semantic_key_validation() {
        assert!(is_valid_semantic_key("Enter"));
        assert!(is_valid_semantic_key("Escape"));
        assert!(is_valid_semantic_key("ArrowUp"));
        assert!(!is_valid_semantic_key("enter"), "大小写敏感——键名规范");
        assert!(!is_valid_semantic_key("Entr"));
    }

    #[test]
    fn single_char_alphanumeric_keys_are_valid() {
        assert!(is_valid_semantic_key("a"));
        assert!(is_valid_semantic_key("A"));
        assert!(is_valid_semantic_key("0"));
        assert!(is_valid_semantic_key("9"));
        assert!(!is_valid_semantic_key("ab"), "多字符非表内键无效");
        assert!(!is_valid_semantic_key(""), "空串无效");
    }

    #[test]
    fn nearest_key_suggestion_finds_close_key() {
        assert_eq!(nearest_key_suggestion("entr"), Some("Enter"));
        assert_eq!(nearest_key_suggestion("ArroUp"), Some("ArrowUp"));
        assert_eq!(nearest_key_suggestion("escape"), Some("Escape"));
        assert_eq!(nearest_key_suggestion("Escape"), Some("Escape"));
    }

    #[test]
    fn invalid_semantic_key_in_keydown_handler_errors_with_suggestion() {
        let source = r#"
component App {
    fn onKey(e) {
        if e.key == "entr" { print("x"); }
    }
    view {
        <input @onKeyDown={onKey} />
    }
}
"#;
        let result = compile_src(source);
        assert!(
            !result.success,
            "非法语义键名 \"entr\" 必须硬失败，got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            result.errors.iter().any(|d| {
                d.severity == Severity::Error
                    && d.kind == DiagnosticKind::E0006
                    && d.message.contains("entr")
                    && d.message.contains("Enter")
            }),
            "必须产生 E0006 硬错误 + 近邻建议（entr→Enter），got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn valid_semantic_key_in_keydown_handler_passes() {
        let source = r#"
component App {
    fn onKey(e) {
        if e.key == "Enter" { print("x"); }
    }
    view {
        <input @onKeyDown={onKey} />
    }
}
"#;
        let result = compile_src(source);
        assert!(
            result.success,
            "合法语义键名 \"Enter\" 必须通过，got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn single_char_key_passes() {
        let source = r#"
component App {
    fn onKey(e) {
        if e.key == "a" { print("x"); }
    }
    view {
        <input @onKeyDown={onKey} />
    }
}
"#;
        let result = compile_src(source);
        assert!(
            result.success,
            "单字符键 \"a\" 经谓词接受，got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn non_literal_key_comparison_skipped() {
        let source = r#"
component App {
    fn onKey(e) {
        let target = "entr";
        if e.key == target { print("x"); }
    }
    view {
        <input @onKeyDown={onKey} />
    }
}
"#;
        let result = compile_src(source);
        assert!(
            result.success,
            "变量比较 v0 范围跳过（V6 警告项），got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn invalid_semantic_key_in_inline_handler_errors() {
        let source = r#"
component App {
    view {
        <input @onKeyDown={(e) => { if e.key == "Entr" { print("x"); } }} />
    }
}
"#;
        let result = compile_src(source);
        assert!(
            !result.success,
            "内联 handler 非法语义键名 \"Entr\" 必须硬失败，got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            result.errors.iter().any(|d| {
                d.severity == Severity::Error
                    && d.kind == DiagnosticKind::E0006
                    && d.message.contains("Entr")
                    && d.message.contains("Enter")
            }),
            "内联 handler 必须产生 E0006 硬错误 + 建议，got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }
}
