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
                TokenType::Font => TokenTypeIR::Font,
                TokenType::Shadow => TokenTypeIR::Shadow,
            };
            (
                t.name.clone(),
                TokenIR {
                    value: t.value.clone(),
                    token_type,
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
        tokens,
        classes,
        variants,
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

/// Resolve a token name: look up in the token map, or return `var(--name)` fallback.
fn resolve_token_ref(name: &str, tokens: &FxHashMap<String, TokenIR>) -> String {
    tokens
        .get(name)
        .map(|t| t.value.clone())
        .unwrap_or_else(|| format!("var(--{})", name))
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
        })
        .collect();

    ClassIR {
        name: class.name.clone(),
        properties,
        pseudo_states,
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
        })
        .collect();

    VariantIR {
        name: variant.name.clone(),
        dimensions,
        defaults,
        base,
        branches,
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
        let module = program.modules.get("input.omni").expect("should have module");
        let ast = module
            .designs
            .first()
            .expect("should have at least one design");
        lower_design(ast)
    }

    #[test]
    fn lower_token_collects_values() {
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

        let primary = design.tokens.get("primary").unwrap();
        assert_eq!(primary.value, "#6366f1");
        assert_eq!(primary.token_type, TokenTypeIR::Color);

        let spacing = design.tokens.get("spacingMd").unwrap();
        assert_eq!(spacing.value, "16px");
        assert_eq!(spacing.token_type, TokenTypeIR::Size);
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
            Some(&"#ffffff".to_string())
        );
        assert_eq!(
            card.properties.get("border-radius"),
            Some(&"8px".to_string())
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
