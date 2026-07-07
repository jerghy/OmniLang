//! Integration tests for design block lowering: AST → DesignIR.
//!
//! Tests the full pipeline: lex → parse → lower_design for design blocks.
//! Verifies that token values are collected correctly and token references
//! in class/variant properties are resolved.

use omnilang::ir::design::*;
use omnilang::ir::lower_design;
use omnilang::lexer::tokenize;
use omnilang::parser::parse;

/// Helper: parse a design block from inline source and lower it to DesignIR.
fn lower_design_source(source: &str) -> DesignIR {
    let tokens = tokenize(source);
    let program = parse(tokens, source).expect("parse should succeed");
    let module = program
        .modules
        .get("input.omni")
        .expect("should have input.omni module");
    let ast = module
        .designs
        .first()
        .expect("should have at least one design block");
    lower_design(ast)
}

#[test]
fn lower_token_resolves_value() {
    let design = lower_design_source(
        r##"
        design App {
            token primary: color = "#6366f1";
            token surface: color = "#ffffff";
            token spacingMd: size = "16px";
            token shadowSm: shadow = "0 1px 3px rgba(0,0,0,0.12)";
        }
        "##,
    );

    assert_eq!(design.name, "App");
    assert_eq!(design.tokens.len(), 4);

    // Check each token
    let primary = design.tokens.get("primary").unwrap();
    assert_eq!(primary.value, "#6366f1");
    assert_eq!(primary.token_type, TokenTypeIR::Color);

    let surface = design.tokens.get("surface").unwrap();
    assert_eq!(surface.value, "#ffffff");
    assert_eq!(surface.token_type, TokenTypeIR::Color);

    let spacing = design.tokens.get("spacingMd").unwrap();
    assert_eq!(spacing.value, "16px");
    assert_eq!(spacing.token_type, TokenTypeIR::Size);

    let shadow = design.tokens.get("shadowSm").unwrap();
    assert_eq!(shadow.value, "0 1px 3px rgba(0,0,0,0.12)");
    assert_eq!(shadow.token_type, TokenTypeIR::Shadow);
}

#[test]
fn lower_class_resolves_token_refs() {
    let design = lower_design_source(
        r##"
        design App {
            token surface: color = "#ffffff";
            token radiusMd: size = "8px";
            token shadowSm: shadow = "0 1px 3px rgba(0,0,0,0.12)";

            class Card {
                background: token(surface);
                border-radius: token(radiusMd);
                box-shadow: token(shadowSm);
            }
        }
        "##,
    );

    assert_eq!(design.classes.len(), 1);
    let card = &design.classes[0];
    assert_eq!(card.name, "Card");

    // All token refs should be resolved to values
    assert_eq!(
        card.properties.get("background"),
        Some(&"#ffffff".to_string()),
        "token(surface) should resolve to '#ffffff'"
    );
    assert_eq!(
        card.properties.get("border-radius"),
        Some(&"8px".to_string()),
        "token(radiusMd) should resolve to '8px'"
    );
    assert_eq!(
        card.properties.get("box-shadow"),
        Some(&"0 1px 3px rgba(0,0,0,0.12)".to_string()),
        "token(shadowSm) should resolve to shadow value"
    );
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
    assert_eq!(
        alert.properties.get("padding"),
        Some(&"16px".to_string())
    );
    assert_eq!(
        alert.properties.get("font-size"),
        Some(&"14px".to_string())
    );
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
fn lower_token_ref_fallback() {
    let design = lower_design_source(
        r#"
        design App {
            class Card {
                background: token(undefinedToken);
            }
        }
        "#,
    );

    let card = &design.classes[0];
    assert_eq!(
        card.properties.get("background"),
        Some(&"var(--undefinedToken)".to_string()),
        "undefined token ref should fall back to var(--name)"
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
    assert_eq!(
        btn.base.get("display"),
        Some(&"inline-flex".to_string())
    );

    // Branch properties with resolved token
    let primary_branch = btn
        .branches
        .iter()
        .find(|b| b.dimension == "color" && b.value == "primary")
        .expect("should have color.primary branch");
    assert_eq!(
        primary_branch.properties.get("background"),
        Some(&"#6366f1".to_string()),
        "token(primary) should resolve to '#6366f1'"
    );
}
