//! Standalone utility functions for OmniView codegen.
use super::super::{CompileOutput, CompiledFile};
use super::OmniViewOutput;
use crate::ir::*;
use rustc_hash::FxHashMap;
pub(super) fn serde_value_to_js(value: &serde_json::Value) -> String {
    crate::codegen::format_json_value(value, &crate::codegen::JSON_OMNIVIEW)
}

// ══════════════════════════════════════════════════════════
// Mapping Helpers
// ══════════════════════════════════════════════════════════

/// Map omni component tags to HTML tags.
/// Delegates to the shared `to_html_tag` and then applies OmniView-specific
/// kebab-case conversion for custom components (web component style).
pub(super) fn map_tag(tag: &str) -> String {
    let base = crate::codegen::shared::html::to_html_tag(tag);
    // If the shared function returned the tag unchanged and it starts with uppercase,
    // convert to kebab-case for web component style (e.g., MyComponent → my-component).
    if base == tag && tag.chars().next().is_some_and(|c| c.is_uppercase()) {
        to_kebab(tag)
    } else {
        base
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
                    if let Some(kind) = handler.event_data() {
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
pub(super) fn map_binary_op(op: &BinaryOpIR) -> &str {
    use BinaryOpIR::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Mod => "%",
        Eq => "===",
        Neq => "!==",
        Lt => "<",
        Gt => ">",
        LtEq => "<=",
        GtEq => ">=",
        And => "&&",
        Or => "||",
        NullishCoalesce => "??",
        Assign => "=",
        AddAssign => "+=",
        SubAssign => "-=",
        MulAssign => "*=",
        DivAssign => "/=",
        ModAssign => "%=",
    }
}

/// Convert CamelCase to kebab-case for custom element tags.
pub(super) fn to_kebab(s: &str) -> String {
    crate::codegen::camel_to_separated(s, '-')
}

/// Parse an attribute template value containing {word} placeholders into parts.
/// E.g., "Item {name} (id: {id})" → [Static("Item "), Dynamic("name"), Static(" (id: "), Dynamic("id"), Static(")")]
pub(super) fn parse_attr_template(template: &str) -> Vec<super::AttrPart> {
    let mut parts = Vec::new();
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        if start > 0 {
            parts.push(super::AttrPart::Static(remaining[..start].to_string()));
        }
        let inner_start = start + 1;
        if let Some(end) = remaining[inner_start..].find('}') {
            let expr = remaining[inner_start..inner_start + end].to_string();
            parts.push(super::AttrPart::Dynamic(expr));
            remaining = &remaining[inner_start + end + 1..];
        } else {
            // Unterminated { — treat as static
            parts.push(super::AttrPart::Static(remaining.to_string()));
            remaining = "";
            break;
        }
    }
    if !remaining.is_empty() {
        parts.push(super::AttrPart::Static(remaining.to_string()));
    }
    parts
}

/// Escape HTML special characters.
pub(super) fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// ══════════════════════════════════════════════════════════
// Codegen Dispatch Wrapper
// ══════════════════════════════════════════════════════════

/// Wrap OmniView output in a complete debug-ready HTML document.
/// Useful for loading directly in a browser for debugging / WASM preview.
pub fn wrap_debug_html(js: &str, css: &str, body_html: &str) -> String {
    /// Embedded @omni/animation runtime (compiled into binary at build time).
    /// Path: workspace_root/packages/omni-animation/dist/omni-animation.js
    const OMNI_ANIMATION_JS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../packages/omni-animation/dist/omni-animation.js"
    ));
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
/// ─── @omni/animation v0.1.0 (embedded) ───
{animation_runtime}
/// ─── App code ───
{js}
</script>
</body>
</html>"#,
        animation_runtime = OMNI_ANIMATION_JS
    )
}

/// Wrap `generate()` into the standard `CompileOutput` format.
/// Build `CompileOutput` from an already-generated `OmniViewOutput`.
/// Separates generation from file-wrapping so callers can extract `components`.
pub fn build_file_output(gen: &OmniViewOutput) -> CompileOutput {
    /// Embedded @omni/animation runtime (compiled into binary at build time).
    const OMNI_ANIMATION_JS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../packages/omni-animation/dist/omni-animation.js"
    ));
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
<script src="omni-animation.js"></script>
<script src="index.js"></script>
</body>
</html>"#,
        gen.html
    );
    CompileOutput::success(vec![
        CompiledFile { path: "index.js".into(), content: gen.js.clone() },
        CompiledFile { path: "index.html".into(), content: html },
        CompiledFile { path: "body.html".into(), content: gen.html.clone() },
        CompiledFile { path: "index.css".into(), content: gen.css.clone() },
        CompiledFile { path: "omni-animation.js".into(), content: OMNI_ANIMATION_JS.to_string() },
    ])
}

///
/// Called by `super::compile()` when the target is `Target::OmniView`.
pub fn compile_files(
    ir: &crate::ir::IRProgram,
    _source_files: &FxHashMap<String, String>,
    editor_mode: bool,
    debug_mode: bool,
) -> CompileOutput {
    let output = super::generate(ir, editor_mode, false, debug_mode);
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
