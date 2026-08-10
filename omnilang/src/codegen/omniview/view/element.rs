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
use super::modal::MODAL_HEADING_TAGS;
use crate::codegen::sanitize::is_dangerous_url;
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
    "open",     // 2026-08-07 B2: <details> 布尔属性——此前落入 data-* fallback 静默丢弃
    "hidden",   // 2026-08-10: 最常见的可见性切换——此前落 data-* fallback 不工作（评审 MEDIUM-2）
    "autoplay", // 2026-08-10: 媒体自动播放（parser 关键字已有，codegen 缺失——评审 MEDIUM-2）
];
const HTML_URL_ATTRS: &[&str] = &["href", "src", "srcset", "action", "formaction", "poster"];

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
                | "loop" | "muted" | "open" | "hidden" | "autoplay" | "poster" | "preload"
                | "tabindex" | "role" | "content" | "charset" | "http-equiv" | "property"
                | "itemprop" | "crossorigin" | "integrity" | "referrerpolicy" | "loading"
                | "decoding" | "async" | "defer" | "nonce" | "download" | "draggable"
                | "spellcheck" | "contenteditable" | "translate" | "autocapitalize" | "capture" => {
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
                        let js_access = if self.name_resolver.signal_names.contains(&js_val) {
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
                    // Unknown attribute. Static values degrade to data-* passthrough
                    // (harmless — the emitted attribute name is data-<name>, not <name>).
                    // DYNAMIC bindings on a bare unknown name lose reactivity silently
                    // (B2: <details open={sig}> emitted static data-open, no __omni_effect);
                    // data-*/aria-* prefixed names are the user's own passthrough channel
                    // and keep the existing silent degradation.
                    if !matches!(value, ExprIR::LiteralExpr { .. })
                        && !name.starts_with("data-")
                        && !name.starts_with("aria-")
                    {
                        self.warnings.push(format!(
                            "unknown attribute '{}' with dynamic binding: OmniView emits it as static data-{} (not reactive); add the attribute to the known-attribute whitelist or use a known attribute",
                            name, name
                        ));
                    }
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

        // ── a11y block (design doc 域 10): aria-* attribute passthrough ──
        // label → aria-label, role → role, live → aria-live, hidden →
        // aria-hidden, alt → alt. Static entries land in the HTML; dynamic
        // entries (hidden/alt expressions) emit a reactive __omni_effect
        // setAttribute binding — the same static-vs-dynamic machinery the
        // real-HTML-attribute arm above uses.
        if let Some(ref a11y) = el.a11y {
            if let Some(ref label) = a11y.label {
                let _ = write!(attrs, " aria-label=\"{}\"", escape_html(label));
            }
            if let Some(ref role) = a11y.role {
                let _ = write!(attrs, " role=\"{}\"", escape_html(role));
            }
            if let Some(ref hidden) = a11y.hidden {
                self.emit_a11y_attr(&mut attrs, nid, "aria-hidden", hidden);
            }
            if let Some(ref live) = a11y.live {
                let _ = write!(attrs, " aria-live=\"{}\"", escape_html(live));
            }
            if let Some(ref alt) = a11y.alt {
                self.emit_a11y_attr(&mut attrs, nid, "alt", alt);
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

        // ── @form HTML5 校验属性（Task 6）──
        // required 规则 → 元素 required 属性；pattern 规则（字面量）→ pattern 属性。
        // 守卫：用户显式书写的同名静态 prop 优先（spec §4.1 V-F5）。
        // 字段匹配：bindings 中 value/checked 的信号路径末段 == FormFieldIR.name。
        if let Some(form) = &self.scope.form_intent {
            for (prop_name, binding) in &el.bindings {
                if prop_name != "value" && prop_name != "checked" {
                    continue;
                }
                let field_name = binding.name.rsplit('.').next().unwrap_or(&binding.name);
                let Some(field) = form.fields.iter().find(|f| f.name == field_name) else {
                    continue;
                };
                if !el.props.contains_key("required")
                    && field.rules.iter().any(|r| r.name == "required")
                {
                    attrs.push_str(" required");
                }
                if !el.props.contains_key("pattern") {
                    if let Some(rule) = field.rules.iter().find(|r| r.name == "pattern") {
                        let pat = self.expr_to_static_str(&rule.value);
                        if !pat.is_empty() {
                            let _ = write!(attrs, " pattern=\"{}\"", escape_html(&pat));
                        }
                    }
                }
            }
        }

        (attrs, class_name_parts)
    }

    /// Emit one a11y-block attribute (design doc 域 10) with the same
    /// static-vs-dynamic machinery as regular props: literal values land in
    /// the static HTML string, expressions emit a reactive
    /// `__omni_effect` setAttribute binding on the element.
    fn emit_a11y_attr(&mut self, attrs: &mut String, nid: &str, name: &str, value: &ExprIR) {
        match value {
            ExprIR::LiteralExpr { value } => {
                if let Some(s) = value.as_str() {
                    let _ = write!(attrs, " {}=\"{}\"", name, escape_html(s));
                } else if let Some(b) = value.as_bool() {
                    let _ = write!(attrs, " {}=\"{}\"", name, b);
                }
                // Non-string/bool literals carry no meaningful aria value —
                // skip (the semantic pass places no type constraint).
            }
            _ => {
                let js_val = self.expr_to_js(value);
                let js_access = if self.name_resolver.signal_names.contains(&js_val) {
                    format!("{}.value", js_val)
                } else {
                    js_val
                };
                let _ = writeln!(self.js,
                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.setAttribute('{}', {}); }} catch(e) {{ __omni_reportError(\"error:bind:attr:{}\", e); }} }});",
                    nid, name, js_access, name
                );
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
                    .name_resolver
                    .name_renames
                    .get(&binding.name)
                    .or_else(|| self.name_resolver.global_renames.get(&binding.name))
                    .cloned()
                    .unwrap_or_else(|| binding.name.clone());
                // Prevent infinite loop: guard flag to stop the effect from re-triggering input event
                self.guard_counter += 1;
                let guard_name = format!("_guard_{}_{}", prop_name, nid.replace('-', "_"));
                let _ = writeln!(self.js, "let {} = false;", guard_name);
                let (event_name, dom_prop) = if prop_name == "checked" {
                    ("change", "checked")
                } else if el.tag == "select" {
                    // `<select>` fires only `change` (no `input` event) —
                    // binding it to "input" would silently never write the
                    // signal back on selection.
                    ("change", "value")
                } else {
                    ("input", "value")
                };
                // 数值信号（Int/Float）：e.target.value 是字符串——parseFloat
                // 保持数值类型。否则 f64 信号被字符串污染，style 绑定的
                // __omni_px 因 typeof 判断（"380" 是 string）无法附加 px，
                // el.style.width = "380" 无单位被浏览器忽略（滑块拖不动尺寸）。
                let value_expr = if self.signal_is_numeric(&binding.name) {
                    "parseFloat(e.target.value)"
                } else {
                    "e.target.value"
                };
                let _ = writeln!(self.js,
                    "{{ const el = __omni_el(\"{0}\"); if (el) el.addEventListener(\"{1}\", (e) => {{ if (!{2}) {{ __omni_subs.get(\"{3}\").set({4}); }} }}); }}",
                    nid, event_name, guard_name, sig_name, value_expr
                );
                // Reactive binding: signal → DOM (guard against programmatic updates)
                let _ = writeln!(self.js,
                    "__omni_effect(() => {{ const el = __omni_el(\"{0}\"); if (el) {{ {1} = true; el.{2} = __omni_subs.get(\"{3}\").value; {1} = false; }} }});",
                    nid, guard_name, prop_name, sig_name
                );
            }
        }
    }

    /// 判断绑定信号是否为数值类型（Int/Float）。
    ///
    /// 绑定名可能是组件信号（"boxWidthNum" → name_renames 解析到全名
    /// `{Comp}_{sig}`）或 store 信号（"Store.field" → `{Store}_{field}`）。
    fn signal_is_numeric(&self, name: &str) -> bool {
        let store_full = name.contains('.').then(|| name.replace('.', "_"));
        self.name_resolver
            .name_renames
            .get(name)
            .map(|f| self.numeric_signal_names.contains(f))
            .or_else(|| {
                store_full
                    .as_ref()
                    .map(|f| self.numeric_signal_names.contains(f))
            })
            .unwrap_or(false)
    }

    /// Emit @form field error aria wiring (Task 6, spec §4.1).
    ///
    /// For every bound form field (bindings "value"/"checked" whose signal
    /// path last segment matches a FormFieldIR.name), a reactive effect syncs
    /// `aria-invalid` / `aria-describedby` from the component's formErrors
    /// signal. The error <span> itself is rendered explicitly by the user
    /// (e.g. `@if form.errors.name { <span id="err-name">...`), so this only
    /// wires the accessibility channel.
    fn emit_form_field_aria(&mut self, el: &ElementIR, nid: &str) {
        let Some(form) = &self.scope.form_intent else {
            return;
        };
        let comp = &self.scope.current_component;
        for (prop_name, binding) in &el.bindings {
            if prop_name != "value" && prop_name != "checked" {
                continue;
            }
            let field_name = binding.name.rsplit('.').next().unwrap_or(&binding.name);
            if !form.fields.iter().any(|f| f.name == field_name) {
                continue;
            }
            let _ = writeln!(
                self.js,
                "__omni_effect(() => {{ const el = __omni_el(\"{0}\"); if (el) {{ const err = __omni_subs.get(\"{1}_formErrors\").value[\"{2}\"]; el.setAttribute('aria-invalid', err ? 'true' : 'false'); el.setAttribute('aria-describedby', err ? 'err-{2}' : ''); }} }});",
                nid, comp, field_name
            );
        }
    }

    /// Emit reactive effects for semantic ref bindings (`focus={sig}`,
    /// `scroll-x={sig}`, `scroll-y={sig}`).
    ///
    /// The bindings are extracted into `ElementIR.semantic_bindings` during
    /// lowering (semantic-level DSL spec §2.2) and validated by the semantic
    /// pass to require a `@ref` on the element. Each binding drives a native
    /// DOM side effect from the bound signal, following the same `__omni_effect`
    /// + `__omni_el(nid)` pattern as `emit_element_bindings`:
    ///
    /// - `focus`    → `el.focus()` / `el.blur()` on the boolean signal
    /// - `scroll-y` → `el.scrollTop = sig`
    /// - `scroll-x` → `el.scrollLeft = sig`
    ///
    /// (`value={sig}` is removed — design D2 — `bind:value` is the sole
    /// two-way form and is emitted by `emit_element_bindings`.)
    fn emit_semantic_bindings(&mut self, el: &ElementIR, nid: &str) {
        for (kind, binding) in &el.semantic_bindings {
            let js_expr = self.expr_to_js(binding);
            let js_access = if self.name_resolver.signal_names.contains(&js_expr) {
                format!("{}.value", js_expr)
            } else {
                js_expr
            };
            match kind.as_str() {
                // focus={sig}: boolean signal drives focus()/blur()
                "focus" => {
                    let _ = writeln!(self.js,
                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) {{ if ({}) {{ el.focus(); }} else {{ el.blur(); }} }} }});",
                        nid, js_access
                    );
                }
                // scroll-y={sig}: vertical scroll position
                "scroll-y" => {
                    let _ = writeln!(self.js,
                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) {{ el.scrollTop = {}; }} }});",
                        nid, js_access
                    );
                }
                // scroll-x={sig}: horizontal scroll position
                "scroll-x" => {
                    let _ = writeln!(self.js,
                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) {{ el.scrollLeft = {}; }} }});",
                        nid, js_access
                    );
                }
                _ => {
                    // Unknown binding kind — the semantic validator rejects
                    // these before codegen, but keep the generator total.
                    self.warnings.push(format!(
                        "OmniView: unknown semantic binding '{kind}' on <{}> (nid={nid})",
                        el.tag
                    ));
                }
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
                let data_attr = match &trigger.kind {
                    TriggerKind::Enter => "data-enter",
                    TriggerKind::Hover => "data-hover",
                    TriggerKind::Press => "data-press",
                    TriggerKind::Focus => "data-focus",
                    TriggerKind::InView => "data-in-view",
                    TriggerKind::Scroll { .. } => "data-scroll",
                    TriggerKind::DragHorizontal { .. } => "data-drag-horizontal",
                    TriggerKind::DragVertical { .. } => "data-drag-vertical",
                    // #[non_exhaustive] — future trigger kinds carry no data attr yet.
                    _ => "",
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
            // Sub-attributes for scroll (v1 control dimensions, Task 5.1)
            for trigger in el
                .motion_behavior()
                .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
            {
                if let TriggerKind::Scroll { start, end, scrub } = &trigger.kind {
                    if let Some(s) = start {
                        let js_val = self.expr_to_js(s);
                        let _ = write!(attrs, " data-scroll-start=\"{}\"", escape_html(&js_val));
                    }
                    if let Some(s) = end {
                        let js_val = self.expr_to_js(s);
                        let _ = write!(attrs, " data-scroll-end=\"{}\"", escape_html(&js_val));
                    }
                    if let Some(sc) = scrub {
                        let js_val = self.expr_to_js(sc);
                        let _ = write!(attrs, " data-scroll-scrub=\"{}\"", escape_html(&js_val));
                    }
                }
                if let TriggerKind::DragHorizontal {
                    constraints,
                    elastic,
                    threshold,
                }
                | TriggerKind::DragVertical {
                    constraints,
                    elastic,
                    threshold,
                } = &trigger.kind
                {
                    if let Some((min, max)) = constraints {
                        let min_js = self.expr_to_js(min);
                        let max_js = self.expr_to_js(max);
                        let _ = write!(
                            attrs,
                            " data-drag-constraints=\"[{}, {}]\"",
                            escape_html(&min_js),
                            escape_html(&max_js)
                        );
                    }
                    if let Some(e) = elastic {
                        let js_val = self.expr_to_js(e);
                        let _ = write!(attrs, " data-drag-elastic=\"{}\"", escape_html(&js_val));
                    }
                    if let Some(e) = threshold {
                        let js_val = self.expr_to_js(e);
                        let _ = write!(attrs, " data-drag-threshold=\"{}\"", escape_html(&js_val));
                    }
                }
            }

            self.emit_motion_triggers_for_element(nid, el);
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

        // Native HTML5 constraint validation intercepts failed submits BEFORE
        // the submit event fires — with required/pattern emitted as real attrs,
        // validateForm (self-drawn errors) would never run on empty/invalid
        // fields. Emit novalidate whenever @form semantics are active so the
        // submit event always fires and formErrors gate/error containers are
        // the single source of truth (Task 6 review Important #1).
        let mut attrs = attrs.to_string();
        if html_tag == "form" && self.scope.form_intent.is_some() && !attrs.contains("novalidate") {
            attrs.push_str(" novalidate");
        }

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
            let _ = writeln!(out, "{}<{}{} />", pad, html_tag, &attrs);
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
            let _ = writeln!(out, "{}<{}{}>", pad, html_tag, &attrs);

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
            let js_access = if self.name_resolver.signal_names.contains(&js_expr) {
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

    /// Resolve an element's `id` prop for the editor's nid_map bookkeeping
    /// (@modal shell, @skeleton placeholder, @errorBoundary shell). Mirrors
    /// the inline-component extraction in `element_to_html` (user `id` wins,
    /// else the auto nid).
    fn resolve_design_id(&self, el: &ElementIR, nid: &str) -> String {
        el.props
            .get("id")
            .and_then(|v| {
                if let ExprIR::LiteralExpr { value } = v {
                    value.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| nid.to_string())
    }

    /// Escape a string for embedding as the CONTENT of a double-quoted JS
    /// string literal. Used by the @errorBoundary try/catch runtime, which
    /// reassigns `host.innerHTML` from deferred-JS strings (the deferred
    /// buffer is emitted raw — no template-literal escaping — so the HTML
    /// payloads must be escaped here, view/mod.rs `emit_dom_and_bindings`).
    fn escape_js_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    /// Emit the @skeleton shimmer CSS once per document (guarded by
    /// `shimmer_css_emitted`): shape classes + a `::after` gradient sweep
    /// animated by `@keyframes omni-shimmer`.
    fn emit_shimmer_css(&mut self) {
        if self.shimmer_css_emitted {
            return;
        }
        self.shimmer_css_emitted = true;
        self.css.push_str(
            ".omni-skeleton { position: relative; overflow: hidden; \
             background: rgba(0, 0, 0, 0.06); border-radius: 4px; }\n\
             .omni-skeleton--text { height: 1em; }\n\
             .omni-skeleton--line { height: 1em; margin-bottom: 0.5em; }\n\
             .omni-skeleton--circle { border-radius: 50%; }\n\
             .omni-skeleton--card { height: 120px; border-radius: 8px; }\n\
             .omni-skeleton::after { content: \"\"; position: absolute; inset: 0; \
             transform: translateX(-100%); \
             background: linear-gradient(90deg, transparent, rgba(255,255,255,0.5), transparent); \
             animation: omni-shimmer 1.5s infinite; }\n\
             @keyframes omni-shimmer { 100% { transform: translateX(100%); } }\n",
        );
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
        //
        // a11y on a component tag: OmniView inlines the child's own view
        // HTML (the consumer tag never becomes a real DOM element), so the
        // a11y intent has no wrapper to attach to — warn instead of
        // silently dropping (React/Flutter carry it on the component).
        if el.a11y.is_some() && self.name_resolver.component_map.contains_key(&el.tag) {
            self.warnings.push(format!(
                "[omniview] a11y 块用于组件标签 <{}>：OmniView 内联组件无 aria-* 包裹机制，\
                 a11y 意图被忽略（React/Flutter 目标不受影响）",
                el.tag
            ));
        }
        // @modal interception: the element node has already been pushed onto
        // the tree (tree_push in this function); emit_modal_shell emits the
        // shell HTML itself and balances the tree with tree_pop. The shell is
        // the element's real wrapper, so nid_map points at the shell div.
        if let Some(modal) = &el.modal {
            let design_id = self.resolve_design_id(el, &nid);
            self.element_tags.insert(nid.clone(), "div".to_string());
            self.nid_map.insert(nid.clone(), design_id);
            return self.emit_modal_shell(el, &modal.open_signal, &modal.slots, indent, &nid);
        }
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

        // @skeleton interception: a static placeholder div + one-shot shimmer
        // CSS (spec §4.3). The element node was pushed on the tree by this
        // function; pop it here (no children are rendered).
        if let Some(skeleton) = &el.skeleton {
            let shape = skeleton.shape.as_deref().unwrap_or("text");
            let design_id = self.resolve_design_id(el, &nid);
            self.element_tags.insert(nid.clone(), "div".to_string());
            self.nid_map.insert(nid.clone(), design_id);
            self.emit_shimmer_css();
            self.tree_pop();
            return format!(
                "{}<div class=\"omni-skeleton omni-skeleton--{}\" data-nid=\"{}\"></div>\n",
                pad, shape, nid
            );
        }

        // @errorBoundary interception (spec §4.4): a static shell div whose
        // content is installed by a deferred try/catch — if rendering the
        // boundary content throws, the fallback slot HTML is installed
        // instead; without a fallback the shell stays empty. The fallback
        // slot must not double-emit (F7): it is skipped in the children loop
        // and rendered only via `boundary.fallback`.
        if let Some(boundary) = &el.error_boundary {
            let design_id = self.resolve_design_id(el, &nid);
            self.element_tags.insert(nid.clone(), "div".to_string());
            self.nid_map.insert(nid.clone(), design_id);

            // Ordering guarantee: the shell starts EMPTY in the static
            // innerHTML, so child bindings (event listeners, interpolation
            // effects) must NOT be emitted before the content is installed —
            // at runtime they would run against the empty shell, `__omni_el`
            // would be null, and the `if (el)` guard would discard them
            // forever (dead listeners / empty interpolations). Capture all
            // child + fallback bindings into a temp buffer via the same
            // `std::mem::swap` idiom as `emit_view`, install the content
            // first (try/catch IIFE), THEN append the captured bindings:
            // runtime order = 先装内容、后绑事件.
            let mut child_deferred = String::new();
            std::mem::swap(&mut self.js, &mut child_deferred);

            let mut content = String::new();
            for child in &el.children {
                if matches!(child, ViewTree::Slot { name: Some(n), .. } if n == "fallback") {
                    continue;
                }
                content.push_str(&self.view_to_html(child, indent + 1));
            }
            let fallback_html = boundary
                .fallback
                .as_ref()
                .map(|fb| {
                    fb.fallback
                        .iter()
                        .map(|c| self.view_to_html(c, indent + 1))
                        .collect::<String>()
                })
                .unwrap_or_default();
            // Restore self.js — all child/fallback bindings now live in child_deferred.
            std::mem::swap(&mut self.js, &mut child_deferred);

            let content_escaped = Self::escape_js_string(&content);
            let fallback_escaped = Self::escape_js_string(&fallback_html);
            self.js.push_str(&format!(
                "(function() {{ var host = __omni_el(\"{nid}\"); if (!host) return; \
                 var content = \"{content_escaped}\"; var fallback = \"{fallback_escaped}\"; \
                 try {{ host.innerHTML = content; }} catch (e) {{ host.innerHTML = fallback; }} }})();\n",
                nid = nid,
                content_escaped = content_escaped,
                fallback_escaped = fallback_escaped,
            ));
            // Bindings execute AFTER the content is installed. If the content
            // install succeeded, the fallback branch's bindings no-op via the
            // `if (el)` guard (its elements don't exist); if it threw and the
            // fallback was installed instead, the fallback bindings are live.
            self.js.push_str(&child_deferred);
            self.tree_pop();
            return format!(
                "{}<div class=\"omni-error-boundary\" data-nid=\"{}\"></div>\n",
                pad, nid
            );
        }

        let (mut attrs, class_name_parts) = self.emit_element_props(el, &nid);

        // F1 (spec §2.1 rule 5): while inside a @modal dialog, the first
        // Heading-class element receives the compiler-generated title id that
        // `aria-labelledby="omni-modal-title"` points at. The flag is set by
        // emit_modal_shell when the dialog contains a heading, and reset once
        // consumed (or after the dialog — see the shell's defensive reset).
        // Known limitation: this check is element-local and cannot see
        // conditional-branch ancestry. If the only heading sits inside a
        // branch that is statically skipped in css_only (canvas) mode, the
        // flag is never consumed and `aria-labelledby="omni-modal-title"`
        // dangles until a later heading appears. Live mode is unaffected
        // (all @if branches are statically in the DOM, display-toggled).
        if self.pending_modal_title_id && MODAL_HEADING_TAGS.contains(&el.tag.as_str()) {
            self.pending_modal_title_id = false;
            if !el.props.contains_key("id") {
                let _ = write!(attrs, " id=\"omni-modal-title\"");
            }
        }

        // Emit merged class attribute if any class names accumulated
        if !class_name_parts.is_empty() {
            let _ = write!(
                attrs,
                " class=\"{}\"",
                escape_html(&class_name_parts.join(" "))
            );
        }

        self.emit_element_inline_style(el, &nid, &mut attrs);

        // Two-way bindings BEFORE user event handlers: on the same DOM event
        // (e.g. select's bind:value + @onChange, both → "input") the listeners
        // run in registration order, so a bind sync registered after the user
        // handler would make the handler read the pre-update signal value.
        self.emit_element_bindings(el, &nid);

        // @form field error aria wiring — reactive from formErrors signal
        self.emit_form_field_aria(el, &nid);

        self.emit_element_events(el, &nid, &mut attrs);

        // @drag is a compiler-synthesized gesture over pointer events — emit
        // its capture + dispatch wiring right after the plain event bindings
        // (ADR-073; @drag/@dragEnd lower into el.drag_binding).
        self.emit_drag_binding(el, &nid);

        self.emit_semantic_bindings(el, &nid);

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
