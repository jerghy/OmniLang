//! Standalone motion validation helper functions.
//!
//! These are free functions (not methods on `SemanticAnalyzer`) used by
//! [`validate_motion`](super::validate_motion) and other passes.

use crate::parser::ast::*;

/// Check if a string is a valid easing function name.
///
/// Delegates to the shared [`NAMED_EASINGS`](crate::codegen::shared::easing::NAMED_EASINGS) table.
pub(super) fn is_valid_easing_name(name: &str) -> bool {
    crate::codegen::shared::easing::resolve_easing(name).is_some()
}

/// Check if a value name is a non-interpolatable CSS keyword.
pub(super) fn is_keyword_value(name: &str) -> bool {
    matches!(
        name,
        "auto" | "none" | "normal" | "inherit" | "initial" | "unset" | "revert"
    )
}

/// Check if a view node tree contains any @enter animation triggers.
pub(super) fn has_enter_leave_animation(nodes: &[ViewNode]) -> bool {
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                for prop in &el.props {
                    if let ElementProp::MotionTrigger(mt) = prop {
                        if matches!(mt.trigger, TriggerKind::Enter) {
                            return true;
                        }
                    }
                }
                if has_enter_leave_animation(&el.children) {
                    return true;
                }
            }
            ViewNode::IfDirective(if_dir) => {
                if has_enter_leave_animation(&if_dir.body) {
                    return true;
                }
                for (_, body) in &if_dir.else_ifs {
                    if has_enter_leave_animation(body) {
                        return true;
                    }
                }
                if let Some(ref else_body) = if_dir.else_body {
                    if has_enter_leave_animation(else_body) {
                        return true;
                    }
                }
            }
            ViewNode::ForDirective(for_dir) => {
                if has_enter_leave_animation(&for_dir.body) {
                    return true;
                }
                if let Some(ref empty_body) = for_dir.empty_body {
                    if has_enter_leave_animation(empty_body) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Parse a duration string (e.g. "300ms", "1.5s") to milliseconds.
/// Returns `Err` with a description if the value cannot be parsed.
pub(super) fn parse_duration_ms(val: &str) -> Result<f64, String> {
    let val = val.trim();
    if let Some(ms_part) = val.strip_suffix("ms") {
        ms_part
            .parse::<f64>()
            .map_err(|_| format!("invalid number in duration: '{}'", val))
    } else if let Some(s_part) = val.strip_suffix('s') {
        s_part
            .parse::<f64>()
            .map(|s| s * 1000.0)
            .map_err(|_| format!("invalid number in duration: '{}'", val))
    } else {
        Err(format!(
            "invalid duration format: '{}' (expected e.g. '300ms' or '1.5s')",
            val
        ))
    }
}
