//! AST → IR lowering for `design` blocks.
//!
//! Walks a parsed `DesignDef` AST and produces a `DesignIR` with:
//! - Token collection into a `FxHashMap<String, TokenIR>` for O(1) lookup
//! - Token reference resolution: `token(name)` → resolved value (or `var(--name)` fallback)
//! - Class property resolution (token references replaced with values)
//! - Variant lowering: base properties + branch properties with token resolution

use crate::ir::design::*;
use crate::ir::lower::expr::lower_expr;
use crate::parser::ast::design::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;

/// Lower a parsed `design` block AST into a `DesignIR`,
/// resolving `TokenValue::Variable` references using the compile-time
/// constant value map. Falls back to `var(--name)` if unresolved.
pub fn lower_design_with_consts(
    ast: &DesignDef,
    const_values: &FxHashMap<String, String>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> DesignIR {
    let tokens: FxHashMap<String, TokenIR> = ast
        .tokens
        .iter()
        .map(|t| {
            let token_type = match t.token_type {
                TokenType::Color => TokenTypeIR::Color,
                TokenType::Size => TokenTypeIR::Size,
                TokenType::Radius => TokenTypeIR::Radius,
                TokenType::FontFamily => TokenTypeIR::FontFamily,
                TokenType::FontSize => TokenTypeIR::FontSize,
                TokenType::FontWeight => TokenTypeIR::FontWeight,
                TokenType::LineHeight => TokenTypeIR::LineHeight,
                TokenType::LetterSpacing => TokenTypeIR::LetterSpacing,
                TokenType::Shadow => TokenTypeIR::Shadow,
                TokenType::Duration => TokenTypeIR::Duration,
                TokenType::Easing => TokenTypeIR::Easing,
                TokenType::Opacity => TokenTypeIR::Opacity,
                TokenType::ZIndex => TokenTypeIR::ZIndex,
            };
            let resolved_value =
                resolve_token_value_ir(&t.value, const_values, variant_map, semantics);
            let theme_values: FxHashMap<String, TokenValueIR> = t
                .theme_values
                .iter()
                .map(|(theme, val)| {
                    (
                        theme.clone(),
                        resolve_token_value_ir(val, const_values, variant_map, semantics),
                    )
                })
                .collect();
            (
                t.name.clone(),
                TokenIR {
                    value: resolved_value,
                    token_type,
                    theme_values,
                    source_span: None,
                },
            )
        })
        .collect();

    let classes: Vec<ClassIR> = ast
        .classes
        .iter()
        .map(|c| lower_class(c, &tokens))
        .collect();
    let variants: Vec<VariantIR> = ast
        .variants
        .iter()
        .map(|v| lower_variant(v, &tokens))
        .collect();

    DesignIR {
        name: ast.name.clone(),
        themes: ast.themes.clone(),
        // Populated by the program-level validate_theme pass (cross-file
        // pairing with the @theme-marked store signal is only visible there).
        theme_signal: None,
        default_theme: ast.default_theme.clone(),
        tokens,
        classes,
        variants,
        source_file: None,
        source_span: Some(ast.span),
    }
}

/// Resolve a `TokenValue` (AST) to `TokenValueIR`, handling Phase 1 compile-time
/// constants and Phase 2 runtime expressions.
fn resolve_token_value_ir(
    value: &TokenValue,
    const_values: &FxHashMap<String, String>,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> TokenValueIR {
    match value {
        TokenValue::Literal(s) => TokenValueIR::Static(s.clone()),
        TokenValue::Variable(name) => {
            if let Some(resolved) = const_values.get(name) {
                TokenValueIR::Static(resolved.clone())
            } else {
                // Phase 1 fallback for unresolved variables: CSS var(--name)
                TokenValueIR::Static(format!("var(--{})", name))
            }
        }
        TokenValue::Expression(expr) => {
            // Phase 2: Lower the AST expression to ExprIR using the standard pipeline
            let expr_ir = lower_expr(expr, variant_map, None, semantics);
            TokenValueIR::Dynamic(expr_ir)
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Token resolution helpers
// ═══════════════════════════════════════════════════════════

/// Resolve a `DesignValue` to a CSS string.
///
/// - `Literal(s)` → the literal string unchanged
/// - `TokenRef(name)` → lookup `name` in `tokens`, return value or `var(--name)` fallback
/// - `Mixed(parts)` → concatenate resolved parts (literal text + token values)
fn resolve_value(value: &DesignValue, _tokens: &FxHashMap<String, TokenIR>) -> String {
    match value {
        DesignValue::Literal(s) => s.clone(),
        DesignValue::TokenRef(name) => resolve_token_ref(name),
        DesignValue::Mixed(parts) => {
            let mut out = String::new();
            for part in parts {
                match part {
                    DesignValuePart::Text(s) => out.push_str(s),
                    DesignValuePart::Token(name) => {
                        // Unquoted literal text ("1px solid") was trimmed at
                        // parse time — restore the space before an inline
                        // token() ref so the CSS compound reads
                        // `1px solid var(--x)` (quoted text keeps its own
                        // trailing space; a compound without the space
                        // fails css_border_to_dart and the border is dropped).
                        if !out.is_empty() && !out.ends_with(char::is_whitespace) {
                            out.push(' ');
                        }
                        out.push_str(&resolve_token_ref(name));
                    }
                }
            }
            out
        }
    }
}

/// Resolve a token name: always emit `var(--name)`.
///
/// Token values are emitted as CSS custom property declarations in `:root { }`
/// by `emit_design_css()`, so class/variant properties always reference via `var()`.
fn resolve_token_ref(name: &str) -> String {
    format!("var(--{})", name)
}

/// Convert a `Vec<DesignProperty>` into a `FxHashMap<String, String>` with resolution.
fn resolve_properties(
    props: &[DesignProperty],
    tokens: &FxHashMap<String, TokenIR>,
) -> FxHashMap<String, String> {
    let mut map = FxHashMap::with_capacity_and_hasher(props.len(), Default::default());
    for prop in props {
        map.insert(prop.name.clone(), resolve_value(&prop.value, tokens));
    }
    map
}

// ═══════════════════════════════════════════════════════════
// Class lowering
// ═══════════════════════════════════════════════════════════

fn lower_class(class: &DesignClassDef, tokens: &FxHashMap<String, TokenIR>) -> ClassIR {
    let properties = resolve_properties(&class.properties, tokens);
    let pseudo_states: Vec<PseudoStateIR> = class
        .pseudo_states
        .iter()
        .map(|ps| PseudoStateIR {
            pseudo: ps.pseudo.clone(),
            properties: resolve_properties(&ps.properties, tokens),
            source_span: None,
        })
        .collect();

    ClassIR {
        name: class.name.clone(),
        properties,
        pseudo_states,
        source_span: None,
    }
}

// ═══════════════════════════════════════════════════════════
// Variant lowering
// ═══════════════════════════════════════════════════════════

fn lower_variant(variant: &VariantDef, tokens: &FxHashMap<String, TokenIR>) -> VariantIR {
    let dimensions: Vec<VariantDimIR> = variant
        .dimensions
        .iter()
        .map(|d| VariantDimIR {
            name: d.name.clone(),
            values: d.values.clone(),
            source_span: None,
        })
        .collect();

    let defaults: FxHashMap<String, String> = variant.defaults.iter().cloned().collect();
    let base = resolve_properties(&variant.base, tokens);

    let branches: Vec<VariantBranchIR> = variant
        .branches
        .iter()
        .map(|b| VariantBranchIR {
            dimension: b.dimension.clone(),
            value: b.value.clone(),
            properties: resolve_properties(&b.properties, tokens),
            source_span: None,
        })
        .collect();

    VariantIR {
        name: variant.name.clone(),
        dimensions,
        defaults,
        base,
        branches,
        source_span: None,
    }
}

/// Validate a `DesignDef` AST for compile-time errors.
///
/// Checks:
/// - All `token(...)` references refer to tokens declared in the same design block.
/// - Variant branches reference valid dimensions.
///
/// Returns a list of human-readable error messages (empty = valid).
pub fn validate_design(ast: &DesignDef) -> Vec<String> {
    let mut errors = Vec::new();

    // Build the set of declared token names
    let token_names: std::collections::HashSet<&str> =
        ast.tokens.iter().map(|t| t.name.as_str()).collect();

    // Walk design values in classes
    for cls in &ast.classes {
        for prop in &cls.properties {
            check_value_tokens(
                &prop.value,
                &token_names,
                &format!("class '{}'", cls.name),
                &mut errors,
            );
        }
        for ps in &cls.pseudo_states {
            for prop in &ps.properties {
                check_value_tokens(
                    &prop.value,
                    &token_names,
                    &format!("class '{}' &:{}", cls.name, ps.pseudo),
                    &mut errors,
                );
            }
        }
    }

    // Walk design values in variants
    for variant in &ast.variants {
        let dim_names: std::collections::HashSet<&str> =
            variant.dimensions.iter().map(|d| d.name.as_str()).collect();

        for prop in &variant.base {
            check_value_tokens(
                &prop.value,
                &token_names,
                &format!("variant '{}' base", variant.name),
                &mut errors,
            );
        }
        for branch in &variant.branches {
            // Validate branch dimension exists
            if !dim_names.contains(branch.dimension.as_str()) {
                errors.push(format!(
                    "variant '{}' branch '{}.{}' references undeclared dimension '{}'",
                    variant.name, branch.dimension, branch.value, branch.dimension
                ));
            }
            for prop in &branch.properties {
                check_value_tokens(
                    &prop.value,
                    &token_names,
                    &format!(
                        "variant '{}' branch '{}.{}'",
                        variant.name, branch.dimension, branch.value
                    ),
                    &mut errors,
                );
            }
        }
    }

    // ── Check 3: Theme name validation ──
    if !ast.themes.is_empty() {
        let theme_names: std::collections::HashSet<&str> =
            ast.themes.iter().map(|t| t.as_str()).collect();

        // Validate per-token theme_values reference only declared themes
        for token in &ast.tokens {
            for (theme_name, _) in &token.theme_values {
                if !theme_names.contains(theme_name.as_str()) {
                    errors.push(format!(
                        "token '{}' references undeclared theme '{}'; declared themes are: [{}]",
                        token.name,
                        theme_name,
                        ast.themes.join(", ")
                    ));
                }
            }
        }

        // Validate @default-theme is one of the declared themes
        if let Some(ref default) = ast.default_theme {
            if !theme_names.contains(default.as_str()) {
                errors.push(format!(
                    "@default-theme '{}' is not in @themes declaration [{}]",
                    default,
                    ast.themes.join(", ")
                ));
            }
        }
    }

    errors
}

fn check_value_tokens(
    value: &DesignValue,
    token_names: &std::collections::HashSet<&str>,
    context: &str,
    errors: &mut Vec<String>,
) {
    match value {
        DesignValue::TokenRef(name) => {
            if !token_names.contains(name.as_str()) {
                errors.push(format!(
                    "undefined token '{}' referenced in {}",
                    name, context
                ));
            }
        }
        DesignValue::Mixed(parts) => {
            for part in parts {
                if let DesignValuePart::Token(name) = part {
                    if !token_names.contains(name.as_str()) {
                        errors.push(format!(
                            "undefined token '{}' referenced in {}",
                            name, context
                        ));
                    }
                }
            }
        }
        DesignValue::Literal(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::lexer::Span;
    use crate::parser::ast::Expr;
    use crate::parser::parse;

    /// Helper: parse a design block from source and lower it.
    fn lower_design_source(source: &str) -> DesignIR {
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let module = program
            .modules
            .get("input.omni")
            .expect("should have module");
        let ast = module
            .designs
            .first()
            .expect("should have at least one design");
        let const_values = FxHashMap::default(); // no consts in test helper
        let variant_map = FxHashMap::default();
        let mut semantics = SemanticResult::default();
        lower_design_with_consts(ast, &const_values, &variant_map, &mut semantics)
    }

    #[test]
    fn lower_token_collects_values() -> Result<(), String> {
        let design = lower_design_source(
            r##"
            design App {
                token primary: color = "#6366f1";
                token spacingMd: size = "16px";
            }
            "##,
        );
        assert_eq!(design.name, "App");
        assert_eq!(design.tokens.len(), 2);

        let primary = design
            .tokens
            .get("primary")
            .ok_or_else(|| "missing token: primary".to_string())?;
        assert!(matches!(primary.value, TokenValueIR::Static(ref s) if s == "#6366f1"));
        assert_eq!(primary.token_type, TokenTypeIR::Color);

        let spacing = design
            .tokens
            .get("spacingMd")
            .ok_or_else(|| "missing token: spacingMd".to_string())?;
        assert!(matches!(spacing.value, TokenValueIR::Static(ref s) if s == "16px"));
        assert_eq!(spacing.token_type, TokenTypeIR::Size);

        Ok(())
    }

    #[test]
    fn lower_class_resolves_token_refs() {
        let design = lower_design_source(
            r##"
            design App {
                token surface: color = "#ffffff";
                token radiusMd: size = "8px";

                class Card {
                    background: token(surface);
                    border-radius: token(radiusMd);
                }
            }
            "##,
        );
        assert_eq!(design.classes.len(), 1);

        let card = &design.classes[0];
        assert_eq!(card.name, "Card");
        assert_eq!(
            card.properties.get("background"),
            Some(&"var(--surface)".to_string())
        );
        assert_eq!(
            card.properties.get("border-radius"),
            Some(&"var(--radiusMd)".to_string())
        );
    }

    #[test]
    fn lower_design_with_themes() {
        let design_def = DesignDef {
            name: "App".to_string(),
            themes: vec!["light".to_string(), "dark".to_string()],
            default_theme: Some("light".to_string()),
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Literal("#6366f1".to_string()),
                theme_values: vec![(
                    "dark".to_string(),
                    TokenValue::Literal("#a5b4fc".to_string()),
                )],
            }],
            classes: vec![],
            variants: vec![],
        };

        let design = lower_design_with_consts(
            &design_def,
            &FxHashMap::default(),
            &FxHashMap::default(),
            &mut SemanticResult::default(),
        );
        assert_eq!(design.name, "App");
        assert_eq!(design.themes, vec!["light", "dark"]);
        assert_eq!(design.default_theme, Some("light".to_string()));

        let primary = design
            .tokens
            .get("primary")
            .expect("should have primary token");
        assert!(matches!(primary.value, TokenValueIR::Static(ref s) if s == "#6366f1"));
        assert!(
            matches!(primary.theme_values.get("dark"), Some(TokenValueIR::Static(ref s)) if s == "#a5b4fc"),
            "expected dark theme override to be \"#a5b4fc\""
        );
        assert!(
            !primary.theme_values.contains_key("light"),
            "no light override in theme_values"
        );
    }

    #[test]
    fn lower_token_ref_fallback() {
        let design = lower_design_source(
            r#"
            design App {
                class Card {
                    background: token(missing);
                }
            }
            "#,
        );
        let card = &design.classes[0];
        assert_eq!(
            card.properties.get("background"),
            Some(&"var(--missing)".to_string())
        );
    }

    // ── Theme validation tests ──

    #[test]
    fn validate_undeclared_theme_in_token() {
        let ast = DesignDef {
            name: "Test".to_string(),
            themes: vec!["light".to_string(), "dark".to_string()],
            default_theme: Some("light".to_string()),
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Literal("#fff".to_string()),
                theme_values: vec![("darkk".to_string(), TokenValue::Literal("#000".to_string()))],
            }],
            classes: vec![],
            variants: vec![],
        };
        let errors = validate_design(&ast);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].contains("darkk"),
            "should mention the bad theme name"
        );
        assert!(
            errors[0].contains("undeclared theme"),
            "should say undeclared theme"
        );
    }

    #[test]
    fn validate_undeclared_default_theme() {
        let ast = DesignDef {
            name: "Test".to_string(),
            themes: vec!["light".to_string(), "dark".to_string()],
            default_theme: Some("highContrast".to_string()),
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![],
            classes: vec![],
            variants: vec![],
        };
        let errors = validate_design(&ast);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].contains("highContrast"),
            "should mention bad default theme"
        );
        assert!(
            errors[0].contains("not in @themes"),
            "should say not in @themes"
        );
    }

    #[test]
    fn validate_valid_themes_no_errors() {
        let ast = DesignDef {
            name: "Test".to_string(),
            themes: vec!["light".to_string(), "dark".to_string()],
            default_theme: Some("light".to_string()),
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Literal("#fff".to_string()),
                theme_values: vec![("dark".to_string(), TokenValue::Literal("#000".to_string()))],
            }],
            classes: vec![],
            variants: vec![],
        };
        let errors = validate_design(&ast);
        assert!(errors.is_empty(), "should have no errors for valid themes");
    }

    #[test]
    fn validate_no_themes_skips_check() {
        let ast = DesignDef {
            name: "Test".to_string(),
            themes: vec![],
            default_theme: None,
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Literal("#fff".to_string()),
                theme_values: vec![("dark".to_string(), TokenValue::Literal("#000".to_string()))],
            }],
            classes: vec![],
            variants: vec![],
        };
        let errors = validate_design(&ast);
        // No @themes declared, so theme_values with undeclared themes are not validated
        assert!(errors.is_empty(), "no themes = skip theme validation");
    }

    #[test]
    fn lower_design_resolves_const_reference() {
        let mut const_values = FxHashMap::default();
        const_values.insert("BRAND".to_string(), "#6366f1".to_string());

        let design_def = DesignDef {
            name: "Test".to_string(),
            themes: vec![],
            default_theme: None,
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Variable("BRAND".to_string()),
                theme_values: vec![],
            }],
            classes: vec![],
            variants: vec![],
        };

        let variant_map = FxHashMap::default();
        let mut semantics = SemanticResult::default();
        let design =
            lower_design_with_consts(&design_def, &const_values, &variant_map, &mut semantics);
        let primary = design.tokens.get("primary").expect("should have primary");
        assert!(matches!(primary.value, TokenValueIR::Static(ref s) if s == "#6366f1"));
        assert!(primary.theme_values.is_empty());
    }

    #[test]
    fn lower_design_variable_fallback_to_var() {
        let const_values = FxHashMap::default(); // empty — nothing defined

        let design_def = DesignDef {
            name: "Test".to_string(),
            themes: vec![],
            default_theme: None,
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Variable("UNKNOWN".to_string()),
                theme_values: vec![],
            }],
            classes: vec![],
            variants: vec![],
        };

        let variant_map = FxHashMap::default();
        let mut semantics = SemanticResult::default();
        let design =
            lower_design_with_consts(&design_def, &const_values, &variant_map, &mut semantics);
        let primary = design.tokens.get("primary").expect("should have primary");
        assert!(matches!(primary.value, TokenValueIR::Static(ref s) if s == "var(--UNKNOWN)"));
    }

    #[test]
    fn lower_expression_token_value_to_dynamic() {
        let design_def = DesignDef {
            name: "Test".to_string(),
            themes: vec![],
            default_theme: None,
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: TokenValue::Expression(Expr::FieldAccess {
                    object: Box::new(Expr::VarRef(
                        "ThemeStore".to_string(),
                        Span {
                            start: 0,
                            end: 0,
                            line: 0,
                            col: 0,
                        },
                    )),
                    field: "primaryColor".to_string(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                }),
                theme_values: vec![],
            }],
            classes: vec![],
            variants: vec![],
        };

        let const_values = FxHashMap::default();
        let variant_map = FxHashMap::default();
        let mut semantics = SemanticResult::default();
        let design =
            lower_design_with_consts(&design_def, &const_values, &variant_map, &mut semantics);

        let primary = design.tokens.get("primary").expect("should have primary");
        assert!(matches!(primary.value, TokenValueIR::Dynamic(_)));
    }

    #[test]
    fn lower_static_token_still_works() {
        let design_def = DesignDef {
            name: "Test".to_string(),
            themes: vec![],
            default_theme: None,
            breakpoints: Vec::new(),

            span: Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
            tokens: vec![TokenDef {
                name: "spacing".to_string(),
                token_type: TokenType::Size,
                value: TokenValue::Literal("16px".to_string()),
                theme_values: vec![],
            }],
            classes: vec![],
            variants: vec![],
        };

        let const_values = FxHashMap::default();
        let variant_map = FxHashMap::default();
        let mut semantics = SemanticResult::default();
        let design =
            lower_design_with_consts(&design_def, &const_values, &variant_map, &mut semantics);

        let spacing = design.tokens.get("spacing").expect("should have spacing");
        assert!(matches!(spacing.value, TokenValueIR::Static(ref s) if s == "16px"));
    }
}
