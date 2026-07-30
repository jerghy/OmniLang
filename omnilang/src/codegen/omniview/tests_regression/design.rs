use crate::codegen::omniview::design::*;
use crate::ir::design::*;
use rustc_hash::FxHashMap;

fn make_test_design() -> DesignIR {
    let mut tokens = FxHashMap::default();
    tokens.insert(
        "surface".to_string(),
        TokenIR {
            value: TokenValueIR::Static("#ffffff".to_string()),
            token_type: TokenTypeIR::Color,
            theme_values: FxHashMap::default(),
            source_span: None,
        },
    );
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
        "radiusMd".to_string(),
        TokenIR {
            value: TokenValueIR::Static("8px".to_string()),
            token_type: TokenTypeIR::Size,
            theme_values: FxHashMap::default(),
            source_span: None,
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
            source_span: None,
        }],
        source_span: None,
    }];

    let variants = vec![VariantIR {
        name: "Button".to_string(),
        dimensions: vec![
            VariantDimIR {
                name: "color".to_string(),
                values: vec!["primary".to_string(), "danger".to_string()],
                source_span: None,
            },
            VariantDimIR {
                name: "size".to_string(),
                values: vec!["sm".to_string(), "md".to_string()],
                source_span: None,
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
            source_span: None,
        }],
        source_span: None,
    }];

    DesignIR {
        name: "AppDesign".to_string(),
        themes: vec![],
        default_theme: None,
        tokens,
        classes,
        variants,
        source_span: None,
    }
}

#[test]
fn emit_class_css_output() {
    let design = make_test_design();
    let css = emit_design_css(&design, "AppDesign").expect("should emit CSS");

    assert!(
        css.contains("[data-design~=\"AppDesign\"] .Card {"),
        "should contain scoped .Card class rule"
    );
    assert!(
        css.contains("--surface: #ffffff;"),
        "should contain token declaration in :root"
    );
    assert!(
        css.contains("background: #ffffff;"),
        "should contain resolved property"
    );
    assert!(
        css.contains("[data-design~=\"AppDesign\"] .Card:hover {"),
        "should contain scoped hover pseudo-state"
    );
    assert!(
        css.contains("box-shadow: 0 4px 12px rgba(0,0,0,0.08);"),
        "should contain pseudo-state property"
    );
}

#[test]
fn emit_variant_css_output() {
    let design = make_test_design();
    let css = emit_design_css(&design, "AppDesign").expect("should emit CSS");

    assert!(
        css.contains("[data-design~=\"AppDesign\"] .Button {"),
        "should contain scoped variant base rule"
    );
    assert!(
        css.contains("display: inline-flex;"),
        "should contain variant base property"
    );
    assert!(
        css.contains("[data-design~=\"AppDesign\"] .Button.primary {"),
        "should contain scoped variant branch rule"
    );
    assert!(
        css.contains("background: #6366f1;"),
        "should contain branch property"
    );
}

#[test]
fn emit_empty_design() {
    let design = DesignIR {
        name: "Empty".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: FxHashMap::default(),
        classes: vec![],
        variants: vec![],
        source_span: None,
    };
    let css = emit_design_css(&design, "Empty").expect("should emit empty CSS");
    assert!(css.is_empty(), "empty design should produce no CSS");
}

#[test]
fn expand_variant_returns_classes() {
    let design = make_test_design();
    let dim_values = vec![
        ("color".to_string(), "primary".to_string()),
        ("size".to_string(), "sm".to_string()),
    ]
    .into_iter()
    .collect();

    let (classes, extra_css) =
        expand_variant(&design, "Button", &dim_values).expect("should expand");

    assert!(
        classes.contains(&"Button".to_string()),
        "should include variant name"
    );
    assert!(
        classes.contains(&"primary".to_string()),
        "should include dimension value"
    );
    assert!(
        classes.contains(&"sm".to_string()),
        "should include dimension value"
    );
    assert!(extra_css.is_empty(), "no additional CSS needed");
}

#[test]
fn expand_variant_applies_defaults() {
    let design = make_test_design();
    // Only supply "color", leaving "size" to fall back to its default ("md")
    let dim_values = vec![("color".to_string(), "primary".to_string())]
        .into_iter()
        .collect();

    let (classes, _) = expand_variant(&design, "Button", &dim_values).expect("should expand");

    assert!(
        classes.contains(&"primary".to_string()),
        "should include explicitly provided dimension value"
    );
    assert!(
        classes.contains(&"md".to_string()),
        "should include default value for missing dimension"
    );
}

#[test]
fn expand_variant_not_found() {
    let design = make_test_design();
    let dim_values = FxHashMap::default();
    let result = expand_variant(&design, "NonExistent", &dim_values);
    assert!(result.is_err(), "should error on unknown variant");
}

#[test]
fn emit_design_css_with_themes() {
    let mut tokens = FxHashMap::default();
    let mut theme_values_dark = FxHashMap::default();
    theme_values_dark.insert("dark".to_string(), TokenValueIR::Static("#a5b4fc".to_string()));
    tokens.insert(
        "primary".to_string(),
        TokenIR {
            value: TokenValueIR::Static("#6366f1".to_string()),
            token_type: TokenTypeIR::Color,
            theme_values: theme_values_dark,
            source_span: None,
        },
    );

    let design = DesignIR {
        name: "AppDesign".to_string(),
        themes: vec!["light".to_string(), "dark".to_string()],
        default_theme: Some("light".to_string()),
        tokens,
        classes: vec![],
        variants: vec![],
        source_span: None,
    };

    let css = emit_design_css(&design, "AppDesign").expect("should emit CSS");

    // Should contain :root default
    assert!(css.contains(":root {\n  --primary: #6366f1;\n}"));
    // Should contain [data-theme="dark"] override
    assert!(css.contains("[data-theme=\"dark\"] {\n  --primary: #a5b4fc;\n}"));
    // Should contain @media (prefers-color-scheme: dark)
    assert!(css.contains("@media (prefers-color-scheme: dark) {"));
    assert!(css.contains(":root:not([data-theme]) {"));
}

#[test]
fn emit_design_css_without_themes_unchanged() {
    // Token with no theme_values — should only emit :root block
    let mut tokens = FxHashMap::default();
    tokens.insert(
        "radiusMd".to_string(),
        TokenIR {
            value: TokenValueIR::Static("8px".to_string()),
            token_type: TokenTypeIR::Size,
            theme_values: FxHashMap::default(),
            source_span: None,
        },
    );

    let design = DesignIR {
        name: "Simple".to_string(),
        themes: vec![],
        default_theme: None,
        tokens,
        classes: vec![],
        variants: vec![],
        source_span: None,
    };

    let css = emit_design_css(&design, "Simple").expect("should emit CSS");
    assert!(css.contains(":root {\n  --radiusMd: 8px;\n}"));
    assert!(
        !css.contains("[data-theme="),
        "no theme blocks when no themes declared"
    );
    assert!(!css.contains("@media"), "no media query when no dark theme");
}
