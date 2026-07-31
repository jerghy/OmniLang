//! Target-agnostic design CSS emission, shared across backend families.
//!
//! Converts `DesignIR` into CSS output:
//! - `:root { --token: value; }` declarations for design tokens
//! - Classes → `{scope}.className { prop: val; }` rules with pseudo-states
//! - Variants → `{scope}.variant-base { ... }`,
//!   `{scope}.variant-dim-value { ... }` utility rules
//!
//! The `scope_prefix` argument is prepended to class/variant rules so each
//! backend can scope design CSS however it needs (e.g. OmniView uses
//! `[data-design~="Name"] `); pass `""` for unscoped output.
//!
//! These CSS blocks are intended to be concatenated into the backend's
//! `<style>` block.  The `expand_variant` helper is used at component-bind
//! time to resolve which class names and additional CSS a variant binding
//! produces.

use crate::codegen::{camel_to_kebab, sanitize_css_value};
use crate::ir::design::*;
use rustc_hash::FxHashMap;
use std::fmt::Write;

/// Emit CSS for all tokens, classes, and variants in a design block.
///
/// Token values are declared on `:root` as CSS custom properties, with
/// per-theme overrides under `[data-theme="..."]` and a
/// `prefers-color-scheme: dark` media query when a "dark" theme is
/// declared.  `scope_prefix` is prepended to each class/variant rule so
/// multiple design blocks on the same page do not conflict.
///
/// Dynamic token values (`TokenValueIR::Dynamic`) are skipped — the
/// backend is responsible for runtime assignment (e.g. JS `setProperty`).
///
/// Returns `Err(msg)` if the design has no name (unlikely, but defensive).
pub fn emit_design_css(design: &DesignIR, scope_prefix: &str) -> Result<String, String> {
    if design.name.is_empty() {
        return Err("design block has an empty name".to_string());
    }

    let mut css = String::new();

    // ── Emit token CSS custom property declarations ──
    if !design.tokens.is_empty() {
        let mut sorted: Vec<_> = design.tokens.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        // Build a map: theme_name → Vec<(token_name, value)>
        // Only Static values go into CSS (Dynamic values use runtime codegen)
        let mut theme_token_map: FxHashMap<&str, Vec<(&String, &String)>> = FxHashMap::default();
        for (name, token) in &sorted {
            for (theme_name, theme_val) in &token.theme_values {
                if let TokenValueIR::Static(s) = theme_val {
                    theme_token_map
                        .entry(theme_name.as_str())
                        .or_default()
                        .push((name, s));
                }
            }
        }

        // 1. Default theme (or first listed value) on :root — always emitted
        css.push_str(":root {\n");
        for (name, token) in &sorted {
            if let TokenValueIR::Static(s) = &token.value {
                let sanitized = sanitize_css_value(s);
                let _ = writeln!(css, "  --{}: {};", name, sanitized);
            }
            // Dynamic values are skipped here; the backend emits runtime
            // CSS custom property assignments (e.g. JS setProperty).
        }
        css.push_str("}\n\n");

        // 2. Per-theme overrides: [data-theme="themeName"] { --token: value; }
        if !theme_token_map.is_empty() {
            let mut theme_names: Vec<&str> = theme_token_map.keys().copied().collect();
            theme_names.sort();
            for theme_name in &theme_names {
                if let Some(pairs) = theme_token_map.get(theme_name) {
                    let _ = writeln!(css, "[data-theme=\"{}\"] {{", theme_name);
                    for (name, val) in pairs {
                        let sanitized = sanitize_css_value(val);
                        let _ = writeln!(css, "  --{}: {};", name, sanitized);
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
                let sanitized = sanitize_css_value(val);
                let _ = writeln!(css, "    --{}: {};", name, sanitized);
            }
            css.push_str("  }\n");
            css.push_str("}\n\n");
        }
    }

    // ── Emit class rules ──
    for class in &design.classes {
        css.push_str(&emit_class_css(class, scope_prefix));
    }

    // ── Emit variant rules ──
    for variant in &design.variants {
        css.push_str(&emit_variant_css(variant, scope_prefix));
    }

    Ok(css)
}

/// Emit CSS for a single class: `{scope}.name { prop: val; }` plus pseudo-states.
fn emit_class_css(class: &ClassIR, scope_prefix: &str) -> String {
    let mut css = String::new();

    if !class.properties.is_empty() {
        let _ = writeln!(css, "{}.{} {{", scope_prefix, class.name);
        for (prop, val) in &class.properties {
            let sanitized = sanitize_css_value(val);
            let _ = writeln!(css, "  {}: {};", camel_to_kebab(prop), sanitized);
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
                let sanitized = sanitize_css_value(val);
                let _ = writeln!(css, "  {}: {};", camel_to_kebab(prop), sanitized);
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
        let _ = writeln!(css, "{}.{} {{", scope_prefix, variant.name);
        for (prop, val) in &variant.base {
            let sanitized = sanitize_css_value(val);
            let _ = writeln!(css, "  {}: {};", camel_to_kebab(prop), sanitized);
        }
        css.push_str("}\n\n");
    }

    // Branch rules: `{scope}.variant-name.dim-value { ... }`
    for branch in &variant.branches {
        if !branch.properties.is_empty() {
            let _ = writeln!(css, "{}.{}.{} {{", scope_prefix, variant.name, branch.value);
            for (prop, val) in &branch.properties {
                let sanitized = sanitize_css_value(val);
                let _ = writeln!(css, "  {}: {};", camel_to_kebab(prop), sanitized);
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
    use crate::ir::ExprIR;

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
            "dynamicAccent".to_string(),
            TokenIR {
                value: TokenValueIR::Dynamic(ExprIR::ident("ThemeStore_accent")),
                token_type: TokenTypeIR::Color,
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
            dimensions: vec![VariantDimIR {
                name: "color".to_string(),
                values: vec!["primary".to_string(), "danger".to_string()],
                source_span: None,
            }],
            defaults: vec![("color".to_string(), "primary".to_string())]
                .into_iter()
                .collect(),
            base: vec![("display".to_string(), "inline-flex".to_string())]
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
    fn emits_scoped_rules_with_custom_prefix() {
        let css = emit_design_css(&make_test_design(), "[data-design~=\"AppDesign\"] ")
            .expect("should emit CSS");

        assert!(css.contains("[data-design~=\"AppDesign\"] .Card {"));
        assert!(
            css.contains("[data-design~=\"AppDesign\"] .Card:hover {"),
            "pseudo-states should be scoped too"
        );
        assert!(css.contains("[data-design~=\"AppDesign\"] .Button {"));
        assert!(css.contains("[data-design~=\"AppDesign\"] .Button.primary {"));
        assert!(css.contains("--surface: #ffffff;"));
    }

    #[test]
    fn emits_unscoped_rules_with_empty_prefix() {
        let css = emit_design_css(&make_test_design(), "").expect("should emit CSS");

        assert!(css.contains("\n.Card {"));
        assert!(css.contains("\n.Button.primary {"));
        assert!(
            !css.contains("[data-design~="),
            "empty scope prefix must not leak any scoping"
        );
    }

    #[test]
    fn skips_dynamic_tokens() {
        let css = emit_design_css(&make_test_design(), "").expect("should emit CSS");

        assert!(
            !css.contains("--dynamicAccent"),
            "dynamic tokens must not appear in CSS\nGot:\n{}",
            css
        );
    }

    #[test]
    fn empty_design_produces_no_css() {
        let design = DesignIR {
            name: "Empty".to_string(),
            themes: vec![],
            default_theme: None,
            tokens: FxHashMap::default(),
            classes: vec![],
            variants: vec![],
            source_span: None,
        };
        let css = emit_design_css(&design, "").expect("should emit CSS");
        assert!(css.is_empty(), "empty design should produce no CSS");
    }

    #[test]
    fn expand_variant_applies_defaults() {
        let design = make_test_design();
        // Only supply "color", leaving "size" (no dim here) defaults path intact
        let dim_values = vec![("color".to_string(), "primary".to_string())]
            .into_iter()
            .collect();

        let (classes, _) = expand_variant(&design, "Button", &dim_values).expect("should expand");

        assert!(classes.contains(&"Button".to_string()));
        assert!(classes.contains(&"primary".to_string()));
    }

    #[test]
    fn expand_variant_not_found() {
        let design = make_test_design();
        let result = expand_variant(&design, "NonExistent", &FxHashMap::default());
        assert!(result.is_err(), "should error on unknown variant");
    }

    #[test]
    fn resolve_class_helpers() {
        let design = make_test_design();

        let props = resolve_class_properties(&design, "Card").expect("should find Card");
        assert_eq!(props.get("background").map(String::as_str), Some("#ffffff"));

        let pseudo = resolve_class_pseudo_states(&design, "Card");
        assert_eq!(pseudo.len(), 1);
        assert_eq!(pseudo[0].pseudo, "hover");

        assert!(resolve_class_properties(&design, "Nope").is_none());
        assert!(resolve_class_pseudo_states(&design, "Nope").is_empty());
    }
}
