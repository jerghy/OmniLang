//! Element rendering: HTML element generation with props, events,
//! bindings, inline styles, motion triggers, and element tree recording.
//!
//! The primary entry point is `element_to_html`, which converts an `ElementIR`
//! into its full HTML representation (including deferred JS bindings written
//! to `self.js` / `self.mount_js` etc.).

use rustc_hash::FxHashMap;
use std::fmt::Write;

use super::super::utils::{escape_html, map_tag, parse_attr_template};
use super::super::ElementTreeNode;
use super::super::OmniViewGenerator;
use crate::ir::*;
use crate::sourcemap::{SourceMapEntry, SourceSpan, TargetSpan, TargetSpanKind};

const HTML_BOOLEAN_ATTRS: &[&str] = &[
    "disabled",
    "checked",
    "required",
    "readonly",
    "selected",
    "autofocus",
    "multiple",
    "controls",
    "loop",
    "muted",
    "async",
    "defer",
];
const HTML_URL_ATTRS: &[&str] = &["href", "src", "srcset", "action", "formaction", "poster"];

/// Check whether a static URL attribute value contains a dangerous protocol
/// scheme. Returns `true` if the value should be blocked.
///
/// This mirrors `__omni_sanitizeUrl` in the generated JS runtime, catching
/// dangerous URLs in literal attribute values at compile time as defense-in-depth
/// against supply-chain compromises where a third-party .omni component
/// contains literal `javascript:` or `data:` URLs.
pub(crate) fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    // Strip CR, LF, TAB — browsers strip these before URL navigation
    let stripped: String = lower
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | '\t'))
        .collect();
    stripped.starts_with("javascript:")
        || stripped.starts_with("data:")
        || stripped.starts_with("vbscript:")
}

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
    /// - Records pre-compiled @for attribute bindings in `self.scope.for_attr_bindings`.
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
                    let is_boolean_attr = HTML_BOOLEAN_ATTRS.contains(&name.as_str());
                    // Dynamic attributes: use ExprIR type matching instead of string heuristic.
                    // The old heuristic (starts_with('{') && ends_with('}')) only caught
                    // simple identifiers like {count}. Complex expressions like
                    // disabled={!isSubmitting} or checked={count > 0} need ExprIR checking.
                    let is_dynamic = !matches!(value, ExprIR::LiteralExpr { .. });
                    if is_dynamic {
                        // Dynamic attribute: bind reactively
                        let js_val = self.expr_to_js(value);
                        let js_access = if self.scope.signal_names.contains(&js_val) {
                            format!("{}.value", js_val)
                        } else {
                            js_val
                        };
                        let is_url_attr = HTML_URL_ATTRS.contains(&name.as_str());
                        if is_boolean_attr {
                            // Boolean attributes must use the DOM property (e.g. el.checked = bool)
                            // rather than setAttribute (which treats all values as strings — "false"
                            // is truthy in HTML and won't uncheck the checkbox).
                            let _ = writeln!(self.js,
                                "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.{} = !!({}); }} catch(e) {{ __omni_reportError(\"error:bind:attr:{}\", e); }} }});",
                                nid, name, js_access, name
                            );
                        } else if is_url_attr {
                            let _ = writeln!(self.js,
                                "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.setAttribute('{}', __omni_sanitizeUrl({})); }} catch(e) {{ __omni_reportError(\"error:bind:attr:{}\", e); }} }});",
                                nid, name, js_access, name
                            );
                        } else {
                            let _ = writeln!(self.js,
                                "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.setAttribute('{}', {}); }} catch(e) {{ __omni_reportError(\"error:bind:attr:{}\", e); }} }});",
                                nid, name, js_access, name
                            );
                        }
                    } else if is_boolean_attr && val_str == "true" {
                        // Boolean attribute: emit just the attribute name (no ="true")
                        let _ = write!(attrs, " {}", name);
                    } else if is_boolean_attr && val_str == "false" {
                        // Boolean attribute with value "false": skip entirely (don't emit)
                    } else if !val_str.is_empty() {
                        let is_static_url = matches!(
                            name.as_str(),
                            "href" | "src" | "srcset" | "action" | "formaction" | "poster"
                        );
                        if is_static_url && is_dangerous_url(&val_str) {
                            // Block dangerous URL schemes in static attributes.
                            // This is defense-in-depth: dynamic bindings are protected
                            // by __omni_sanitizeUrl at runtime; this catches literal
                            // dangerous URLs at compile time (supply-chain defense).
                            self.warnings.push(format!(
                                "Blocked dangerous URL in static {} attribute: scheme not allowed",
                                name
                            ));
                            // Emit a safe fallback value
                            let _ = write!(attrs, " {}=\"#blocked\"", name);
                        } else {
                            let _ = write!(attrs, " {}=\"{}\"", name, escape_html(&val_str));
                        }
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
                    if self.scope.in_for_template
                        && (val_str.contains('{') && val_str.contains('}'))
                    {
                        // Pre-compiled attribute binding for @for templates
                        let attr_name = if name.starts_with("aria-") || name.starts_with("data-") {
                            name.clone()
                        } else {
                            format!("data-{}", name)
                        };
                        let parts = parse_attr_template(&val_str);
                        self.scope
                            .for_attr_bindings
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
            let in_design = self.design_class_names.contains(class_name);
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
                    .scope
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
    /// Writes HTML data attributes for @enter, @hover, @press, @in-view, @scroll, @drag-horizontal, @drag-vertical triggers
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
                    TriggerKind::Hover => "data-hover",
                    TriggerKind::Press => "data-press",
                    TriggerKind::InView => "data-in-view",
                    TriggerKind::Scroll => "data-scroll",
                    TriggerKind::DragHorizontal => "data-drag-horizontal",
                    TriggerKind::DragVertical => "data-drag-vertical",
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

    /// Emit motion @paused per-element pause override.
    ///
    /// These are deferred JS bindings written to `self.mount_js` that connect
    /// runtime animation state to signals and external controllers.
    fn emit_element_motion_bindings(&mut self, el: &ElementIR, nid: &str) {
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
                 \x20 var anims = __omni_animations.get(\"{nid}\");\n\
                 \x20 if (anims) {{\n\
                 \x20\x20 anims.forEach((anim) => {{\n\
                 \x20\x20\x20 if (_paused) {{ anim.pause(); }} else {{ anim.play(); }}\n\
                 \x20\x20 }});\n\
                 \x20 }}\n\
                 }})();\n",
                nid = nid,
                paused = paused_js
            );
        }
    }

    /// Emit the HTML closing portion for an element: handle void tags, fragment
    /// wrappers, normal tags with children, css_only mode, interpolation effects,
    /// class map dynamic bindings, and source map recording.
    ///
    /// In css_only mode, capture element metadata for the editor's
    /// element tree: design ID, resolved CSS classes, HTML tag, and
    /// nid mapping. Called from both void-tag and normal-tag branches
    /// of `emit_element_html_close`.
    fn emit_element_css_only_capture(
        &mut self,
        el: &ElementIR,
        nid: &str,
        html_tag: &str,
        class_name_parts: &[String],
    ) {
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
                self.emit_element_css_only_capture(el, nid, html_tag, class_name_parts);
                return String::new();
            }
            let _ = writeln!(out, "{}<{}{} />", pad, html_tag, attrs);
            // Emit any interpolation _effect() calls
            for effect in interp_effects {
                self.js.push_str(effect);
            }
            self.emit_class_map_effects(el, nid);
            self.tree_pop();
            return out;
        }

        // For <fragment> wrapper inside @for template: emit children without wrapper.
        if self.scope.in_for_template && html_tag == "fragment" {
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
                self.emit_element_css_only_capture(el, nid, html_tag, class_name_parts);
                return String::new();
            }

            let _ = writeln!(out, "{}</{}>", pad, html_tag);
            self.tree_pop();
        }

        // Emit any interpolation _effect() calls
        for effect in interp_effects {
            self.js.push_str(effect);
        }

        self.emit_class_map_effects(el, nid);

        // Record source map entry
        self.source_map.push(SourceMapEntry {
            node_id: nid.to_string(),
            omni_span: SourceSpan {
                line: el.source_span.map_or(0, |s| s.line),
                col: el.source_span.map_or(0, |s| s.col),
                len: el.source_span.map_or(0, |s| (s.end - s.start) as u32),
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

    /// Emit `__omni_effect` calls for dynamic class toggling (class_map_bindings).
    ///
    /// For each entry in `el.class_map_bindings`, emits an effect that calls
    /// `el.classList.toggle(className, !!value)` so the DOM class is kept in
    /// sync with the reactive expression.
    fn emit_class_map_effects(&mut self, el: &ElementIR, nid: &str) {
        for (class_name, binding) in &el.class_map_bindings {
            let js_expr = self.expr_to_js(binding);
            let js_access = if self.scope.signal_names.contains(&js_expr) {
                format!("{}.value", js_expr)
            } else {
                js_expr
            };
            let _ = writeln!(
                self.js,
                "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.classList.toggle(\"{}\", !!({})); }});",
                nid, class_name, js_access
            );
        }
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
        // Collect ref → nid mapping for wrapper object generation
        if let Some(ref ref_name) = el.ref_binding {
            self.push_ref_nid(ref_name.clone(), nid.clone());
        }
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
                    if self.scope.in_for_template {
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
        if self.scope.in_for_template && !el.class_map_bindings.is_empty() {
            for (class_name, expr) in &el.class_map_bindings {
                let js_expr = self.expr_to_js(expr);
                self.scope
                    .for_class_bindings
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
