//! Standalone utility functions for OmniView codegen.
use super::super::{CompileOutput, CompiledFile};
use crate::ir::*;
use std::collections::HashMap;

pub(super) fn serde_value_to_js(value: &serde_json::Value) -> String {
    crate::codegen::format_json_value(value, &crate::codegen::JSON_OMNIVIEW)
}

// ══════════════════════════════════════════════════════════
// Mapping Helpers
// ══════════════════════════════════════════════════════════

/// Map omni component tags to HTML tags.
pub(super) fn map_tag(tag: &str) -> String {
    match tag {
        "Text" => "span".into(),
        "Button" => "button".into(),
        "Container" | "Card" => "div".into(),
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
        t if t.chars().next().is_some_and(|c| c.is_uppercase()) => {
            // Custom component → web component style kebab-case
            to_kebab(t)
        }
        t => t.to_string(),
    }
}

/// Map omni event names to DOM event names.
pub(super) fn map_event(name: &str) -> String {
    match name {
        "onPress" | "onClick" | "onTap" => "click".into(),
        "onChange" | "onInput" => "input".into(),
        "onFocus" => "focus".into(),
        "onBlur" => "blur".into(),
        "onKeyDown" => "keydown".into(),
        "onKeyUp" => "keyup".into(),
        "onSubmit" => "submit".into(),
        "onScroll" => "scroll".into(),
        "onMouseEnter" => "mouseenter".into(),
        "onMouseLeave" => "mouseleave".into(),
        "onMouseMove" => "mousemove".into(),
        n if n.starts_with("on") => n[2..].to_lowercase(),
        n => n.to_string(),
    }
}

/// Emit JS extraction code for a single event data field.
/// Returns (extraction_expression, fallback_value_literal).
/// `pub(crate)` so other codegen targets (Vue, JS) can reuse the DOM-based mapping.
pub(crate) fn field_extract_js(kind: &EventDataKind, field: &str, raw: &str) -> (String, String) {
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
            format!("__mergeTouches({})", raw),
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

        // ScrollEvent
        (EventDataKind::ScrollEvent, "x") => (format!("{}.target?.scrollLeft", raw), "0".into()),
        (EventDataKind::ScrollEvent, "y") => (format!("{}.target?.scrollTop", raw), "0".into()),

        // InputEvent
        (EventDataKind::InputEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),
        (EventDataKind::InputEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // ToggleEvent
        (EventDataKind::ToggleEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // SubmitEvent — Web
        (EventDataKind::SubmitEvent, "values") => (
            format!("Object.fromEntries(new FormData({}.target))", raw),
            "{}".into()
        ),

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),

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

        // SelectEvent
        (EventDataKind::SelectEvent, "start") => (format!("{}.target?.selectionStart", raw), "0".into()),
        (EventDataKind::SelectEvent, "end") => (format!("{}.target?.selectionEnd", raw), "0".into()),
        (EventDataKind::SelectEvent, "text") => (
            format!("({}.target?.value ?? '').substring({}.target?.selectionStart ?? 0, {}.target?.selectionEnd ?? 0)", raw, raw, raw),
            "\"\"".into()
        ),

        // ResizeEvent
        (EventDataKind::ResizeEvent, "width") => ("window.innerWidth".into(), "0".into()),
        (EventDataKind::ResizeEvent, "height") => ("window.innerHeight".into(), "0".into()),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => (format!("{}.target?.currentTime", raw), "0".into()),
        (EventDataKind::MediaEvent, "duration") => (format!("{}.target?.duration", raw), "0".into()),

        // MediaLoadEvent
        (EventDataKind::MediaLoadEvent, "naturalWidth") => (format!("{}.target?.naturalWidth", raw), "0".into()),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => (format!("{}.target?.naturalHeight", raw), "0".into()),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => (format!("{}.target?.error?.message", raw), "\"unknown\"".into()),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => (format!("{}.animationName", raw), "\"\"".into()),
        (EventDataKind::AnimationEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => (format!("{}.propertyName", raw), "\"\"".into()),
        (EventDataKind::TransitionEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        _ => (format!("undefined /* unknown field {} for {:?} */", field, kind), "undefined".into()),
    }
}

/// Emit a JS object literal for event data extraction from a raw event variable.
/// Only emits fields in `used_fields` (or all fields from schema if None).
/// `pub(crate)` so other codegen targets (Vue, JS) can reuse the DOM-based mapping.
pub(crate) fn emit_event_object(
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
        Some(fields) if !fields.is_empty() => {
            // Only emit used fields, in schema order
            schema
                .iter()
                .filter(|f| fields.contains(&f.name))
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
        Some(_) => {
            // Empty vec — no fields used
            return "{}".to_string();
        }
        None => {
            // Conservative — emit all fields
            schema
                .iter()
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
    };

    format!("{{ {} }}", fields.join(", "))
}

/// Recursively walk a ViewTree to collect all DOM event types and EventDataKinds
/// used within the tree. Used by @for codegen for dynamic event subscription.
pub(super) fn collect_for_events(
    tree: &ViewTree,
    event_types: &mut Vec<String>,
    event_kinds: &mut Vec<(String, EventDataKind)>,
) {
    match tree {
        ViewTree::Element(el) => {
            for (event_name, handler) in &el.events {
                let js_event = map_event(event_name);
                if !event_types.contains(&js_event) {
                    event_types.push(js_event.clone());
                    if let Some(ref kind) = handler.event_data {
                        event_kinds.push((js_event.clone(), kind.clone()));
                    }
                }
            }
            for child in &el.children {
                collect_for_events(child, event_types, event_kinds);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_for_events(body, event_types, event_kinds);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                collect_for_events(fallback, event_types, event_kinds);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_for_events(body, event_types, event_kinds);
            }
            if let Some(ref empty) = list.empty_body {
                collect_for_events(empty, event_types, event_kinds);
            }
        }
        _ => {}
    }
}

/// Map omni binary operators to JS operators.
pub(super) fn map_binary_op(op: &str) -> &str {
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
        _ => op,
    }
}

/// Convert CamelCase to kebab-case for custom element tags.
pub(super) fn to_kebab(s: &str) -> String {
    crate::codegen::camel_to_separated(s, '-')
}

/// Escape HTML special characters.
pub(super) fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ══════════════════════════════════════════════════════════
// Codegen Dispatch Wrapper
// ══════════════════════════════════════════════════════════

/// Wrap OmniView output in a complete debug-ready HTML document.
/// Useful for loading directly in a browser for debugging / WASM preview.
pub fn wrap_debug_html(js: &str, css: &str, body_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OmniView Debug</title>
<style>
/* OmniView reset */
*, *::before, *::after {{ box-sizing: border-box; }}
body {{ margin: 0; padding: 8px; font-family: system-ui, sans-serif; }}
{css}
</style>
</head>
<body>
{body_html}
<script>
{js}
</script>
</body>
</html>"#
    )
}

/// Wrap `generate()` into the standard `CompileOutput` format.
///
/// Called by `super::compile()` when the target is `Target::OmniView`.
pub fn compile_files(
    ir: &crate::ir::IRProgram,
    _source_files: &HashMap<String, String>,
) -> CompileOutput {
    let output = super::generate(ir, false);
    // Wrap body HTML in a proper document that references the separate JS/CSS files
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OmniView App</title>
<link rel="stylesheet" href="index.css">
</head>
<body>
{}
<script src="index.js"></script>
</body>
</html>"#,
        output.html
    );
    CompileOutput::success(vec![
        CompiledFile {
            path: "index.js".into(),
            content: output.js,
        },
        CompiledFile {
            path: "index.html".into(),
            content: html,
        },
        CompiledFile {
            path: "body.html".into(),
            content: output.html,
        },
        CompiledFile {
            path: "index.css".into(),
            content: output.css,
        },
    ])
}
