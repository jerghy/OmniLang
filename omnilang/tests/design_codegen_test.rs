#![cfg(feature = "codegen-omniview")]
//! Integration tests for design block CSS codegen.
//!
//! Tests `emit_design_css` and `expand_variant` by constructing
//! `DesignIR` directly (post-lowering) and verifying CSS output.

use omnilang::codegen::omniview::design::*;
use omnilang::ir::design::*;
use std::collections::HashMap;

/// Create a test DesignIR with a Card class and Button variant.
fn make_test_design() -> DesignIR {
    let mut tokens = HashMap::new();
    tokens.insert(
        "surface".to_string(),
        TokenIR {
            value: "#ffffff".to_string(),
            token_type: TokenTypeIR::Color,
        },
    );
    tokens.insert(
        "primary".to_string(),
        TokenIR {
            value: "#6366f1".to_string(),
            token_type: TokenTypeIR::Color,
        },
    );
    tokens.insert(
        "radiusMd".to_string(),
        TokenIR {
            value: "8px".to_string(),
            token_type: TokenTypeIR::Size,
        },
    );

    let classes = vec![ClassIR {
        name: "Card".to_string(),
        properties: vec![
            ("background".to_string(), "#ffffff".to_string()),
            ("border-radius".to_string(), "8px".to_string()),
        ]
        .into_iter()
        .collect(),
        pseudo_states: vec![PseudoStateIR {
            pseudo: "hover".to_string(),
            properties: vec![(
                "box-shadow".to_string(),
                "0 4px 12px rgba(0,0,0,0.08)".to_string(),
            )]
            .into_iter()
            .collect(),
        }],
    }];

    let variants = vec![VariantIR {
        name: "Button".to_string(),
        dimensions: vec![
            VariantDimIR {
                name: "color".to_string(),
                values: vec!["primary".to_string(), "danger".to_string()],
            },
            VariantDimIR {
                name: "size".to_string(),
                values: vec!["sm".to_string(), "md".to_string()],
            },
        ],
        defaults: vec![
            ("color".to_string(), "primary".to_string()),
            ("size".to_string(), "md".to_string()),
        ]
        .into_iter()
        .collect(),
        base: vec![
            ("display".to_string(), "inline-flex".to_string()),
            ("border-radius".to_string(), "var(--radiusMd)".to_string()),
        ]
        .into_iter()
        .collect(),
        branches: vec![VariantBranchIR {
            dimension: "color".to_string(),
            value: "primary".to_string(),
            properties: vec![("background".to_string(), "#6366f1".to_string())]
                .into_iter()
                .collect(),
        }],
    }];

    DesignIR {
        name: "AppDesign".to_string(),
        tokens,
        classes,
        variants,
    }
}

#[test]
fn emit_class_css_output() {
    let design = make_test_design();
    let css = emit_design_css(&design).expect("should emit CSS");

    // Verify class rule
    assert!(
        css.contains(".Card {"),
        "CSS should contain .Card class selector"
    );
    assert!(
        css.contains("background: #ffffff;"),
        "CSS should contain background property with resolved value"
    );
    assert!(
        css.contains("border-radius: 8px;"),
        "CSS should contain border-radius property"
    );

    // Verify pseudo-state rule
    assert!(
        css.contains(".Card:hover {"),
        "CSS should contain .Card:hover pseudo-state selector"
    );
    assert!(
        css.contains("box-shadow: 0 4px 12px rgba(0,0,0,0.08);"),
        "CSS should contain pseudo-state property with value"
    );
}

#[test]
fn emit_variant_css_output() {
    let design = make_test_design();
    let css = emit_design_css(&design).expect("should emit CSS");

    // Verify variant base rule
    assert!(
        css.contains(".Button {"),
        "CSS should contain .Button variant base rule"
    );
    assert!(
        css.contains("display: inline-flex;"),
        "CSS should contain variant base property"
    );
    assert!(
        css.contains("border-radius: var(--radiusMd);"),
        "CSS should contain variant base property with token reference"
    );

    // Verify variant branch rule
    assert!(
        css.contains(".Button.primary {"),
        "CSS should contain .Button.primary branch rule"
    );
    assert!(
        css.contains("background: #6366f1;"),
        "CSS should contain branch property with resolved value"
    );
}

#[test]
fn expand_variant_returns_classes() {
    let design = make_test_design();
    let mut dim_values = HashMap::new();
    dim_values.insert("color".to_string(), "primary".to_string());
    dim_values.insert("size".to_string(), "sm".to_string());

    let (classes, extra_css) =
        expand_variant(&design, "Button", &dim_values).expect("should expand variant");

    // Should include the variant name and all matching dimension values
    assert!(
        classes.contains(&"Button".to_string()),
        "classes should include 'Button'"
    );
    assert!(
        classes.contains(&"primary".to_string()),
        "classes should include 'primary'"
    );
    assert!(
        classes.contains(&"sm".to_string()),
        "classes should include 'sm'"
    );
    assert_eq!(classes.len(), 3, "should have exactly 3 classes");

    // No additional CSS needed for standard branches
    assert!(
        extra_css.is_empty(),
        "expand_variant should not produce extra CSS for standard branches"
    );
}

#[test]
fn expand_variant_partial_dimensions() {
    let design = make_test_design();
    let mut dim_values = HashMap::new();
    dim_values.insert("color".to_string(), "danger".to_string());
    // size is omitted — should not be included

    let (classes, _) = expand_variant(&design, "Button", &dim_values).expect("should expand");

    assert!(classes.contains(&"Button".to_string()));
    assert!(classes.contains(&"danger".to_string()));
    assert_eq!(
        classes.len(),
        2,
        "should only include variant name and 'danger'"
    );
}

#[test]
fn expand_variant_not_found() {
    let design = make_test_design();
    let dim_values = HashMap::new();
    let result = expand_variant(&design, "NonExistent", &dim_values);
    assert!(
        result.is_err(),
        "should return error for unknown variant name"
    );
}

#[test]
fn emit_empty_design_produces_no_css() {
    let design = DesignIR {
        name: "Empty".to_string(),
        tokens: HashMap::new(),
        classes: vec![],
        variants: vec![],
    };
    let css = emit_design_css(&design).expect("should emit CSS for empty design");
    assert!(
        css.is_empty(),
        "empty design should produce empty CSS string"
    );
}

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
            },
            PseudoStateIR {
                pseudo: "focus".to_string(),
                properties: vec![
                    ("border-color".to_string(), "#6366f1".to_string()),
                    ("outline".to_string(), "none".to_string()),
                ]
                .into_iter()
                .collect(),
            },
        ],
    }];

    let design = DesignIR {
        name: "FormDesign".to_string(),
        tokens: HashMap::new(),
        classes,
        variants: vec![],
    };

    let css = emit_design_css(&design).expect("should emit CSS");

    assert!(css.contains(".Input {"));
    assert!(css.contains(".Input:hover {"));
    assert!(css.contains("border-color: #999;"));
    assert!(css.contains(".Input:focus {"));
    assert!(css.contains("border-color: #6366f1;"));
}
