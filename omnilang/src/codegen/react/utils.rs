// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

use super::super::capitalize_first;
use crate::ir::*;

/// Check if a StatementIR references a given identifier anywhere in its tree.
fn statement_contains_ident(stmt: &StatementIR, ident: &str) -> bool {
    match stmt {
        StatementIR::LetStmtIR { value, .. } => value
            .as_ref()
            .is_some_and(|v| getter_expr_contains_ident(v, ident)),
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
                || else_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| statement_contains_ident(s, ident))
                })
        }
        StatementIR::ReturnStmtIR { value } => value
            .as_ref()
            .is_some_and(|v| getter_expr_contains_ident(v, ident)),
        StatementIR::ExprStmtIR { expr } => getter_expr_contains_ident(expr, ident),
        StatementIR::MutationStmt {
            intent: Some(intent),
            ..
        } => {
            intent.target.name == *ident
                || intent
                    .amount
                    .as_ref()
                    .is_some_and(|a| getter_expr_contains_ident(a, ident))
                || intent
                    .item
                    .as_ref()
                    .is_some_and(|i| getter_expr_contains_ident(i, ident))
        }
        StatementIR::BlockStmtIR { body } => body
            .statements
            .iter()
            .any(|s| statement_contains_ident(s, ident)),
        StatementIR::WhileStmtIR { condition, body } => {
            getter_expr_contains_ident(condition, ident)
                || body
                    .statements
                    .iter()
                    .any(|s| statement_contains_ident(s, ident))
        }
        StatementIR::ForInStmtIR { iterable, body, .. } => {
            getter_expr_contains_ident(iterable, ident)
                || body
                    .statements
                    .iter()
                    .any(|s| statement_contains_ident(s, ident))
        }
        StatementIR::ThrowStmtIR { expr } => getter_expr_contains_ident(expr, ident),
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body
                .statements
                .iter()
                .any(|s| statement_contains_ident(s, ident))
                || catch_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| statement_contains_ident(s, ident))
                })
                || finally_body.as_ref().is_some_and(|b| {
                    b.statements
                        .iter()
                        .any(|s| statement_contains_ident(s, ident))
                })
        }
        _ => false,
    }
}

/// Check if any statement in the action body references the given identifier.
/// Returns true if the identifier is referenced anywhere in the action's statements.
pub(super) fn action_body_contains_ident(action: &crate::ir::FunctionIR, ident: &str) -> bool {
    action
        .body
        .statements
        .iter()
        .any(|s| statement_contains_ident(s, ident))
}

/// Check if an ExprIR references a given identifier anywhere in its tree.
pub(super) fn getter_expr_contains_ident(expr: &ExprIR, ident: &str) -> bool {
    match expr {
        ExprIR::IdentifierExpr { name, .. } => name == ident,
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
/// Delegates to the shared `to_html_tag` and then applies React-specific
/// lowercasing for custom components (JSX treats uppercase tags as components).
pub(super) fn to_react_tag(tag: &str) -> String {
    let base = crate::codegen::shared::html::to_html_tag(tag);
    // If the shared function returned the tag unchanged and it starts with uppercase,
    // lowercase the first letter to avoid React treating it as a component reference.
    if base == tag && tag.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars = tag.chars();
        chars.next().unwrap().to_lowercase().collect::<String>() + chars.as_str()
    } else {
        base
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
