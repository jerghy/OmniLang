//! CSS codegen for `design` blocks (OmniView backend).
//!
//! The target-agnostic CSS emission logic (tokens → `:root` custom
//! properties, themes → `[data-theme]` overrides, classes → `.Class`
//! rules, variants → `.variant-base` + `.dim-value` rules) lives in
//! `crate::codegen::shared::design` and is shared with future backends
//! (React, Flutter).  This module adds the OmniView-specific parts:
//!
//! - The `[data-design~="<scope>"]` selector prefix scoping design rules
//!   to a design's root wrapper element
//! - `emit_design_css_editor` — editor-canvas variant that omits the
//!   scope prefix (the canvas has no data-design wrapper element)
//! - `emit_dynamic_token_effects` — runtime JS (`__omni_effect` +
//!   `setProperty`) assigning dynamic token values on `<html>`

use crate::codegen::shared::design as shared_design;
use crate::ir::design::DesignIR;
use crate::ir::ExprIR;
use std::fmt::Write;

pub use crate::codegen::shared::design::{
    expand_variant, resolve_class_properties, resolve_class_pseudo_states,
};

/// Emit CSS for all tokens, classes, and variants in a design block.
///
/// The emitted CSS is scoped to `[data-design~="<scope>"]` so that
/// multiple design blocks on the same page do not conflict.  Token
/// values are declared on `:root` as CSS custom properties.  Dynamic
/// token values are skipped — `emit_dynamic_token_effects` assigns
/// them at runtime via JS.
///
/// Returns `Err(msg)` if the design has no name (unlikely, but defensive).
pub fn emit_design_css(design: &DesignIR, scope: &str) -> Result<String, String> {
    let scope_prefix = format!("[data-design~=\"{}\"] ", scope);
    shared_design::emit_design_css(design, &scope_prefix)
}

/// Like `emit_design_css` but omits the `[data-design~="..."]` scope
/// prefix for editor canvas compatibility (the canvas has no data-design
/// wrapper element).  `scope` is accepted for call-site symmetry but is
/// unused.
pub fn emit_design_css_editor(design: &DesignIR, _scope: &str) -> Result<String, String> {
    shared_design::emit_design_css(design, "")
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
pub fn emit_dynamic_token_effects<F>(tokens: &[(String, ExprIR)], mut expr_to_js: F) -> String
where
    F: FnMut(&ExprIR) -> String,
{
    if tokens.is_empty() {
        return String::new();
    }
    let mut js = String::new();
    js.push_str("__omni_effect(() => {\n");
    js.push_str("  const root = document.documentElement;\n");
    js.push_str("  if (!root) return;\n");
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

// Unit tests for the shared emission logic live in
// `crate::codegen::shared::design::tests` and `tests_regression/design.rs`.
