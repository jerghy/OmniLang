//! Conditional rendering: css_only (canvas preview) mode evaluation.
//!
//! When `css_only` is active the codegen evaluates `@if` / `@else if` branches
//! statically against debug-patched signal default values so the Poiesis editor
//! canvas only renders the active branch.
//!
//! The live-mode counterpart `emit_conditional_html` lives in `editor.rs`.

use super::super::OmniViewGenerator;
use crate::ir::*;
use rustc_hash::FxHashMap;

impl OmniViewGenerator {
    /// Generate element tree entries for only the active conditional branch
    /// when in `css_only` (canvas preview) mode.
    ///
    /// Evaluates each branch's condition against `signal_defaults` (which
    /// reflect the debug-patched signal values from the store). Only the
    /// first matching branch is rendered; unmatched branches are skipped.
    /// Complex conditions that can't be statically evaluated fall back to
    /// the `else` branch (or nothing if no `else` exists).
    ///
    /// Branch bodies are wrapped in a synthetic `<fragment>` element by the
    /// IR lowerer. We unwrap it here so branch children bypass the fragment.
    ///
    /// Each active branch is wrapped in a `<div>` container node in the
    /// element tree. This matches the non-css_only path (`emit_conditional_html`)
    /// which wraps each branch in a `<div>` for display toggling. Without this
    /// wrapper, conditional children flow inline with adjacent elements instead
    /// of occupying their own block row.
    pub(super) fn emit_conditional_css_only(
        &mut self,
        cond: &ConditionalIntent,
        indent: usize,
    ) -> String {
        for branch in &cond.branches {
            if eval_condition_for_canvas(&branch.condition, &self.signal_defaults) {
                if let Some(ref body) = branch.body {
                    let wrapper_nid = self.next_nid("if-branch");
                    self.tree_push(wrapper_nid, "div".to_string());
                    self.render_branch_body(body, indent);
                    self.tree_pop();
                }
                return String::new();
            }
        }
        // No condition matched — render fallback (else branch) if present
        if let Some(ref fallback) = cond.fallback {
            let wrapper_nid = self.next_nid("if-else");
            self.tree_push(wrapper_nid, "div".to_string());
            self.render_branch_body(fallback, indent);
            self.tree_pop();
        }
        String::new()
    }

    /// Render a branch body into the element tree, unwrapping any synthetic
    /// `<fragment>` wrapper so children appear at the correct nesting level.
    pub(super) fn render_branch_body(&mut self, body: &ViewTree, indent: usize) {
        match body {
            ViewTree::Element(el) if el.tag == "fragment" => {
                // Unwrap fragment: render each child directly
                for child in &el.children {
                    self.view_to_html(child, indent);
                }
            }
            _ => {
                self.view_to_html(body, indent);
            }
        }
    }
}

// ══════════════════════════════════════════════════════════
// Canvas conditional evaluation (css_only mode)
// ══════════════════════════════════════════════════════════

/// Statically evaluate a conditional branch expression against signal default values.
///
/// Used in `css_only` (canvas preview) mode to determine which `@if` / `@else if`
/// branch to render. Signal default values are patched by the TS-side debug signal
/// panel before WASM compilation, so they reflect the user's debug overrides.
///
/// Returns `false` for any expression that can't be statically evaluated, causing
/// a fallback to the `@else` branch (or empty output).
fn eval_condition_for_canvas(expr: &ExprIR, signal_defaults: &FxHashMap<String, String>) -> bool {
    match expr {
        ExprIR::LiteralExpr { value } => is_truthy_str(&literal_to_string(value)),
        ExprIR::IdentifierExpr { name, .. } => signal_defaults
            .get(name)
            .map(|v| is_truthy_str(v))
            .unwrap_or(false),
        ExprIR::BinaryOpExpr { op, left, right } => match op {
            BinaryOpIR::Eq => {
                let l = signal_or_literal_value(left, signal_defaults);
                let r = signal_or_literal_value(right, signal_defaults);
                matches!((l, r), (Some(lv), Some(rv)) if lv == rv)
            }
            BinaryOpIR::Neq => {
                let l = signal_or_literal_value(left, signal_defaults);
                let r = signal_or_literal_value(right, signal_defaults);
                matches!((l, r), (Some(lv), Some(rv)) if lv != rv)
            }
            BinaryOpIR::And => {
                eval_condition_for_canvas(left, signal_defaults)
                    && eval_condition_for_canvas(right, signal_defaults)
            }
            BinaryOpIR::Or => {
                eval_condition_for_canvas(left, signal_defaults)
                    || eval_condition_for_canvas(right, signal_defaults)
            }
            // Gt, Lt, GtEq, LtEq — numeric comparison
            BinaryOpIR::Gt | BinaryOpIR::Lt | BinaryOpIR::GtEq | BinaryOpIR::LtEq => {
                let l = signal_or_literal_value(left, signal_defaults);
                let r = signal_or_literal_value(right, signal_defaults);
                match (
                    l.and_then(|v| v.parse::<f64>().ok()),
                    r.and_then(|v| v.parse::<f64>().ok()),
                ) {
                    (Some(ln), Some(rn)) => match op {
                        BinaryOpIR::Gt => ln > rn,
                        BinaryOpIR::Lt => ln < rn,
                        BinaryOpIR::GtEq => ln >= rn,
                        BinaryOpIR::LtEq => ln <= rn,
                        _ => false,
                    },
                    _ => false,
                }
            }
            _ => false,
        },
        ExprIR::UnaryOpExpr {
            op: UnaryOpIR::Not,
            operand,
        } => !eval_condition_for_canvas(operand, signal_defaults),
        // Field access like `Store.field` — look up "Store.field" in signal_defaults
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                let key = format!("{}.{}", name, field);
                signal_defaults
                    .get(&key)
                    .map(|v| is_truthy_str(v))
                    .unwrap_or(false)
            } else {
                false
            }
        }
        _ => false, // Complex expression — can't evaluate, default to inactive
    }
}

/// Extract a comparable string value from an expression.
///
/// For `IdentifierExpr`: looks up the signal's debug-patched default value.
/// For `LiteralExpr`: extracts the raw value (strings without quotes, numbers
/// as decimal strings, booleans as "true"/"false").
/// Returns `None` for complex expressions that can't be statically resolved.
fn signal_or_literal_value(
    expr: &ExprIR,
    signal_defaults: &FxHashMap<String, String>,
) -> Option<String> {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => signal_defaults.get(name).cloned(),
        ExprIR::LiteralExpr { value } => Some(literal_to_string(value)),
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                let key = format!("{}.{}", name, field);
                signal_defaults.get(&key).cloned()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Convert a JSON literal value to a stripped string for comparison.
///
/// Strings are returned without surrounding quotes to match the format
/// used by `expr_default_display` when populating `signal_defaults`.
fn literal_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

/// Check if a string value from `signal_defaults` is truthy.
///
/// Handles values stored by `expr_default_display`: strings are unquoted,
/// numbers and booleans are their literal JS representations.
fn is_truthy_str(v: &str) -> bool {
    !v.is_empty() && v != "false" && v != "null" && v != "undefined" && v != "0" && v != "0.0"
}
