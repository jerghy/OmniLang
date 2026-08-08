//! Validate the `a11y: { ... }` semantic accessibility block (design doc
//! 域 10, syntax spec §11).
//!
//! The block is the ONLY accessibility surface: it is a semantic-level
//! element attribute (`label`/`role`/`hidden`/`live`/`alt`) that maps to
//! aria-* on the web targets and to `Semantics(...)`/`ExcludeSemantics(...)`
//! on Flutter — the review's MEDIUM-2 (a11y intent used to be expressible
//! only as raw aria-* attrs that Flutter silently dropped) and MEDIUM-1
//! (raw `aria-*` attrs still pass through on web while Flutter keeps
//! dropping the accessible name).
//!
//! This pass enforces the contracts after lowering:
//! - Raw `aria-*` attributes are a DELETED form (domain MEDIUM-1 + D-D):
//!   web targets pass them through verbatim, Flutter silently drops the
//!   semantic — one intent, not all surfaces. Hard-errors with the
//!   migration hint to the `a11y: { ... }` block.
//! - `role` must be in the WAI-ARIA-canonical, Flutter-mappable subset
//!   (button/textbox/link/checkbox/radio/img). Legacy Flutter Semantics flag
//!   names (textField/image) warn with the canonical suggestion; other roles
//!   warn — they degrade on Flutter (no Semantics flag) but pass through on
//!   the web targets (design doc: 不可映射 role → warning + 生成代码注释).
//! - `live` accepts only `"polite"` / `"assertive"` (the BNF restricts the
//!   value; the parser accepts any string literal so this pass reports the
//!   precise diagnostic).
//! - `alt` is the image-only alternative-text surface — on any non-`img`
//!   tag it degrades on every target (warn). `alt=` as a plain DOM
//!   attribute is NOT an a11y surface (D-D boundary ruling: it stays a DOM
//!   prop, Flutter maps it to `semanticLabel` via the prop table).
//! - `hidden` on a keyboard-focusable element hides it from assistive tech
//!   while keyboard focus can still land on it — an a11y mismatch (warn).
//!
//! `label` is a plain string, `hidden`/`alt` are arbitrary expressions —
//! no further constraints.

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::walk_view;
use crate::ir::ComponentIR;

/// Keyboard-focusable element tags — `a11y: { hidden }` on one of these is
/// an a11y mismatch (hidden from assistive tech, still keyboard-focusable).
pub const FOCUSABLE_TAGS: &[&str] = &[
    "button", "input", "textarea", "select", "a", "iframe", "audio", "video", "summary",
];

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
///
/// Traversal uses the shared IR walker (`ir::walk::walk_view`) with a
/// per-element callback — the a11y checks only inspect `ElementIR` fields, so
/// the pass is a pure filter over the walker (Task 5.5 walker convergence).
pub fn validate_a11y(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let Some(ref view) = component.view else {
        return;
    };
    walk_view(view, &mut |el| check_element(el, component, diagnostics));
}

/// Validate a single element's a11y surface: raw `aria-*` props (deleted
/// form), the `a11y: { ... }` block's role/live/alt/hidden contracts.
fn check_element(
    el: &crate::ir::ElementIR,
    component: &ComponentIR,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Raw `aria-*` attributes are a deleted form (domain MEDIUM-1 +
    // D-D): they pass through on the web targets but Flutter
    // silently drops the semantic (accessible name lost). Hard-error
    // with the a11y-block migration hint — one diagnostic per
    // attribute so each offending surface is named.
    for name in el.props.keys() {
        if name.starts_with("aria-") {
            diagnostics.push(error_diag(
                component,
                format!(
                    "aria-* 原生属性已删除：'{name}' 仅 web 透传、\
                     Flutter 丢失可访问名称；请迁移到 \
                     a11y: {{ label/role/hidden/live/alt }} 语义块"
                ),
            ));
        }
    }
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
        // alt is the image-only alternative-text surface; on any
        // other tag the intent degrades on every target. (The plain
        // `alt=` DOM attribute is NOT an a11y surface — D-D ruling.)
        if a11y.alt.is_some() && el.tag != "img" {
            diagnostics.push(warning_diag(
                component,
                format!(
                    "a11y alt 仅对 <img> 有意义（→ alt / Image \
                     semanticLabel）；<{}> 上会降级，请改用 label",
                    el.tag
                ),
            ));
        }
        // hidden on a keyboard-focusable element: assistive tech
        // hides it but keyboard focus can still land on it — the
        // honest fix is conditional rendering.
        if a11y.hidden.is_some() && FOCUSABLE_TAGS.contains(&el.tag.as_str()) {
            diagnostics.push(warning_diag(
                component,
                format!(
                    "a11y hidden 使可聚焦元素 <{}> 对辅助技术不可见，\
                     但键盘仍可聚焦；请改用条件渲染移除元素",
                    el.tag
                ),
            ));
        }
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
    use crate::ir::ViewTree;
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
            drag_binding: None,
        }
    }

    /// Build a minimal ComponentIR wrapping the given elements in a view.
    fn make_component(name: &str, els: Vec<ViewTree>) -> ComponentIR {
        ComponentIR {
            form_intent: None,
            load_intent: None,
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
            form_intent: None,
            load_intent: None,
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

    /// Build an a11y block carrying only `alt` (image alternative text).
    fn a11y_alt_block() -> A11yIR {
        A11yIR {
            alt: Some(ExprIR::LiteralExpr {
                value: serde_json::Value::String("logo".into()),
            }),
            ..A11yIR::default()
        }
    }

    /// Build an a11y block carrying only `hidden`.
    fn a11y_hidden_block() -> A11yIR {
        A11yIR {
            hidden: Some(ExprIR::LiteralExpr {
                value: serde_json::Value::Bool(true),
            }),
            ..A11yIR::default()
        }
    }

    #[test]
    fn raw_aria_prop_hard_diagnosed_with_migration_hint() {
        // Task 4.4 (domain MEDIUM-1 + D-D): a raw `aria-*` attribute in the
        // element props is a deleted form — web targets pass it through but
        // Flutter silently drops the semantic (accessible name lost). Must
        // hard-error with the a11y-block migration hint.
        let mut el = make_el("div", A11yIR::default());
        el.props.insert(
            "aria-label".into(),
            ExprIR::LiteralExpr {
                value: serde_json::Value::String("x".into()),
            },
        );
        let comp = make_component("Test", vec![ViewTree::Element(Box::new(el))]);
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1, "got: {:?}", diagnostics);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(
            diagnostics[0].message.contains("aria-label")
                && diagnostics[0].message.contains("a11y"),
            "must name the attribute and carry the migration hint: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn multiple_raw_aria_props_each_diagnosed() {
        let mut el = make_el("div", A11yIR::default());
        for name in ["aria-label", "aria-hidden", "aria-live"] {
            el.props.insert(
                name.into(),
                ExprIR::LiteralExpr {
                    value: serde_json::Value::String("x".into()),
                },
            );
        }
        let comp = make_component("Test", vec![ViewTree::Element(Box::new(el))]);
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert_eq!(
            diagnostics.len(),
            3,
            "one diagnostic per aria-* attr: {:?}",
            diagnostics
        );
    }

    #[test]
    fn plain_non_aria_props_not_diagnosed() {
        let mut el = make_el("img", A11yIR::default());
        el.props.insert(
            "alt".into(),
            ExprIR::LiteralExpr {
                value: serde_json::Value::String("logo".into()),
            },
        );
        el.props.insert(
            "src".into(),
            ExprIR::LiteralExpr {
                value: serde_json::Value::String("icon.png".into()),
            },
        );
        let comp = make_component("Test", vec![ViewTree::Element(Box::new(el))]);
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "plain DOM props (alt=, src=) are not an a11y surface: {:?}",
            diagnostics
        );
    }

    #[test]
    fn a11y_alt_on_non_image_warns() {
        // Task 4.4 alt= boundary: `a11y: { alt }` is the image-only semantic
        // surface — on any other tag the intent degrades on every target.
        let comp = make_component(
            "Test",
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                a11y_alt_block(),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1, "got: {:?}", diagnostics);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("alt") && diagnostics[0].message.contains("img"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn a11y_alt_on_image_no_warning() {
        // The canonical pairing — img + alt — must not warn.
        let comp = make_component(
            "Test",
            vec![ViewTree::Element(Box::new(make_el(
                "img",
                a11y_alt_block(),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "img + alt must not warn, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn a11y_hidden_on_focusable_element_warns() {
        // hidden + keyboard-focusable = a11y mismatch: assistive tech hides
        // the element but keyboard focus can still land on it.
        let comp = make_component(
            "Test",
            vec![ViewTree::Element(Box::new(make_el(
                "button",
                a11y_hidden_block(),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1, "got: {:?}", diagnostics);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("hidden") && diagnostics[0].message.contains("button"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn a11y_hidden_on_non_focusable_no_warning() {
        // <p a11y: { hidden }> — the corpus pattern (A11yDemo) — must not warn.
        let comp = make_component(
            "Test",
            vec![ViewTree::Element(Box::new(make_el(
                "p",
                a11y_hidden_block(),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_a11y(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "hidden on a non-focusable element must not warn, got: {:?}",
            diagnostics
        );
    }
}
