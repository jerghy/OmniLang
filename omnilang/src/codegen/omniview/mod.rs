//! OmniView codegen: IR → JS + HTML + CSS + SourceMap.
//!
//! Generates a self-contained iframe-ready document with:
//!   1. Inline mini-signal runtime (~200 lines)
//!   2. Component signals, computeds, functions
//!   3. Static HTML structure with data-nid attributes
//!   4. Event bindings (click, input, etc.)
//!   5. Reactive text bindings via _effect()
//!   6. CSS style rules
//!   7. SourceMap: nodeId → source location

mod editor;
mod list;
mod list_render;
mod motion;
mod motion_triggers;
mod runtime;
mod stmt;
mod store;
mod timeline;
mod utils;
mod view;
use utils::*;
pub use utils::{compile_files, wrap_debug_html};
pub(crate) use utils::{emit_event_object, field_extract_js};

use super::{sanitize_css_value, try_format_stdlib_call, Target};
use crate::ir::*;
use crate::sourcemap::{SourceMapEntry, SourceSpan, TargetSpan, TargetSpanKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Output of the OmniView codegen — standalone JS/HTML/CSS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniViewOutput {
    /// JavaScript code (inline signal runtime + generated component)
    pub js: String,
    /// HTML markup with data-nid attributes
    pub html: String,
    /// CSS style rules
    pub css: String,
    /// Source map entries
    pub source_map: Vec<SourceMapEntry>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default)]
    pub components: HashMap<String, ComponentOutput>,
}

/// Per-component compiled output for hot-replace.
/// Note: `html` and `css` are reserved for future per-component HTML/CSS splitting;
/// currently only `js` is populated by the delta tracker in `generate_component()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOutput {
    pub js: String,
    pub html: String,
    pub css: String,
}

/// Generate OmniView output from an IR program.
pub fn generate(ir: &IRProgram, editor_mode: bool) -> OmniViewOutput {
    let mut gen = OmniViewGenerator {
        js: String::new(),
        html: String::new(),
        css: String::new(),
        source_map: Vec::new(),
        node_counter: 0,
        current_nid_path: Vec::new(),
        signal_names: std::collections::HashSet::new(),
        store_signal_names: std::collections::HashSet::new(),
        store_field_names: std::collections::HashSet::new(),
        store_names: std::collections::HashSet::new(),
        name_renames: std::collections::HashMap::new(),
        global_renames: std::collections::HashMap::new(),
        component_map: std::collections::HashMap::new(),
        emitting_root_view: false,
        current_component: String::new(),
        in_for_template: false,
        deferred_js: String::new(),
        all_deferred_js: String::new(),
        dom_emitted: false,
        guard_counter: 0,
        mount_js: String::new(),
        editor_mode,
        comp_mount_js: String::new(),
        component_js: HashMap::new(),
        enter_trigger_nids: Vec::new(),
        for_state_names: std::collections::HashMap::new(),
        motion_names: std::collections::HashSet::new(),
    };

    gen.emit_js_header();
    gen.emit_signal_runtime();
    gen.emit_animation_runtime();

    // Emit all stores as global JS variables (signals, getters, actions)
    for (store_name, store) in ir.all_stores() {
        gen.emit_store(store_name, store);
    }

    // Emit all module-level public functions (e.g., lib.omni utilities)
    for func in ir.all_functions() {
        gen.js.push_str(&format!("\n// pub fn {}\n", func.name));
        gen.emit_function(func);
    }

    // Emit module-level motion definitions (@motion blocks defined outside any component).
    // These must be emitted before component-level motions because they're referenced by
    // component view triggers like @enter={motionName}.
    for module in ir.modules.values() {
        if !module.motions.is_empty() {
            gen.js.push_str(&format!(
                "\n/// ── Module Motion Definitions: {} ──\n\n",
                module.name
            ));
            for motion in &module.motions {
                gen.motion_names.insert(motion.name.clone());
                gen.emit_motion_js(motion);
            }
        }
        if !module.timelines.is_empty() {
            gen.js.push_str(&format!(
                "\n/// ── Module Timeline Definitions: {} ──\n\n",
                module.name
            ));
            for timeline in &module.timelines {
                gen.emit_timeline_js(timeline);
            }
        }
    }

    // Build component map for view inlining
    for (comp_name, comp) in ir.all_components() {
        gen.component_map
            .insert(comp_name.to_string(), comp.clone());
        // Pre-register ALL component function & event renames globally.
        // These never get cleared so template interpolations like {getModStatus()}
        // resolve correctly even when the component is inlined as a child.
        for f in &comp.fns {
            let prefixed = format!("{}_{}", comp_name, f.name);
            gen.global_renames.insert(f.name.clone(), prefixed);
        }
        for ev_name in &comp.event_names {
            let prefixed = format!("{}_{}", comp_name, ev_name);
            gen.global_renames.insert(ev_name.clone(), prefixed);
        }
    }

    // Find the root component (prefer "App", otherwise use the last component)
    let all_comps: Vec<(&str, &ComponentIR)> = ir.all_components().into_iter().collect();
    let root_name = if all_comps.iter().any(|(n, _)| *n == "App") {
        "App"
    } else {
        all_comps.last().map(|(n, _)| *n).unwrap_or("App")
    };

    // Initialize editor bridge globals before component factories reference them
    if gen.editor_mode {
        gen.js.push_str(
            "window.__omni_signals = {};\nwindow.__omni_components = window.__omni_components || {};\n"
        );
    }

    // Generate each component (signals, fns). Only root component emits its view.
    for (comp_name, comp) in ir.all_components() {
        gen.generate_component(comp_name, comp, ir, comp_name == root_name);
    }

    // Emit global __mergeTouches helper for TouchEvent support
    gen.js.push_str(
        "function __mergeTouches(e) {\n\
         \x20 const seen = new Map();\n\
         \x20 for (const t of e.touches)\n\
         \x20   seen.set(t.identifier, { x: t.clientX, y: t.clientY, id: t.identifier });\n\
         \x20 for (const t of e.changedTouches)\n\
         \x20   if (!seen.has(t.identifier))\n\
         \x20     seen.set(t.identifier, { x: t.clientX, y: t.clientY, id: t.identifier });\n\
         \x20 return [...seen.values()];\n\
         }\n",
    );

    // Build per-component output map for editor hot-replace
    let mut components: HashMap<String, ComponentOutput> = HashMap::new();
    for (name, js) in &gen.component_js {
        components.insert(
            name.clone(),
            ComponentOutput {
                js: js.clone(),
                html: String::new(),
                css: String::new(),
            },
        );
    }

    // Emit ONE innerHTML insertion with accumulated HTML + ALL deferred bindings
    gen.emit_dom_and_bindings();
    // Inject editor bridge code (postMessage, nid_map, highlight, hot-replace)
    gen.emit_editor_bridge();
    gen.finalize_js();

    OmniViewOutput {
        js: gen.js,
        html: gen.html,
        css: gen.css,
        source_map: gen.source_map,
        components,
    }
}

struct OmniViewGenerator {
    js: String,
    html: String,
    css: String,
    source_map: Vec<SourceMapEntry>,
    node_counter: u32,
    current_nid_path: Vec<String>,
    /// Set of signal names in the current component (for .value / .set() generation)
    signal_names: std::collections::HashSet<String>,
    /// Store signal/getter names with prefix (e.g. "TodoStore_items") — need .value access
    store_signal_names: std::collections::HashSet<String>,
    /// All store field names with prefix (e.g. "TodoStore_items", "TodoStore_add") — defined globally
    store_field_names: std::collections::HashSet<String>,
    /// Store names (without prefix) for recognizing store references like "TodoStore"
    store_names: std::collections::HashSet<String>,
    /// Map from store-local / component-local signal names to flat JS names.
    /// Cleared per component — only contains the CURRENT component's signals.
    name_renames: std::collections::HashMap<String, String>,
    /// Global function/event renames — never cleared.
    /// All component functions/events are registered here so template calls
    /// like {getModStatus()} resolve to the correct prefixed name.
    global_renames: std::collections::HashMap<String, String>,
    /// Cache of all component IRs for view inlining (keyed by component name)
    component_map: std::collections::HashMap<String, ComponentIR>,
    /// Set of known motion names — used to detect @click={motionName} patterns
    motion_names: std::collections::HashSet<String>,
    /// Whether we're currently emitting the root component's view
    emitting_root_view: bool,
    /// Current component name prefix for scoping function names
    current_component: String,
    /// When true, we're generating HTML for a @for template.
    /// Text interpolations use data-bind-text markers instead of _effect() calls.
    in_for_template: bool,
    /// Deferred event/reactive binding JS — emitted AFTER DOM insertion.
    /// This ensures the DOM exists before we try to bind events.
    deferred_js: String,
    /// Accumulated deferred JS from ALL components (flushed once after final innerHTML).
    all_deferred_js: String,
    /// Track whether we've already emitted the innerHTML insertion script.
    dom_emitted: bool,
    /// Unique counter for guard variable names (bind:value / bind:checked).
    /// Separate from node_counter to guarantee uniqueness even when nids overlap.
    guard_counter: u32,
    /// Accumulated mount lifecycle JS — called after DOM insertion.
    mount_js: String,
    /// When true, emit editor bridge code (postMessage, nid_map, highlight, hot-replace).
    editor_mode: bool,
    /// Per-component mount JS buffer for editor mode factory pattern.
    /// Accumulates mount lifecycle and effect code for the current component
    /// and is flushed inside the factory IIFE before the return.
    comp_mount_js: String,
    /// Per-component JS deltas for hot-replace (keyed by component name).
    /// Captured at the end of each `generate_component()` call.
    component_js: HashMap<String, String>,
    /// (nid, motion_name, trigger_kind) pairs for replay on signal change.
    /// trigger_kind is "enter" or "timeline" — used to build correct replay code.
    enter_trigger_nids: Vec<(String, String, String)>,
    /// @for template: maps child component state names -> (original_name, default_js)
    for_state_names: std::collections::HashMap<String, (String, String)>,
}

impl OmniViewGenerator {
    // ══════════════════════════════════════════════════════
    // Headers
    // ══════════════════════════════════════════════════════

    // ══════════════════════════════════════════════════════
    // Store Emission
    // ══════════════════════════════════════════════════════

    /// Emit a store's signals, getters, and actions as global JS variables.
    /// Naming convention: `{StoreName}_{fieldName}` for all store fields.

    // ══════════════════════════════════════════════════════
    // View → HTML Generation
    // ══════════════════════════════════════════════════════

    fn next_nid(&mut self, tag: &str) -> String {
        self.node_counter += 1;
        format!("el-{}-{:03}", tag, self.node_counter)
    }

    fn view_to_html(&mut self, view: &ViewTree, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        match view {
            ViewTree::Element(el) => self.element_to_html(el, indent),
            ViewTree::Text(text) => {
                if text.trim().is_empty() {
                    String::new()
                } else {
                    format!("{}{}\n", pad, escape_html(text))
                }
            }
            ViewTree::Interpolation(expr) => {
                if self.in_for_template {
                    // Inside @for template: use data-bind-text marker
                    let nid = self.next_nid("interp");
                    let js_expr = self.expr_to_js(expr);
                    format!(
                        "{}<span data-nid=\"{}\" data-bind-text=\"{}\"></span>\n",
                        pad,
                        nid,
                        escape_html(&js_expr)
                    )
                } else {
                    // Emit placeholder span + deferred _effect() binding
                    let nid = self.next_nid("interp");
                    let js_expr = self.expr_to_js(expr);
                    // Strip trailing () for signal/computed name lookup
                    let js_expr_base = js_expr.trim_end_matches("()");
                    let value_access = if self.signal_names.contains(js_expr_base) {
                        format!("{}.value", js_expr_base)
                    } else {
                        js_expr
                    };
                    // Write to self.js (which is the deferred buffer after emit_view swap)
                    self.js.push_str(&format!(
                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) try {{ el.textContent = String({}); }} catch(e) {{}} }});\n",
                        nid, value_access
                    ));
                    format!("{}<span data-nid=\"{}\"></span>\n", pad, nid)
                }
            }
            ViewTree::Conditional(cond) => self.emit_conditional_html(cond, indent),
            ViewTree::ListRendering(list) => self.emit_list_rendering_html(list, indent),
            ViewTree::Slot(name) => match name {
                Some(ref n) => format!("{}<slot name=\"{}\"></slot>\n", pad, n),
                None => format!("{}<slot></slot>\n", pad),
            },
        }
    }

    fn element_to_html(&mut self, el: &ElementIR, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let _pad_inner = "  ".repeat(indent + 1);
        let nid = self.next_nid(&el.tag);
        let mut out = String::new();

        // Map omni tags to HTML tags
        let html_tag = map_tag(&el.tag);

        // If this is a known component, inline its view HTML.
        // When inside a @for template, props are mapped to their value
        // expressions so that data-bind-text markers reference the @for
        // loop variable; event handlers and prop-wiring effects are skipped
        // (they cannot work correctly for dynamically-cloned elements).
        if let Some(child_comp) = self.component_map.get(&el.tag).cloned() {
            if let Some(ref child_view) = child_comp.view {
                // Save and update current component context
                let saved_component = self.current_component.clone();
                self.current_component = child_comp.name.clone();
                let saved_signal_names: std::collections::HashSet<String> =
                    self.signal_names.iter().cloned().collect();
                let saved_renames_child: std::collections::HashMap<String, String> = self
                    .name_renames
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                if self.in_for_template {
                    // Inside @for template: map component prop names to their
                    // value expressions so data-bind-text markers resolve to
                    // the @for loop variable (e.g. text={item} → "text" → "item").
                    // States are NOT added to signal_names — instead they go into
                    // for_state_names so expr_to_js emits element-scoped expressions
                    // (__omni_forGetState) rather than .value reads on global signals.
                    for state in &child_comp.states {
                        let prefixed = format!("{}_{}", child_comp.name, state.name);
                        let default_js = self.expr_to_js(&state.initial_value);
                        self.name_renames
                            .insert(state.name.clone(), prefixed.clone());
                        self.for_state_names
                            .insert(prefixed, (state.name.clone(), default_js));
                    }
                    for c in &child_comp.computeds {
                        let prefixed = format!("{}_{}", child_comp.name, c.name);
                        self.name_renames.insert(c.name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                    // Add child component functions so class: map and call
                    // expressions in the inlined view resolve to the correct
                    // prefixed name (e.g. isActive -> TabPanel_isActive).
                    for f in &child_comp.fns {
                        let prefixed = format!("{}_{}", child_comp.name, f.name);
                        self.name_renames.insert(f.name.clone(), prefixed);
                    }
                    // Props: map to their value expression (which references
                    // the @for loop variable), not to prefixed signal names.
                    // Do NOT add to signal_names — the resolved name (e.g. "item")
                    // is not a signal, and adding the prop name would cause
                    // expr_to_js to emit "text.value" instead of the mapped "item".
                    for (prop_name, prop_value) in &el.props {
                        let js_value = self.expr_to_js(prop_value);
                        self.name_renames.insert(prop_name.clone(), js_value);
                    }
                    // Default props that aren't passed: use prefixed names.
                    for prop_name in child_comp.prop_defaults.keys() {
                        if !el.props.contains_key(prop_name) {
                            let prefixed = format!("{}_{}", child_comp.name, prop_name);
                            self.name_renames
                                .insert(prop_name.clone(), prefixed.clone());
                            self.signal_names.insert(prefixed);
                        }
                    }
                } else {
                    // Normal (non-@for) mode: use prefixed signal names.
                    for state in &child_comp.states {
                        let prefixed = format!("{}_{}", child_comp.name, state.name);
                        self.name_renames
                            .insert(state.name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                    for c in &child_comp.computeds {
                        let prefixed = format!("{}_{}", child_comp.name, c.name);
                        self.name_renames.insert(c.name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                    // Add child component functions so class: map and call
                    // expressions in the inlined view resolve to the correct
                    // prefixed name (e.g. isActive -> TabPanel_isActive).
                    for f in &child_comp.fns {
                        let prefixed = format!("{}_{}", child_comp.name, f.name);
                        self.name_renames.insert(f.name.clone(), prefixed);
                    }
                    for prop_name in child_comp.prop_defaults.keys() {
                        let prefixed = format!("{}_{}", child_comp.name, prop_name);
                        self.name_renames
                            .insert(prop_name.clone(), prefixed.clone());
                        self.signal_names.insert(prefixed);
                    }
                }

                // Generate child HTML inline
                let child_html = self.view_to_html(child_view, indent + 1);

                // Restore parent context
                self.current_component = saved_component;
                self.signal_names = saved_signal_names;
                self.name_renames = saved_renames_child;

                // Emit wrapper div with nid for event bindings on this component
                // For @for, add data-for-state-names for per-instance state sync
                let for_states_attr = if self.in_for_template && !child_comp.states.is_empty() {
                    let names: Vec<&str> =
                        child_comp.states.iter().map(|s| s.name.as_str()).collect();
                    format!(" data-for-state-names=\"{}\"", names.join(","))
                } else if self.in_for_template {
                    " data-for-state-names=\"\"".to_string()
                } else {
                    String::new()
                };
                out.push_str(&format!(
                    "{}<div data-nid=\"{}\" data-component=\"{}\"{for_states}>\n",
                    pad,
                    nid,
                    el.tag,
                    for_states = for_states_attr
                ));
                out.push_str(&child_html);
                out.push_str(&format!("{}</div>\n", pad));

                if !self.in_for_template {
                    // Wire parent props to child signals via reactive effects
                    for (prop_name, prop_value) in &el.props {
                        let js_value = self.expr_to_js(prop_value);
                        let child_prefixed = format!("{}_{}", child_comp.name, prop_name);
                        self.js.push_str(&format!(
                            "__omni_effect(() => {{ {}.set({}); }});\n",
                            child_prefixed, js_value
                        ));
                    }

                    // Emit event bindings for the component element
                    for (event_name, handler) in &el.events {
                        let js_event = map_event(event_name);
                        let pd = if handler.needs_prevent_default || js_event == "submit" {
                            " e.preventDefault();"
                        } else {
                            ""
                        };
                        let sp = if handler.needs_stop_propagation {
                            " e.stopPropagation();"
                        } else {
                            ""
                        };
                        match handler.kind.as_str() {
                            "fn_ref" => {
                                if let Some(ref fn_name) = handler.r#ref {
                                    if self.motion_names.contains(fn_name.as_str()) {
                                        // Motion reference: create AnimationPlayer and play on event
                                        self.js.push_str(&format!(
                                            "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ if (el.offsetParent !== null) {{ var p = __omni_motion_{motion}(el, {{}}); __omni_animations.set(\"{nid}_{js_event}\", p); p.play(); }} }}); }}\n",
                                            nid = nid,
                                            js_event = js_event,
                                            motion = fn_name,
                                        ));
                                    } else {
                                        let prefixed =
                                            format!("{}_{}", self.current_component, fn_name);
                                        let event_obj = emit_event_object(
                                            &handler.event_data,
                                            &handler.used_fields,
                                            "e",
                                        );
                                        self.js.push_str(&format!(
                                            "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {prefixed}({event_obj}); }}); }}\n",
                                            nid = nid, js_event = js_event, sp = sp, pd = pd, prefixed = prefixed, event_obj = event_obj
                                        ));
                                    }
                                }
                            }
                            "store_action" => {
                                if let Some(ref action_path) = handler.r#ref {
                                    let var_name = action_path.replace('.', "_");
                                    self.js.push_str(&format!(
                                        "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ {} {} {}(); }}); }}\n",
                                        nid, js_event, sp, pd, var_name
                                    ));
                                }
                            }
                            "inline_fn" => {
                                let wrapper_name =
                                    format!("__on{}_{}", event_name, nid.replace('-', "_"));
                                let params_str = handler.params.join(", ");
                                self.js.push_str(&format!(
                                    "function {}({}) {{\n",
                                    wrapper_name, params_str
                                ));
                                if let Some(ref body) = handler.body {
                                    for stmt in &body.statements {
                                        self.emit_statement(stmt);
                                    }
                                }
                                self.js.push_str("}\n");
                                let event_obj = emit_event_object(
                                    &handler.event_data,
                                    &handler.used_fields,
                                    "e",
                                );
                                self.js.push_str(&format!(
                                    "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {wrapper_name}({event_obj}); }}); }}\n",
                                    nid = nid, js_event = js_event, sp = sp, pd = pd, wrapper_name = wrapper_name, event_obj = event_obj
                                ));
                            }
                            _ => {}
                        }
                    }
                }

                return out;
            }
        }

        // Build attributes
        let mut attrs = format!(" data-nid=\"{}\"", nid);

        // Static props
        for (name, value) in &el.props {
            match name.as_str() {
                "testId" | "test_id" => {
                    attrs.push_str(&format!(
                        " data-testid=\"{}\"",
                        self.expr_to_static_str(value)
                    ));
                }
                // Real HTML attributes — emit as-is, not data-*
                "class" | "id" | "placeholder" | "type" | "value" | "disabled" | "checked"
                | "required" | "readonly" | "href" | "src" | "alt" | "title" | "name" | "rows"
                | "cols" | "max" | "min" | "step" | "autocomplete" | "autofocus" | "multiple"
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
                        self.js.push_str(&format!(
                            "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.setAttribute('{}', {}); }});\n",
                            nid, name, js_access
                        ));
                    } else if is_boolean_attr && val_str == "true" {
                        // Boolean attribute: emit just the attribute name (no ="true")
                        attrs.push_str(&format!(" {}", name));
                    } else if is_boolean_attr && val_str == "false" {
                        // Boolean attribute with value "false": skip entirely (don't emit)
                    } else if !val_str.is_empty() {
                        attrs.push_str(&format!(" {}=\"{}\"", name, escape_html(&val_str)));
                    } else {
                        attrs.push_str(&format!(" {}", name));
                    }
                }
                _ => {
                    // Emit as data attribute for unknown props,
                    // but preserve aria-* and data-* attributes as-is.
                    let val_str = self.expr_to_static_str(value);
                    if val_str.is_empty() {
                        continue;
                    }
                    if name.starts_with("aria-") || name.starts_with("data-") {
                        // Real HTML aria/data attributes — emit as-is
                        attrs.push_str(&format!(" {}=\"{}\"", name, escape_html(&val_str)));
                    } else {
                        attrs.push_str(&format!(" data-{}=\"{}\"", name, escape_html(&val_str)));
                    }
                }
            }
        }

        // Hero shared element transition tag — emit as data attribute
        if let Some(ref hero) = el.hero_tag {
            attrs.push_str(&format!(" data-hero=\"{}\"", escape_html(hero)));
        }
        // Optional custom transition motion name
        if let Some(ref transition) = el.hero_transition {
            let trans_str = self.expr_to_static_str(transition);
            if !trans_str.is_empty() {
                attrs.push_str(&format!(
                    " data-hero-transition=\"{}\"",
                    escape_html(&trans_str)
                ));
            }
        }

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
                                self.js.push_str(&format!(
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                    nid
                                ));
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
                                self.js.push_str(&format!(
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                    nid
                                ));
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
                                    self.js.push_str(&format!(
                                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) ",
                                        nid
                                    ));
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
                                to_css_property(prop),
                                sanitize_css_value(s)
                            ));
                        }
                        _ => {}
                    }
                }
                if !static_style_parts.is_empty() {
                    attrs.push_str(&format!(
                        " style=\"{}\"",
                        escape_html(&static_style_parts.join("; "))
                    ));
                }
            }
        }

        // Events → generate JS bindings
        for (event_name, handler) in &el.events {
            let js_event = map_event(event_name);
            if self.in_for_template {
                // Inside @for template: emit data-event-* attributes for delegation.
                // The @for container gets a single delegated listener that reads these.
                let val = match handler.kind.as_str() {
                    "fn_ref" => {
                        if let Some(ref fn_name) = handler.r#ref {
                            let prefixed = format!("{}_{}", self.current_component, fn_name);
                            format!("fn:{}", escape_html(&prefixed))
                        } else {
                            continue;
                        }
                    }
                    "store_action" => {
                        if let Some(ref action_path) = handler.r#ref {
                            let var_name = action_path.replace('.', "_");
                            format!("store:{}", escape_html(&var_name))
                        } else {
                            continue;
                        }
                    }
                    "inline_fn" => {
                        let wrapper_name = format!("__on{}_{}", event_name, nid.replace('-', "_"));
                        let params_str = handler.params.join(", ");
                        self.js
                            .push_str(&format!("function {}({}) {{\n", wrapper_name, params_str));
                        if let Some(ref body) = handler.body {
                            for stmt in &body.statements {
                                self.emit_statement(stmt);
                            }
                        }
                        self.js.push_str("}\n");
                        format!("fn:{}", escape_html(&wrapper_name))
                    }
                    _ => continue,
                };
                attrs.push_str(&format!(" data-event-{}=\"{}\"", js_event, val));
                // Register handler in the global lookup so the delegated
                // listener can find it (functions are IIFE-local, not on
                // window).
                let handler_name = match handler.kind.as_str() {
                    "fn_ref" => handler
                        .r#ref
                        .as_ref()
                        .map(|n| format!("{}_{}", self.current_component, n)),
                    "store_action" => handler.r#ref.as_ref().map(|p| p.replace('.', "_")),
                    "inline_fn" => Some(format!("__on{}_{}", event_name, nid.replace('-', "_"))),
                    _ => None,
                };
                if let Some(ref hname) = handler_name {
                    self.js
                        .push_str(&format!("__omni_forHandlers[\"{}\"] = {};\n", hname, hname));
                }
            } else {
                // ResizeEvent binds to window, not el.addEventListener
                if let Some(EventDataKind::ResizeEvent) = handler.event_data {
                    let event_obj =
                        emit_event_object(&handler.event_data, &handler.used_fields, "_e");
                    let handler_call = match handler.kind.as_str() {
                        "fn_ref" => {
                            if let Some(ref fn_name) = handler.r#ref {
                                let prefixed = format!("{}_{}", self.current_component, fn_name);
                                format!("{}({})", prefixed, event_obj)
                            } else {
                                continue;
                            }
                        }
                        "store_action" => {
                            if let Some(ref action_path) = handler.r#ref {
                                let var_name = action_path.replace('.', "_");
                                format!("{}()", var_name)
                            } else {
                                continue;
                            }
                        }
                        "inline_fn" => {
                            let wrapper_name =
                                format!("__on{}_{}", event_name, nid.replace('-', "_"));
                            let params_str = handler.params.join(", ");
                            self.js.push_str(&format!(
                                "function {}({}) {{\n",
                                wrapper_name, params_str
                            ));
                            if let Some(ref body) = handler.body {
                                for stmt in &body.statements {
                                    self.emit_statement(stmt);
                                }
                            }
                            self.js.push_str("}\n");
                            format!("{}({})", wrapper_name, event_obj)
                        }
                        _ => continue,
                    };
                    self.js.push_str(&format!(
                        "{{ window.addEventListener(\"resize\", (_e) => {{ {} }}); }}\n",
                        handler_call
                    ));
                    continue;
                }
                let pd = if handler.needs_prevent_default || js_event == "submit" {
                    " e.preventDefault();"
                } else {
                    ""
                };
                let sp = if handler.needs_stop_propagation {
                    " e.stopPropagation();"
                } else {
                    ""
                };
                match handler.kind.as_str() {
                    "fn_ref" => {
                        if let Some(ref fn_name) = handler.r#ref {
                            if self.motion_names.contains(fn_name.as_str()) {
                                // Motion reference: create AnimationPlayer and play on event
                                self.js.push_str(&format!(
                                    "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ if (el.offsetParent !== null) {{ var p = __omni_motion_{motion}(el, {{}}); __omni_animations.set(\"{nid}_{js_event}\", p); p.play(); }} }}); }}\n",
                                    nid = nid,
                                    js_event = js_event,
                                    motion = fn_name,
                                ));
                            } else {
                                let prefixed = format!("{}_{}", self.current_component, fn_name);
                                let event_obj = emit_event_object(
                                    &handler.event_data,
                                    &handler.used_fields,
                                    "e",
                                );
                                self.js.push_str(&format!(
                                    "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {prefixed}({event_obj}); }}); }}\n",
                                    nid = nid, js_event = js_event, sp = sp, pd = pd, prefixed = prefixed, event_obj = event_obj
                                ));
                            }
                        }
                    }
                    "store_action" => {
                        if let Some(ref action_path) = handler.r#ref {
                            let var_name = action_path.replace('.', "_");
                            self.js.push_str(&format!(
                                "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{ {} {} {}(); }}); }}\n",
                                nid, js_event, sp, pd, var_name
                            ));
                        }
                    }
                    "inline_fn" => {
                        let wrapper_name = format!("__on{}_{}", event_name, nid.replace('-', "_"));
                        let params_str = handler.params.join(", ");
                        self.js
                            .push_str(&format!("function {}({}) {{\n", wrapper_name, params_str));
                        if let Some(ref body) = handler.body {
                            for stmt in &body.statements {
                                self.emit_statement(stmt);
                            }
                        }
                        self.js.push_str("}\n");
                        let event_obj =
                            emit_event_object(&handler.event_data, &handler.used_fields, "e");
                        self.js.push_str(&format!(
                            "{{ const el = __omni_el(\"{nid}\"); if (el) el.addEventListener(\"{js_event}\", (e) => {{ {sp} {pd} {wrapper_name}({event_obj}); }}); }}\n",
                            nid = nid, js_event = js_event, sp = sp, pd = pd, wrapper_name = wrapper_name, event_obj = event_obj
                        ));
                    }
                    _ => {}
                }
            }
        }

        // Bindings (two-way)
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
                self.js.push_str(&format!("let {} = false;\n", guard_name));
                let (event_name, dom_prop) = if prop_name == "checked" {
                    ("change", "checked")
                } else {
                    ("input", "value")
                };
                self.js.push_str(&format!(
                    "{{ const el = __omni_el(\"{0}\"); if (el) el.addEventListener(\"{1}\", (e) => {{ if (!{2}) {{ {3}.set(e.target.{4}); }} }}); }}\n",
                    nid, event_name, guard_name, sig_name, dom_prop
                ));
                // Reactive binding: signal → DOM (guard against programmatic updates)
                self.js.push_str(&format!(
                    "__omni_effect(() => {{ const el = __omni_el(\"{0}\"); if (el) {{ {1} = true; el.{2} = {3}.value; {1} = false; }} }});\n",
                    nid, guard_name, prop_name, sig_name
                ));
            }
        }

        // Pre-scan children for interpolations → add data-bind-text to attrs
        // This MUST happen BEFORE writing the opening tag.
        // Note: under normal lowering, {expr} becomes ViewTree::Interpolation, not
        // ViewTree::Text. This pre-scan is a defense-in-depth fallback that should
        // never trigger for well-formed IR. If it does trigger, we validate that
        // the expression is a safe identifier before injecting into JS code.
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
                        attrs.push_str(&format!(" data-bind-text=\"{}\"", escape_html(expr)));
                    } else if is_safe {
                        // Generate reactive text binding for signals after DOM insert
                        interp_effects.push(format!(
                            "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.textContent = String({}.value); }});\n",
                            nid, expr
                        ));
                    }
                    // If not safe and not in template, silently skip — this fallback
                    // should never trigger for well-formed IR.
                }
            }
        }

        // Motion triggers — emit animation setup for @enter, @hover, @press, @focus, @leave
        // IMPORTANT: MUST be before the void element return below, otherwise void
        // elements like <input> never get their motion triggers emitted!
        if !el.motion_triggers.is_empty() {
            // Also emit HTML data attributes for each trigger so runtime can discover them
            for trigger in &el.motion_triggers {
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
                    attrs.push_str(&format!(" {}=\"{}\"", data_attr, escape_html(motion_name)));
                }
            }
            // Sub-attributes for in-view
            if let Some(ref once) = el.in_view_once {
                let js_val = self.expr_to_js(once);
                attrs.push_str(&format!(" data-in-view-once=\"{}\"", escape_html(&js_val)));
            }
            if let Some(ref repeat) = el.in_view_repeat {
                let js_val = self.expr_to_js(repeat);
                attrs.push_str(&format!(
                    " data-in-view-repeat=\"{}\"",
                    escape_html(&js_val)
                ));
            }
            if let Some(ref threshold) = el.in_view_threshold {
                let js_val = self.expr_to_js(threshold);
                attrs.push_str(&format!(
                    " data-in-view-threshold=\"{}\"",
                    escape_html(&js_val)
                ));
            }
            // Sub-attributes for scroll
            if let Some(ref s) = el.scroll_start {
                attrs.push_str(&format!(" data-scroll-start=\"{}\"", escape_html(s)));
            }
            if let Some(ref s) = el.scroll_end {
                attrs.push_str(&format!(" data-scroll-end=\"{}\"", escape_html(s)));
            }
            if let Some(ref scrub) = el.scroll_scrub {
                let js_val = self.expr_to_js(scrub);
                attrs.push_str(&format!(" data-scroll-scrub=\"{}\"", escape_html(&js_val)));
            }
            // Sub-attributes for drag
            if let Some(ref s) = el.drag_constraints {
                attrs.push_str(&format!(" data-drag-constraints=\"{}\"", escape_html(s)));
            }
            if let Some(ref elastic) = el.drag_elastic {
                let js_val = self.expr_to_js(elastic);
                attrs.push_str(&format!(" data-drag-elastic=\"{}\"", escape_html(&js_val)));
            }
            if let Some(ref scrub) = el.drag_scrub {
                let js_val = self.expr_to_js(scrub);
                attrs.push_str(&format!(" data-drag-scrub=\"{}\"", escape_html(&js_val)));
            }
            if let Some(ref momentum) = el.drag_momentum {
                let js_val = self.expr_to_js(momentum);
                attrs.push_str(&format!(" data-drag-momentum=\"{}\"", escape_html(&js_val)));
            }
            if let Some(ref threshold) = el.drag_threshold {
                let js_val = self.expr_to_js(threshold);
                attrs.push_str(&format!(
                    " data-drag-threshold=\"{}\"",
                    escape_html(&js_val)
                ));
            }
            if let Some(ref vel) = el.drag_velocity_max {
                let js_val = self.expr_to_js(vel);
                attrs.push_str(&format!(
                    " data-drag-velocity-max=\"{}\"",
                    escape_html(&js_val)
                ));
            }

            self.emit_motion_triggers_for_element(&nid, &el);
        }

        // @driven-by — external progress control for animations on this element
        if let Some(ref driven_by_expr) = el.driven_by {
            let driven_js = self.expr_to_js(driven_by_expr);
            // When driven-by is set, the animation progress is bound to the signal value.
            // We seek ALL animations on this element whenever the signal changes.
            // The signal is auto-tracked by __omni_effect.
            self.mount_js.push_str(&format!(
                "// @driven-by binding\n\
                 __omni_effect(() => {{\n\
                 \x20 const progress = Math.max(0, Math.min(1, Number({driven})));\n\
                 \x20 __omni_animations.forEach((player, key) => {{\n\
                 \x20\x20 if (key.startsWith(\"{nid}_\")) {{\n\
                 \x20\x20\x20 player.seek(progress);\n\
                 \x20\x20 }}\n\
                 \x20 }});\n\
                 }});\n",
                nid = nid,
                driven = driven_js
            ));
        }

        // @paused override — per-element pause override for all animations
        if let Some(paused) = &el.paused_override {
            let paused_js = self.expr_to_js(paused);
            self.mount_js.push_str(&format!(
                "// @paused override\n\
                 (function() {{\n\
                 \x20 var _paused = Boolean({paused});\n\
                 \x20 __omni_animations.forEach((player, key) => {{\n\
                 \x20\x20 if (key.startsWith(\"{nid}_\")) {{\n\
                 \x20\x20\x20 if (_paused) {{ player.pause(); }} else {{ player.resume(); }}\n\
                 \x20\x20 }}\n\
                 \x20 }});\n\
                 }})();\n",
                nid = nid,
                paused = paused_js
            ));
        }

        // states: — multi-state animation map (reserved or custom signal-driven)
        if let Some(ref states_ir) = el.states {
            self.emit_states_js(&nid, states_ir);
        }

        // Class map dynamic bindings for @for templates: add data-class-expr attr
        if self.in_for_template && !el.class_map_bindings.is_empty() {
            let mut class_map = serde_json::Map::new();
            for (class_name, expr) in &el.class_map_bindings {
                let js_expr = self.expr_to_js(expr);
                class_map.insert(class_name.clone(), serde_json::Value::String(js_expr));
            }
            if let Ok(json) = serde_json::to_string(&class_map) {
                attrs.push_str(&format!(" data-class-expr=\"{}\"", escape_html(&json)));
            }
        }

        // Self-closing tags (after motion/driven-by/states emit so animations are set up)
        let void_tags = [
            "br", "hr", "img", "input", "meta", "link", "area", "base", "col", "embed", "source",
            "track", "wbr",
        ];
        if void_tags.contains(&html_tag.as_str()) {
            out.push_str(&format!("{}<{}{} />\n", pad, html_tag, attrs));
            // Emit any interpolation _effect() calls
            for effect in &interp_effects {
                self.js.push_str(effect);
            }
            // Class map dynamic bindings
            if !el.class_map_bindings.is_empty() {
                if self.in_for_template {
                    // Inside @for template: store class expressions as JSON data attribute
                    // so the runtime can eval them per-item in the forEach scope.
                    let mut class_map = serde_json::Map::new();
                    for (class_name, expr) in &el.class_map_bindings {
                        let js_expr = self.expr_to_js(expr);
                        class_map.insert(class_name.clone(), serde_json::Value::String(js_expr));
                    }
                    if let Ok(json) = serde_json::to_string(&class_map) {
                        attrs.push_str(&format!(" data-class-expr=\"{}\"", escape_html(&json)));
                    }
                } else {
                    for (class_name, expr) in &el.class_map_bindings {
                        let js_expr = self.expr_to_js(expr);
                        let js_access = if self.signal_names.contains(&js_expr) {
                            format!("{}.value", js_expr)
                        } else {
                            js_expr
                        };
                        self.js.push_str(&format!(
                            "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.classList.toggle(\"{}\", !!({})); }});\n",
                            nid, class_name, js_access
                        ));
                    }
                }
            }
            return out;
        }

        // For <fragment> wrapper inside @for template: emit children without wrapper.
        // The <fragment> custom element would strip <tr>/<td> children when parsed
        // by the browser inside a table context (<tbody>, <template> within <tbody>).
        if self.in_for_template && html_tag == "fragment" {
            for child in &el.children {
                let child_html = self.view_to_html(child, indent + 1);
                out.push_str(&child_html);
            }
            // Fragment attrs like data-class-expr are handled via data-* on inner elements
            // Skip the closing tag since we didn't emit an opening tag.
        } else {
            out.push_str(&format!("{}<{}{}>\n", pad, html_tag, attrs));

            // Children
            for child in &el.children {
                let child_html = self.view_to_html(child, indent + 1);
                out.push_str(&child_html);
            }

            out.push_str(&format!("{}</{}>\n", pad, html_tag));
        }

        // Emit any interpolation _effect() calls
        for effect in &interp_effects {
            self.js.push_str(effect);
        }

        // Class map dynamic bindings: for each dynamic class, emit an effect
        // that toggles it reactively via classList.toggle().
        // When inside @for template, this is handled by data-class-expr attrs instead.
        if !self.in_for_template && !el.class_map_bindings.is_empty() {
            for (class_name, expr) in &el.class_map_bindings {
                let js_expr = self.expr_to_js(expr);
                let js_access = if self.signal_names.contains(&js_expr) {
                    format!("{}.value", js_expr)
                } else {
                    js_expr
                };
                self.js.push_str(&format!(
                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.classList.toggle(\"{}\", !!({})); }});\n",
                    nid, class_name, js_access
                ));
            }
        }

        // Record source map entry
        self.source_map.push(SourceMapEntry {
            node_id: nid.clone(),
            omni_span: SourceSpan {
                line: el.source_line,
                col: el.source_col,
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
    // Motion / Animation Codegen
    // ══════════════════════════════════════════════════════

    /// Convert a MotionBodyIR to a JS config object string for AnimationPlayer.
    fn motion_body_to_config(&self, body: &MotionBodyIR) -> String {
        match body {
            MotionBodyIR::Keyframe(kf) => self.keyframe_to_config(kf),
            MotionBodyIR::Timeline(tl) => self.timeline_body_to_config(tl),
            MotionBodyIR::CallbackOnly(cb) => self.callback_to_config(cb),
        }
    }

    /// Convert a KeyframeIR to a JS config object fragment.
    fn keyframe_to_config(&self, kf: &KeyframeIR) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Extract from (position 0.0) and to (position 1.0) stops
        let from_stop = kf
            .stops
            .iter()
            .find(|s| (s.position - 0.0).abs() < f64::EPSILON);
        let to_stop = kf
            .stops
            .iter()
            .find(|s| (s.position - 1.0).abs() < f64::EPSILON);

        if let Some(from) = from_stop {
            let props: Vec<String> = from
                .properties
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                .collect();
            if !props.is_empty() {
                parts.push(format!("\"from\": {{{}}}", props.join(", ")));
            }
        }

        if let Some(to) = to_stop {
            let props: Vec<String> = to
                .properties
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                .collect();
            if !props.is_empty() {
                parts.push(format!("\"to\": {{{}}}", props.join(", ")));
            }
        }

        // `over` duration (in milliseconds)
        match &kf.over {
            ExprIR::LiteralExpr { value } => {
                if let Some(n) = value.as_f64() {
                    parts.push(format!("\"over\": {}", n));
                }
            }
            _ => {
                let js = self.expr_to_js(&kf.over);
                parts.push(format!("\"over\": {}", js));
            }
        }

        // Easing
        if let Some(ref ease) = kf.ease {
            parts.push(format!("\"easing\": {}", self.easing_to_js(ease)));
        }

        // Path animation
        if let Some(ref path) = kf.path {
            parts.push(format!("\"path\": \"{}\"", path.replace('"', "\\\"")));
            if let Some(rotate) = kf.path_rotate {
                parts.push(format!("\"pathRotate\": {}", rotate));
            }
        }

        // Playback config properties
        let playback_parts = self.playback_to_config(&kf.playback);
        parts.extend(playback_parts);

        // Path animation support
        if let Some(ref path) = kf.path {
            parts.push(format!("\"path\": \"{}\"", path.replace('"', "\\\"")));
        }
        if let Some(path_rotate) = kf.path_rotate {
            if path_rotate {
                parts.push("\"pathRotate\": true".to_string());
            }
        }

        // Callback blocks (onStart, onUpdate, onComplete, onRepeat, onInterrupt)
        if !kf.callbacks.is_empty() {
            let callbacks: Vec<String> = kf
                .callbacks
                .iter()
                .map(|(kind, expr)| {
                    let name = match kind {
                        CallbackKindIR::OnStart => "onStart",
                        CallbackKindIR::OnUpdate => "onUpdate",
                        CallbackKindIR::OnComplete => "onComplete",
                        CallbackKindIR::OnRepeat => "onRepeat",
                        CallbackKindIR::OnInterrupt => "onInterrupt",
                    };
                    let body_js = self.expr_to_js(expr);
                    format!("\"{}\": function() {{ {} }}", name, body_js)
                })
                .collect();
            parts.push(format!("\"callbacks\": {{{}}}", callbacks.join(", ")));
        }

        parts.join(", ")
    }

    /// Convert a TimelineIR body to a JS config fragment.
    fn timeline_body_to_config(&self, tl: &TimelineIR) -> String {
        // For inline timeline in a motion definition, we serialize as a
        // keyframe-like structure. The timeline orchestration is handled
        // at the element trigger level via emit_timeline_js.
        let mut parts: Vec<String> = Vec::new();
        parts.push("\"type\": \"timeline\"".into());
        if !tl.items.is_empty() {
            parts.push(format!("\"itemCount\": {}", tl.items.len()));
        }
        let playback_parts = self.playback_to_config(&tl.playback);
        parts.extend(playback_parts);
        parts.join(", ")
    }

    /// Convert a CallbackBodyIR to a JS config fragment.
    fn callback_to_config(&self, cb: &CallbackBodyIR) -> String {
        let mut parts: Vec<String> = Vec::new();
        // `over` duration
        match &cb.over {
            ExprIR::LiteralExpr { value } => {
                if let Some(n) = value.as_f64() {
                    parts.push(format!("\"over\": {}", n));
                }
            }
            _ => {
                let js = self.expr_to_js(&cb.over);
                parts.push(format!("\"over\": {}", js));
            }
        }
        // Callback functions
        if !cb.callbacks.is_empty() {
            let callbacks: Vec<String> = cb
                .callbacks
                .iter()
                .map(|(kind, expr)| {
                    let name = match kind {
                        CallbackKindIR::OnStart => "onStart",
                        CallbackKindIR::OnUpdate => "onUpdate",
                        CallbackKindIR::OnComplete => "onComplete",
                        CallbackKindIR::OnRepeat => "onRepeat",
                        CallbackKindIR::OnInterrupt => "onInterrupt",
                    };
                    // Callback body is an expression that becomes a function body
                    let body_js = self.expr_to_js(expr);
                    format!("\"{}\": function() {{ {} }}", name, body_js)
                })
                .collect();
            parts.push(format!("\"callbacks\": {{{}}}", callbacks.join(", ")));
        }
        let playback_parts = self.playback_to_config(&cb.playback);
        parts.extend(playback_parts);
        parts.join(", ")
    }

    /// Convert an EasingIR to a JS string or expression.
    fn easing_to_js(&self, easing: &EasingIR) -> String {
        match easing {
            EasingIR::Named(name) => format!("\"{}\"", name),
            EasingIR::CubicBezier { x1, y1, x2, y2 } => {
                format!("\"cubic-bezier({},{},{},{})\"", x1, y1, x2, y2)
            }
            EasingIR::Spring {
                tension,
                friction,
                velocity,
            } => {
                format!(
                    "new SpringSolver({}, {}, 1, {})",
                    tension,
                    friction,
                    velocity.unwrap_or(0.0)
                )
            }
            EasingIR::Back {
                intensity: _,
                direction,
            } => {
                // The parser stores the full name ("backIn"), also support short form ("in")
                let dir = direction.as_deref().unwrap_or("backOut");
                let name = match dir {
                    "backIn" | "in" => "backIn",
                    "backOut" | "out" => "backOut",
                    "backInOut" | "inOut" => "backInOut",
                    _ => "backOut",
                };
                format!("\"{}\"", name)
            }
            EasingIR::Elastic {
                amplitude: _,
                period: _,
                direction,
            } => {
                // The parser stores the full name ("elasticIn"), also support short form ("in")
                let dir = direction.as_deref().unwrap_or("elasticOut");
                let name = match dir {
                    "elasticIn" | "in" => "elasticIn",
                    "elasticOut" | "out" => "elasticOut",
                    "elasticInOut" | "inOut" => "elasticInOut",
                    _ => "elasticOut",
                };
                format!("\"{}\"", name)
            }
            EasingIR::Steps { count, direction } => {
                format!("\"steps({},{})\"", count, direction)
            }
        }
    }

    /// Convert PlaybackIR to a list of JS config property strings.
    fn playback_to_config(&self, playback: &PlaybackIR) -> Vec<String> {
        let mut parts = Vec::new();
        if let Some(ref loop_count) = playback.loop_count {
            match loop_count {
                LoopValue::Infinite => parts.push("\"iterations\": Infinity".into()),
                LoopValue::Count(n) => parts.push(format!("\"iterations\": {}", n)),
            }
        }
        if let Some(ref direction) = playback.direction {
            let dir_str = match direction {
                DirectionValue::Normal => "normal",
                DirectionValue::Reverse => "reverse",
                DirectionValue::Alternate => "alternate",
                DirectionValue::AlternateReverse => "alternate-reverse",
            };
            parts.push(format!("\"direction\": \"{}\"", dir_str));
        }
        if let Some(ref fill) = playback.fill {
            let fill_str = match fill {
                FillValue::None => "none",
                FillValue::Forwards => "forwards",
                FillValue::Backwards => "backwards",
                FillValue::Both => "both",
            };
            parts.push(format!("\"fill\": \"{}\"", fill_str));
        }
        if let Some(ref delay) = playback.delay {
            match delay {
                ExprIR::LiteralExpr { value } => {
                    if let Some(n) = value.as_f64() {
                        parts.push(format!("\"delay\": {}", n));
                    }
                }
                _ => {
                    let js = self.expr_to_js(delay);
                    parts.push(format!("\"delay\": {}", js));
                }
            }
        }
        if playback.paused {
            parts.push("\"paused\": true".into());
        }
        if playback.respect_reduced_motion {
            parts.push("\"respectReducedMotion\": true".into());
        }
        if let Some(ref when) = playback.when {
            let when_str = match when {
                WhenValue::Idle => "beforeChildren",
                WhenValue::Active => "afterChildren",
                WhenValue::IdleOrActive => "withChildren",
            };
            parts.push(format!("\"when\": \"{}\"", when_str));
        }
        if let Some(ref dc) = playback.delay_children {
            let js = self.expr_to_js(dc);
            parts.push(format!("\"delayChildren\": {}", js));
        }
        if let Some(ref sc) = playback.stagger_children {
            let js = self.expr_to_js(sc);
            parts.push(format!("\"staggerChildren\": {}", js));
        }
        if let Some(ref sf) = playback.stagger_from {
            let from_str = match sf {
                StaggerDirection::First => "start",
                StaggerDirection::Last => "end",
                StaggerDirection::Center => "center",
                StaggerDirection::Edges => "edges",
                StaggerDirection::Random => "edges",
            };
            parts.push(format!("\"staggerFrom\": \"{}\"", from_str));
        }
        if let Some(ref rd) = playback.repeat_delay {
            let js = self.expr_to_js(rd);
            parts.push(format!("\"repeatDelay\": {}", js));
        }
        if playback.repeat_refresh {
            parts.push("\"repeatRefresh\": true".into());
        }
        if let Some(ref ts) = playback.time_scale {
            let js = self.expr_to_js(ts);
            parts.push(format!("\"timeScale\": {}", js));
        }
        if let Some(ref interrupt) = playback.interrupt {
            let int_str = match interrupt {
                InterruptValue::Compose => "compose",
                InterruptValue::Cancel => "cancel",
                InterruptValue::Finish => "finish",
                InterruptValue::Replace => "replace",
            };
            parts.push(format!("\"interrupt\": \"{}\"", int_str));
        }
        parts
    }

    /// Check if a property is a CSS transform property.
    fn is_transform_prop(key: &str) -> bool {
        matches!(
            key,
            "x" | "y"
                | "scale"
                | "scaleX"
                | "scaleY"
                | "rotate"
                | "rotateX"
                | "rotateY"
                | "rotateZ"
        )
    }

    // ══════════════════════════════════════════════════════
    // Expression → JavaScript
    // ══════════════════════════════════════════════════════

    pub(super) fn expr_to_js(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => serde_value_to_js(value),
            ExprIR::IdentifierExpr { name } => {
                let resolved = self
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.global_renames.get(name.as_str()))
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                if self.signal_names.contains(resolved.as_str()) {
                    format!("{}.value", resolved)
                } else if self.signal_names.contains(name.as_str()) {
                    format!("{}.value", name)
                } else if self.in_for_template
                    && self.for_state_names.contains_key(resolved.as_str())
                {
                    let (orig_name, default_js) = &self.for_state_names[resolved.as_str()];
                    format!("__omni_forGetState(el, '{}', {})", orig_name, default_js)
                } else {
                    resolved
                }
            }
            ExprIR::BinaryOpExpr { op, left, right } => {
                let l = self.expr_to_js(left);
                let r = self.expr_to_js(right);
                let js_op = map_binary_op(op);
                format!("{} {} {}", l, js_op, r)
            }
            ExprIR::UnaryOpExpr { op, operand } => {
                let o = self.expr_to_js(operand);
                format!("{}{}", op, o)
            }
            ExprIR::FieldAccessExpr { object, field } => {
                let raw_obj_name = if let ExprIR::IdentifierExpr { name } = object.as_ref() {
                    Some(name.clone())
                } else {
                    None
                };
                let obj = self.expr_to_js(object);
                let check_name = raw_obj_name.as_deref().unwrap_or(&obj);
                if self.store_names.contains(check_name) || self.store_names.contains(&obj) {
                    let renamed_obj = self
                        .name_renames
                        .get(check_name)
                        .or_else(|| self.name_renames.get(obj.as_str()))
                        .cloned()
                        .unwrap_or_else(|| obj.clone());
                    let var_name = format!("{}_{}", renamed_obj, field);
                    if self.signal_names.contains(&var_name) {
                        format!("{}.value", var_name)
                    } else {
                        var_name
                    }
                } else {
                    format!("{}.{}", obj, field)
                }
            }
            ExprIR::OptionalChainExpr { object, field } => {
                let raw_obj_name = if let ExprIR::IdentifierExpr { name } = object.as_ref() {
                    Some(name.clone())
                } else {
                    None
                };
                let obj = self.expr_to_js(object);
                let check_name = raw_obj_name.as_deref().unwrap_or(&obj);
                if self.store_names.contains(check_name) || self.store_names.contains(&obj) {
                    let renamed_obj = self
                        .name_renames
                        .get(check_name)
                        .or_else(|| self.name_renames.get(obj.as_str()))
                        .cloned()
                        .unwrap_or_else(|| obj.clone());
                    let var_name = format!("{}_{}", renamed_obj, field);
                    if self.signal_names.contains(&var_name) {
                        format!("{}?.value", var_name)
                    } else {
                        var_name
                    }
                } else {
                    format!("{}?.{}", obj, field)
                }
            }
            ExprIR::CallExpr { name, args } => {
                if name == "println" || name == "print" {
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                    return format!("console.log({})", arg_strs.join(", "));
                }
                let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                if let Some(native) = try_format_stdlib_call(name, &arg_strs, Target::OmniView) {
                    return native;
                }
                let resolved = self
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.global_renames.get(name.as_str()))
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                format!("{}({})", resolved, arg_strs.join(", "))
            }
            ExprIR::StoreFieldAccessExpr {
                store_alias,
                field_name,
                call_args,
                ..
            } => {
                let var_name = format!("{}_{}", store_alias, field_name);
                if call_args.is_empty() {
                    if self.signal_names.contains(&var_name) {
                        format!("{}.value", var_name)
                    } else {
                        var_name.clone()
                    }
                } else {
                    let arg_strs: Vec<String> =
                        call_args.iter().map(|a| self.expr_to_js(a)).collect();
                    format!("{}({})", var_name, arg_strs.join(", "))
                }
            }
            ExprIR::TernaryIR {
                cond,
                then_expr,
                else_expr,
            } => {
                let c = self.expr_to_js(cond);
                let t = self.expr_to_js(then_expr);
                let e = self.expr_to_js(else_expr);
                format!("{} ? {} : {}", c, t, e)
            }
            ExprIR::MethodCallExpr {
                method,
                args,
                object,
                ..
            } => {
                let raw_store_name = if let ExprIR::IdentifierExpr { name } = object.as_ref() {
                    name.clone()
                } else {
                    String::new()
                };
                let base = self.expr_to_js(object);
                if self.store_names.contains(&raw_store_name) || self.store_names.contains(&base) {
                    let renamed_base = self
                        .name_renames
                        .get(raw_store_name.as_str())
                        .or_else(|| self.name_renames.get(base.as_str()))
                        .cloned()
                        .unwrap_or_else(|| base.clone());
                    let var_name = format!("{}_{}", renamed_base, method);
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                    return format!("{}({})", var_name, arg_strs.join(", "));
                }
                let obj = if self.signal_names.contains(&base) {
                    format!("{}.value", base)
                } else {
                    base
                };
                match method.as_str() {
                    "to_string" => format!("String({})", obj),
                    "len" => format!("({} || []).length", obj),
                    "is_empty" => format!("({} || []).length === 0", obj),
                    "get" => {
                        let idx = args.first().map(|a| self.expr_to_js(a)).unwrap_or_default();
                        format!("({} || [])[{}]", obj, idx)
                    }
                    "push" => {
                        let vals: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                        format!(
                            "((_a) => {{ _a.push({}); return _a.slice(); }})({} || [])",
                            vals.join(", "),
                            obj
                        )
                    }
                    "pop" => format!(
                        "((_a) => {{ _a.pop(); return _a.slice(); }})({} || [])",
                        obj
                    ),
                    _ => format!(
                        "{}.{}({})",
                        obj,
                        method,
                        args.iter()
                            .map(|a| self.expr_to_js(a))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                }
            }
            ExprIR::ArrayExpr { elements } => {
                let items: Vec<String> = elements.iter().map(|e| self.expr_to_js(e)).collect();
                format!("[{}]", items.join(", "))
            }
            ExprIR::StructExpr { fields, .. } => {
                let entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            ExprIR::VariantLiteral {
                variant, fields, ..
            } => {
                let mut entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                    .collect();
                entries.insert(0, format!("\"_variant\": \"{}\"", variant));
                format!("{{{}}}", entries.join(", "))
            }
            ExprIR::BlockExpr { statements, tail } => {
                if statements.is_empty() {
                    self.expr_to_js(tail)
                } else {
                    let mut js = String::from("(function(){ ");
                    for stmt in statements {
                        js.push_str(&self.stmt_to_js_inline(stmt));
                        js.push(';');
                    }
                    js.push_str("return ");
                    js.push_str(&self.expr_to_js(tail));
                    js.push_str("; })()");
                    js
                }
            }
            ExprIR::IndexExpr { object, index } => {
                let obj = self.expr_to_js(object);
                let idx = self.expr_to_js(index);
                let obj_access = if self.signal_names.contains(&obj) {
                    format!("{}.value", obj)
                } else {
                    obj
                };
                format!("({} || [])[{}]", obj_access, idx)
            }
            ExprIR::AwaitExpr { operand } => {
                format!("await {}", self.expr_to_js(operand))
            }
            ExprIR::MatchExpr { matched, arms } => {
                let matched_js = self.expr_to_js(matched);
                let cases: Vec<String> = arms
                    .iter()
                    .map(|arm| {
                        let body_js = self.block_to_expr(&arm.body);
                        match &arm.pattern.kind {
                            MatchPatternKindIR::Variant { name, bindings } => {
                                let let_bindings: Vec<String> = bindings
                                    .iter()
                                    .map(|b| format!(" const {} = _v.{}; ", b, b))
                                    .collect();
                                format!(
                                    "if (_v._variant === \"{}\") {{ {} return {}; }}",
                                    name,
                                    let_bindings.join(""),
                                    body_js
                                )
                            }
                            MatchPatternKindIR::Wildcard => {
                                format!("return {};", body_js)
                            }
                            MatchPatternKindIR::Literal { value } => {
                                let v = self.js_value(value);
                                format!("if (_v._variant === {}) {{ return {}; }}", v, body_js)
                            }
                        }
                    })
                    .collect();
                format!(
                    "/*match*/ (() => {{ const _v = {}; {}; throw new Error('match: no arm matched'); }})()",
                    matched_js,
                    cases.join(" ")
                )
            }
            ExprIR::ClosureExpr { params, body } => {
                let body_js = self.expr_to_js(body);
                let params_str = params.join(", ");
                format!("(({}) => {})", params_str, body_js)
            }
        }
    }

    fn block_to_expr(&self, block: &BlockIR) -> String {
        if let Some(last) = block.statements.last() {
            match last {
                StatementIR::ExprStmtIR { expr } => self.expr_to_js(expr),
                StatementIR::ReturnStmtIR { value } => value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into()),
                _ => "undefined".into(),
            }
        } else {
            "undefined".into()
        }
    }

    fn js_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "null".into(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => {
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t")
                    .replace('\0', "\\0");
                format!("\"{}\"", escaped)
            }
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.js_value(v)).collect();
                format!("[{}]", items.join(", "))
            }
            serde_json::Value::Object(_) => "({})".into(),
        }
    }

    fn expr_to_static_str(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                _ => String::new(),
            },
            ExprIR::IdentifierExpr { name } => format!("{{{}}}", name),
            _ => String::new(),
        }
    }

    // ══════════════════════════════════════════════════════
    // Style → CSS
    // ══════════════════════════════════════════════════════

    fn expr_to_css(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => match value {
                serde_json::Value::String(s) => sanitize_css_value(s),
                // Emit number as-is; dimensions carry their unit as strings from IR.
                // Strip trailing ".0" from integer-valued floats (serde_json stores as f64).
                serde_json::Value::Number(n) => {
                    let s = n.to_string();
                    if s.ends_with(".0") {
                        s[..s.len() - 2].to_string()
                    } else {
                        s
                    }
                }
                _ => String::new(),
            },
            _ => String::new(),
        }
    }
}

// ══════════════════════════════════════════════════════════
// Value Serialization
// ══════════════════════════════════════════════════════════

/// Convert a serde_json::Value to JavaScript syntax.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_debug_html() {
        let html = wrap_debug_html(
            "/// OmniView Native Runtime v2 — reactive signal system\nconsole.log('hi');",
            "body { color: red; }",
            "<div>Hello</div>",
        );
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("OmniView Debug"));
        assert!(html.contains("console.log('hi');"));
        assert!(html.contains("body { color: red; }"));
        assert!(html.contains("<div>Hello</div>"));
        assert!(html.contains("OmniView Native Runtime v2"));
    }
}
