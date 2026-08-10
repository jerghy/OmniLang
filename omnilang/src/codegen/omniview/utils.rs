//! Standalone utility functions for OmniView codegen.
use super::super::{CodegenWarning, CompileOutput, CompiledFile};
use super::OmniViewOutput;
use crate::ir::*;
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
///
/// Alias resolution happens in the ONE shared table
/// (`shared::event::canonical_event_name`) — a misspelled alias
/// (onDoubleClick/onDblClick/doubleclick/...) can never wire a different
/// DOM event. The only backend-specific rule left here is the documented
/// merge of `change` into the `input` DOM event (per-keystroke semantics,
/// matching React's onChange shim for text fields).
///
/// `tag` 为归一化后的 HTML 标签（map_tag 结果，如 "select"）；`is_component`
/// 表示目标是否为组件实例元素。两个例外保留 `change`：
/// - `<select>`：其 input 事件先于 bind:value 写回用的 change 触发，handler
///   绑定 input 会读到写回前的旧信号值；
/// - 组件实例：emit 侧 dispatch 名 = 去 on 前缀（CustomEvent('change')），
///   监听 input 永不匹配（组件级 onChange 静默失效）。
pub(crate) fn map_event(name: &str, tag: &str, is_component: bool) -> String {
    match crate::codegen::shared::event::canonical_event_name(name) {
        // The DOM event is `dblclick` — naive on-prefix lowering gives
        // `doubleclick`, which never fires (historical drift bug).
        Some("change") if tag == "select" || is_component => "change".into(),
        Some("change") => "input".into(),
        Some(canonical) => canonical.to_string(),
        None if name.starts_with("on") => name[2..].to_lowercase(),
        None => name.to_string(),
    }
}

/// v4 3.3 (domain HIGH-1): canonical 事件名在 DOM 无对应事件
/// （hover/longpress 为 Flutter 便捷形状混入 canonical 表——
/// 登记册域 1 借词纪律的变体）。map_event 对它们直通发射，产生永不被
/// 触发的死监听器。返回警告文本；其余 canonical 返回 None（map_event
/// 正常处理）。
///
/// Task 6.1b (gate FAIL-2): toggle 是 `<details>` 的真实 DOM 事件——
/// details 上 `addEventListener("toggle")` 会触发，不跳过（map_event
/// 正常处理）；其他元素（tag != "details"，tag 为归一化后 HTML 标签）
/// 与 hover/longpress 同族跳过。
pub(crate) fn event_missing_js_mapping(event_name: &str, tag: &str) -> Option<String> {
    let canonical = crate::codegen::shared::event::canonical_event_name(event_name)?;
    match canonical {
        "hover" | "longpress" => Some(format!(
            "事件 '@on{canonical}' 在 OmniView/DOM 无对应事件——处理器已跳过（Flutter 提供 onLongPress/onHover/onChanged 映射）"
        )),
        // toggle: 真实 DOM 事件仅 <details> 元素存在。
        "toggle" if tag != "details" => Some(format!(
            "事件 '@on{canonical}' 在 OmniView/DOM 仅 <details> 元素有对应事件——其他元素上处理器已跳过（Flutter 提供 onChanged 映射）"
        )),
        _ => None,
    }
}

/// CSS 属性需要 px 单位的集合（与 @omni/animation 运行时的 PX_PROPS 对齐）。
///
/// style 绑定直写通道（`style: { width: sig }` → `el.style.width = ...`）对
/// 数值信号（f64）必须附加 px——`el.style.width = 200` 无单位时浏览器忽略
/// 该声明（width 是长度属性，无单位值非法）。动画的 buildProperties 已在
/// 运行时处理 px，直写通道在编译期用 `__omni_px()` 包装（数值→px，字符串
/// 原样）。
pub(super) fn is_px_style_prop(prop: &str) -> bool {
    matches!(
        prop,
        "width"
            | "height"
            | "margin"
            | "padding"
            | "borderRadius"
            | "borderWidth"
            | "marginTop"
            | "marginRight"
            | "marginBottom"
            | "marginLeft"
            | "paddingTop"
            | "paddingRight"
            | "paddingBottom"
            | "paddingLeft"
            | "borderTopLeftRadius"
            | "borderTopRightRadius"
            | "borderBottomLeftRadius"
            | "borderBottomRightRadius"
            | "fontSize"
            | "top"
            | "left"
            | "right"
            | "bottom"
            | "minWidth"
            | "minHeight"
            | "maxWidth"
            | "maxHeight"
            | "gap"
            | "rowGap"
            | "columnGap"
            | "letterSpacing"
            | "lineHeight"
    )
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
console.warn(
  '%cOmniView Debug Mode%c\n' +
  'This page was generated in DEBUG mode and has NO Content-Security-Policy.\n' +
  'Do NOT deploy this HTML to production. Use the React production backend instead.',
  'font-weight: bold; font-size: 14px;', ''
);
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

/// Shared HTML document wrapper used by both `OmniViewCodegen::generate` and
/// `build_file_output`. Produces a standalone HTML document that references
/// separate JS/CSS files.
///
/// When `include_animation_script` is true, includes an additional
/// `<script src="omni-animation.js">` tag before the main `index.js` script.
///
/// When `include_csp` is true, inserts a Content-Security-Policy meta tag
/// after the charset declaration. Editor mode should pass `false` for CSP
/// (it relies on iframe sandbox instead).
pub(super) fn html_document_wrapper(
    body_html: &str,
    include_animation_script: bool,
    include_csp: bool,
) -> String {
    let script_tags = if include_animation_script {
        r#"<script src="omni-animation.js"></script>
<script src="index.js"></script>"#
    } else {
        r#"<script src="index.js"></script>"#
    };
    let csp_meta = if include_csp {
        r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self'; base-uri 'self'; frame-ancestors 'self'; form-action 'self'; object-src 'none'; frame-src 'none';">"
"#
    } else {
        r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'self';">"
"#
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
{}<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OmniView App</title>
<link rel="stylesheet" href="index.css">
</head>
<body>
{}
{}
</body>
</html>"#,
        csp_meta, body_html, script_tags
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
    let html = html_document_wrapper(&gen.html, true, true);
    let mut files = vec![
        CompiledFile {
            path: "index.js".into(),
            content: gen.js.clone(),
        },
        CompiledFile {
            path: "index.html".into(),
            content: html,
        },
        CompiledFile {
            path: "body.html".into(),
            content: gen.html.clone(),
        },
        CompiledFile {
            path: "index.css".into(),
            content: gen.css.clone(),
        },
        CompiledFile {
            path: "omni-animation.js".into(),
            content: OMNI_ANIMATION_JS.to_string(),
        },
    ];
    // Conditionally add the .d.ts file if types were generated
    if !gen.types.is_empty() {
        files.push(CompiledFile {
            path: "index.d.ts".into(),
            content: gen.types.clone(),
        });
    }
    // Forward generator diagnostics (unknown attribute, undefined class,
    // unknown variant, ...) into the standard CompileOutput warning channel
    // so the CLI surfaces them (previously dropped — `CompileOutput::success`
    // starts with an empty warning list, see compile_multi_internal_post).
    let mut output = CompileOutput::success(files);
    output.warnings = gen
        .warnings
        .iter()
        .map(|w| CodegenWarning {
            message: w.clone(),
            file: None,
            line: 0,
            col: 0,
        })
        .collect();
    output
}
