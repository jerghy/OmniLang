//! Inline style emission: static CSS style attributes and dynamic signal-driven
//! style effects for `ElementIR` nodes.
//!
//! Contains `emit_element_inline_style` which processes `ElementIR.inline_style`
//! and writes either:
//! - Static `style="..."` attributes (for literal string values), or
//! - Deferred `__omni_effect(...)` JS calls (for `$signal`, `$field`, `$call`,
//!   `$concat` values), or
//! - Pre-compiled per-clone style bindings collected for `@for` templates
//!   (emitted inside the loop's `forEach`, where the loop variable is in
//!   scope — see `emit_list_dom` in list.rs).
//!
//! Style-channel JSON markers (设计文档 域 8, C2 fix):
//! - `$signal: "name"`      → reactive signal read (`name.value`)
//! - `$ident: "name"`       → bare identifier (loop vars, params, consts)
//! - `$field: [obj, field]` → REAL field access on `obj` (`obj.field`)
//! - `$call` / `$concat`    → function calls / concatenations, args resolved
//!   recursively through the same markers

use std::fmt::Write;

use super::super::utils::escape_html;
use super::super::OmniViewGenerator;
use crate::codegen::{camel_to_kebab, sanitize_css_value};
use crate::ir::*;

/// Build the style-assignment JS for the per-clone `@for` style bindings
/// (element variable is `_el` inside the loop's `forEach` closure). Same
/// shape as `OmniViewGenerator::emit_style_assignment` (store.rs), which
/// writes `el.style.<prop>=...` for top-level effects.
pub(crate) fn style_assignment_js(prop: &str, value_expr: &str) -> String {
    if prop.starts_with("--") {
        format!("_el.style.setProperty(\"{}\",{});", prop, value_expr)
    } else {
        format!("_el.style.{}={};", prop, value_expr)
    }
}

impl OmniViewGenerator {
    /// Translate a style-channel JSON value into a JS expression string.
    /// Handles `$signal`, `$ident`, `$field`, `$call`, `$concat` markers
    /// recursively — this is the OmniView counterpart of the shared
    /// `format_style_value_expr` (React/Vue).
    fn style_json_to_js(&mut self, val: &serde_json::Value) -> String {
        match val {
            serde_json::Value::String(s) => {
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                format!("\"{}\"", escaped)
            }
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Object(obj) => {
                if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
                    let prefixed = self
                        .name_resolver
                        .name_renames
                        .get(sig_name)
                        .or_else(|| self.name_resolver.global_renames.get(sig_name))
                        .cloned()
                        .unwrap_or_else(|| sig_name.to_string());
                    return format!("{}.value", prefixed);
                }
                if let Some(ident) = obj.get("$ident").and_then(|v| v.as_str()) {
                    return ident.to_string();
                }
                if let Some(field_arr) = obj.get("$field").and_then(|v| v.as_array()) {
                    if let (Some(obj_v), Some(field)) = (field_arr.first(), field_arr.get(1)) {
                        let obj_str = self.style_json_to_js(obj_v);
                        let field_str = field.as_str().unwrap_or_default();
                        let is_ident = !field_str.is_empty()
                            && field_str
                                .chars()
                                .next()
                                .map(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
                                .unwrap_or(false)
                            && field_str
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
                        if is_ident {
                            return format!("{}.{}", obj_str, field_str);
                        }
                        return format!("{}['{}']", obj_str, field_str);
                    }
                }
                if let Some(call_obj) = obj.get("$call").and_then(|v| v.as_object()) {
                    if let (Some(fn_name), Some(fn_args)) = (
                        call_obj.get("name").and_then(|v| v.as_str()),
                        call_obj.get("args").and_then(|v| v.as_array()),
                    ) {
                        let prefixed = self
                            .name_resolver
                            .global_renames
                            .get(fn_name)
                            .cloned()
                            .unwrap_or_else(|| {
                                format!("{}_{}", self.scope.current_component, fn_name)
                            });
                        let arg_strs: Vec<String> =
                            fn_args.iter().map(|a| self.style_json_to_js(a)).collect();
                        return format!("{}({})", prefixed, arg_strs.join(", "));
                    }
                }
                if let Some(parts) = obj.get("$concat").and_then(|v| v.as_array()) {
                    let js_parts: Vec<String> = parts
                        .iter()
                        .map(|p| self.style_json_to_js(p))
                        .collect();
                    if js_parts.is_empty() {
                        return String::new();
                    }
                    if js_parts.len() == 1 {
                        return js_parts[0].clone();
                    }
                    return format!("({})", js_parts.join(" + "));
                }
                String::new()
            }
            serde_json::Value::Bool(b) => b.to_string(),
            _ => String::new(),
        }
    }

    /// Emit inline style: static values are appended to `attrs`, dynamic signals
    /// (via `$signal`, `$field`, `$call`, `$concat`) generate deferred JS effects
    /// written to `self.js` — or, inside an `@for` template, are collected into
    /// `for_style_bindings` and emitted per-clone inside the loop's `forEach`
    /// (where the loop variable is in scope and the clone element exists).
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
                        serde_json::Value::Object(_) => {
                            // Dynamic style value → JS expression, emitted
                            // as a deferred effect (or a per-clone binding
                            // inside @for templates).
                            let js_expr = self.style_json_to_js(val);
                            if js_expr.is_empty() {
                                continue;
                            }
                            let js_prop = prop; // camelCase for el.style
                            if self.scope.in_for_template {
                                self.scope
                                    .for_style_bindings
                                    .push((nid.to_string(), js_prop.to_string(), js_expr));
                            } else {
                                let _ = write!(
                                    self.js,
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                    nid
                                );
                                self.emit_style_assignment(
                                    js_prop,
                                    &format!("__omni_sanitizeCss({})", js_expr),
                                );
                                self.js.push_str(" });\n");
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
