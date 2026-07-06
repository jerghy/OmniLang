//! Expression formatting, helpers, and event data extraction for Dioxus codegen.

use crate::ir::*;

// ═══════════════════════════════════════════════════════════
// Block expression formatting
// ═══════════════════════════════════════════════════════════

pub(super) fn format_block_as_dioxus_expr(block: &BlockIR) -> String {
    super::super::format_block_as_expr(block, &format_expr_ir_dioxus, "None")
}

// ═══════════════════════════════════════════════════════════
// Expression formatting (Rust)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir_dioxus(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value_dioxus(value),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir_dioxus(object), field)
        }
        ExprIR::OptionalChainExpr { object, field, .. } => {
            format!("{}.{}", format_expr_ir_dioxus(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let rs_op = binary_op_to_dioxus(op);
            let lhs = format_expr_ir_dioxus(left);
            let rhs = format_expr_ir_dioxus(right);
            format!("{} {} {}", lhs, rs_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let rs_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", rs_op, format_expr_ir_dioxus(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir_dioxus(cond);
            let t = format_expr_ir_dioxus(then_expr);
            let e = format_expr_ir_dioxus(else_expr);
            format!("if {} {{ {} }} else {{ {} }}", c, t, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir_dioxus).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args.iter().map(format_expr_ir_dioxus).collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements.iter().map(format_expr_ir_dioxus).collect();
            format!("vec![{}]", items.join(", "))
        }
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_expr_ir_dioxus(v)))
                .collect();
            format!("{{ {} }}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            union_type,
            variant,
            fields,
        } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_expr_ir_dioxus(v)))
                .collect();
            format!("{}::{} {{ {} }}", union_type, variant, entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            is_optional: _,
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_ir_dioxus(object);
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir_dioxus).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr {
            statements: _,
            tail,
        } => {
            // Only tail expression emitted; preceding statements preserved in IR for future codegen
            format_expr_ir_dioxus(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir_dioxus(object);
            let idx = format_expr_ir_dioxus(index);
            format!("({obj})[{idx} as usize]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("{}.await", format_expr_ir_dioxus(operand))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir_dioxus(matched);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body = format_block_as_dioxus_expr(&arm.body);
                    match &arm.pattern.kind {
                        MatchPatternKindIR::Variant { name, .. } => {
                            format!("    UnionType::{} {{ .. }} => {},", name, body)
                        }
                        MatchPatternKindIR::Wildcard => {
                            format!("    _ => {},", body)
                        }
                        _ => String::new(),
                    }
                })
                .collect();
            format!("match &{} {{\n{}\n}}", m, cases.join("\n"))
        }
        ExprIR::ClosureExpr { params, body } => {
            let b = format_expr_ir_dioxus(body);
            let p = params.join(", ");
            format!("(move |{p}| {b})")
        }
    }
}

/// Format a JSON value as a Rust literal expression.
pub(super) fn format_json_value_dioxus(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".into(),
        serde_json::Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\".to_string()", escaped)
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<_> = arr.iter().map(format_json_value_dioxus).collect();
            format!("vec![{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let items: Vec<_> = obj
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_json_value_dioxus(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
    }
}

/// Map OMNI binary operator name to Dioxus/Rust operator.
pub(super) fn binary_op_to_dioxus(op: &str) -> &str {
    super::super::binary_op_to_str(op, false, false)
}

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

pub(super) fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c.to_ascii_lowercase());
        }
    }
    result
}

/// Map OmniView attribute names to Dioxus attribute names.
pub(super) fn to_dioxus_attr(prop_name: &str) -> String {
    match prop_name {
        "class" | "className" => "class".into(),
        "for" | "htmlFor" => "r#for".into(),
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

/// Map OmniView event names to Dioxus event handler prop names.
pub(super) fn to_dioxus_event(event_name: &str) -> String {
    match event_name {
        "click" => "onclick".into(),
        "change" => "onchange".into(),
        "input" => "oninput".into(),
        "submit" => "onsubmit".into(),
        "focus" => "onfocus".into(),
        "blur" => "onblur".into(),
        "keydown" => "onkeydown".into(),
        "keyup" => "onkeyup".into(),
        "keypress" => "onkeypress".into(),
        "mouseenter" => "onmouseenter".into(),
        "mouseleave" => "onmouseleave".into(),
        "mousedown" => "onmousedown".into(),
        "mouseup" => "onmouseup".into(),
        "mousemove" => "onmousemove".into(),
        "mouseover" => "onmouseover".into(),
        "mouseout" => "onmouseout".into(),
        "dblclick" => "ondblclick".into(),
        "scroll" => "onscroll".into(),
        "wheel" => "onwheel".into(),
        "touchstart" => "ontouchstart".into(),
        "touchend" => "ontouchend".into(),
        "touchmove" => "ontouchmove".into(),
        "contextmenu" => "oncontextmenu".into(),
        "drag" => "ondrag".into(),
        "dragstart" => "ondragstart".into(),
        "dragend" => "ondragend".into(),
        "dragover" => "ondragover".into(),
        "dragleave" => "ondragleave".into(),
        "drop" => "ondrop".into(),
        _ => event_name.to_string(),
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
        (EventDataKind::SubmitEvent, "values") => "HashMap::new()".to_string(),

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

        _ => format!("todo!(\"field {}\")", field),
    }
}

/// Emit a Rust HashMap literal for Dioxus event data extraction.
pub(super) fn emit_event_object_dioxus(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
) -> String {
    let kind = match event_data {
        Some(k) => k,
        None => return "HashMap::new()".to_string(),
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
        Some(_) => return "HashMap::new()".to_string(),
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
