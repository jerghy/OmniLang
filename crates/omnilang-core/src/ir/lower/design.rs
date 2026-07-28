//! AST → IR lowering for `design` blocks.
//!
//! Walks a parsed `DesignDef` AST and produces a `DesignIR` with:
//! - Token collection into a `FxHashMap<String, TokenIR>` for O(1) lookup
//! - Token reference resolution: `token(name)` → resolved value (or `var(--name)` fallback)
//! - Class property resolution (token references replaced with values)
//! - Variant lowering: base properties + branch properties with token resolution

use crate::ir::design::*;
use crate::parser::ast::design::*;
use rustc_hash::FxHashMap;
/// Lower a parsed `design` block AST into a `DesignIR`.
///
/// This function performs:
/// 1. Token collection — maps token names to their `TokenIR` values
/// 2. Class lowering — resolves `token(...)` references in class properties
/// 3. Variant lowering — resolves `token(...)` references in variant properties
pub fn lower_design(ast: &DesignDef) -> DesignIR {
    // ── Step 1: Collect tokens ──
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
            // Convert per-theme value overrides from Vec<(String, String)> to FxHashMap
            let theme_values: FxHashMap<String, String> = t.theme_values.iter().cloned().collect();
            (
                t.name.clone(),
                TokenIR {
                    value: t.value.clone(),
                    token_type,
                    theme_values,
                    source_span: None,
                },
            )
        })
        .collect();

    // ── Step 2: Lower classes ──
    let classes: Vec<ClassIR> = ast
        .classes
        .iter()
        .map(|c| lower_class(c, &tokens))
        .collect();

    // ── Step 3: Lower variants ──
    let variants: Vec<VariantIR> = ast
        .variants
        .iter()
        .map(|v| lower_variant(v, &tokens))
        .collect();

    DesignIR {
        name: ast.name.clone(),
        themes: ast.themes.clone(),
        default_theme: ast.default_theme.clone(),
        tokens,
        classes,
        variants,
        source_span: None,
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
fn resolve_value(value: &DesignValue, tokens: &FxHashMap<String, TokenIR>) -> String {
    match value {
        DesignValue::Literal(s) => s.clone(),
        DesignValue::TokenRef(name) => resolve_token_ref(name, tokens),
        DesignValue::Mixed(parts) => parts
            .iter()
            .map(|part| match part {
                DesignValuePart::Text(s) => s.clone(),
                DesignValuePart::Token(name) => resolve_token_ref(name, tokens),
            })
            .collect(),
    }
}

/// Resolve a token name: always emit `var(--name)`.
///
/// Token values are emitted as CSS custom property declarations in `:root { }`
/// by `emit_design_css()`, so class/variant properties always reference via `var()`.
fn resolve_token_ref(name: &str, _tokens: &FxHashMap<String, TokenIR>) -> String {
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
    use crate::parser::parse;

    /// Helper: parse a design block from source and lower it.
    fn lower_design_source(source: &str) -> DesignIR {
        let tokens = tokenize(source);
        let program = parse(tokens, source).expect("parse should succeed");
        let module = program
            .modules
            .get("input.omni")
            .expect("should have module");
        let ast = module
            .designs
            .first()
            .expect("should have at least one design");
        lower_design(ast)
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
        assert_eq!(primary.value, "#6366f1");
        assert_eq!(primary.token_type, TokenTypeIR::Color);

        let spacing = design
            .tokens
            .get("spacingMd")
            .ok_or_else(|| "missing token: spacingMd".to_string())?;
        assert_eq!(spacing.value, "16px");
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
            tokens: vec![TokenDef {
                name: "primary".to_string(),
                token_type: TokenType::Color,
                value: "#6366f1".to_string(),
                theme_values: vec![("dark".to_string(), "#a5b4fc".to_string())],
            }],
            classes: vec![],
            variants: vec![],
        };

        let design = lower_design(&design_def);
        assert_eq!(design.name, "App");
        assert_eq!(design.themes, vec!["light", "dark"]);
        assert_eq!(design.default_theme, Some("light".to_string()));

        let primary = design
            .tokens
            .get("primary")
            .expect("should have primary token");
        assert_eq!(primary.value, "#6366f1");
        assert_eq!(
            primary.theme_values.get("dark"),
            Some(&"#a5b4fc".to_string())
        );
        assert!(
            primary.theme_values.get("light").is_none(),
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
}
