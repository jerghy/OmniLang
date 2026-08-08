//! Validate element event names against the shared canonical event table
//! (Task 4.2, D-B) — unknown event names warn with the nearest legal
//! suggestion instead of compiling clean and emitting a dead handler.
//!
//! Failure mode this closes (ai HIGH): `@onTotallyBogus={f}` used to report
//! "No issues found" — React emitted `onTotallyBogus` verbatim (runtime
//! no-op), Flutter only failed later in `dart analyze` with no .omni-side
//! hint. Every backend now gets a compile-time warning naming the event and
//! suggesting the nearest legal spelling.
//!
//! Whitelist = the canonical table's accepted names (the `press`/`click`/
//! `tap` alias families are built into `canonical_event_name`, so aliases
//! are legal by construction) PLUS custom events declared anywhere in the
//! program (`event onAdd: String;` — bound as `@onAdd={...}` on component
//! instances by the corpus). Events on component-instance elements are
//! custom-event bindings and are not validated against the DOM table.
//!
//! Runs post-IR-merge (see `analyse.rs` Phase 3e) so cross-file custom event
//! declarations and component names are visible — the per-file `lower()`
//! stage cannot distinguish an imported component's custom-event binding
//! from a typo'd DOM event. The single-file `run_pipeline` path (lib.rs)
//! registers the same pass with its own file-scope program (review I-1).
//!
//! KNOWN BLIND SPOT (explicit decision, review M-1): events on
//! component-instance elements are skipped wholesale — a custom event name
//! typed on the WRONG component (or on a DOM element, when any component
//! declares that name) is not caught, because the whitelist includes every
//! program-wide `event` declaration rather than resolving each element's
//! tag to ITS component's declared set. Per-element tag→component
//! resolution is future work; the whitelist contract here is "name is a
//! legal event somewhere in the program", which eliminates the ai HIGH
//! silent no-op without cross-referencing imports.
//!
//! 语义事件分级（spec 2026-08-08 §3.1，原则 8「语义化即切换」）：语义事件的
//! 旧 DOM 拼写（@onClick/@onMouseEnter/@onMouseLeave）在既有未知事件名
//! warning 检查之后追加硬错误 + 中文迁移提示（use @onPress/@onEnter/@onLeave），
//! 无透传共存期。命中判定 = canonical 同值：legacy_spelling(s) 与用户拼写的
//! canonical 相同即命中（"onClick" 与 "@click" 的 canonical 都是 "click"，
//! 裸拼写同样命中）；排除语义名自身（onPress 的 canonical "click" 与其 legacy
//! "onClick" 同值，须放行）。组件实例元素与自定义事件仍整体跳过（既有逻辑不动）。

use crate::codegen::shared::event_names::{
    canonical_event_name, legacy_spelling, nearest_event_suggestion, SEMANTIC_EVENT_NAMES,
};
use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ComponentIR, IRProgram, ViewTree};
use crate::lexer::Span;
use rustc_hash::FxHashSet;

/// Validate every element event name in every component view.
pub fn validate_event_names(ir: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    // Program-wide sets: component tags (elements with these tags carry
    // custom-event bindings, not DOM events) and declared custom events
    // (`event onX` — legal to bind regardless of the DOM table).
    let mut component_names: FxHashSet<&str> = FxHashSet::default();
    let mut custom_events: FxHashSet<&str> = FxHashSet::default();
    for module in ir.modules.values() {
        for component in module.components.values() {
            component_names.insert(component.name.as_str());
            for e in &component.events {
                custom_events.insert(e.name.as_str());
            }
        }
    }

    for module in ir.modules.values() {
        for component in module.components.values() {
            if let Some(ref view) = component.view {
                walk_view_tree(
                    view,
                    component,
                    &component_names,
                    &custom_events,
                    diagnostics,
                );
            }
        }
    }
}

/// Recursively walk the view tree, validating each element's event names.
fn walk_view_tree(
    tree: &ViewTree,
    component: &ComponentIR,
    component_names: &FxHashSet<&str>,
    custom_events: &FxHashSet<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match tree {
        ViewTree::Element(el) => {
            // Component instantiations bind custom events — skip the DOM
            // table check (matches codegen's component_map behavior).
            if !component_names.contains(el.tag.as_str()) {
                for name in el.events.keys() {
                    if canonical_event_name(name).is_none()
                        && !custom_events.contains(name.as_str())
                    {
                        diagnostics.push(warning_diag(
                            component,
                            el.source_span,
                            match nearest_event_suggestion(name) {
                                Some(s) => format!("未知事件名 \"@{name}\"——最近合法建议 \"@{s}\""),
                                None => format!("未知事件名 \"@{name}\""),
                            },
                        ));
                    }

                    // 语义事件分级（原则 8）：旧名 → 硬错误 + 迁移提示。命中判定：
                    // 某语义事件 s 的 legacy_spelling 与用户拼写 canonical 同值——
                    // "onClick" 与 "click" 的 canonical 都是 "click"，裸拼写同样命中。
                    // `name != s` 排除语义名自身（onPress 的 canonical "click" 与
                    // 其 legacy "onClick" 同值，须放行）。
                    for s in SEMANTIC_EVENT_NAMES {
                        if let Some(legacy) = legacy_spelling(s) {
                            if name != s
                                && canonical_event_name(legacy) == canonical_event_name(name)
                            {
                                diagnostics.push(error_diag(
                                    component,
                                    el.source_span,
                                    format!("事件名 \"@{name}\" 已弃用——使用 \"@{s}\""),
                                ));
                            }
                        }
                    }
                }
            }
            for child in &el.children {
                walk_view_tree(
                    child,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    walk_view_tree(body, component, component_names, custom_events, diagnostics);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                walk_view_tree(
                    fallback,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                walk_view_tree(body, component, component_names, custom_events, diagnostics);
            }
            if let Some(ref empty) = list.empty_body {
                walk_view_tree(
                    empty,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Slot { ref fallback, .. } => {
            for child in fallback {
                walk_view_tree(
                    child,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_tree(
                    child,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}

/// Build a warning diagnostic (E0006) positioned at the offending element's
/// source span when available (falls back to the component span).
fn warning_diag(component: &ComponentIR, el_span: Option<Span>, message: String) -> Diagnostic {
    let (line, col) = el_span
        .or(component.source_span)
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    Diagnostic {
        severity: Severity::Warning,
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

/// Build an error diagnostic (E0006, severity Error) — 语义事件分级硬错误通道
/// （原则 8）：旧事件名在编译期被拒绝 + 迁移提示。与 `warning_diag` 同构，
/// 仅 severity 不同（既有 E0006 通道）。
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
    use crate::codegen::shared::event_names::SEMANTIC_EVENT_NAMES;

    fn compile_src(source: &str) -> crate::CompileResult {
        crate::compile(source)
    }

    /// 语义事件分级的旧名（@onClick/@onMouseEnter/@onMouseLeave）必须产生
    /// E0006 硬错误 + 中文迁移提示（use @onPress/@onEnter/@onLeave）。
    /// @ 前缀拼写、无 @ 拼写与裸 stem 拼写（@click）同规则命中（canonical 同值）。
    #[test]
    fn legacy_event_names_hard_error_with_migration_hint() {
        let cases: &[(&str, &str, &str)] = &[
            ("@onClick={f}", "onClick", "onPress"),
            ("@click={f}", "click", "onPress"),
            ("onClick={f}", "onClick", "onPress"),
            ("@onMouseEnter={f}", "onMouseEnter", "onEnter"),
            ("@mouseenter={f}", "mouseenter", "onEnter"),
            ("@onMouseLeave={f}", "onMouseLeave", "onLeave"),
        ];
        for (attr, legacy, semantic) in cases {
            let source = format!(
                "component App {{\n    fn f(e) {{}}\n    view {{\n        <div {attr} />\n    }}\n}}"
            );
            let result = compile_src(&source);
            assert!(
                !result.success,
                "`{attr}` 必须硬失败（旧名 {legacy} 已弃用），got: {:?}",
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
                        && d.message.contains(legacy)
                        && d.message.contains(semantic)
                }),
                "`{attr}` 必须产生 E0006 硬错误 + 迁移提示（{legacy}→{semantic}），got: {:?}",
                result
                    .errors
                    .iter()
                    .map(|d| (d.severity.clone(), d.message.clone()))
                    .collect::<Vec<_>>()
            );
        }
    }

    /// 14 个语义事件名全部通过验证（无 Error 诊断）。注意：@onEnter/@onLeave
    /// 的 canonical 目前为 None（enter/leave 别名入表属 Task 3），会走既有
    /// 未知事件名 warning——本测试只断言不产生 Error。
    #[test]
    fn semantic_event_names_pass_validation() {
        let fns = SEMANTIC_EVENT_NAMES
            .iter()
            .enumerate()
            .map(|(i, _)| format!("fn f{i}(e) {{}}"))
            .collect::<Vec<_>>()
            .join("\n    ");
        let attrs = SEMANTIC_EVENT_NAMES
            .iter()
            .map(|n| format!("@{n}={{{n}}}"))
            .collect::<Vec<_>>()
            .join(" ");
        let source = format!(
            "component App {{\n    {fns}\n    view {{\n        <div {attrs} />\n    }}\n}}"
        );
        let result = compile_src(&source);
        assert!(
            result.success,
            "14 个语义事件名必须全部通过验证，got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            result.errors.iter().all(|d| d.severity != Severity::Error),
            "语义事件名不得产生 Error 诊断，got: {:?}",
            result.errors
        );
    }

    /// 组件自定义事件（event onX）与组件实例元素不受旧名检查影响：
    /// 实例元素整体跳过 DOM 事件名检查（既有跳过逻辑不动）。
    #[test]
    fn custom_events_exempt_from_legacy_check() {
        let source = r#"
component Child {
    event onX: String;
    view { <div /> }
}
component App {
    fn f(e) {}
    view {
        <Child @onX={f} />
        <Child @onClick={f} />
    }
}
"#;
        let result = compile_src(source);
        assert!(
            result.success,
            "组件实例上的自定义事件/旧名拼写必须豁免（实例元素跳过），got: {:?}",
            result
                .errors
                .iter()
                .map(|d| (d.severity.clone(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }
}
