//! Expression formatting, helpers, and event data extraction for Dioxus codegen.

use crate::codegen::shared::expr::{FormatExpr, RustFormatExpr};
use crate::ir::*;

/// Dioxus-family expression formatter.
///
/// Uses the shared `RustFormatExpr` with `move_closures: true` (required by Dioxus).
fn dioxus_format_expr() -> RustFormatExpr {
    RustFormatExpr {
        move_closures: true,
        event_param_name: None,
    }
}

// ═══════════════════════════════════════════════════════════
// Block expression formatting
// ═══════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir_dioxus(expr: &ExprIR) -> String {
    dioxus_format_expr().format_expr(expr)
}

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

pub(super) fn to_snake_case(name: &str) -> String {
    crate::codegen::camel_to_separated(name, '_')
}

/// Map OmniView attribute names to Dioxus attribute names.
/// Delegates to the shared `to_html_attr` and then overrides with
/// Dioxus-specific syntax (e.g., `r#for` for the `for` attribute).
pub(super) fn to_dioxus_attr(prop_name: &str) -> String {
    let base = crate::codegen::shared::html::to_html_attr(prop_name);
    // Dioxus uses raw identifier syntax for `for` (Rust keyword)
    match base {
        "for" => "r#for".into(),
        _ => base.to_string(),
    }
}

/// Map OmniView event names to Dioxus event handler prop names.
///
/// Alias resolution happens in the ONE shared table
/// (`shared::event::canonical_event_name`) — this adapter only applies the
/// Dioxus convention (`on` + lowercase canonical DOM name).
pub(crate) fn to_dioxus_event(event_name: &str) -> String {
    match crate::codegen::shared::event::canonical_event_name(event_name) {
        Some(canonical) => format!("on{}", canonical),
        // Custom / unmatched events: prefix with "on"
        None => {
            let stem = event_name
                .strip_prefix("on")
                .unwrap_or(event_name)
                .to_lowercase();
            format!("on{}", stem)
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Dioxus event data extraction (web_sys conventions)
// ═══════════════════════════════════════════════════════════

/// Dioxus target field extraction — uses web_sys Rust API with method-call syntax.
/// Properties like `clientX` become `client_x()` (snake_case, function call).
pub(super) fn field_extract_dioxus(kind: &EventDataKind, field: &str, raw: &str) -> String {
    let _js = |f: &str| format!("{}.{}()", raw, f);
    match (kind, field) {
        // PointerEvent
        (EventDataKind::PointerEvent, "x") => format!("{}.client_x()", raw),
        (EventDataKind::PointerEvent, "y") => format!("{}.client_y()", raw),
        (EventDataKind::PointerEvent, "button") => format!("{}.button()", raw),
        (EventDataKind::PointerEvent, "pressure") => format!("{}.pressure()", raw),
        (EventDataKind::PointerEvent, "tiltX") => format!("{}.tilt_x()", raw),
        (EventDataKind::PointerEvent, "tiltY") => format!("{}.tilt_y()", raw),
        (EventDataKind::PointerEvent, "pointerType") => format!("{}.pointer_type()", raw),
        (EventDataKind::PointerEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let key = match m {
                "alt" => "alt_key()",
                "ctrl" => "ctrl_key()",
                "shift" => "shift_key()",
                _ => "meta_key()",
            };
            format!("{}.{}", raw, key)
        }

        // TouchEvent
        (EventDataKind::TouchEvent, "touches") => "Vec::<(f64,f64,f64)>::new()".to_string(),
        (EventDataKind::TouchEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let key = match m {
                "alt" => "alt_key()",
                "ctrl" => "ctrl_key()",
                "shift" => "shift_key()",
                _ => "meta_key()",
            };
            format!("{}.{}", raw, key)
        }

        // WheelEvent
        (EventDataKind::WheelEvent, "x") => format!("{}.client_x()", raw),
        (EventDataKind::WheelEvent, "y") => format!("{}.client_y()", raw),
        (EventDataKind::WheelEvent, "deltaX") => format!("{}.delta_x()", raw),
        (EventDataKind::WheelEvent, "deltaY") => format!("{}.delta_y()", raw),
        (EventDataKind::WheelEvent, "deltaZ") => format!("{}.delta_z()", raw),
        (EventDataKind::WheelEvent, "deltaMode") => format!("{}.delta_mode()", raw),
        (EventDataKind::WheelEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let key = match m {
                "alt" => "alt_key()",
                "ctrl" => "ctrl_key()",
                "shift" => "shift_key()",
                _ => "meta_key()",
            };
            format!("{}.{}", raw, key)
        }

        // KeyEvent
        (EventDataKind::KeyEvent, "key") => format!("{}.key()", raw),
        (EventDataKind::KeyEvent, "code") => format!("{}.code()", raw),
        (EventDataKind::KeyEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let key = match m {
                "alt" => "alt_key()",
                "ctrl" => "ctrl_key()",
                "shift" => "shift_key()",
                _ => "meta_key()",
            };
            format!("{}.{}", raw, key)
        }

        // ScrollEvent
        (EventDataKind::ScrollEvent, "x") => "0.0".to_string(),
        (EventDataKind::ScrollEvent, "y") => "0.0".to_string(),

        // InputEvent
        (EventDataKind::InputEvent, "value") => format!("{}.data().value()", raw),
        (EventDataKind::InputEvent, "checked") => "false".to_string(),

        // ToggleEvent
        (EventDataKind::ToggleEvent, "checked") => "false".to_string(),

        // SubmitEvent
        (EventDataKind::SubmitEvent, "values") => "FxHashMap::default()".to_string(),

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => format!("{}.data().value()", raw),

        // ClipboardEvent
        (EventDataKind::ClipboardEvent, "text") => "String::new()".to_string(),

        // DragEvent
        (EventDataKind::DragEvent, "x") => format!("{}.client_x()", raw),
        (EventDataKind::DragEvent, "y") => format!("{}.client_y()", raw),
        (EventDataKind::DragEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let key = match m {
                "alt" => "alt_key()",
                "ctrl" => "ctrl_key()",
                "shift" => "shift_key()",
                _ => "meta_key()",
            };
            format!("{}.{}", raw, key)
        }

        // DropEvent
        (EventDataKind::DropEvent, "x") => format!("{}.client_x()", raw),
        (EventDataKind::DropEvent, "y") => format!("{}.client_y()", raw),
        (EventDataKind::DropEvent, "files") => "Vec::<String>::new()".to_string(),
        (EventDataKind::DropEvent, "text") => "String::new()".to_string(),
        (EventDataKind::DropEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let key = match m {
                "alt" => "alt_key()",
                "ctrl" => "ctrl_key()",
                "shift" => "shift_key()",
                _ => "meta_key()",
            };
            format!("{}.{}", raw, key)
        }

        // ContextMenuEvent
        (EventDataKind::ContextMenuEvent, "x") => format!("{}.client_x()", raw),
        (EventDataKind::ContextMenuEvent, "y") => format!("{}.client_y()", raw),
        (EventDataKind::ContextMenuEvent, m @ ("alt" | "ctrl" | "shift" | "meta")) => {
            let key = match m {
                "alt" => "alt_key()",
                "ctrl" => "ctrl_key()",
                "shift" => "shift_key()",
                _ => "meta_key()",
            };
            format!("{}.{}", raw, key)
        }

        // SelectEvent
        (EventDataKind::SelectEvent, "start") => "0".to_string(),
        (EventDataKind::SelectEvent, "end") => "0".to_string(),
        (EventDataKind::SelectEvent, "text") => "String::new()".to_string(),

        // ResizeEvent
        (EventDataKind::ResizeEvent, "width") => "window.inner_width()".to_string(),
        (EventDataKind::ResizeEvent, "height") => "window.inner_height()".to_string(),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => "0.0".to_string(),
        (EventDataKind::MediaEvent, "duration") => "0.0".to_string(),

        // MediaLoadEvent
        (EventDataKind::MediaLoadEvent, "naturalWidth") => "0.0".to_string(),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => "0.0".to_string(),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => "String::new()".to_string(),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => "String::new()".to_string(),
        (EventDataKind::AnimationEvent, "elapsedTime") => "0.0".to_string(),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => "String::new()".to_string(),
        (EventDataKind::TransitionEvent, "elapsedTime") => "0.0".to_string(),

        _ => format!("String::new() /* UNSUPPORTED event field: {} */", field),
    }
}

/// Emit a Rust FxHashMap literal for Dioxus event data extraction.
pub(super) fn emit_event_object_dioxus(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
) -> String {
    let kind = match event_data {
        Some(k) => k,
        None => return format!("{}.detail()", raw_var),
    };

    let schema = event_data_schema(kind);
    let fields: Vec<String> = match used_fields {
        Some(fields) if !fields.is_empty() => schema
            .iter()
            .filter(|f| fields.contains(&f.name))
            .map(|f| {
                let extract = field_extract_dioxus(kind, &f.name, raw_var);
                format!("(\"{}\", {})", f.name, extract)
            })
            .collect(),
        Some(_) => return "FxHashMap::default()".to_string(),
        None => schema
            .iter()
            .map(|f| {
                let extract = field_extract_dioxus(kind, &f.name, raw_var);
                format!("(\"{}\", {})", f.name, extract)
            })
            .collect(),
    };

    format!("::std::collections::HashMap::from([{}])", fields.join(", "))
}
