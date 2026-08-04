//! Validate the `a11y: { ... }` semantic accessibility block (design doc
//! 域 10, syntax spec §11).
//!
//! The block is a semantic-level element attribute (`label`/`role`/`hidden`/
//! `live`/`alt`) that maps to aria-* on the web targets and to
//! `Semantics(...)`/`ExcludeSemantics(...)` on Flutter — the review's
//! MEDIUM-2 (a11y intent used to be expressible only as raw aria-* attrs
//! that Flutter silently dropped).
//!
//! This pass enforces the two value contracts after lowering:
//! - `role` must be in the WAI-ARIA-canonical, Flutter-mappable subset
//!   (button/textbox/link/checkbox/radio/img). Legacy Flutter Semantics flag
//!   names (textField/image) warn with the canonical suggestion; other roles
//!   warn — they degrade on Flutter (no Semantics flag) but pass through on
//!   the web targets (design doc: 不可映射 role → warning + 生成代码注释).
//! - `live` accepts only `"polite"` / `"assertive"` (the BNF restricts the
//!   value; the parser accepts any string literal so this pass reports the
//!   precise diagnostic).
//!
//! `label` is a plain string, `hidden`/`alt` are arbitrary expressions —
//! no further constraints.

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ComponentIR, ViewTree};

/// Flutter-mappable semantic roles (design doc 域 10 映射表), expressed in
/// WAI-ARIA canonical names (domain HIGH-2: the vocabulary used to be Flutter
/// Semantics flag shapes — `textField`/`image` — so the web targets emitted
/// illegal `role="textField"`). Each name maps to a `Semantics` flag
/// (`button: true` etc.); `textbox`/`img` map internally to
/// `textField: true`/`image: true`. Roles outside this subset warn — Flutter
/// degrades (no flag) with a generated-code comment, the web targets pass
/// the role through as an attribute.
pub const FLUTTER_ROLE_SUBSET: &[&str] = &["button", "textbox", "link", "checkbox", "radio", "img"];

/// Legacy Flutter Semantics flag names accepted before the vocabulary became
/// WAI-ARIA-canonical — each maps to its canonical ARIA role. Using them
/// warns with the canonical replacement (migration hint); the Flutter
/// codegen still maps them internally so existing sources keep working.
pub const LEGACY_FLAG_ROLES: &[(&str, &str)] = &[("textField", "textbox"), ("image", "img")];

/// Valid `live` values — aria-live polite/assertive (design doc BNF).
pub const LIVE_VALUES: &[&str] = &["polite", "assertive"];

/// Validate a11y blocks on every element of the component's view tree.
pub fn validate_a11y(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let Some(ref view) = component.view else {
        return;
    };
    walk_view_tree(view, component, diagnostics);
}

/// Recursively walk the view tree, validating every element's `a11y` block.
fn walk_view_tree(tree: &ViewTree, component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    match tree {
        ViewTree::Element(el) => {
            if let Some(ref a11y) = el.a11y {
                if let Some(ref role) = a11y.role {
                    if let Some(canonical) = LEGACY_FLAG_ROLES
                        .iter()
                        .find(|(flag, _)| *flag == role.as_str())
                        .map(|(_, canonical)| *canonical)
                    {
                        // Legacy Flutter Semantics flag name — not a
                        // WAI-ARIA role. Warn with the canonical replacement
                        // (migration hint); codegen still maps it so the
                        // source keeps working while migrating.
                        diagnostics.push(warning_diag(
                            component,
                            format!(
                                "a11y role '{role}' 是 Flutter Semantics 旗标名，\
                                 非 WAI-ARIA 角色；请改用规范名 '{canonical}' \
                                 （Flutter 内部映射回原旗标）"
                            ),
                        ));
                    } else if !FLUTTER_ROLE_SUBSET.contains(&role.as_str()) {
                        diagnostics.push(warning_diag(
                            component,
                            format!(
                                "a11y role '{role}' 不在 Flutter 可映射角色子集 \
                                 (button/textbox/link/checkbox/radio/img)；\
                                 Flutter 目标降级（无 Semantics flag），其余目标透传"
                            ),
                        ));
                    }
                }
                if let Some(live) = &a11y.live {
                    if !LIVE_VALUES.contains(&live.as_str()) {
                        diagnostics.push(error_diag(
                            component,
                            format!(
                                "a11y live 仅支持 \"polite\"/\"assertive\"，得到 \"{}\"",
                                live.as_str()
                            ),
                        ));
                    }
                }
            }
            for child in &el.children {
                walk_view_tree(child, component, diagnostics);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    walk_view_tree(body, component, diagnostics);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                walk_view_tree(fallback, component, diagnostics);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                walk_view_tree(body, component, diagnostics);
            }
            if let Some(ref empty) = list.empty_body {
                walk_view_tree(empty, component, diagnostics);
            }
        }
        ViewTree::Slot { ref fallback, .. } => {
            for child in fallback {
                walk_view_tree(child, component, diagnostics);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_tree(child, component, diagnostics);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}

/// Build an error diagnostic at the component's source location.
fn error_diag(component: &ComponentIR, message: String) -> Diagnostic {
    make_diag(component, Severity::Error, message)
}

/// Build a warning diagnostic at the component's source location.
fn warning_diag(component: &ComponentIR, message: String) -> Diagnostic {
    make_diag(component, Severity::Warning, message)
}

/// Shared diagnostic construction: E0006 (validation error), positioned at
/// the component's `source_span` / `source_file`.
fn make_diag(component: &ComponentIR, severity: Severity, message: String) -> Diagnostic {
    let (line, col) = component
        .source_span
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    Diagnostic {
        severity,
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
    use crate::ir::{A11yIR, BindingIR, ConstructionKind, ElementIR, ExprIR, ListRenderingIntent};
    use rustc_hash::FxHashMap;

    /// Build a minimal ElementIR with an a11y block.
    fn make_el(tag: &str, a11y: A11yIR) -> ElementIR {
        ElementIR {
            tag: tag.to_string(),
            a11y: Some(a11y),
            children: vec![],
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
        }
    }

    /// Build a minimal ComponentIR wrapping the given elements in a view.
    fn make_component(name: &str, els: Vec<ViewTree>) -> ComponentIR {
        ComponentIR {
            name: name.to_string(),
            view: if els.is_empty() {
                None
            } else {
                Some(ViewTree::Fragment(els))
            },
            source_file: Some("test.omni".into()),
            ..ComponentIR::default()
        }
    }

    fn a11y_with(role: Option<&str>, live: Option<&str>) -> A11yIR {
        A11yIR {
            label: None,
            role: role.map(String::from),
            hidden: None,
            live: live.map(String::from),
            alt: None,
        }
    }

    #[test]
    fn mappable_role_no_diagnostic() {
        // button/textbox/link/checkbox/radio/img — no warnings at all.
        for role in FLUTTER_ROLE_SUBSET {
            let comp = make_component(
                "Test",
                vec![ViewTree::Element(Box::new(make_el(
                    "button",
                    a11y_with(Some(role), None),
                )))],
            );
            let mut diagnostics = Vec::new();
            validate_a11y(&comp, &mut diagnostics);
            assert!(
                diagnostics.is_empty(),
                "role '{role}' must not diagnose, got: {:?}",
                diagnostics
            );
        }
    }

    #[test]
    fn legacy_flag_role_suggests_canonical_name() {
        // textField/image are Flutter Semantics flag names, not WAI-ARIA
        // roles — warn once with the canonical replacement (Task 4.3,
        // domain HIGH-2: web output must never carry role="textField").
        for (legacy, canonical) in LEGACY_FLAG_ROLES {
            let comp = make_component(
                "Test",
                vec![ViewTree::Element(Box::new(make_el(
                    "input",
                    a11y_with(Some(legacy), None),
                )))],
            );
            let mut diagnostics = Vec::new();
            validate_a11y(&comp, &mut diagnostics);
            assert_eq!(
                diagnostics.len(),
                1,
                "legacy role '{legacy}' must warn exactly once, got: {:?}",
                diagnostics
            );
            assert_eq!(diagnostics[0].severity, Severity::Warning);
            assert!(
                diagnostics[0].message.contains(canonical),
                "legacy role '{legacy}' must suggest canonical '{canonical}', got: {}",
                diagnostics[0].message
            );
        }
    }

    #[test]
    fn unmappable_role_warns() {
        // A real ARIA role outside the Flutter subset warns (honest
        // degradation: no Semantics flag on Flutter, passthrough elsewhere).
        let comp = make_component(
            "Test",
            vec![ViewTree::Element(Box::new(make_el(
                "input",
                a11y_with(Some("searchbox"), None),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("searchbox")
                && diagnostics[0].message.contains("Flutter"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn live_invalid_value_errors() {
        // live accepts only polite/assertive — anything else is a hard error.
        let comp = make_component(
            "Test",
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                a11y_with(None, Some("off")),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(
            diagnostics[0].message.contains("polite"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn live_polite_and_assertive_pass() {
        for live in LIVE_VALUES {
            let comp = make_component(
                "Test",
                vec![ViewTree::Element(Box::new(make_el(
                    "div",
                    a11y_with(None, Some(live)),
                )))],
            );
            let mut diagnostics = Vec::new();
            validate_a11y(&comp, &mut diagnostics);
            assert!(
                diagnostics.is_empty(),
                "live '{live}' must not diagnose, got: {:?}",
                diagnostics
            );
        }
    }

    #[test]
    fn a11y_inside_list_body_is_checked() {
        // a11y blocks nested inside a @for body must also be validated.
        let comp = ComponentIR {
            name: "TestComp".into(),
            view: Some(ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: ExprIR::ident("items"),
                item_binding: BindingIR {
                    name: "item".into(),
                    r#type: None,
                },
                key_expr: None,
                body: Some(ViewTree::Element(Box::new(make_el(
                    "button",
                    a11y_with(Some("menu"), None),
                )))),
                empty_body: None,
                scroll_hint: false,
            }))),
            source_file: Some("test.omni".into()),
            ..ComponentIR::default()
        };
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("menu") && d.severity == Severity::Warning),
            "expected role-subset warning inside @for body, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn element_without_a11y_block_no_diagnostics() {
        let comp = make_component(
            "Test",
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                A11yIR::default(),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "elements without a11y intent must not diagnose, got: {:?}",
            diagnostics
        );
    }
}
