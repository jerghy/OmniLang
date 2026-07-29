//! Inline style emission: static CSS style attributes and dynamic signal-driven
//! style effects for `ElementIR` nodes.
//!
//! Contains `emit_element_inline_style` which processes `ElementIR.inline_style`
//! and writes either:
//! - Static `style="..."` attributes (for literal string values), or
//! - Deferred `__omni_effect(...)` JS calls (for `$signal`, `$concat`, `$call` values).

use std::fmt::Write;

use super::super::utils::escape_html;
use super::super::OmniViewGenerator;
use crate::codegen::{camel_to_kebab, sanitize_css_value};
use crate::ir::*;

impl OmniViewGenerator {
    /// Emit inline style: static values are appended to `attrs`, dynamic signals
    /// (via `$signal`, `$concat`, `$call`) generate deferred JS effects written
    /// to `self.js`.
    pub(super) fn emit_element_inline_style(
        &mut self,
        el: &ElementIR,
        nid: &str,
        attrs: &mut String,
    ) {
        self.assert_can_emit_js();
        // Inline style: { key: value, ... } — generate static style attr + dynamic JS effects
        if let Some(ref inline_style) = el.inline_style {
            if let Some(style_obj) = inline_style.as_object() {
                let mut static_style_parts: Vec<String> = Vec::new();
                for (prop, val) in style_obj {
                    match val {
                        serde_json::Value::Object(obj) if obj.contains_key("$signal") => {
                            // Dynamic style: signal reference → JS effect
                            if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
                                let prefixed = self
                                    .name_renames
                                    .get(sig_name)
                                    .or_else(|| self.global_renames.get(sig_name))
                                    .cloned()
                                    .unwrap_or_else(|| sig_name.to_string());
                                // Use camelCase for JS el.style, not kebab-case
                                let js_prop = prop;
                                let _ = write!(
                                    self.js,
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                    nid
                                );
                                self.emit_style_assignment(
                                    js_prop,
                                    &format!("__omni_sanitizeCss({}.value)", prefixed),
                                );
                                self.js.push_str(" });\n");
                            }
                        }
                        serde_json::Value::Object(obj) if obj.contains_key("$concat") => {
                            // Concatenated dynamic style → JS effect
                            if let Some(parts) = obj.get("$concat").and_then(|v| v.as_array()) {
                                let mut js_parts: Vec<String> = Vec::new();
                                for part in parts {
                                    if let Some(part_obj) = part.as_object() {
                                        if let Some(sig_name) =
                                            part_obj.get("$signal").and_then(|v| v.as_str())
                                        {
                                            let prefixed = self
                                                .name_renames
                                                .get(sig_name)
                                                .cloned()
                                                .unwrap_or_else(|| sig_name.to_string());
                                            js_parts.push(format!("{}.value", prefixed));
                                        }
                                    } else if let Some(s) = part.as_str() {
                                        js_parts.push(format!("\"{}\"", s));
                                    }
                                }
                                let js_prop = prop; // camelCase for el.style
                                let _ = write!(
                                    self.js,
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                    nid
                                );
                                self.emit_style_assignment(
                                    js_prop,
                                    &format!("__omni_sanitizeCss({})", js_parts.join(" + ")),
                                );
                                self.js.push_str(" });\n");
                            }
                        }
                        serde_json::Value::Object(obj) if obj.contains_key("$call") => {
                            // Dynamic style: function call → JS effect
                            if let Some(call_obj) = obj.get("$call") {
                                if let (Some(fn_name), Some(fn_args)) = (
                                    call_obj.get("name").and_then(|v| v.as_str()),
                                    call_obj.get("args").and_then(|v| v.as_array()),
                                ) {
                                    let prefixed =
                                        self.global_renames.get(fn_name).cloned().unwrap_or_else(
                                            || format!("{}_{}", self.current_component, fn_name),
                                        );
                                    let mut arg_strs: Vec<String> = Vec::new();
                                    for arg in fn_args {
                                        match arg {
                                            serde_json::Value::String(s) => {
                                                let escaped = s
                                                    .replace('\\', "\\\\")
                                                    .replace('"', "\\\"")
                                                    .replace('\n', "\\n")
                                                    .replace('\r', "\\r")
                                                    .replace('\t', "\\t");
                                                arg_strs.push(format!("\"{}\"", escaped));
                                            }
                                            _ => {
                                                arg_strs.push(arg.to_string());
                                            }
                                        }
                                    }
                                    let js_prop = prop; // camelCase for el.style
                                    let _ = write!(
                                        self.js,
                                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                        nid
                                    );
                                    self.emit_style_assignment(
                                        js_prop,
                                        &format!(
                                            "__omni_sanitizeCss({}({}))",
                                            prefixed,
                                            arg_strs.join(", ")
                                        ),
                                    );
                                    self.js.push_str(" });\n");
                                }
                            }
                        }
                        serde_json::Value::String(s) => {
                            static_style_parts.push(format!(
                                "{}: {}",
                                camel_to_kebab(prop),
                                sanitize_css_value(s)
                            ));
                        }
                        _ => {}
                    }
                }
                if !static_style_parts.is_empty() {
                    let _ = write!(
                        attrs,
                        " style=\"{}\"",
                        escape_html(&static_style_parts.join("; "))
                    );
                }
            }
        }
    }
}
