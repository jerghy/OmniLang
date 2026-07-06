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
