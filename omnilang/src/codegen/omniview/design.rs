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
use crate::ir::ExprIR;
use rustc_hash::FxHashMap;
use std::fmt::Write;

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
                let _ = writeln!(css, "  --{}: {};", name, s);
            }
            // Dynamic values are skipped here; they are emitted as runtime
            // CSS custom property assignments via __omni_effect (Task 6).
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
                        let _ = writeln!(css, "  --{}: {};", name, val);
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
                let _ = writeln!(css, "    --{}: {};", name, val);
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
                let _ = writeln!(css, "  {}: {};", camel_to_kebab(prop), val);
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

/// Emit a single JS `__omni_effect` block for all dynamic design tokens.
///
/// Takes a pre-collected slice of `(token_name, ExprIR)` pairs and a
/// function that converts each `ExprIR` into its JavaScript expression string.
///
/// Generates a single effect containing all `setProperty` calls:
/// ```javascript
/// __omni_effect(() => {
///   const root = document.documentElement;
///   root.style.setProperty("--primary", __omni_sanitizeCss(expr1));
///   root.style.setProperty("--accent", __omni_sanitizeCss(expr2));
/// });
/// ```
///
/// This sets CSS custom properties on `<html>` at runtime, giving them
/// the highest CSS specificity (inline style on root element).
pub fn emit_dynamic_token_effects<F>(
    tokens: &[(String, ExprIR)],
    mut expr_to_js: F,
) -> String
where
    F: FnMut(&ExprIR) -> String,
{
    if tokens.is_empty() {
        return String::new();
    }
    let mut js = String::new();
    js.push_str("__omni_effect(() => {\n");
    js.push_str("  const root = document.documentElement;\n");
    for (name, expr) in tokens {
        let js_expr = expr_to_js(expr);
        let _ = writeln!(
            js,
            "  root.style.setProperty(\"--{}\", __omni_sanitizeCss({}));",
            name, js_expr
        );
    }
    js.push_str("\n});\n");
    js
}

// Tests moved to tests_regression/design.rs
