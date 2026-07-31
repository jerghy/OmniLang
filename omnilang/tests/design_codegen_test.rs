#![cfg(feature = "codegen-omniview")]
//! Integration tests for design block CSS codegen.
//!
//! Tests `emit_design_css` by constructing `DesignIR` directly
//! (post-lowering) and verifying CSS output.
//!
//! Duplicate tests removed — unique scenarios only.
//! See `omnilang/src/codegen/omniview/design.rs` for the unit test suite.

use omnilang::codegen::omniview::design::*;
use omnilang::ir::design::*;
use omnilang::ir::ExprIR;
use rustc_hash::FxHashMap;

#[test]
fn emit_class_with_multiple_pseudo_states() {
    let classes = vec![ClassIR {
        name: "Input".to_string(),
        properties: vec![
            ("border".to_string(), "1px solid #ccc".to_string()),
            ("padding".to_string(), "8px".to_string()),
        ]
        .into_iter()
        .collect(),
        pseudo_states: vec![
            PseudoStateIR {
                pseudo: "hover".to_string(),
                properties: vec![("border-color".to_string(), "#999".to_string())]
                    .into_iter()
                    .collect(),
                source_span: None,
            },
            PseudoStateIR {
                pseudo: "focus".to_string(),
                properties: vec![
                    ("border-color".to_string(), "#6366f1".to_string()),
                    ("outline".to_string(), "none".to_string()),
                ]
                .into_iter()
                .collect(),
                source_span: None,
            },
        ],
        source_span: None,
    }];

    let design = DesignIR {
        name: "FormDesign".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: FxHashMap::default(),
        classes,
        variants: vec![],
        source_span: None,
    };

    let css = emit_design_css(&design, "FormDesign").expect("should emit CSS");

    assert!(css.contains("[data-design~=\"FormDesign\"] .Input {"));
    assert!(css.contains("[data-design~=\"FormDesign\"] .Input:hover {"));
    assert!(css.contains("border-color: #999;"));
    assert!(css.contains("[data-design~=\"FormDesign\"] .Input:focus {"));
    assert!(css.contains("border-color: #6366f1;"));
}

#[test]
fn design_token_from_const_produces_correct_css() {
    let source = r##"
import omniview;

const BRAND_COLOR = "#6366f1";
const CARD_RADIUS = "12px";

design AppDesign {
    token primary: color = BRAND_COLOR;
    token radiusLg: size = CARD_RADIUS;

    class Card {
        background: token(primary);
        border-radius: token(radiusLg);
    }
}

component App {
    view {
        <div class="Card">Hello</div>
    }
}
"##;

    let result = omnilang::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed, got errors: {:?}",
        result.errors
    );
    let css = result.files.expect("should have compiled files").css;

    // Token declarations on :root should contain resolved const values
    assert!(
        css.contains("--primary: #6366f1;"),
        "CSS should contain --primary: #6366f1;\nGot:\n{}",
        css
    );
    assert!(
        css.contains("--radiusLg: 12px;"),
        "CSS should contain --radiusLg: 12px;\nGot:\n{}",
        css
    );

    // Class properties should reference via var()
    assert!(
        css.contains("background: var(--primary);"),
        "CSS should contain background: var(--primary);\nGot:\n{}",
        css
    );
    assert!(
        css.contains("border-radius: var(--radiusLg);"),
        "CSS should contain border-radius: var(--radiusLg);\nGot:\n{}",
        css
    );
}

#[test]
fn css_output_skips_dynamic_tokens() {
    let mut tokens = FxHashMap::default();
    tokens.insert(
        "primary".to_string(),
        TokenIR {
            value: TokenValueIR::Static("#6366f1".to_string()),
            token_type: TokenTypeIR::Color,
            theme_values: FxHashMap::default(),
            source_span: None,
        },
    );
    tokens.insert(
        "dynamicAccent".to_string(),
        TokenIR {
            value: TokenValueIR::Dynamic(ExprIR::ident("ThemeStore_primaryColor")),
            token_type: TokenTypeIR::Color,
            theme_values: FxHashMap::default(),
            source_span: None,
        },
    );

    let design = DesignIR {
        name: "App".to_string(),
        themes: vec![],
        default_theme: None,
        tokens,
        classes: vec![],
        variants: vec![],
        source_span: None,
    };

    let css = emit_design_css(&design, "App").unwrap();

    // Static token should appear in :root
    assert!(
        css.contains("--primary: #6366f1"),
        "Static token --primary should appear in CSS\nGot:\n{}",
        css
    );
    // Dynamic token should NOT appear in :root
    assert!(
        !css.contains("--dynamicAccent"),
        "Dynamic token --dynamicAccent should NOT appear in CSS\nGot:\n{}",
        css
    );
}

#[test]
fn e2e_dynamic_token_full_pipeline() {
    let src = r##"
        store ThemeStore {
            signal primaryColor: string = "#6366f1";
        }

        design App {
            token primary: color = ThemeStore.primaryColor;
            token spacing: size = "16px";
        }
    "##;

    let sources: FxHashMap<String, String> = [("test.omni".to_string(), src.to_string())]
        .into_iter()
        .collect();

    let path_mapping = FxHashMap::default();
    let result = omnilang::analyse::analyse_multi(&sources, &path_mapping);

    assert!(
        result.success,
        "analysis should succeed: {:?}",
        result.diagnostics
    );

    let ir = result.ir.expect("should have IR");

    // Verify the token was lowered as Dynamic
    let module = ir.modules.values().next().expect("should have module");
    let design = module.designs.first().expect("should have design");
    let primary = design
        .tokens
        .get("primary")
        .expect("should have primary token");
    assert!(
        matches!(primary.value, TokenValueIR::Dynamic(_)),
        "ThemeStore.primaryColor should be Dynamic, got {:?}",
        primary.value
    );

    // Verify spacing is still Static
    let spacing = design
        .tokens
        .get("spacing")
        .expect("should have spacing token");
    assert!(
        matches!(spacing.value, TokenValueIR::Static(_)),
        "static value should remain Static"
    );
}

#[test]
fn emits_js_effect_for_dynamic_token() {
    let tokens = vec![(
        "primary".to_string(),
        ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "ThemeStore".to_string(),
                type_ann: None,
            }),
            field: "primaryColor".to_string(),
        },
    )];

    let js = emit_dynamic_token_effects(&tokens, |expr| match expr {
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                format!("{}_{}.value", name, field)
            } else {
                String::new()
            }
        }
        _ => String::new(),
    });

    assert!(
        js.contains("__omni_effect"),
        "should emit __omni_effect, got:\n{}",
        js
    );
    assert!(
        js.contains("--primary"),
        "should set --primary CSS variable, got:\n{}",
        js
    );
    assert!(
        js.contains("ThemeStore_primaryColor.value"),
        "should reference signal value, got:\n{}",
        js
    );
    assert!(
        js.contains("style.setProperty"),
        "should use setProperty, got:\n{}",
        js
    );
    assert!(
        js.contains("__omni_sanitizeCss"),
        "should call __omni_sanitizeCss, got:\n{}",
        js
    );
}
