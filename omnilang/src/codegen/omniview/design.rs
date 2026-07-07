//! CSS codegen for `design` blocks.
//!
//! Converts `DesignIR` into CSS output:
//! - Classes → `.className { prop: val; }` rules with pseudo-states
//! - Variants → `.variant-base { ... }`, `.variant-dim-value { ... }` utility rules
//!
//! These CSS blocks are intended to be concatenated into the Omniview output's
//! `<style>` block.  The `expand_variant` helper is used at component-bind time
//! to resolve which class names and additional CSS a variant binding produces.

use crate::ir::design::*;
use rustc_hash::FxHashMap;
/// Emit CSS for all classes and variants in a design block.
///
/// Returns a single CSS string containing:
/// 1. Class rules: `.className { prop: val; }\n.className:pseudo { ... }`
/// 2. Variant base rules: `.variant-base { ... }`
/// 3. Variant branch rules: `.variant-name.dim-value { ... }`
///
/// Returns `Err(msg)` if the design has no name (unlikely, but defensive).
pub fn emit_design_css(design: &DesignIR) -> Result<String, String> {
    if design.name.is_empty() {
        return Err("design block has an empty name".to_string());
    }

    let mut css = String::new();

    // ── Emit class rules ──
    for class in &design.classes {
        css.push_str(&emit_class_css(class));
    }

    // ── Emit variant rules ──
    for variant in &design.variants {
        css.push_str(&emit_variant_css(variant));
    }

    Ok(css)
}

/// Emit CSS for a single class: `.name { prop: val; }` plus pseudo-states.
fn emit_class_css(class: &ClassIR) -> String {
    let mut css = String::new();

    if !class.properties.is_empty() {
        css.push_str(&format!(".{} {{\n", class.name));
        for (prop, val) in &class.properties {
            css.push_str(&format!("\t{}: {};\n", prop, val));
        }
        css.push_str("}\n\n");
    }

    for ps in &class.pseudo_states {
        if !ps.properties.is_empty() {
            css.push_str(&format!(".{}:{} {{\n", class.name, ps.pseudo));
            for (prop, val) in &ps.properties {
                css.push_str(&format!("\t{}: {};\n", prop, val));
            }
            css.push_str("}\n\n");
        }
    }

    css
}

/// Emit CSS for variant base and branch rules.
fn emit_variant_css(variant: &VariantIR) -> String {
    let mut css = String::new();

    // Base rule: `.variant-name { ... }`
    if !variant.base.is_empty() {
        css.push_str(&format!(".{} {{\n", variant.name));
        for (prop, val) in &variant.base {
            css.push_str(&format!("\t{}: {};\n", prop, val));
        }
        css.push_str("}\n\n");
    }

    // Branch rules: `.variant-name.dim-value { ... }`
    for branch in &variant.branches {
        if !branch.properties.is_empty() {
            css.push_str(&format!(".{}.{} {{\n", variant.name, branch.value));
            for (prop, val) in &branch.properties {
                css.push_str(&format!("\t{}: {};\n", prop, val));
            }
            css.push_str("}\n\n");
        }
    }

    css
}

/// Expand a variant binding for use in component CSS.
///
/// Given a variant name and a set of dimension→value bindings, returns:
/// - `classes`: CSS class names to apply to the element
/// - `css`: any additional CSS required (e.g. for value-specific overrides)
///
/// For example, with `variant Button` and `{ "color": "primary", "size": "sm" }`:
/// - classes: `["Button", "primary", "sm"]`
/// - css: `""` (already emitted via `emit_design_css`)
///
/// Returns an error if the variant name is not found in the design.
pub fn expand_variant(
    design: &DesignIR,
    variant_name: &str,
    dimension_values: &FxHashMap<String, String>,
) -> Result<(Vec<String>, String), String> {
    let variant = design
        .variants
        .iter()
        .find(|v| v.name == variant_name)
        .ok_or_else(|| format!("variant '{}' not found in design '{}'", variant_name, design.name))?;

    let mut classes = Vec::new();
    classes.push(variant_name.to_string());

    // Add dimension values that match declared dimensions
    for dim in &variant.dimensions {
        if let Some(val) = dimension_values.get(&dim.name) {
            if dim.values.contains(val) {
                classes.push(val.clone());
            }
        }
    }

    // Additional CSS is already handled by emit_design_css, so return empty
    Ok((classes, String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_design() -> DesignIR {
        let mut tokens = FxHashMap::default();
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
                properties: vec![("box-shadow".to_string(), "0 4px 12px rgba(0,0,0,0.08)".to_string())]
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

        assert!(css.contains(".Card {"), "should contain .Card class rule");
        assert!(
            css.contains("background: #ffffff;"),
            "should contain resolved property"
        );
        assert!(css.contains(".Card:hover {"), "should contain hover pseudo-state");
        assert!(
            css.contains("box-shadow: 0 4px 12px rgba(0,0,0,0.08);"),
            "should contain pseudo-state property"
        );
    }

    #[test]
    fn emit_variant_css_output() {
        let design = make_test_design();
        let css = emit_design_css(&design).expect("should emit CSS");

        assert!(css.contains(".Button {"), "should contain variant base rule");
        assert!(
            css.contains("display: inline-flex;"),
            "should contain variant base property"
        );
        assert!(
            css.contains(".Button.primary {"),
            "should contain variant branch rule"
        );
        assert!(
            css.contains("background: #6366f1;"),
            "should contain branch property"
        );
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

        assert!(classes.contains(&"Button".to_string()), "should include variant name");
        assert!(classes.contains(&"primary".to_string()), "should include dimension value");
        assert!(classes.contains(&"sm".to_string()), "should include dimension value");
        assert!(extra_css.is_empty(), "no additional CSS needed");
    }

    #[test]
    fn expand_variant_not_found() {
        let design = make_test_design();
        let dim_values = FxHashMap::default();
        let result = expand_variant(&design, "NonExistent", &dim_values);
        assert!(result.is_err(), "should error on unknown variant");
    }

    #[test]
    fn emit_empty_design() {
        let design = DesignIR {
            name: "Empty".to_string(),
            tokens: FxHashMap::default(),
            classes: vec![],
            variants: vec![],
        };
        let css = emit_design_css(&design).expect("should emit empty CSS");
        assert!(css.is_empty(), "empty design should produce no CSS");
    }
}
