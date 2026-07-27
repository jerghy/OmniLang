//! Element rendering: HTML element generation with props, events,
//! bindings, inline styles, motion triggers, and element tree recording.
//!
//! The primary entry point is `element_to_html`, which converts an `ElementIR`
//! into its full HTML representation (including deferred JS bindings written
//! to `self.js` / `self.mount_js` etc.).

use rustc_hash::FxHashMap;
use std::fmt::Write;

use super::super::utils::{escape_html, map_event, map_tag, parse_attr_template};
use super::super::ElementTreeNode;
use super::super::OmniViewGenerator;
use crate::codegen::emit_event_object;
use crate::codegen::{camel_to_kebab, sanitize_css_value};
use crate::ir::*;
use crate::sourcemap::{SourceMapEntry, SourceSpan, TargetSpan, TargetSpanKind};

impl OmniViewGenerator {
    // ═══════════════════════════════════════════════════════════════
    // Element Tree Helpers
    // ═══════════════════════════════════════════════════════════════

    /// Begin recording a tree node. Called when we emit an opening tag.
    pub(super) fn tree_push(&mut self, nid: String, tag: String) {
        self.element_tree_stack.push(ElementTreeNode {
            nid,
            tag,
            text: None,
            children: Vec::new(),
        });
    }

    /// Add a text leaf node to the current parent's children.
    pub(super) fn tree_text(&mut self, nid: String, text: String) {
        let node = ElementTreeNode {
            nid,
            tag: "#text".to_string(),
            text: Some(text),
            children: Vec::new(),
        };
        if let Some(parent) = self.element_tree_stack.last_mut() {
            parent.children.push(node);
        } else {
            self.element_tree.push(node);
        }
    }

    /// Finish recording a tree node. Pops the stack and appends to parent.
    pub(super) fn tree_pop(&mut self) {
        if let Some(node) = self.element_tree_stack.pop() {
            if let Some(parent) = self.element_tree_stack.last_mut() {
                parent.children.push(node);
            } else {
                self.element_tree.push(node);
            }
        }
    }

    /// Allocate the next node ID (nid) for a given HTML tag.
    ///
    /// `pub(crate)` so that `editor.rs` (outside `view/`) can call it
    /// via `self.next_nid()`. Sibling modules within `view/` also see it.
    pub(crate) fn next_nid(&mut self, tag: &str) -> String {
        self.node_counter += 1;
        format!("el-{}-{:03}", tag, self.node_counter)
    }

    // ═══════════════════════════════════════════════════════════════
    // Element → HTML
    // ═══════════════════════════════════════════════════════════════

    /// Extract element attributes and class names from an `ElementIR`.
    ///
    /// Returns `(attrs_string, class_name_parts)` where:
    /// - `attrs_string` contains the static HTML attributes (starting with `data-nid=...`)
    /// - `class_name_parts` contains individual class names from both `class` prop
    ///   and design variant expansion.
    ///
    /// Side-effects:
    /// - Writes deferred JS (reactive attribute bindings, signal-to-DOM effects) to `self.js`.
    /// - Records pre-compiled @for attribute bindings in `self.for_attr_bindings`.
    /// - Pushes warnings for unknown/unresolved design variants and class references.
    fn emit_element_props(&mut self, el: &ElementIR, nid: &str) -> (String, Vec<String>) {
        let html_tag = map_tag(&el.tag);
        // Build attributes
        let mut attrs = format!(" data-nid=\"{}\"", nid);
        // Collect class names separately (from props + variant expansion) so
        // we can emit a single merged `class="..."` attribute at the end.
        let mut class_name_parts: Vec<String> = Vec::new();

        // Static props (skip synthetic/internal props prefixed with '_')
        for (name, value) in &el.props {
            if name.starts_with('_') {
                continue; // skip internal/synthetic props
            }
            match name.as_str() {
                "testId" | "test_id" => {
                    let _ = write!(attrs, " data-testid=\"{}\"", self.expr_to_static_str(value));
                }
                // class is collected separately — merged with variant classes at the end
                "class" => {
                    let class_str = self.expr_to_static_str(value);
                    if !class_str.is_empty() {
                        class_name_parts.extend(class_str.split_whitespace().map(String::from));
                    }
                }
                // Real HTML attributes — emit as-is, not data-*
                "id" | "placeholder" | "type" | "value" | "disabled" | "checked" | "required"
                | "readonly" | "href" | "src" | "alt" | "title" | "name" | "rows" | "cols"
                | "max" | "min" | "step" | "autocomplete" | "autofocus" | "multiple"
                | "pattern" | "selected" | "for" | "action" | "method" | "enctype" | "accept"
                | "controls" | "width" | "height" | "coords" | "shape" | "media" | "srcset"
                | "sizes" | "target" | "rel" | "lang" | "dir" | "datetime" | "cite" | "list"
                | "loop" | "muted" | "poster" | "preload" | "tabindex" | "role" | "content"
                | "charset" | "http-equiv" | "property" | "itemprop" | "crossorigin"
                | "integrity" | "referrerpolicy" | "loading" | "decoding" | "async" | "defer"
                | "nonce" | "download" | "draggable" | "spellcheck" | "contenteditable"
                | "translate" | "autocapitalize" | "capture" => {
                    let val_str = self.expr_to_static_str(value);
                    // Known boolean attributes with value "true" — emit bare attribute name
                    let is_boolean_attr = matches!(
                        name.as_str(),
                        "disabled"
                            | "checked"
                            | "required"
                            | "readonly"
                            | "selected"
                            | "autofocus"
                            | "multiple"
                            | "controls"
                            | "loop"
                            | "muted"
                            | "async"
                            | "defer"
                    );
                    // Dynamic attributes (identifier references): emit reactive binding
                    if val_str.starts_with('{') && val_str.ends_with('}') {
                        // Dynamic attribute: bind reactively
                        let js_val = self.expr_to_js(value);
                        let js_access = if self.signal_names.contains(&js_val) {
                            format!("{}.value", js_val)
                        } else {
                            js_val
                        };
                        if is_boolean_attr {
                            // Boolean attributes must use the DOM property (e.g. el.checked = bool)
                            // rather than setAttribute (which treats all values as strings — "false"
                            // is truthy in HTML and won't uncheck the checkbox).
                            let _ = writeln!(self.js,
                                "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.{} = !!({}); }});",
                                nid, name, js_access
                            );
                        } else {
                            let _ = writeln!(self.js,
                                "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.setAttribute('{}', {}); }});",
                                nid, name, js_access
                            );
                        }
                    } else if is_boolean_attr && val_str == "true" {
                        // Boolean attribute: emit just the attribute name (no ="true")
                        let _ = write!(attrs, " {}", name);
                    } else if is_boolean_attr && val_str == "false" {
                        // Boolean attribute with value "false": skip entirely (don't emit)
                    } else if !val_str.is_empty() {
                        let _ = write!(attrs, " {}=\"{}\"", name, escape_html(&val_str));
                    } else {
                        let _ = write!(attrs, " {}", name);
                    }
                }
                _ => {
                    // Emit as data attribute for unknown props,
                    // but preserve aria-* and data-* attributes as-is.
                    let val_str = self.expr_to_static_str(value);
                    if val_str.is_empty() {
                        continue;
                    }
                    if self.in_for_template && (val_str.contains('{') && val_str.contains('}')) {
                        // Pre-compiled attribute binding for @for templates
                        let attr_name = if name.starts_with("aria-") || name.starts_with("data-") {
                            name.clone()
                        } else {
                            format!("data-{}", name)
                        };
                        let parts = parse_attr_template(&val_str);
                        self.for_attr_bindings
                            .push((nid.to_string(), attr_name, parts));
                    } else if name.starts_with("aria-") || name.starts_with("data-") {
                        // Real HTML aria/data attributes — emit as-is
                        let _ = write!(attrs, " {}=\"{}\"", name, escape_html(&val_str));
                    } else {
                        let _ = write!(attrs, " data-{}=\"{}\"", name, escape_html(&val_str));
                    }
                }
            }
        }

        // ── Expand design variant binding into CSS class names ──
        // Variant bindings are stored as a synthetic `_design_variant` prop
        // during IR lowering, encoded as a JSON object.
        if let Some(ExprIR::LiteralExpr { value }) = el.props.get("_design_variant") {
            if let (Some(design_name), Some(variant_name), Some(dim_values)) = (
                value.get("design_name").and_then(|v| v.as_str()),
                value.get("variant_name").and_then(|v| v.as_str()),
                value.get("dimension_values"),
            ) {
                    let dim_map: FxHashMap<String, String> = dim_values
                        .as_object()
                        .map(|obj| {
                            obj.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect()
                        })
                        .unwrap_or_default();

                    if let Some(design) = self.design_map.get(design_name) {
                        match crate::codegen::omniview::design::expand_variant(
                            design,
                            variant_name,
                            &dim_map,
                        ) {
                            Ok((variant_classes, _extra_css)) => {
                                class_name_parts.extend(variant_classes);
                            }
                            Err(e) => {
                                self.warnings.push(format!(
                                    "variant '{}' not found in design '{}': {}",
                                    variant_name, design_name, e
                                ));
                            }
                        }
                    } else {
                        self.warnings.push(format!(
                            "design '{}' not found for variant binding",
                            design_name
                        ));
                    }
            }
        }

        // ── Validate class names against design_map + component styles ──
        for class_name in &class_name_parts {
            // Skip empty names
            if class_name.is_empty() {
                continue;
            }
            // Check component-local style classes
            if self.component_style_classes.contains(class_name) {
                continue;
            }
            // Check all registered design blocks for this class
            let in_design = self
                .design_map
                .values()
                .any(|d| d.classes.iter().any(|c| c.name == *class_name));
            if in_design {
                continue;
            }
            // Not found anywhere — warn
            self.warnings.push(format!(
                "class '{}' referenced on <{}> (nid={}) is not defined in any design block or component style",
                class_name, html_tag, nid
            ));
        }

        (attrs, class_name_parts)
    }

    /// Emit inline style: static values are appended to `attrs`, dynamic signals
    /// (via `$signal`, `$concat`, `$call`) generate deferred JS effects written
    /// to `self.js`.
    fn emit_element_inline_style(&mut self, el: &ElementIR, nid: &str, attrs: &mut String) {
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
                                let _ =
                                    write!(self.js,
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                    nid
                                );
                                self.emit_style_assignment(js_prop, &format!("{}.value", prefixed));
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
                                let _ =
                                    write!(self.js,
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                    nid
                                );
                                self.emit_style_assignment(js_prop, &js_parts.join(" + "));
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
                                    let _ = write!(self.js,
                                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                        nid
                                    );
                                    self.emit_style_assignment(
                                        js_prop,
                                        &format!("{}({})", prefixed, arg_strs.join(", ")),
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

    /// Emit event bindings: for @for templates, emit data-event-* attributes;
    /// for normal mode, generate addEventListener JS calls.
    fn emit_element_events(&mut self, el: &ElementIR, nid: &str, attrs: &mut String) {
        for (event_name, handler) in &el.events {
            let js_event = map_event(event_name);
            if self.in_for_template {
                // Inside @for template: emit data-event-* attributes for delegation.
                // The @for container gets a single delegated listener that reads these.
                let val = match handler {
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        let prefixed = format!("{}_{}", self.current_component, r#ref);
                        format!("fn:{}", escape_html(&prefixed))
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        let var_name = r#ref.replace('.', "_");
                        format!("store:{}", escape_html(&var_name))
                    }
                    EventHandlerIR::InlineFn { body, params, .. } => {
                        let wrapper_name = format!("__on{}_{}", event_name, nid.replace('-', "_"));
                        let params_str = params.join(", ");
                        let _ = writeln!(self.js, "function {}({}) {{", wrapper_name, params_str);
                        for stmt in &body.statements {
                            self.emit_statement(stmt);
                        }
                        self.js.push_str("}\n");
                        format!("fn:{}", escape_html(&wrapper_name))
                    }
                };
                let _ = write!(attrs, " data-event-{}=\"{}\"", js_event, val);
                // Register handler in the global lookup so the delegated
                // listener can find it (functions are IIFE-local, not on
                // window).
                let handler_name = match handler {
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        Some(format!("{}_{}", self.current_component, r#ref))
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => Some(r#ref.replace('.', "_")),
                    EventHandlerIR::InlineFn { .. } => {
                        Some(format!("__on{}_{}", event_name, nid.replace('-', "_")))
                    }
                };
                if let Some(ref hname) = handler_name {
                    let _ = writeln!(self.js, "__omni_forHandlers[\"{}\"] = {};", hname, hname);
                }
            } else {
                // ResizeEvent binds to window, not el.addEventListener
                if let Some(EventDataKind::ResizeEvent) = handler.event_data() {
                    let event_obj =
                        emit_event_object(handler.event_data(), handler.used_fields(), "_e");
                    let handler_call = match handler {
                        EventHandlerIR::FnRef { r#ref, .. } => {
                            let prefixed = format!("{}_{}", self.current_component, r#ref);
                            format!("{}({})", prefixed, event_obj)
                        }
                        EventHandlerIR::StoreAction { r#ref, .. } => {
                            let var_name = r#ref.replace('.', "_");
                            format!("{}()", var_name)
                        }
                        EventHandlerIR::InlineFn { body, params, .. } => {
                            let wrapper_name =
                                format!("__on{}_{}", event_name, nid.replace('-', "_"));
                            let params_str = params.join(", ");
                            let _ =
                                writeln!(self.js, "function {}({}) {{", wrapper_name, params_str);
                            for stmt in &body.statements {
                                self.emit_statement(stmt);
                            }
                            self.js.push_str("}\n");
                            format!("{}({})", wrapper_name, event_obj)
                        }
                    };
                    let _ = writeln!(
                        self.js,
                        "{{ window.addEventListener(\"resize\", (_e) => {{ {} }}); }}",
                        handler_call
                    );
                    continue;
                }
                let pd = if handler.needs_prevent_default() || js_event == "submit" {
                    " e.preventDefault();"
                } else {
                    ""
                };
                let sp = if handler.needs_stop_propagation() {
                    " e.stopPropagation();"
                } else {
                    ""
                };
                match handler {
                    EventHandlerIR::FnRef { r#ref, .. } => {
                        if self.motion_names.contains(r#ref.as_str()) {
                            // Motion reference: call factory and play on event
                            let _ = writeln!(self.js,
                                "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ if (el.offsetParent !== null) {{ var p = __omni_motion_{motion}(el, {{}}); __omni_animations.set(\"{nid}_{js_event}\", p); p.play(); }} }}); }}",
                                nid = nid,
                                js_event = js_event,
                                motion = r#ref,
                            );
                        } else {
                            let prefixed = format!("{}_{}", self.current_component, r#ref);
                            let event_obj =
                                emit_event_object(handler.event_data(), handler.used_fields(), "e");
                            let _ = writeln!(self.js,
                                "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {prefixed}({event_obj}); }}); }}",
                                nid = nid, js_event = js_event, sp = sp, pd = pd, prefixed = prefixed, event_obj = event_obj
                            );
                        }
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        let var_name = r#ref.replace('.', "_");
                        let _ = writeln!(self.js,
                            "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ {} {} {}(); }}); }}",
                            nid, js_event, sp, pd, var_name
                        );
                    }
                    EventHandlerIR::InlineFn { body, params, .. } => {
                        let wrapper_name = format!("__on{}_{}", event_name, nid.replace('-', "_"));
                        let params_str = params.join(", ");
                        let _ = writeln!(self.js, "function {}({}) {{", wrapper_name, params_str);
                        for stmt in &body.statements {
                            self.emit_statement(stmt);
                        }
                        self.js.push_str("}\n");
                        let event_obj =
                            emit_event_object(handler.event_data(), handler.used_fields(), "e");
                        let _ = writeln!(self.js,
                            "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {wrapper_name}({event_obj}); }}); }}",
                            nid = nid, js_event = js_event, sp = sp, pd = pd, wrapper_name = wrapper_name, event_obj = event_obj
                        );
                    }
                }
            }
        }
    }

    /// Emit two-way bindings (bind:value / bind:checked).
    ///
    /// For each binding on `value` or `checked`, this emits a guard variable and
    /// two pieces of JS:
    /// 1. A DOM event listener that writes the new value back to the signal.
    /// 2. A reactive effect that reads the signal and updates the DOM, protected
    ///    by the guard flag to prevent infinite loops.
    fn emit_element_bindings(&mut self, el: &ElementIR, nid: &str) {
        for (prop_name, binding) in &el.bindings {
            if prop_name == "value" || prop_name == "checked" {
                // Resolve signal name through renames (component-prefixed or global)
                let sig_name = self
                    .name_renames
                    .get(&binding.name)
                    .or_else(|| self.global_renames.get(&binding.name))
                    .cloned()
                    .unwrap_or_else(|| binding.name.clone());
                // Prevent infinite loop: guard flag to stop the effect from re-triggering input event
                self.guard_counter += 1;
                let guard_name = format!("_guard_{}_{}", prop_name, nid.replace('-', "_"));
                let _ = writeln!(self.js, "let {} = false;", guard_name);
                let (event_name, dom_prop) = if prop_name == "checked" {
                    ("change", "checked")
                } else {
                    ("input", "value")
                };
                let _ = writeln!(self.js,
                    "{{ const el = __omni_el(\"{0}\"); if (el) el.addEventListener(\"{1}\", (e) => {{ if (!{2}) {{ __omni_subs.get(\"{3}\").set(e.target.{4}); }} }}); }}",
                    nid, event_name, guard_name, sig_name, dom_prop
                );
                // Reactive binding: signal → DOM (guard against programmatic updates)
                let _ = writeln!(self.js,
                    "__omni_effect(() => {{ const el = __omni_el(\"{0}\"); if (el) {{ {1} = true; el.{2} = __omni_subs.get(\"{3}\").value; {1} = false; }} }});",
                    nid, guard_name, prop_name, sig_name
                );
            }
        }
    }

    /// Emit motion trigger data-* attributes and sub-attributes (in-view, scroll, drag).
    ///
    /// Writes HTML data attributes for @enter, @hover, @press, @focus, @leave triggers
    /// and their sub-attributes (data-in-view-*, data-scroll-*, data-drag-*) to `attrs`,
    /// and calls `emit_motion_triggers_for_element` for deferred JS setup.
    fn emit_element_motion_attrs(&mut self, el: &ElementIR, nid: &str, attrs: &mut String) {
        if el.motion_behavior().is_some_and(|m| !m.triggers.is_empty()) {
            // Also emit HTML data attributes for each trigger so runtime can discover them
            for trigger in el
                .motion_behavior()
                .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
            {
                let data_attr = match trigger.kind {
                    TriggerKind::Enter => "data-enter",
                    TriggerKind::Leave => "data-leave",
                    TriggerKind::Hover => "data-hover",
                    TriggerKind::Unhover => "data-unhover",
                    TriggerKind::Press => "data-press",
                    TriggerKind::Focus => "data-focus",
                    TriggerKind::Animate => "data-animate",
                    TriggerKind::InView => "data-in-view",
                    TriggerKind::Scroll => "data-scroll",
                    TriggerKind::DragHorizontal => "data-drag-horizontal",
                    TriggerKind::DragVertical => "data-drag-vertical",
                    TriggerKind::Timeline => "data-timeline",
                };
                if let Some(ref motion_name) = trigger.motion_ref {
                    let _ = write!(attrs, " {}=\"{}\"", data_attr, escape_html(motion_name));
                }
            }
            // Sub-attributes for in-view
            if let Some(once) = el.motion_behavior().and_then(|m| m.in_view_once.as_ref()) {
                let js_val = self.expr_to_js(once);
                let _ = write!(attrs, " data-in-view-once=\"{}\"", escape_html(&js_val));
            }
            if let Some(repeat) = el.motion_behavior().and_then(|m| m.in_view_repeat.as_ref()) {
                let js_val = self.expr_to_js(repeat);
                let _ = write!(attrs, " data-in-view-repeat=\"{}\"", escape_html(&js_val));
            }
            if let Some(threshold) = el
                .motion_behavior()
                .and_then(|m| m.in_view_threshold.as_ref())
            {
                let js_val = self.expr_to_js(threshold);
                let _ = write!(
                    attrs,
                    " data-in-view-threshold=\"{}\"",
                    escape_html(&js_val)
                );
            }
            // Sub-attributes for scroll
            if let Some(s) = el.scroll_behavior().and_then(|sb| sb.start.as_ref()) {
                let _ = write!(attrs, " data-scroll-start=\"{}\"", escape_html(s));
            }
            if let Some(s) = el.scroll_behavior().and_then(|sb| sb.end.as_ref()) {
                let _ = write!(attrs, " data-scroll-end=\"{}\"", escape_html(s));
            }
            if let Some(scrub) = el.scroll_behavior().and_then(|sb| sb.scrub.as_ref()) {
                let js_val = self.expr_to_js(scrub);
                let _ = write!(attrs, " data-scroll-scrub=\"{}\"", escape_html(&js_val));
            }
            // Sub-attributes for drag
            if let Some(s) = el.drag_behavior().and_then(|db| db.constraints.as_ref()) {
                let _ = write!(attrs, " data-drag-constraints=\"{}\"", escape_html(s));
            }
            if let Some(elastic) = el.drag_behavior().and_then(|db| db.elastic.as_ref()) {
                let js_val = self.expr_to_js(elastic);
                let _ = write!(attrs, " data-drag-elastic=\"{}\"", escape_html(&js_val));
            }
            if let Some(scrub) = el.drag_behavior().and_then(|db| db.scrub.as_ref()) {
                let js_val = self.expr_to_js(scrub);
                let _ = write!(attrs, " data-drag-scrub=\"{}\"", escape_html(&js_val));
            }
            if let Some(momentum) = el.drag_behavior().and_then(|db| db.momentum.as_ref()) {
                let js_val = self.expr_to_js(momentum);
                let _ = write!(attrs, " data-drag-momentum=\"{}\"", escape_html(&js_val));
            }
            if let Some(threshold) = el.drag_behavior().and_then(|db| db.threshold.as_ref()) {
                let js_val = self.expr_to_js(threshold);
                let _ = write!(attrs, " data-drag-threshold=\"{}\"", escape_html(&js_val));
            }
            if let Some(vel) = el.drag_behavior().and_then(|db| db.velocity_max.as_ref()) {
                let js_val = self.expr_to_js(vel);
                let _ = write!(
                    attrs,
                    " data-drag-velocity-max=\"{}\"",
                    escape_html(&js_val)
                );
            }

            self.emit_motion_triggers_for_element(nid, el);
        }
    }

    /// Emit motion bindings — @driven-by external progress control,
    /// @paused per-element pause override, and states: animation map.
    ///
    /// These are deferred JS bindings written to `self.mount_js` that connect
    /// runtime animation state to signals and external controllers.
    fn emit_element_motion_bindings(&mut self, el: &ElementIR, nid: &str) {
        // @driven-by — external progress control for animations on this element
        if let Some(driven_by_expr) = el.motion_behavior().and_then(|m| m.driven_by.as_ref()) {
            let driven_js = self.expr_to_js(driven_by_expr);
            // When driven-by is set, the animation progress is bound to the signal value.
            // We seek ALL animations on this element whenever the signal changes.
            // The signal is auto-tracked by __omni_effect.
            let _ = write!(
                self.mount_js,
                "// @driven-by binding\n\
                 __omni_effect(() => {{\n\
                 \x20 const progress = Math.max(0, Math.min(1, Number({driven})));\n\
                 \x20 __omni_animations.forEach((anim, key) => {{\n\
                 \x20\x20 if (key.startsWith(\"{nid}_\")) {{\n\
                 \x20\x20\x20 var dur = (anim.effect && anim.effect.getTiming && anim.effect.getTiming().duration) || 300;\n\
                 \x20\x20\x20 anim.currentTime = progress * dur;\n\
                 \x20\x20 }}\n\
                 \x20 }});\n\
                 }});\n",
                nid = nid,
                driven = driven_js
            );
        }

        // @paused override — per-element pause override for all animations
        if let Some(paused) = el
            .motion_behavior()
            .and_then(|m| m.paused_override.as_ref())
        {
            let paused_js = self.expr_to_js(paused);
            let _ = write!(
                self.mount_js,
                "// @paused override\n\
                 (function() {{\n\
                 \x20 var _paused = Boolean({paused});\n\
                 \x20 __omni_animations.forEach((anim, key) => {{\n\
                 \x20\x20 if (key.startsWith(\"{nid}_\")) {{\n\
                 \x20\x20\x20 if (_paused) {{ anim.pause(); }} else {{ anim.play(); }}\n\
                 \x20\x20 }}\n\
                 \x20 }});\n\
                 }})();\n",
                nid = nid,
                paused = paused_js
            );
        }

        // states: — multi-state animation map (reserved or custom signal-driven)
        if let Some(states_ir) = el.motion_behavior().and_then(|m| m.states.as_ref()) {
            self.emit_states_js(nid, states_ir);
        }
    }

    /// Emit the HTML closing portion for an element: handle void tags, fragment
    /// wrappers, normal tags with children, css_only mode, interpolation effects,
    /// class map dynamic bindings, and source map recording.
    ///
    /// This is the second half of `element_to_html`, extracted as its own method.
    #[allow(clippy::too_many_arguments)]
    fn emit_element_html_close(
        &mut self,
        el: &ElementIR,
        nid: &str,
        html_tag: &str,
        attrs: &str,
        class_name_parts: &[String],
        interp_effects: &[String],
        indent: usize,
    ) -> String {
        let pad = "  ".repeat(indent);
        let mut out = String::new();

        // Self-closing tags (after motion/driven-by/states emit so animations are set up)
        let void_tags = [
            "br", "hr", "img", "input", "meta", "link", "area", "base", "col", "embed", "source",
            "track", "wbr",
        ];
        if void_tags.contains(&html_tag) {
            if self.css_only {
                let design_id = el
                    .props
                    .get("id")
                    .and_then(|v| {
                        if let ExprIR::LiteralExpr { value } = v {
                            value.as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| nid.to_string());
                // Resolve design class references ("AppDesign.card" → "card")
                let resolved_classes: Vec<String> = class_name_parts
                    .iter()
                    .map(|c| {
                        if let Some(dot_pos) = c.find('.') {
                            c[dot_pos + 1..].to_string()
                        } else {
                            c.clone()
                        }
                    })
                    .collect();
                self.element_classes
                    .insert(nid.to_string(), resolved_classes);
                self.element_tags
                    .insert(nid.to_string(), html_tag.to_string());
                // Record nid → design element ID mapping for editor selection
                self.nid_map.insert(nid.to_string(), design_id);
                self.tree_pop();
                return String::new();
            }
            let _ = writeln!(out, "{}<{}{} />", pad, html_tag, attrs);
            // Emit any interpolation _effect() calls
            for effect in interp_effects {
                self.js.push_str(effect);
            }
            // Class map dynamic bindings
            if !self.in_for_template && !el.class_map_bindings.is_empty() {
                for (class_name, expr) in &el.class_map_bindings {
                    let js_expr = self.expr_to_js(expr);
                    let js_access = if self.signal_names.contains(&js_expr) {
                        format!("{}.value", js_expr)
                    } else {
                        js_expr
                    };
                    let _ = writeln!(self.js,
                            "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.classList.toggle(\"{}\", !!({})); }});",
                            nid, class_name, js_access
                        );
                }
            }
            self.tree_pop();
            return out;
        }

        // For <fragment> wrapper inside @for template: emit children without wrapper.
        if self.in_for_template && html_tag == "fragment" {
            for child in &el.children {
                let child_html = self.view_to_html(child, indent + 1);
                out.push_str(&child_html);
            }
            self.tree_pop(); // fragment has no wrapper, pop after children
        } else {
            let _ = writeln!(out, "{}<{}{}>", pad, html_tag, attrs);

            // Children
            for child in &el.children {
                let child_html = self.view_to_html(child, indent + 1);
                out.push_str(&child_html);
            }

            // In css_only mode: capture element mappings now that children are done,
            // then return empty string (no HTML/JS needed).
            if self.css_only {
                let design_id = el
                    .props
                    .get("id")
                    .and_then(|v| {
                        if let ExprIR::LiteralExpr { value } = v {
                            value.as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| nid.to_string());
                // Resolve design class references ("AppDesign.card" → "card")
                let resolved_classes: Vec<String> = class_name_parts
                    .iter()
                    .map(|c| {
                        if let Some(dot_pos) = c.find('.') {
                            c[dot_pos + 1..].to_string()
                        } else {
                            c.clone()
                        }
                    })
                    .collect();
                self.element_classes
                    .insert(nid.to_string(), resolved_classes);
                self.element_tags
                    .insert(nid.to_string(), html_tag.to_string());
                self.nid_map.insert(nid.to_string(), design_id);
                self.tree_pop();
                return String::new();
            }

            let _ = writeln!(out, "{}</{}>", pad, html_tag);
            self.tree_pop();
        }

        // Emit any interpolation _effect() calls
        for effect in interp_effects {
            self.js.push_str(effect);
        }

        // Class map dynamic bindings: for each dynamic class, emit an effect
        if !self.in_for_template && !el.class_map_bindings.is_empty() {
            for (class_name, expr) in &el.class_map_bindings {
                let js_expr = self.expr_to_js(expr);
                let js_access = if self.signal_names.contains(&js_expr) {
                    format!("{}.value", js_expr)
                } else {
                    js_expr
                };
                let _ = writeln!(self.js,
                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.classList.toggle(\"{}\", !!({})); }});",
                    nid, class_name, js_access
                );
            }
        }

        // Record source map entry
        self.source_map.push(SourceMapEntry {
            node_id: nid.to_string(),
            omni_span: SourceSpan {
                line: el.source_span.map_or(0, |s| s.line),
                col: el.source_span.map_or(0, |s| s.col),
                len: 0,
            },
            target_spans: vec![TargetSpan {
                file: "index.html".into(),
                line: 0,
                col: 0,
                len: 0,
                kind: TargetSpanKind::DomElement,
            }],
        });

        out
    }

    /// Convert a single `ElementIR` node into its HTML representation.
    ///
    /// Generates the opening tag (with data-nid, static attrs, variant classes,
    /// hero attrs, inline style, event data-attrs, motion data-attrs), then
    /// recurses into children, and finally writes the closing tag.
    ///
    /// Side-effects:
    /// - Writes deferred JS (events, reactive effects, motion triggers) to
    ///   `self.js` / `self.mount_js`.
    /// - Accumulates element classes and tags in `self.element_classes` /
    ///   `self.element_tags` (used in css_only mode and for the React editor).
    /// - Builds the structured element tree in `self.element_tree_stack`.
    /// - Records source-map entries in `self.source_map`.
    /// - Pushes warnings for unknown/unresolved class references.
    pub(super) fn element_to_html(&mut self, el: &ElementIR, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let _pad_inner = "  ".repeat(indent + 1);
        let nid = self.next_nid(&el.tag);
        // Map omni tags to HTML tags
        let html_tag = map_tag(&el.tag);
        self.tree_push(nid.clone(), html_tag.clone());

        // Try to inline a known component's view HTML.
        // When inside a @for template, props are mapped to their value
        // expressions so that data-bind-text markers reference the @for
        // loop variable; event handlers and prop-wiring effects are skipped
        // (they cannot work correctly for dynamically-cloned elements).
        if let Some(inlined) = self.try_inline_component(el, indent, &nid, &pad) {
            // Record nid_map + element_tags for the component wrapper so the
            // editor can select custom components (not just their children).
            let design_id = el
                .props
                .get("id")
                .and_then(|v| {
                    if let ExprIR::LiteralExpr { value } = v {
                        value.as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| nid.clone());
            self.element_tags.insert(nid.clone(), html_tag.to_string());
            self.nid_map.insert(nid.clone(), design_id);
            self.tree_pop(); // pop the placeholder node; inlined component pushes its own
            return inlined;
        }

        let (mut attrs, class_name_parts) = self.emit_element_props(el, &nid);

        // Emit merged class attribute if any class names accumulated
        if !class_name_parts.is_empty() {
            let _ = write!(
                attrs,
                " class=\"{}\"",
                escape_html(&class_name_parts.join(" "))
            );
        }

        // Hero shared element transition tag — emit as data attribute
        if let Some(hero) = el.hero_behavior().and_then(|h| h.tag.as_ref()) {
            let _ = write!(attrs, " data-hero=\"{}\"", escape_html(hero));
        }
        // Optional custom transition motion name
        if let Some(transition) = el.hero_behavior().and_then(|h| h.transition.as_ref()) {
            let trans_str = self.expr_to_static_str(transition);
            if !trans_str.is_empty() {
                let _ = write!(
                    attrs,
                    " data-hero-transition=\"{}\"",
                    escape_html(&trans_str)
                );
            }
        }

        self.emit_element_inline_style(el, &nid, &mut attrs);

        self.emit_element_events(el, &nid, &mut attrs);

        self.emit_element_bindings(el, &nid);

        // Pre-scan children for interpolations → add data-bind-text to attrs
        // This MUST happen BEFORE writing the opening tag.
        let mut interp_effects: Vec<String> = Vec::new();
        for child in &el.children {
            if let ViewTree::Text(ref text) = child {
                if text.starts_with('{') && text.ends_with('}') {
                    let expr = &text[1..text.len() - 1];
                    // Only allow safe identifier expressions (alphanumeric, underscore, dot)
                    let is_safe = expr
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '.');
                    if self.in_for_template {
                        // Inside @for template: use data-bind-text marker
                        let _ = write!(attrs, " data-bind-text=\"{}\"", escape_html(expr));
                    } else if is_safe {
                        // Generate reactive text binding for signals after DOM insert
                        interp_effects.push(format!(
                            "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.textContent = String({}.value); }});\n",
                            nid, expr
                        ));
                    }
                }
            }
        }

        // Motion triggers — emit animation setup for @enter, @hover, @press, @focus, @leave
        self.emit_element_motion_attrs(el, &nid, &mut attrs);

        self.emit_element_motion_bindings(el, &nid);

        // Class map dynamic bindings for @for templates: collect pre-compiled bindings
        if self.in_for_template && !el.class_map_bindings.is_empty() {
            for (class_name, expr) in &el.class_map_bindings {
                let js_expr = self.expr_to_js(expr);
                self.for_class_bindings
                    .push((nid.clone(), class_name.clone(), js_expr));
            }
        }

        self.emit_element_html_close(
            el,
            &nid,
            &html_tag,
            &attrs,
            &class_name_parts,
            &interp_effects,
            indent,
        )
    }
}
