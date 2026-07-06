// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

use crate::codegen::sanitize_css_value;
use crate::ir::*;

/// Check if a StatementIR references a given identifier anywhere in its tree.
fn statement_contains_ident(stmt: &StatementIR, ident: &str) -> bool {
    match stmt {
        StatementIR::LetStmtIR { value, .. } => value
            .as_ref()
            .map_or(false, |v| getter_expr_contains_ident(v, ident)),
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            getter_expr_contains_ident(condition, ident)
                || then_body
                    .statements
                    .iter()
                    .any(|s| statement_contains_ident(s, ident))
                || else_ifs.iter().any(|(cond, body)| {
                    getter_expr_contains_ident(cond, ident)
                        || body
                            .statements
                            .iter()
                            .any(|s| statement_contains_ident(s, ident))
                })
                || else_body.as_ref().map_or(false, |b| {
                    b.statements
                        .iter()
                        .any(|s| statement_contains_ident(s, ident))
                })
        }
        StatementIR::ReturnStmtIR { value } => value
            .as_ref()
            .map_or(false, |v| getter_expr_contains_ident(v, ident)),
        StatementIR::ExprStmtIR { expr } => getter_expr_contains_ident(expr, ident),
        StatementIR::MutationStmt { intent, .. } => {
            if let Some(intent) = intent {
                intent.target.name == *ident
                    || intent
                        .amount
                        .as_ref()
                        .map_or(false, |a| getter_expr_contains_ident(a, ident))
                    || intent
                        .item
                        .as_ref()
                        .map_or(false, |i| getter_expr_contains_ident(i, ident))
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if an ExprIR references a given identifier anywhere in its tree.
pub(super) fn getter_expr_contains_ident(expr: &ExprIR, ident: &str) -> bool {
    match expr {
        ExprIR::IdentifierExpr { name } => name == ident,
        ExprIR::BinaryOpExpr { left, right, .. } => {
            getter_expr_contains_ident(left, ident) || getter_expr_contains_ident(right, ident)
        }
        ExprIR::UnaryOpExpr { operand, .. } => getter_expr_contains_ident(operand, ident),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            getter_expr_contains_ident(cond, ident)
                || getter_expr_contains_ident(then_expr, ident)
                || getter_expr_contains_ident(else_expr, ident)
        }
        ExprIR::CallExpr { ref args, .. } => {
            args.iter().any(|a| getter_expr_contains_ident(a, ident))
        }
        ExprIR::FieldAccessExpr { ref object, .. }
        | ExprIR::OptionalChainExpr { ref object, .. } => getter_expr_contains_ident(object, ident),
        ExprIR::MethodCallExpr {
            ref object,
            ref args,
            ..
        } => {
            getter_expr_contains_ident(object, ident)
                || args.iter().any(|a| getter_expr_contains_ident(a, ident))
        }
        ExprIR::StoreFieldAccessExpr { ref call_args, .. } => call_args
            .iter()
            .any(|a| getter_expr_contains_ident(a, ident)),
        ExprIR::BlockExpr { statements, tail } => {
            statements
                .iter()
                .any(|s| statement_contains_ident(s, ident))
                || getter_expr_contains_ident(tail, ident)
        }
        _ => false,
    }
}

pub(super) fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub(super) fn format_json_value_tsx(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\"", escaped)
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<_> = arr.iter().map(format_json_value_tsx).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let items: Vec<_> = obj
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_json_value_tsx(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
    }
}

pub(super) fn binary_op_to_tsx(op: &str) -> &str {
    match op {
        "add" => "+",
        "sub" => "-",
        "mul" => "*",
        "div" => "/",
        "mod" => "%",
        "eq" => "===",
        "neq" => "!==",
        "lt" => "<",
        "gt" => ">",
        "lteq" => "<=",
        "gteq" => ">=",
        "and" => "&&",
        "or" => "||",
        "assign" => "=",
        "addassign" => "+=",
        "subassign" => "-=",
        "mulassign" => "*=",
        "divassign" => "/=",
        "modassign" => "%=",
        _ => op,
    }
}

pub(super) fn format_css_value(value: &ExprIR) -> String {
    match value {
        ExprIR::LiteralExpr { value } => match value {
            serde_json::Value::String(s) => sanitize_css_value(s),
            serde_json::Value::Number(n) => format!("{}px", n),
            _ => format!("{}", value),
        },
        _ => String::new(),
    }
}

pub(super) fn to_css_prop(omni_prop: &str) -> String {
    let mut result = String::new();
    for c in omni_prop.chars() {
        if c.is_uppercase() {
            result.push('-');
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Map OmniView attribute names to JSX attribute names.
pub(super) fn to_jsx_attr(prop_name: &str) -> String {
    match prop_name {
        "class" | "className" => "className".into(),
        "for" | "htmlFor" => "htmlFor".into(),
        "tabindex" => "tabIndex".into(),
        "readonly" => "readOnly".into(),
        "maxlength" => "maxLength".into(),
        "minlength" => "minLength".into(),
        "autocomplete" => "autoComplete".into(),
        "autofocus" => "autoFocus".into(),
        "autoplay" => "autoPlay".into(),
        "enctype" => "encType".into(),
        "formaction" => "formAction".into(),
        "formenctype" => "formEncType".into(),
        "formmethod" => "formMethod".into(),
        "formnovalidate" => "formNoValidate".into(),
        "formtarget" => "formTarget".into(),
        "datetime" => "dateTime".into(),
        "srcset" => "srcSet".into(),
        "testId" => "data-testid".into(),
        _ => prop_name.to_string(),
    }
}

/// Map OmniView custom element tags to their React/HTML equivalents.
/// OmniView uses PascalCase aliases (Button, Card, Text, Flex, etc.) for native HTML elements.
pub(super) fn to_react_tag(tag: &str) -> String {
    match tag {
        "Text" => "span".into(),
        "Button" => "button".into(),
        "Card" | "Container" => "div".into(),
        "Flex" => "div".into(),
        "Heading" => "h2".into(),
        "Image" | "Img" => "img".into(),
        "TextInput" | "Input" => "input".into(),
        "Link" => "a".into(),
        "List" => "ul".into(),
        "ListItem" => "li".into(),
        "CheckBox" | "Checkbox" => "input".into(),
        "Select" => "select".into(),
        "TextArea" | "Textarea" => "textarea".into(),
        // For other PascalCase tags not in the map, lowercase the first letter
        // to avoid React treating them as component references.
        t if t.chars().next().is_some_and(|c| c.is_uppercase()) => t.to_lowercase(),
        t => t.to_string(),
    }
}

/// Map OmniView event names to React event handler prop names.
/// Event names from the IR may or may not have an "on" prefix — we normalize first.
pub(super) fn to_react_event(event_name: &str) -> String {
    // Strip any existing "on" prefix to normalize (IR may carry "onPress" etc.)
    let stem = event_name.strip_prefix("on").unwrap_or(event_name);
    // Lowercase for case-insensitive matching
    let stem_lower = stem.to_lowercase();
    match stem_lower.as_str() {
        "click" => "onClick".into(),
        "change" => "onChange".into(),
        "input" => "onInput".into(),
        "submit" => "onSubmit".into(),
        "focus" => "onFocus".into(),
        "blur" => "onBlur".into(),
        "keydown" => "onKeyDown".into(),
        "keyup" => "onKeyUp".into(),
        "keypress" => "onKeyPress".into(),
        "mouseenter" => "onMouseEnter".into(),
        "mouseleave" => "onMouseLeave".into(),
        "mousedown" => "onMouseDown".into(),
        "mouseup" => "onMouseUp".into(),
        "mousemove" => "onMouseMove".into(),
        "mouseover" => "onMouseOver".into(),
        "mouseout" => "onMouseOut".into(),
        "dblclick" => "onDoubleClick".into(),
        "scroll" => "onScroll".into(),
        "wheel" => "onWheel".into(),
        "touchstart" => "onTouchStart".into(),
        "touchend" => "onTouchEnd".into(),
        "touchmove" => "onTouchMove".into(),
        "contextmenu" => "onContextMenu".into(),
        "drag" => "onDrag".into(),
        "dragstart" => "onDragStart".into(),
        "dragend" => "onDragEnd".into(),
        "dragover" => "onDragOver".into(),
        "dragleave" => "onDragLeave".into(),
        "drop" => "onDrop".into(),
        "press" => "onClick".into(),
        _ => format!("on{}", capitalize_first(stem)),
    }
}

// ═══════════════════════════════════════════════════════════
// React event data extraction
// ═══════════════════════════════════════════════════════════

/// React target field extraction — mostly identical to Web but uses
/// SyntheticEvent conventions (e.target guaranteed during handler, no ?. needed).
fn field_extract_react(kind: &EventDataKind, field: &str, raw: &str) -> (String, String) {
    match (kind, field) {
        // PointerEvent
        (EventDataKind::PointerEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::PointerEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::PointerEvent, "button") => (format!("{}.button", raw), "0".into()),
        (EventDataKind::PointerEvent, "pressure") => (format!("{}.pressure", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltX") => (format!("{}.tiltX", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltY") => (format!("{}.tiltY", raw), "0".into()),
        (EventDataKind::PointerEvent, "pointerType") => (format!("{}.pointerType", raw), "\"\"".into()),
        (EventDataKind::PointerEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // TouchEvent
        (EventDataKind::TouchEvent, "touches") => (
            format!("[...({}.touches)].map(t => ({{ x: t.clientX, y: t.clientY, id: t.identifier }}))", raw),
            "[]".into()
        ),
        (EventDataKind::TouchEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // WheelEvent
        (EventDataKind::WheelEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::WheelEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaX") => (format!("{}.deltaX", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaY") => (format!("{}.deltaY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaZ") => (format!("{}.deltaZ", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaMode") => (format!("{}.deltaMode", raw), "0".into()),
        (EventDataKind::WheelEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // KeyEvent
        (EventDataKind::KeyEvent, "key") => (format!("{}.key", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, "code") => (format!("{}.code", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ScrollEvent — React guarantees e.target
        (EventDataKind::ScrollEvent, "x") => (format!("{}.target.scrollLeft", raw), "0".into()),
        (EventDataKind::ScrollEvent, "y") => (format!("{}.target.scrollTop", raw), "0".into()),

        // InputEvent — React guarantees e.target
        (EventDataKind::InputEvent, "value") => (format!("{}.target.value", raw), "\"\"".into()),
        (EventDataKind::InputEvent, "checked") => (format!("{}.target.checked", raw), "false".into()),

        // ToggleEvent — React guarantees e.target
        (EventDataKind::ToggleEvent, "checked") => (format!("{}.target.checked", raw), "false".into()),

        // SubmitEvent
        (EventDataKind::SubmitEvent, "values") => (
            format!("Object.fromEntries(new FormData({}.target))", raw),
            "{}".into()
        ),

        // FocusEvent — React guarantees e.target
        (EventDataKind::FocusEvent, "value") => (format!("{}.target.value", raw), "\"\"".into()),

        // ClipboardEvent
        (EventDataKind::ClipboardEvent, "text") => (
            format!("(function(){{try{{return {}.clipboardData?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),

        // DragEvent
        (EventDataKind::DragEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DragEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DragEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // DropEvent
        (EventDataKind::DropEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DropEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DropEvent, "files") => (
            format!("[...({}.dataTransfer?.files ?? [])].map(f => f.name)", raw),
            "[]".into()
        ),
        (EventDataKind::DropEvent, "text") => (
            format!("(function(){{try{{return {}.dataTransfer?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),
        (EventDataKind::DropEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ContextMenuEvent
        (EventDataKind::ContextMenuEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // SelectEvent — React guarantees e.target
        (EventDataKind::SelectEvent, "start") => (format!("{}.target.selectionStart", raw), "0".into()),
        (EventDataKind::SelectEvent, "end") => (format!("{}.target.selectionEnd", raw), "0".into()),
        (EventDataKind::SelectEvent, "text") => (
            format!("({}.target.value ?? '').substring({}.target.selectionStart ?? 0, {}.target.selectionEnd ?? 0)", raw, raw, raw),
            "\"\"".into()
        ),

        // ResizeEvent
        (EventDataKind::ResizeEvent, "width") => ("window.innerWidth".into(), "0".into()),
        (EventDataKind::ResizeEvent, "height") => ("window.innerHeight".into(), "0".into()),

        // MediaEvent — React guarantees e.target
        (EventDataKind::MediaEvent, "currentTime") => (format!("{}.target.currentTime", raw), "0".into()),
        (EventDataKind::MediaEvent, "duration") => (format!("{}.target.duration", raw), "0".into()),

        // MediaLoadEvent — React guarantees e.target
        (EventDataKind::MediaLoadEvent, "naturalWidth") => (format!("{}.target.naturalWidth", raw), "0".into()),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => (format!("{}.target.naturalHeight", raw), "0".into()),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => (format!("{}.target?.error?.message", raw), "\"unknown\"".into()),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => (format!("{}.animationName", raw), "\"\"".into()),
        (EventDataKind::AnimationEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => (format!("{}.propertyName", raw), "\"\"".into()),
        (EventDataKind::TransitionEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        _ => (format!("undefined /* {} */", field), "undefined".into()),
    }
}

/// Emit a JS object literal for React event data extraction.
/// Only emits fields in `used_fields` (or all fields from schema if None).
pub(super) fn emit_event_object_react(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
) -> String {
    let kind = match event_data {
        Some(k) => k,
        None => return "{}".to_string(),
    };

    let schema = event_data_schema(kind);
    let fields: Vec<String> = match used_fields {
        Some(fields) if !fields.is_empty() => schema
            .iter()
            .filter(|f| fields.contains(&f.name))
            .map(|f| {
                let (extract, fallback) = field_extract_react(kind, &f.name, raw_var);
                format!("{}: {} ?? {}", f.name, extract, fallback)
            })
            .collect(),
        Some(_) => return "{}".to_string(),
        None => schema
            .iter()
            .map(|f| {
                let (extract, fallback) = field_extract_react(kind, &f.name, raw_var);
                format!("{}: {} ?? {}", f.name, extract, fallback)
            })
            .collect(),
    };

    format!("{{ {} }}", fields.join(", "))
}
