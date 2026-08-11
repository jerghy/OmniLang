//! Style-selector diagnostics (batch 3, spec 2026-08-09 §2.3) — W-301.
//!
//! The parser records unsupported selector parts (sibling `+`/`~`, child
//! `>`, attribute `[attr]`, parameterized pseudo-classes, pseudo-elements,
//! selector lists) on `StyleRuleIR.unsupported_selector` instead of silently
//! truncating them. This pass turns those records into `Severity::Warning`
//! diagnostics (honest degradation, ADR-054 — build does not fail), covering
//! top-level style rules AND rules inside `@media` blocks. Complex selectors
//! are a 留生态 domain-14 entry; support grows with corpus demand.

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{IRProgram, StyleRuleIR};
use crate::lexer::Span;

/// Emit the W-301 warning for one style rule with a recorded skip.
/// Anchor = the owning component's source_file/source_span (component-level
/// convention, same as validate_feedback_modal/validate_route_navigation —
/// precise rule spans would need IR to carry them).
fn check_rule(
    rule: &StyleRuleIR,
    file: &Option<String>,
    span: &Option<Span>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(us) = &rule.unsupported_selector {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::E0006,
            message: format!(
                "样式规则选择器「{}」包含暂不支持的语法部分（{}），已截断到「{}」继续解析——该部分 web 端按 CSS 透传处理、Flutter 端不生效；复杂选择器为留生态（semantic-universe 域 14 注记），支持随语料需求再议",
                us.text, us.category, rule.class_name
            ),
            file: file.clone(),
            line: span.map(|s| s.line).unwrap_or(0),
            col: span.map(|s| s.col).unwrap_or(0),
            end_line: None,
            end_col: None,
            suggestion: None,
        });
    }
}

/// Program-level pass: `unsupported_selector` records → W-301 warnings.
/// Runs on the merged IR (top-level styles + @media block rules).
pub fn validate_style_selectors(program: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    for (_, comp) in program.iter_all_components() {
        for rule in &comp.styles {
            check_rule(rule, &comp.source_file, &comp.source_span, diagnostics);
        }
        for mb in &comp.media_blocks {
            for rule in &mb.rules {
                check_rule(rule, &comp.source_file, &comp.source_span, diagnostics);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Severity as Sev;

    fn compile_warnings(src: &str) -> Vec<Diagnostic> {
        let result = crate::compile(src);
        result
            .errors
            .into_iter()
            .filter(|d| matches!(d.severity, Sev::Warning))
            .collect()
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
        let w = warnings
            .iter()
            .find(|d| d.message.contains("暂不支持的语法部分"))
            .expect("W-301 present");
        assert!(w.message.contains("兄弟选择器"), "got: {}", w.message);
        // lexer 丢弃选择器内的空白——skipped 文本为 `+span`（无空格）
        assert!(
            w.message.contains("input:checked +span"),
            "got: {}",
            w.message
        );
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
        assert!(
            warnings.iter().any(|d| d.message.contains("子选择器")),
            "got: {warnings:?}"
        );
    }
}
