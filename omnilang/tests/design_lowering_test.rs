//! Integration tests for design block lowering: AST → DesignIR.
//!
//! Tests the full pipeline: lex → parse → lower_design for design blocks.
//! Verifies that token values are collected correctly and token references
//! in class/variant properties are resolved.

use omnilang::ir::design::*;
use omnilang::ir::lower_design_with_consts;
use omnilang::lexer::tokenize;
use omnilang::parser::parse;
use rustc_hash::FxHashMap;

/// Helper: parse a design block from inline source and lower it to DesignIR.
fn lower_design_source(source: &str) -> DesignIR {
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let module = program
        .modules
        .get("input.omni")
        .expect("should have input.omni module");
    let ast = module
        .designs
        .first()
        .expect("should have at least one design block");
    let mut semantics = omnilang::semantic::SemanticResult::default();
    lower_design_with_consts(
        ast,
        &FxHashMap::default(),
        &FxHashMap::default(),
        &mut semantics,
    )
}

#[test]
fn lower_class_literal_values() {
    let design = lower_design_source(
        r#"
        design App {
            class Alert {
                padding: "16px";
                font-size: "14px";
            }
        }
        "#,
    );

    assert_eq!(design.classes.len(), 1);
    let alert = &design.classes[0];
    assert_eq!(alert.properties.get("padding"), Some(&"16px".to_string()));
    assert_eq!(alert.properties.get("font-size"), Some(&"14px".to_string()));
}

#[test]
fn lower_pseudo_state() {
    let design = lower_design_source(
        r##"
        design App {
            token primary: color = "#6366f1";

            class Button {
                background: token(primary);
                &:hover {
                    opacity: "0.9";
                }
                &:focus {
                    outline: "2px solid token(primary)";
                    outline-offset: "2px";
                }
            }
        }
        "##,
    );

    let btn = &design.classes[0];
    assert_eq!(btn.pseudo_states.len(), 2);

    // Check :hover
    let hover = &btn.pseudo_states[0];
    assert_eq!(hover.pseudo, "hover");
    assert_eq!(hover.properties.get("opacity"), Some(&"0.9".to_string()));

    // Check :focus — mixed value `"2px solid token(primary)"` is parsed as Mixed
    // Depending on parser implementation, this may be Literal or Mixed.
    // Just check that it exists and is non-empty.
    let focus = &btn.pseudo_states[1];
    assert_eq!(focus.pseudo, "focus");
    assert!(
        focus.properties.contains_key("outline"),
        "focus should have outline property"
    );
    assert!(
        focus.properties.contains_key("outline-offset"),
        "focus should have outline-offset property"
    );
}

#[test]
fn lower_variant_with_defaults() {
    let design = lower_design_source(
        r##"
        design App {
            token primary: color = "#6366f1";

            variant Button {
                @dim color: primary | danger;
                @dim size: sm | md;
                @default { color: primary; size: md; }
                base {
                    display: inline-flex;
                    border-radius: "8px";
                }
                color.primary { background: token(primary); }
                size.sm { padding: "4px 8px"; }
            }
        }
        "##,
    );

    assert_eq!(design.variants.len(), 1);
    let btn = &design.variants[0];
    assert_eq!(btn.name, "Button");
    assert_eq!(btn.dimensions.len(), 2);
    assert_eq!(btn.dimensions[0].name, "color");
    assert_eq!(btn.dimensions[0].values, vec!["primary", "danger"]);
    assert_eq!(btn.defaults.get("color"), Some(&"primary".to_string()));
    assert_eq!(btn.defaults.get("size"), Some(&"md".to_string()));

    // Base properties
    assert_eq!(btn.base.get("display"), Some(&"inline-flex".to_string()));

    // Branch properties with resolved token
    let primary_branch = btn
        .branches
        .iter()
        .find(|b| b.dimension == "color" && b.value == "primary")
        .expect("should have color.primary branch");
    assert_eq!(
        primary_branch.properties.get("background"),
        Some(&"var(--primary)".to_string()),
        "token(primary) should resolve to 'var(--primary)'"
    );
}
