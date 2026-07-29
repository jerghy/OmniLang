//! CSS codegen for `design` blocks.
//!
//! Converts `DesignIR` into CSS output:
//! - `:root { --token: value; }` declarations for design tokens
//! - Classes → `[data-design~="Name"] .className { prop: val; }` rules with pseudo-states
//! - Variants → `[data-design~="Name"] .variant-base { ... }`,
//!   `[data-design~="Name"] .variant-dim-value { ... }` utility rules
//!
//! These CSS blocks are intended to be concatenated into the Omniview output's
//! `<style>` block.  The `expand_variant` helper is used at component-bind time
//! to resolve which class names and additional CSS a variant binding produces.

use crate::codegen::{camel_to_kebab, sanitize_css_value};
use crate::ir::design::*;
use rustc_hash::FxHashMap;

/// Emit CSS for all tokens, classes, and variants in a design block.
///
/// The emitted CSS is scoped to `[data-design~="<scope>"]` so that multiple
/// design blocks on the same page do not conflict.  Token values are declared
/// on `:root` as CSS custom properties.
///
/// When `editor_mode` is true, the `[data-design~="..."]` scope prefix is
/// omitted so that the CSS works inside the editor canvas (which has no
/// data-design wrapper element).
///
/// Returns `Err(msg)` if the design has no name (unlikely, but defensive).
pub fn emit_design_css(design: &DesignIR, scope: &str) -> Result<String, String> {
    emit_design_css_impl(design, scope, false)
}

/// Like `emit_design_css` but with an `editor_mode` flag that strips
/// the `[data-design~="..."]` scope prefix for editor canvas compatibility.
pub fn emit_design_css_editor(design: &DesignIR, scope: &str) -> Result<String, String> {
    emit_design_css_impl(design, scope, true)
}

fn emit_design_css_impl(
    design: &DesignIR,
    scope: &str,
    editor_mode: bool,
) -> Result<String, String> {
    if design.name.is_empty() {
        return Err("design block has an empty name".to_string());
    }

    let scope_prefix = if editor_mode {
        String::new()
    } else {
        format!("[data-design~=\"{}\"] ", scope)
    };
    let mut css = String::new();

    // ── Emit token CSS custom property declarations ──
    if !design.tokens.is_empty() {
        let mut sorted: Vec<_> = design.tokens.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        // Build a map: theme_name → Vec<(token_name, value)>
        let mut theme_token_map: FxHashMap<&str, Vec<(&String, &String)>> = FxHashMap::default();
        for (name, token) in &sorted {
            for (theme_name, theme_val) in &token.theme_values {
                theme_token_map
                    .entry(theme_name.as_str())
                    .or_default()
                    .push((name, theme_val));
            }
        }

        // 1. Default theme (or first listed value) on :root — always emitted
        css.push_str(":root {\n");
        for (name, token) in &sorted {
            css.push_str(&format!("  --{}: {};\n", name, token.value));
        }
        css.push_str("}\n\n");

        // 2. Per-theme overrides: [data-theme="themeName"] { --token: value; }
        if !theme_token_map.is_empty() {
            let mut theme_names: Vec<&str> = theme_token_map.keys().copied().collect();
            theme_names.sort();
            for theme_name in &theme_names {
                if let Some(pairs) = theme_token_map.get(theme_name) {
                    css.push_str(&format!("[data-theme=\"{}\"] {{\n", theme_name));
                    for (name, val) in pairs {
                        css.push_str(&format!("  --{}: {};\n", name, val));
                    }
                    css.push_str("}\n\n");
                }
            }
        }

        // 3. @media (prefers-color-scheme: dark) auto-detection
        // Use the dark theme values (if declared), scoped to :root:not([data-theme])
        // so it doesn't override explicit user choice
        if let Some(dark_pairs) = theme_token_map.get("dark") {
            css.push_str("@media (prefers-color-scheme: dark) {\n");
            css.push_str("  :root:not([data-theme]) {\n");
            for (name, val) in dark_pairs {
                css.push_str(&format!("    --{}: {};\n", name, val));
            }
            css.push_str("  }\n");
            css.push_str("}\n\n");
        }
    }

    // ── Emit class rules ──
    for class in &design.classes {
        css.push_str(&emit_class_css(class, &scope_prefix));
    }

    // ── Emit variant rules ──
    for variant in &design.variants {
        css.push_str(&emit_variant_css(variant, &scope_prefix));
    }

    Ok(css)
}

/// Emit CSS for a single class: `{scope}.name { prop: val; }` plus pseudo-states.
fn emit_class_css(class: &ClassIR, scope_prefix: &str) -> String {
    let mut css = String::new();

    if !class.properties.is_empty() {
        css.push_str(&format!("{}.{} {{\n", scope_prefix, class.name));
        for (prop, val) in &class.properties {
            let sanitized = sanitize_css_value(val);
            css.push_str(&format!("  {}: {};\n", camel_to_kebab(prop), sanitized));
        }
        css.push_str("}\n\n");
    }

    for ps in &class.pseudo_states {
        if !ps.properties.is_empty() {
            // Pseudo-classes use `:hover` syntax, pseudo-elements use `::before`
            let pseudo_selector = if ps.pseudo.starts_with("::") {
                format!("{}.{}{} {{\n", scope_prefix, class.name, ps.pseudo)
            } else {
                format!("{}.{}:{} {{\n", scope_prefix, class.name, ps.pseudo)
            };
            css.push_str(&pseudo_selector);
            for (prop, val) in &ps.properties {
                css.push_str(&format!("  {}: {};\n", camel_to_kebab(prop), val));
            }
            css.push_str("}\n\n");
        }
    }

    css
}

/// Emit CSS for variant base and branch rules.
fn emit_variant_css(variant: &VariantIR, scope_prefix: &str) -> String {
    let mut css = String::new();

    // Base rule: `{scope}.variant-name { ... }`
    if !variant.base.is_empty() {
        css.push_str(&format!("{}.{} {{\n", scope_prefix, variant.name));
        for (prop, val) in &variant.base {
            let sanitized = sanitize_css_value(val);
            css.push_str(&format!("  {}: {};\n", camel_to_kebab(prop), sanitized));
        }
        css.push_str("}\n\n");
    }

    // Branch rules: `{scope}.variant-name.dim-value { ... }`
    for branch in &variant.branches {
        if !branch.properties.is_empty() {
            css.push_str(&format!(
                "{}.{}.{} {{\n",
                scope_prefix, variant.name, branch.value
            ));
            for (prop, val) in &branch.properties {
                let sanitized = sanitize_css_value(val);
                css.push_str(&format!("  {}: {};\n", camel_to_kebab(prop), sanitized));
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
        .ok_or_else(|| {
            format!(
                "variant '{}' not found in design '{}'",
                variant_name, design.name
            )
        })?;

    let mut classes = Vec::new();
    classes.push(variant_name.to_string());

    // Add dimension values that match declared dimensions,
    // falling back to defaults for any missing dimensions
    for dim in &variant.dimensions {
        let val = dimension_values
            .get(&dim.name)
            .or_else(|| variant.defaults.get(&dim.name));
        if let Some(val) = val {
            if dim.values.contains(val) {
                classes.push(val.clone());
            }
        }
    }

    // Additional CSS is already handled by emit_design_css, so return empty
    Ok((classes, String::new()))
}

/// Look up a class by name and return its resolved CSS properties.
/// Returns `None` if the class is not found in the given design.
pub fn resolve_class_properties<'a>(
    design: &'a DesignIR,
    class_name: &str,
) -> Option<&'a FxHashMap<String, String>> {
    design
        .classes
        .iter()
        .find(|c| c.name == class_name)
        .map(|c| &c.properties)
}

/// Look up a class by name and return its pseudo-state properties.
/// Returns an empty slice if the class is not found or has no pseudo-states.
pub fn resolve_class_pseudo_states<'a>(
    design: &'a DesignIR,
    class_name: &str,
) -> &'a [PseudoStateIR] {
    design
        .classes
        .iter()
        .find(|c| c.name == class_name)
        .map(|c| c.pseudo_states.as_slice())
        .unwrap_or(&[])
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

                theme_values: FxHashMap::default(),
                source_span: None,
            },
        );
        tokens.insert(
            "primary".to_string(),
            TokenIR {
                value: "#6366f1".to_string(),
                token_type: TokenTypeIR::Color,

                theme_values: FxHashMap::default(),
                source_span: None,
            },
        );
        tokens.insert(
            "radiusMd".to_string(),
            TokenIR {
                value: "8px".to_string(),
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
        theme_values_dark.insert("dark".to_string(), "#a5b4fc".to_string());
        tokens.insert(
            "primary".to_string(),
            TokenIR {
                value: "#6366f1".to_string(),
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
                value: "8px".to_string(),
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
}
