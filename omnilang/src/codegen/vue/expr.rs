//! Expression and statement generation for Vue codegen, plus helpers.

use super::CodeWriter;
use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::{JsonFormatConfig, Target, JSON_VUE};
use crate::ir::*;

/// Vue expression formatter.
///
/// Uses JS-family syntax with expression-body closures.
pub(crate) struct VueFormatExpr;

impl FormatExpr for VueFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &JSON_VUE
    }

    fn codegen_target(&self) -> Target {
        Target::Vue
    }

    fn strict_eq(&self) -> bool {
        true
    }

    fn null_fallback(&self) -> &str {
        "undefined"
    }

    fn format_closure(&self, params: &str, body: &str) -> String {
        if params.is_empty() {
            format!("(() => ({body}))")
        } else {
            format!("(({params}) => ({body}))")
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Statement generation (Vue script setup)
// ═══════════════════════════════════════════════════════════

/// Vue statement formatter — always uses `const`, TS annotations, `.value` mutations.
pub(super) struct VueStmtFormatter {
    pub expr: VueFormatExpr,
}

impl FormatStmt for VueStmtFormatter {
    type ExprFormatter = VueFormatExpr;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn use_const_for_immutable(&self) -> bool {
        true
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "const"
    }

    fn format_let_type_ann(&self, ty: &Option<TypeIR>) -> String {
        ty.as_ref()
            .map(|t| {
                let ann = JsTypeFormatter.format_type(t.kind.as_str(), &t.type_args);
                if ann.is_empty() {
                    String::new()
                } else {
                    format!(": {}", ann)
                }
            })
            .unwrap_or_default()
    }

    fn mutation_target_expr(&self, target: &str) -> String {
        format!("{}.value", target)
    }
}

pub(super) fn generate_statement_vue(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = VueStmtFormatter {
        expr: VueFormatExpr,
    };
    fmt.format_stmt(w, stmt);
}

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

/// Map OmniView attribute names to Vue HTML attribute names.
/// Vue uses standard HTML attribute names (lowercase, kebab-case).
pub(super) fn to_vue_attr(prop_name: &str) -> String {
    match prop_name {
        "class" | "className" => "class".into(),
        "for" | "htmlFor" => "for".into(),
        "tabindex" => "tabindex".into(),
        "readonly" => "readonly".into(),
        "maxlength" => "maxlength".into(),
        "minlength" => "minlength".into(),
        "autocomplete" => "autocomplete".into(),
        "autofocus" => "autofocus".into(),
        "autoplay" => "autoplay".into(),
        "enctype" => "enctype".into(),
        "formaction" => "formaction".into(),
        "formenctype" => "formenctype".into(),
        "formmethod" => "formmethod".into(),
        "formnovalidate" => "formnovalidate".into(),
        "formtarget" => "formtarget".into(),
        _ => prop_name.to_string(),
    }
}

/// Map OmniView event names to Vue event directive suffixes.
/// Vue uses @click, @change, etc. — bare DOM event names.
pub(super) fn to_vue_event(event_name: &str) -> String {
    // Strip any existing "on" prefix to normalize (IR may carry "onPress" etc.)
    let stem = event_name.strip_prefix("on").unwrap_or(event_name);
    // Lowercase for case-insensitive matching
    let stem_lower = stem.to_lowercase();
    match stem_lower.as_str() {
        "click" | "press" | "tap" => "click".into(),
        "change" => "change".into(),
        "input" => "input".into(),
        "submit" => "submit".into(),
        "focus" => "focus".into(),
        "blur" => "blur".into(),
        "keydown" => "keydown".into(),
        "keyup" => "keyup".into(),
        "mouseenter" => "mouseenter".into(),
        "mouseleave" => "mouseleave".into(),
        "mousedown" => "mousedown".into(),
        "mouseup" => "mouseup".into(),
        "mousemove" => "mousemove".into(),
        "dblclick" => "dblclick".into(),
        "scroll" => "scroll".into(),
        "wheel" => "wheel".into(),
        "touchstart" => "touchstart".into(),
        "touchend" => "touchend".into(),
        "touchmove" => "touchmove".into(),
        "contextmenu" => "contextmenu".into(),
        "drag" => "drag".into(),
        "dragstart" => "dragstart".into(),
        "dragend" => "dragend".into(),
        "dragover" => "dragover".into(),
        "dragleave" => "dragleave".into(),
        "drop" => "drop".into(),
        // Custom events: use the stem directly (Vue will emit as component event name)
        _ => stem.to_lowercase(),
    }
}

/// Format a single expression IR node as a Vue expression string (test helper).
#[cfg(test)]
pub(super) fn format_expr_vue(expr: &ExprIR) -> String {
    VueFormatExpr.format_expr(expr)
}
