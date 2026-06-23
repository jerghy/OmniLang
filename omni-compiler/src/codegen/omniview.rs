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

    // Build per-component output map for editor hot-replace
    let mut components: HashMap<String, ComponentOutput> = HashMap::new();
    for (name, js) in &gen.component_js {
        components.insert(name.clone(), ComponentOutput {
            js: js.clone(),
            html: String::new(),
            css: String::new(),
        });
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
}

impl OmniViewGenerator {
    // ══════════════════════════════════════════════════════
    // Headers
    // ══════════════════════════════════════════════════════

    fn emit_js_header(&mut self) {
        self.js
            .push_str("/// OmniView — generated by omni-compiler v0.1.0 (fixed)\n");
        self.js.push_str("/// Source: .omni → Rust → JS/HTML/CSS\n");
        self.js.push_str("(function() {\n");
        self.js.push_str("\"use strict\";\n\n");
    }

    /// Emit the inline mini-signal runtime (~200 lines).
    /// Based on the design from Spec §3.7.
    fn emit_signal_runtime(&mut self) {
        self.js.push_str(
            r#"/// ─── OmniView Native Runtime v2 — reactive signal system ───
/// Inline runtime for OmniView-compiled output. No external dependencies.
let __omni_activeEffect = null;
const __omni_subs = new Map();
const __omni_forHandlers = {};

function __omni_signal(name, init) {
  let val = init;
  const subs = new Set();
  const signal = {
    get value() {
      if (__omni_activeEffect) {
        subs.add(__omni_activeEffect);
        __omni_activeEffect._deps.add(() => subs.delete(__omni_activeEffect));
      }
      return val;
    },
    set value(v) {
      if (val !== v) {
        val = v;
        const s = new Set(subs);
        s.forEach(e => {
          const prev = __omni_activeEffect;
          __omni_activeEffect = e;
          e.run();
          __omni_activeEffect = prev;
        });
      }
    },
    get() { return signal.value; },
    set(v) { signal.value = v; },
    peek() { return val; },
  };
  __omni_subs.set(name, signal);
  return signal;
}

function __omni_computed(name, fn) {
  let val, dirty = true;
  const signal = __omni_signal(name, undefined);
  const run = () => {
    dirty = false;
    const prev = __omni_activeEffect;
    const deps = new Set();
    __omni_activeEffect = { _deps: deps, run: () => { dirty = true; signal.set(fn()); } };
    const newVal = fn();
    __omni_activeEffect = prev;
    val = newVal;
    signal.peek(); // ensure dependency is tracked
    return val;
  };
  run();
  return {
    get value() {
      signal.value; // trigger dependency tracking on every read
      return dirty ? run() : val;
    },
    get() { return signal.value; },
    _signal: signal,
  };
}

function __omni_effect(fn) {
  const e = { _deps: new Set(), run: fn };
  const prev = __omni_activeEffect;
  __omni_activeEffect = e;
  fn();
  __omni_activeEffect = prev;
  return () => e._deps.forEach(d => d());
}

function __omni_el(nid) {
  return document.querySelector('[data-nid="' + nid + '"]');
}

function __omni_bindText(nid) {
  const el = __omni_el(nid);
  return {
    set text(v) { if (el) el.textContent = String(v); },
  };
}

/// ─── End Signal Runtime ────────────────────────────

"#,
        );
    }

    /// Embed the OmniView animation runtime JS.
    fn emit_animation_runtime(&mut self) {
        self.js.push_str("/// ─── OmniView Motion Runtime ───\n");
        self.js
            .push_str(include_str!("../../../build/omni-motion-runtime.js"));
        self.js.push('\n');
    }

    // ══════════════════════════════════════════════════════
    // Store Emission
    // ══════════════════════════════════════════════════════

    /// Emit a store's signals, getters, and actions as global JS variables.
    /// Naming convention: `{StoreName}_{fieldName}` for all store fields.
    fn emit_store(&mut self, _store_name: &str, store: &StoreIR) {
        let prefix = |field: &str| format!("{}_{}", store.name, field);
        self.store_names.insert(store.name.clone());

        // Set up rename mapping: store-local names → flat JS names
        // This ensures getter/action bodies reference the correct JS variables
        // Save and restore the rename map to prevent cross-store pollution
        let saved_renames: std::collections::HashMap<String, String> =
            std::mem::take(&mut self.name_renames);
        // Also temporarily populate signal_names with flat names so .value is emitted
        let saved_signals: Vec<String> = self.signal_names.iter().cloned().collect();

        // Emit signals
        for sig_name in store.signals.keys() {
            let var_name = prefix(sig_name);
            let init_val = store
                .signal_defaults
                .get(sig_name)
                .map(serde_value_to_js)
                .unwrap_or_else(|| "undefined".into());

            self.js.push_str(&format!(
                "const {} = __omni_signal(\"{}\", {});\n",
                var_name, var_name, init_val
            ));
            // Map local name → flat name for getter/action bodies (must clone before moves)
            self.name_renames.insert(sig_name.clone(), var_name.clone());
            self.store_signal_names.insert(var_name.clone());
            self.store_field_names.insert(var_name.clone());
            // Also add flat name to signal_names so .value is emitted
            self.signal_names.insert(var_name);
        }

        // Emit getters as computed
        for (getter_name, body) in &store.getters {
            let var_name = prefix(getter_name);
            let body_js = self.expr_to_js(body);
            self.js.push_str(&format!(
                "const {} = __omni_computed(\"{}\", () => ({}));\n",
                var_name, var_name, body_js
            ));
            // Map local getter name → flat name for action bodies
            self.name_renames
                .insert(getter_name.clone(), var_name.clone());
            // Also add flat name to signal_names so .value is emitted when referenced
            self.signal_names.insert(var_name.clone());
            self.store_signal_names.insert(var_name.clone());
            self.store_field_names.insert(var_name);
        }

        // Emit actions as functions
        for action in &store.actions {
            let func_name = prefix(&action.name);
            let params: Vec<String> = action.params.iter().map(|p| p.name.clone()).collect();
            self.js.push_str(&format!(
                "function {}({}) {{\n",
                func_name,
                params.join(", ")
            ));
            for stmt in &action.body.statements {
                self.emit_statement(stmt);
            }
            self.js.push_str("}\n\n");
            self.store_field_names.insert(func_name.clone());
            // Map local action name → flat name for intra-store calls
            self.name_renames.insert(action.name.clone(), func_name);
        }

        // Restore signal_names (keep store flat names for component use)
        for name in saved_signals {
            self.signal_names.insert(name);
        }
        // Merge back saved renames (keep new entries from this store)
        for (k, v) in saved_renames {
            self.name_renames.entry(k).or_insert(v);
        }

        self.js.push('\n');
    }

    // ══════════════════════════════════════════════════════
    // Component Generation
    // ══════════════════════════════════════════════════════

    fn generate_component(
        &mut self,
        name: &str,
        comp: &ComponentIR,
        ir: &IRProgram,
        is_root: bool,
    ) {
        let js_before = self.js.len();
        self.current_component = name.to_string();
        // Clear component-local signal renames (global function/event renames persist).
        self.name_renames.clear();
        self.signal_names.clear();
        self.comp_mount_js.clear();

        // Editor mode: wrap component in a factory IIFE so it can be hot-replaced
        // via __omni_replaceComponent in the editor bridge.
        if self.editor_mode {
            self.js.push_str(&format!(
                "window.__omni_components[\"{}\"] = (function() {{\n",
                name
            ));
        }
        // Register THIS component's signal/computed names with component prefix.
        for state in &comp.states {
            let prefixed = format!("{}_{}", name, state.name);
            self.name_renames
                .insert(state.name.clone(), prefixed.clone());
            self.signal_names.insert(prefixed);
        }
        for c in &comp.computeds {
            let prefixed = format!("{}_{}", name, c.name);
            self.name_renames.insert(c.name.clone(), prefixed.clone());
            self.signal_names.insert(prefixed);
        }
        // Add store signal/getter names so store field access uses .value
        for store_name in &comp.store_refs {
            if let Some(store) = ir.all_stores().get(store_name.as_str()) {
                for sig_name in store.signals.keys() {
                    let full_name = format!("{}_{}", store_name, sig_name);
                    self.signal_names.insert(full_name);
                }
                for getter_name in store.getters.keys() {
                    let full_name = format!("{}_{}", store_name, getter_name);
                    self.signal_names.insert(full_name);
                }
            } else {
                // Inline store (defined inside component body, not in all_stores):
                // its fields are merged into component states/computeds/actions.
                // Map store name → component name so MethodCallExpr like
                // CartStore.addItem() resolves to ComponentName_addItem().
                self.store_names.insert(store_name.clone());
                self.name_renames
                    .insert(store_name.clone(), name.to_string());
            }
        }

        self.js
            .push_str(&format!("\n/// ── Component: {} ──\n\n", name));

        // Signal declarations
        for state in &comp.states {
            self.emit_signal_decl(state);
        }

        // Computed declarations
        for computed in &comp.computeds {
            self.emit_computed_decl(computed);
        }

        // Prop defaults — emit as signals so they're accessible in view/fn code.
        // Use COMPONENT-PREFIXED names to avoid collisions.
        for (prop_name, prop_default) in &comp.prop_defaults {
            let init_val = self.expr_to_js(prop_default);
            let prefixed = format!("{}_{}", name, prop_name);
            self.js.push_str(&format!(
                "const {} = __omni_signal(\"{}\", {});\n",
                prefixed, prefixed, init_val
            ));
            self.name_renames
                .insert(prop_name.clone(), prefixed.clone());
            self.signal_names.insert(prefixed);
        }

        // Event prop stubs — dispatch custom DOM events on the component's wrapper element.
        // Names are prefixed with component name to avoid global collisions.
        // Must be emitted BEFORE component functions so the functions can call them.
        let fn_prefix = format!("{}_", name);
        for event_name in &comp.event_names {
            let prefixed_name = format!("{}{}", fn_prefix, event_name);
            // Map event name → prefixed name so CallExpr in function bodies resolves correctly
            self.name_renames
                .insert(event_name.clone(), prefixed_name.clone());
        }
        for event_name in &comp.event_names {
            let prefixed_name = format!("{}{}", fn_prefix, event_name);
            let dom_event = if event_name.len() > 2 && event_name.starts_with("on") {
                event_name[2..].to_lowercase()
            } else {
                event_name.to_lowercase()
            };
            self.js
                .push_str(&format!("function {}(...args) {{\n", prefixed_name));
            self.js.push_str(&format!(
                "  const el = document.querySelector('[data-component=\"{}\"]');\n",
                name
            ));
            self.js.push_str(&format!(
                "  if (el) el.dispatchEvent(new CustomEvent('{}', {{ detail: args[0], bubbles: false }}));\n",
                dom_event
            ));
            self.js.push_str("}\n");
        }

        // Functions — prefix with component name to avoid global name collisions.
        // name_renames already has event→prefixed mappings, so function bodies will
        // correctly call AddTodo_onAdd() instead of onAdd().
        for f in &comp.fns {
            let prefixed_name = format!("{}{}", fn_prefix, f.name);
            // Add to rename map so expr_to_js can resolve bare fn calls in templates
            self.name_renames
                .insert(f.name.clone(), prefixed_name.clone());
            self.emit_function_named(f, &prefixed_name);
        }

        // Motion definitions — emit JS functions that create AnimationPlayer instances.
        // Each @motion block generates a const __omni_motion_{name} function.
        if !comp.motions.is_empty() {
            self.js
                .push_str(&format!("\n/// ── Motion Definitions: {} ──\n\n", name));
            for motion in &comp.motions {
                self.emit_motion_js(motion);
            }
        }

        // Timeline definitions — emit JS that orchestrates animations by element ID.
        if !comp.timelines.is_empty() {
            self.js
                .push_str(&format!("\n/// ── Timeline Definitions: {} ──\n\n", name));
            for timeline in &comp.timelines {
                self.emit_timeline_js(timeline);
            }
        }

        // View → HTML + reactive bindings (only root component emits its view;
        // child component views are inlined during root's view generation)
        if is_root {
            if let Some(ref view) = comp.view {
                self.emitting_root_view = true;
                self.emit_view(view);
                self.emitting_root_view = false;
            }
        }

        // Styles → CSS
        for style in &comp.styles {
            self.emit_style(style);
        }

        // Lifecycle hooks: @mount / @unmount
        if !comp.lifecycles.is_empty() {
            // Save component context for inline function name resolution
            let saved_component = self.current_component.clone();
            self.current_component = name.to_string();
            let saved_signal_names: std::collections::HashSet<String> =
                self.signal_names.iter().cloned().collect();
            // Merge component's signals for proper .value access in lifecycle bodies
            for state in &comp.states {
                let prefixed = format!("{}_{}", name, state.name);
                self.signal_names.insert(prefixed);
            }
            for lifecycle in &comp.lifecycles {
                let kind = &lifecycle.kind;
                // Generate JS for the lifecycle body statements into a temp buffer,
                // then move to mount_js or unmount_js so they don't stay in self.js.
                let old_js_len = self.js.len();
                for stmt in &lifecycle.body.statements {
                    self.emit_statement(stmt);
                }
                let body_js = self.js[old_js_len..].to_string();
                // Truncate self.js — these statements will be placed in the
                // appropriate lifecycle section.
                self.js.truncate(old_js_len);
                match kind.as_str() {
                    "mount" => {
                        if self.editor_mode {
                            self.comp_mount_js.push_str(&format!("// @mount {}\n", name));
                            self.comp_mount_js.push_str(&body_js);
                        } else {
                            self.mount_js.push_str(&format!("// @mount {}\n", name));
                            self.mount_js.push_str(&body_js);
                        }
                    }
                    "unmount" => {
                        self.js.push_str(&format!(
                            "// @unmount {}\nwindow.__omni_unmount_{} = () => {{\n{}}};\n",
                            name, name, body_js
                        ));
                    }
                    _ => {}
                }
            }
            self.signal_names = saved_signal_names;
            self.current_component = saved_component;
        }

        // @effect hooks — stored separately in comp.effects (not in comp.lifecycles)
        if !comp.effects.is_empty() {
            let saved_component = self.current_component.clone();
            self.current_component = name.to_string();
            let saved_signal_names: std::collections::HashSet<String> =
                self.signal_names.iter().cloned().collect();
            for state in &comp.states {
                let prefixed = format!("{}_{}", name, state.name);
                self.signal_names.insert(prefixed);
            }
            for effect in &comp.effects {
                let old_js_len = self.js.len();
                for stmt in &effect.body.statements {
                    self.emit_statement(stmt);
                }
                let body_js = self.js[old_js_len..].to_string();
                self.js.truncate(old_js_len);

                // Emit reactive effect via __omni_effect, which auto-tracks
                // signal dependencies and re-runs when they change.
                // Placed in mount_js so DOM elements are available.
                if self.editor_mode {
                    self.comp_mount_js.push_str(&format!(
                        "// @effect {}\n__omni_effect(() => {{\n{}}});\n",
                        name, body_js
                    ));
                } else {
                    self.mount_js.push_str(&format!(
                        "// @effect {}\n__omni_effect(() => {{\n{}}});\n",
                        name, body_js
                    ));
                }
            }
            self.signal_names = saved_signal_names;
            self.current_component = saved_component;
        }

        if self.editor_mode {
            // Flush component mount JS inside the factory, before return
            if !self.comp_mount_js.is_empty() {
                self.js.push_str("\n  // ── Mount Lifecycle ──\n");
                self.js.push_str(&self.comp_mount_js);
                self.comp_mount_js.clear();
            }

            // Collect signal variable names (prefixed) for the factory return object
            let mut signal_entries: Vec<String> = Vec::new();
            for state in &comp.states {
                let prefixed = format!("{}_{}", name, state.name);
                signal_entries.push(format!("{}: {}", prefixed, prefixed));
            }
            for c in &comp.computeds {
                let prefixed = format!("{}_{}", name, c.name);
                signal_entries.push(format!("{}: {}", prefixed, prefixed));
            }
            for (prop_name, _) in &comp.prop_defaults {
                let prefixed = format!("{}_{}", name, prop_name);
                signal_entries.push(format!("{}: {}", prefixed, prefixed));
            }

            let signal_entries_str = signal_entries.join(", ");

            self.js.push_str(&format!(
                "  return {{\n    signals: {{ {} }},\n    teardown: function() {{\n      var root = document.querySelector('[data-component=\"{}\"]');\n      if (root) root.remove();\n    }}\n  }};\n}})();\n",
                signal_entries_str, name
            ));
        }

        self.js.push('\n');

        // Capture per-component JS delta for editor hot-replace (only in editor mode)
        if self.editor_mode {
            let component_js = self.js[js_before..].to_string();
            self.component_js.insert(name.to_string(), component_js);
        }
    }

    /// Finalize the JS output with closing IIFE wrapper.
    fn finalize_js(&mut self) {
        self.js.push_str("\n})();\n");
    }

    // ══════════════════════════════════════════════════════
    // Signal Declaration
    // ══════════════════════════════════════════════════════

    fn emit_signal_decl(&mut self, state: &StateIntent) {
        let init_val = self.expr_to_js(&state.initial_value);
        let prefixed = format!("{}_{}", self.current_component, state.name);
        self.js.push_str(&format!(
            "const {} = __omni_signal(\"{}\", {});\n",
            prefixed, prefixed, init_val
        ));
    }

    fn emit_computed_decl(&mut self, computed: &ComputedIntent) {
        let body = self.expr_to_js(&computed.body);
        let prefixed = format!("{}_{}", self.current_component, computed.name);
        self.js.push_str(&format!(
            "const {} = __omni_computed(\"{}\", () => ({}));\n",
            prefixed, prefixed, body
        ));
    }

    // ══════════════════════════════════════════════════════
    // Function Emission
    // ══════════════════════════════════════════════════════

    fn emit_function(&mut self, f: &FunctionIR) {
        self.emit_function_named(f, &f.name);
    }

    fn emit_function_named(&mut self, f: &FunctionIR, name: &str) {
        // Sanitize parameter names to avoid JS reserved words (e.g. `default` → `default_`)
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| super::sanitize_js_ident(&p.name))
            .collect();
        // Register renamed params so body references also use the sanitized name
        for (i, p) in f.params.iter().enumerate() {
            let sanitized = &params[i];
            if sanitized != &p.name {
                self.name_renames.insert(p.name.clone(), sanitized.clone());
            }
        }
        let async_kw = if f.is_async { "async " } else { "" };
        let has_ret = f.return_type.is_some();
        self.js.push_str(&format!(
            "{}function {}({}) {{\n",
            async_kw,
            name,
            params.join(", ")
        ));
        for (i, stmt) in f.body.statements.iter().enumerate() {
            let is_last = i == f.body.statements.len() - 1;
            // Implicit return: if the function has a return type and the last
            // statement is a trailing expression (not an explicit return), wrap it
            // with `return ` so the value flows back to the caller.
            if is_last && has_ret {
                if let StatementIR::ExprStmtIR { expr } = stmt {
                    self.js
                        .push_str(&format!("return {};\n", self.expr_to_js(expr)));
                    continue;
                }
            }
            self.emit_statement(stmt);
        }
        self.js.push_str("}\n\n");
    }

    fn emit_statement(&mut self, stmt: &StatementIR) {
        match stmt {
            StatementIR::LetStmtIR { name, value, .. } => {
                let safe_name = super::sanitize_js_ident(name);
                // Register rename for body references
                if safe_name != *name {
                    self.name_renames.insert(name.clone(), safe_name.clone());
                }
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js
                    .push_str(&format!("  let {} = {};\n", safe_name, val));
            }
            StatementIR::ReturnStmtIR { value } => {
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js.push_str(&format!("  return {};\n", val));
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                let cond = self.expr_to_js(condition);
                self.js.push_str(&format!("  if ({}) {{\n", cond));
                for s in &then_body.statements {
                    self.emit_statement(s);
                }
                for (elif_cond, elif_body) in else_ifs {
                    let elif_js = self.expr_to_js(elif_cond);
                    self.js
                        .push_str(&format!("  }} else if ({}) {{\n", elif_js));
                    for s in &elif_body.statements {
                        self.emit_statement(s);
                    }
                }
                if let Some(ref else_b) = else_body {
                    self.js.push_str("  } else {\n");
                    for s in &else_b.statements {
                        self.emit_statement(s);
                    }
                }
                self.js.push_str("  }\n");
            }
            StatementIR::ExprStmtIR { expr } => {
                let code = self.expr_to_js(expr);
                self.js.push_str(&format!("  {};\n", code));
            }
            StatementIR::MutationStmt { intent } => {
                if let Some(ref intent) = intent {
                    self.emit_mutation(intent);
                }
            }
            StatementIR::WhileStmtIR { condition, body } => {
                let cond = self.expr_to_js(condition);
                self.js.push_str(&format!("  while ({}) {{\n", cond));
                for s in &body.statements {
                    self.emit_statement(s);
                }
                self.js.push_str("  }\n");
            }
            StatementIR::ForInStmtIR {
                var_name,
                iterable,
                body,
            } => {
                let iter = self.expr_to_js(iterable);
                self.js
                    .push_str(&format!("  for (const {} of {}) {{\n", var_name, iter));
                for s in &body.statements {
                    self.emit_statement(s);
                }
                self.js.push_str("  }\n");
            }
            StatementIR::BreakStmtIR => {
                self.js.push_str("  break;\n");
            }
            StatementIR::ContinueStmtIR => {
                self.js.push_str("  continue;\n");
            }
            StatementIR::ThrowStmtIR { expr } => {
                let e = self.expr_to_js(expr);
                self.js.push_str(&format!("  throw {};\n", e));
            }
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_var,
                catch_body,
                finally_body,
            } => {
                self.js.push_str("  try {\n");
                for s in &try_body.statements {
                    self.emit_statement(s);
                }
                if let Some(ref cv) = catch_var {
                    self.js.push_str(&format!("  }} catch ({}) {{\n", cv));
                } else if catch_body.is_some() {
                    self.js.push_str("  } catch (_) {\n");
                }
                if let Some(ref cb) = catch_body {
                    for s in &cb.statements {
                        self.emit_statement(s);
                    }
                }
                if let Some(ref fb) = finally_body {
                    self.js.push_str("  } finally {\n");
                    for s in &fb.statements {
                        self.emit_statement(s);
                    }
                }
                self.js.push_str("  }\n");
            }
            _ => {
                // Placeholder for unhandled statements
                self.js.push_str("  /* unhandled stmt */\n");
            }
        }
    }

    fn emit_mutation(&mut self, intent: &MutationIntent) {
        let target = self
            .name_renames
            .get(intent.target.name.as_str())
            .cloned()
            .unwrap_or_else(|| intent.target.name.clone());
        // Check if target is a known signal (component-local or store).
        // If not, it's a local variable — emit regular JS assignment.
        let is_signal =
            self.signal_names.contains(&target) || self.store_signal_names.contains(&target);
        let op = &intent.operation;
        if !is_signal {
            // Local variable mutation — plain JS assignment
            let amt = intent
                .amount
                .as_ref()
                .map(|e| self.expr_to_js(e))
                .unwrap_or_else(|| "undefined".into());
            match op.as_str() {
                "direct_assign" => {
                    self.js.push_str(&format!("  {} = {};\n", target, amt));
                }
                "increment" => {
                    self.js.push_str(&format!("  {} += {};\n", target, amt));
                }
                "decrement" => {
                    self.js.push_str(&format!("  {} -= {};\n", target, amt));
                }
                "multiply" => {
                    self.js.push_str(&format!("  {} *= {};\n", target, amt));
                }
                "divide" => {
                    self.js.push_str(&format!("  {} /= {};\n", target, amt));
                }
                "modulo" => {
                    self.js.push_str(&format!("  {} %= {};\n", target, amt));
                }
                _ => {
                    self.js.push_str(&format!("  {} = {};\n", target, amt));
                }
            }
            return;
        }
        match op.as_str() {
            "direct_assign" => {
                let val = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js.push_str(&format!("  {}.set({});\n", target, val));
            }
            "increment" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value + {});\n", target, target, amt));
            }
            "decrement" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value - {});\n", target, target, amt));
            }
            "multiply" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value * {});\n", target, target, amt));
            }
            "divide" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value / {});\n", target, target, amt));
            }
            "modulo" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value % {});\n", target, target, amt));
            }
            "push" => {
                let item = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js.push_str(&format!(
                    "  {}.value.push({}); {}.set({}.value.slice());\n",
                    target, item, target, target
                ));
            }
            "pop" => {
                self.js.push_str(&format!(
                    "  {}.value.pop(); {}.set({}.value.slice());\n",
                    target, target, target
                ));
            }
            _ => {
                self.js
                    .push_str(&format!("  /* mutation: {} {} */\n", target, op));
            }
        }
    }

    // ══════════════════════════════════════════════════════
    // View → HTML Generation
    // ══════════════════════════════════════════════════════

    fn emit_view(&mut self, view: &ViewTree) {
        self.current_nid_path.clear();

        // Swap js with deferred_js so view_to_html event/effect code
        // goes into the deferred buffer (emitted after DOM insertion)
        std::mem::swap(&mut self.js, &mut self.deferred_js);

        // Generate initial DOM HTML (events/effects go to deferred_js)
        self.html.push_str(&format!(
            "<div id=\"root\" data-component=\"{}\">\n",
            self.current_component
        ));
        let html = self.view_to_html(view, 1);
        self.html.push_str(&html);
        self.html.push_str("</div>\n");

        // Swap back — events/effects are now in deferred_js
        std::mem::swap(&mut self.js, &mut self.deferred_js);

        // Accumulate deferred_js — emit only once after ALL components
        if !self.deferred_js.is_empty() {
            self.all_deferred_js.push_str(&self.deferred_js);
            self.deferred_js.clear();
        }
    }

    /// Emit ONE innerHTML insertion + ALL accumulated deferred bindings/effects.
    /// Called once after all components have been generated.
    fn emit_dom_and_bindings(&mut self) {
        if self.dom_emitted || self.html.is_empty() {
            return;
        }
        self.dom_emitted = true;

        // Emit DOM insertion script
        self.js.push_str("\n/// ── Initial DOM ──\n");
        self.js
            .push_str("document.getElementById(\"root\").innerHTML = `");
        let escaped = self
            .html
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("$", "\\$");
        self.js.push_str(&escaped);
        self.js.push_str("`;\n\n");

        // Emit ALL accumulated deferred event bindings and reactive effects
        if !self.all_deferred_js.is_empty() {
            self.js
                .push_str("\n/// ── Event Bindings & Reactive Effects ──\n");
            self.js.push_str(&self.all_deferred_js);
        }

        // Emit mount lifecycle hooks — run after DOM is inserted
        if !self.mount_js.is_empty() {
            self.js.push_str("\n/// ── Mount Lifecycle ──\n");
            self.js.push_str(&self.mount_js);
        }
    }

    /// Inject editor bridge code when `editor_mode` is enabled.
    /// Provides:
    ///   - window.__omni_signals / __omni_components for external access
    ///   - __omni_buildNidMap() — scans DOM for [data-nid] elements
    ///   - Click → postMessage omni-select
    ///   - postMessage handlers: omni-replace-component, omni-highlight, omni-clear-hl
    ///   - window.__omni_editor — highlight/clearHighlight helpers
    ///   - __omni_replaceComponent(name, factoryCode) — snapshot signals, teardown, restore
    ///   - parent.postMessage omni-rendered on load
    ///
    /// Bridge code below is ES5-compatible (no arrow functions, no template literals).
    fn emit_editor_bridge(&mut self) {
        if !self.editor_mode {
            return;
        }
        self.js.push_str(
            r#"
/// ── Editor Bridge ──
window.__omni_signals = {};
window.__omni_components = {};

function __omni_buildNidMap() {
  var map = {};
  document.querySelectorAll('[data-nid]').forEach(function(el) {
    map[el.dataset.nid] = {
      elementId: el.id || '',
      tag: el.tagName.toLowerCase(),
      component: (el.closest('[data-component]') || {}).dataset ? el.closest('[data-component]').dataset.component : '',
    };
  });
  return map;
}

document.addEventListener('click', function(e) {
  var el = e.target.closest('[data-nid]');
  if (el) {
    e.stopPropagation();
    parent.postMessage({ type: 'omni-select', nid: el.dataset.nid }, '*');
  }
});

window.addEventListener('message', function(e) {
  switch (e.data.type) {
    case 'omni-replace-component':
      __omni_replaceComponent(e.data.name, e.data.js);
      break;
    case 'omni-highlight':
      __omni_editor.highlight(e.data.nid);
      break;
    case 'omni-clear-hl':
      __omni_editor.clearHighlight();
      break;
  }
});

window.__omni_editor = {
  highlight: function(nid) {
    var el = document.querySelector('[data-nid="' + nid + '"]');
    if (el) { el.style.outline = '2px solid #4F46E5'; }
  },
  clearHighlight: function() {
    document.querySelectorAll('[data-nid]').forEach(function(el) {
      el.style.outline = '';
    });
  },
};

function __omni_replaceComponent(name, factoryCode) {
  var oldComp = window.__omni_components[name];
  var savedSignals = {};
  if (oldComp && oldComp.signals) {
    Object.keys(oldComp.signals).forEach(function(k) {
      savedSignals[k] = oldComp.signals[k].peek();
    });
    if (oldComp.teardown) oldComp.teardown();
  }
  window.__omni_components[name] = (0, eval)(factoryCode);
  var newComp = window.__omni_components[name];
  if (newComp && newComp.signals) {
    Object.keys(savedSignals).forEach(function(k) {
      if (newComp.signals[k]) newComp.signals[k].set(savedSignals[k]);
    });
  }
  parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, '*');
}

parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, '*');
"#,
        );
    }

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
                    let value_access = if self.signal_names.contains(&js_expr) {
                        format!("{}.value", js_expr)
                    } else {
                        js_expr
                    };
                    // Write to self.js (which is the deferred buffer after emit_view swap)
                    self.js.push_str(&format!(
                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.textContent = String({}); }});\n",
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

    /// Generate HTML + JS for @if/@else-if/@else conditional rendering.
    ///
    /// Strategy:
    ///   - Generate each branch's HTML in a hidden wrapper div.
    ///   - Generate a JS _effect() that evaluates conditions and toggles display.
    fn emit_conditional_html(&mut self, cond: &ConditionalIntent, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let nid = self.next_nid("if");
        let mut out = String::new();
        let mut branch_nids: Vec<String> = Vec::new();

        // Generate HTML for each branch (all hidden by default)
        for (i, branch) in cond.branches.iter().enumerate() {
            let branch_nid = format!("{}-b{}", nid, i);
            let display = "none";
            out.push_str(&format!(
                "{}<div data-nid=\"{}\" style=\"display:{}\">\n",
                pad, branch_nid, display
            ));
            if let Some(ref body) = branch.body {
                out.push_str(&self.view_to_html(body, indent + 1));
            }
            out.push_str(&format!("{}</div>\n", pad));
            branch_nids.push(branch_nid);
        }

        // Fallback branch (else)
        let fallback_nid = if cond.fallback.is_some() {
            let fb_nid = format!("{}-else", nid);
            out.push_str(&format!(
                "{}<div data-nid=\"{}\" style=\"display:none\">\n",
                pad, fb_nid
            ));
            if let Some(ref fallback) = cond.fallback {
                out.push_str(&self.view_to_html(fallback, indent + 1));
            }
            out.push_str(&format!("{}</div>\n", pad));
            Some(fb_nid)
        } else {
            None
        };

        // Generate the reactive JS _effect()
        let mut conditions_js = Vec::new();
        for (i, branch) in cond.branches.iter().enumerate() {
            let cond_js = self.expr_to_js(&branch.condition);
            conditions_js.push((cond_js, branch_nids[i].clone()));
        }

        let is_wait_mode = cond.transition_mode.as_deref() == Some("wait");

        self.js
            .push_str(&format!("\n/// ── @if conditional {} ──\n", nid));

        // Emit all NIDs as a JS array for efficient bulk toggling
        let nid_list: Vec<String> = branch_nids
            .iter()
            .chain(fallback_nid.iter())
            .map(|n| format!("\"{}\"", n))
            .collect();
        let nid_array = format!("[{}]", nid_list.join(", "));

        if is_wait_mode {
            // Wait mode: play @leave on old branch, wait for completion, then show new branch
            self.js.push_str(&format!(
                "var _current_{nid} = null;\n\
                 __omni_effect(() => {{\n\
                 \x20 const _nids = {nids};\n\
                 \x20 const _hideAll = (except) => {{ _nids.forEach(id => {{ const e = __omni_el(id); if (e && id !== except) e.style.display = \"none\"; }}); }};\n\
                 \x20 const _showNow = (nid) => {{ const e = __omni_el(nid); if (e) e.style.display = \"block\"; _hideAll(nid); _current_{nid} = nid; }};\n\
                 \x20 const _transition = (newNid) => {{\n\
                 \x20\x20 const oldNid = _current_{nid};\n\
                 \x20\x20 if (oldNid === newNid) return;\n\
                 \x20\x20 if (!oldNid) {{ _showNow(newNid); return; }}\n\
                 \x20\x20 // Play @leave on old branch, then show new\n\
                 \x20\x20 const leavePlayer = __omni_animations.get(oldNid + \"_leave\");\n\
                 \x20\x20 if (leavePlayer) {{\n\
                 \x20\x20\x20 leavePlayer.play();\n\
                 \x20\x20\x20 // Wait for leave animation to complete\n\
                 \x20\x20\x20 const checkComplete = setInterval(() => {{\n\
                 \x20\x20\x20\x20 if (leavePlayer.completed || !leavePlayer.playing) {{\n\
                 \x20\x20\x20\x20\x20 clearInterval(checkComplete);\n\
                 \x20\x20\x20\x20\x20 _showNow(newNid);\n\
                 \x20\x20\x20\x20\x20 // Trigger @enter on new branch\n\
                 \x20\x20\x20\x20\x20 const enterPlayer = __omni_animations.get(newNid + \"_enter\");\n\
                 \x20\x20\x20\x20\x20 if (enterPlayer && enterPlayer.config?.paused !== false) enterPlayer.play();\n\
                 \x20\x20\x20\x20 }}\n\
                 \x20\x20\x20 }}, 16);\n\
                 \x20\x20 }} else {{\n\
                 \x20\x20\x20 _showNow(newNid);\n\
                 \x20\x20 }}\n\
                 \x20 }};\n",
                nid = nid,
                nids = nid_array
            ));
        } else {
            // Sync mode (default): instant branch switching, @enter/@leave coexist in DOM
            self.js.push_str("__omni_effect(() => {\n");
            self.js
                .push_str(&format!("  const _nids = {};\n", nid_array));
            self.js.push_str("  const _show = (nid) => { _nids.forEach(id => { const e = __omni_el(id); if (e) e.style.display = (id === nid) ? \"block\" : \"none\"; }); };\n");
        }

        // For each branch, build the if/else if chain
        for (i, (cond_js, branch_nid)) in conditions_js.iter().enumerate() {
            let if_word = if i == 0 { "if" } else { "else if" };
            if is_wait_mode {
                self.js.push_str(&format!(
                    "  {} ({}) {{ _transition(\"{}\"); }}\n",
                    if_word, cond_js, branch_nid
                ));
            } else {
                self.js.push_str(&format!(
                    "  {} ({}) {{ _show(\"{}\"); }}\n",
                    if_word, cond_js, branch_nid
                ));
            }
        }

        // Else clause (fallback or hide-all)
        if let Some(ref fb_nid) = fallback_nid {
            if is_wait_mode {
                self.js
                    .push_str(&format!("  else {{ _transition(\"{}\"); }}\n", fb_nid));
            } else {
                self.js
                    .push_str(&format!("  else {{ _show(\"{}\"); }}\n", fb_nid));
            }
        } else {
            // No @else — hide all branches when no condition matches
            if is_wait_mode {
                self.js
                    .push_str("  else { _hideAll(null); _current_{nid} = null; }\n");
            } else {
                self.js.push_str("  else { _nids.forEach(id => { const e = __omni_el(id); if (e) e.style.display = \"none\"; }); }\n");
            }
        }

        self.js.push_str("});\n");

        out
    }

    /// Generate HTML + JS for @for list rendering.
    ///
    /// Strategy:
    ///   - Generate a `<template>` element containing one item's HTML.
    ///   - Generate a container div.
    ///   - Generate a JS _effect() that iterates the data and clones the template.
    fn emit_list_rendering_html(&mut self, list: &ListRenderingIntent, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let nid = self.next_nid("for");
        let template_nid = format!("{}-tpl", nid);
        let container_nid = format!("{}-ctr", nid);
        let item_var = &list.item_binding.name;
        let iterable_js = self.expr_to_js(&list.iterable);

        // Generate template for one item (with in_for_template flag)
        let item_html = if let Some(ref body) = list.body {
            self.in_for_template = true;
            let html = self.view_to_html(body, indent + 1);
            self.in_for_template = false;
            html
        } else {
            format!("{}  <!-- empty item -->\n", pad)
        };

        let mut out = String::new();
        out.push_str(&format!(
            "{}<template data-nid=\"{}\">\n{}</template>\n",
            pad, template_nid, item_html
        ));
        out.push_str(&format!("{}<div data-nid=\"{}\">\n", pad, container_nid));

        // Empty state placeholder
        if list.empty_body.is_some() {
            let empty_nid = format!("{}-empty", nid);
            out.push_str(&format!("{}  <div data-nid=\"{}\">\n", pad, empty_nid));
            if let Some(ref empty_body) = list.empty_body {
                out.push_str(&self.view_to_html(empty_body, indent + 2));
            }
            out.push_str(&format!("{}  </div>\n", pad));
        }

        out.push_str(&format!("{}</div>\n", pad));

        // Generate the reactive JS _effect()
        self.js.push_str(&format!(
            "\n/// ── @for {item} in ... ──\n",
            item = item_var
        ));
        self.js.push_str("__omni_effect(() => {\n");
        self.js
            .push_str(&format!("  const data = {};\n", iterable_js));
        self.js
            .push_str(&format!("  const tpl = __omni_el(\"{}\");\n", template_nid));
        self.js.push_str(&format!(
            "  const ctr = __omni_el(\"{}\");\n",
            container_nid
        ));
        self.js.push_str("  if (!tpl || !ctr) return;\n");

        // Re-render loop
        // If key_expr is present, use key-based identity tracking for enter/leave animations.
        // Otherwise, clear and re-render all items.
        let has_key = list.key_expr.is_some();
        if has_key {
            let key_js = self.expr_to_js(list.key_expr.as_ref().unwrap());
            self.js.push_str("  // Key-based identity tracking\n");
            self.js.push_str(&format!("  const keyFn = (item) => {{ try {{ return String({key_js}); }} catch(e) {{ return null; }} }};\n", key_js = key_js));
            self.js
                .push_str("  ctr._forKeys = ctr._forKeys || new Map();\n");
            self.js
                .push_str("  const oldKeys = new Set(ctr._forKeys.keys());\n");
            self.js.push_str("  const newKeys = new Set();\n");
        } else {
            self.js.push_str("  // Clear existing rendered items\n");
            self.js.push_str(
                "  ctr.querySelectorAll('[data-for-item]').forEach(el => el.remove());\n",
            );
        }

        // Handle empty state
        if list.empty_body.is_some() {
            let empty_nid = format!("{}-empty", nid);
            self.js.push_str(&format!(
                "  const emptyEl = __omni_el(\"{}\");\n",
                empty_nid
            ));
            self.js.push_str("  if (!data || data.length === 0) {\n");
            self.js
                .push_str("    if (emptyEl) emptyEl.style.display = \"block\";\n");
            self.js.push_str("    return;\n");
            self.js.push_str("  }\n");
            self.js
                .push_str("  if (emptyEl) emptyEl.style.display = \"none\";\n");
        } else {
            self.js
                .push_str("  if (!data || data.length === 0) return;\n");
        }

        // FLIP: record element positions before re-render (for @for with key)
        if has_key {
            self.js.push_str(&format!(
                "  __omni_flip_before('[data-nid=\"{}\"]');\n",
                container_nid
            ));
        }

        // Iterate and clone
        self.js
            .push_str(&format!("  data.forEach(({}, _idx) => {{\n", item_var));

        if has_key {
            self.js.push_str("    const _key = keyFn(item);\n");
            self.js
                .push_str("    if (_key != null) newKeys.add(_key);\n");
            self.js
                .push_str("    // Reuse existing element if key matches\n");
            self.js
                .push_str("    let clone = _key != null ? ctr._forKeys.get(_key) : null;\n");
            self.js.push_str("    const isNew = !clone;\n");
            self.js.push_str("    if (isNew) {\n");
            self.js
                .push_str("      clone = tpl.content.firstElementChild.cloneNode(true);\n");
            self.js.push_str("      if (_key != null) { ctr._forKeys.set(_key, clone); clone.setAttribute('data-for-key', _key); }\n");
            self.js.push_str("    }\n");
        } else {
            self.js
                .push_str("    const clone = tpl.content.firstElementChild.cloneNode(true);\n");
        }
        self.js
            .push_str("    clone.setAttribute('data-for-item', _idx);\n");
        self.js.push_str("    // Bind text interpolation markers\n");
        self.js
            .push_str("    clone.querySelectorAll('[data-bind-text]').forEach(el => {\n");
        self.js
            .push_str("      const expr = el.getAttribute('data-bind-text');\n");
        self.js.push_str("      if (expr) {\n");
        self.js.push_str("        try {\n");
        // eval() runs in the enclosing closure scope (IIFE), so signals like
        // TodoItem_checked are accessible — unlike new Function() which is global-scope only.
        self.js.push_str(
            "          // eval() runs in the enclosing IIFE scope — all signals are accessible\n",
        );
        self.js
            .push_str("          el.textContent = String(eval(expr));\n");
        self.js
            .push_str("        } catch(e) { el.textContent = ''; }\n");
        self.js.push_str("      }\n");
        self.js.push_str("    });\n");
        // Also interpolate data-* attributes that contain {item} patterns.
        // This allows component props like data-text=\"{item}\" to receive the
        // actual item value instead of the literal template string.
        self.js
            .push_str("    // Bind data-* attribute interpolation\n");
        self.js.push_str("    clone.querySelectorAll('[data-text],[data-value],[data-label],[data-key]').forEach(el => {\n");
        self.js
            .push_str("      Array.from(el.attributes).forEach(attr => {\n");
        self.js
            .push_str("        if (attr.name.startsWith('data-') && attr.value.includes('{')) {\n");
        self.js.push_str("          try {\n");
        self.js.push_str(
            "            // Convert {item} patterns to ${item} for JS template literals\n",
        );
        self.js.push_str(
            "            const jsTemplate = attr.value.replace(/\\{(\\w+)\\}/g, '${$1}');\n",
        );
        self.js.push_str(&format!(
            "            const resolved = eval('`' + jsTemplate + '`');\n",
        ));
        self.js
            .push_str("            el.setAttribute(attr.name, resolved);\n");
        self.js
            .push_str("          } catch(e) { /* keep original */ }\n");
        self.js.push_str("        }\n");
        self.js.push_str("      });\n");
        self.js.push_str("    });\n");
        if has_key {
            self.js
                .push_str("    if (isNew) { ctr.appendChild(clone); }\n");
        } else {
            self.js.push_str("    ctr.appendChild(clone);\n");
        }
        self.js.push_str("  });\n");

        // Key-based cleanup: remove items whose keys disappeared
        if has_key {
            self.js
                .push_str("  // Remove items whose keys disappeared (play @leave then remove)\n");
            self.js.push_str("  oldKeys.forEach(key => {\n");
            self.js.push_str("    if (!newKeys.has(key)) {\n");
            self.js
                .push_str("      const el = ctr._forKeys.get(key);\n");
            self.js.push_str("      if (el) {\n");
            self.js
                .push_str("        const leavingNid = el.getAttribute('data-nid');\n");
            self.js.push_str("        if (leavingNid) {\n");
            self.js
                .push_str("          const player = __omni_getAnimation(leavingNid, 'leave');\n");
            self.js.push_str("          if (player) {\n");
            self.js.push_str("            player.play().then(() => { el.remove(); ctr._forKeys.delete(key); });\n");
            self.js.push_str("          } else {\n");
            self.js
                .push_str("            el.remove(); ctr._forKeys.delete(key);\n");
            self.js.push_str("          }\n");
            self.js.push_str("        } else {\n");
            self.js
                .push_str("          el.remove(); ctr._forKeys.delete(key);\n");
            self.js.push_str("        }\n");
            self.js.push_str("      }\n");
            self.js.push_str("    }\n");
            self.js.push_str("  });\n");
        }
        // FLIP: animate elements to new positions
        if has_key {
            self.js.push_str(&format!(
                "  __omni_flip_after('[data-nid=\"{}\"]');\n",
                container_nid
            ));
        }

        // Attach delegated event listeners on the container for @for item events.
        // Elements inside the template have data-event-* attributes (e.g.
        // data-event-click="fn:handlerName") that we read via event bubbling.
        self.js
            .push_str("  if (!ctr.dataset.forEventsAttached) {\n");
        self.js
            .push_str("    ctr.dataset.forEventsAttached = 'true';\n");
        self.js.push_str("    const _forHandle = (e) => {\n");
        self.js
            .push_str("      const t = e.target.closest('[data-event-' + e.type + ']');\n");
        self.js.push_str("      if (!t) return;\n");
        self.js
            .push_str("      const attr = t.getAttribute('data-event-' + e.type);\n");
        self.js.push_str("      if (!attr) return;\n");
        self.js
            .push_str("      if (e.type === 'submit') e.preventDefault();\n");
        self.js.push_str("      if (attr.startsWith('fn:')) {\n");
        self.js
            .push_str("        const fn = __omni_forHandlers[attr.slice(3)];\n");
        self.js
            .push_str("        if (typeof fn === 'function') fn(e.detail);\n");
        self.js
            .push_str("      } else if (attr.startsWith('store:')) {\n");
        self.js
            .push_str("        const fn = __omni_forHandlers[attr.slice(6)];\n");
        self.js
            .push_str("        if (typeof fn === 'function') fn();\n");
        self.js.push_str("      }\n");
        self.js.push_str("    };\n");
        self.js
            .push_str("    ctr.addEventListener('click', _forHandle);\n");
        self.js
            .push_str("    ctr.addEventListener('submit', _forHandle);\n");
        self.js
            .push_str("    ctr.addEventListener('input', _forHandle);\n");
        self.js
            .push_str("    ctr.addEventListener('change', _forHandle);\n");
        self.js
            .push_str("    ctr.addEventListener('focus', _forHandle);\n");
        self.js
            .push_str("    ctr.addEventListener('blur', _forHandle);\n");
        self.js
            .push_str("    ctr.addEventListener('keydown', _forHandle);\n");
        self.js.push_str("  }\n");

        self.js.push_str("});\n");

        out
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
                out.push_str(&format!(
                    "{}<div data-nid=\"{}\" data-component=\"{}\">\n",
                    pad, nid, el.tag
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
                        let pd = if js_event == "submit" {
                            " e.preventDefault();"
                        } else {
                            ""
                        };
                        match handler.kind.as_str() {
                            "fn_ref" => {
                                if let Some(ref fn_name) = handler.r#ref {
                                    let prefixed =
                                        format!("{}_{}", self.current_component, fn_name);
                                    self.js.push_str(&format!(
                                        "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{{} {}(e.detail); }}); }}\n",
                                        nid, js_event, pd, prefixed
                                    ));
                                }
                            }
                            "store_action" => {
                                if let Some(ref action_path) = handler.r#ref {
                                    let var_name = action_path.replace('.', "_");
                                    self.js.push_str(&format!(
                                        "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{{} {}(); }}); }}\n",
                                        nid, js_event, pd, var_name
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
                                self.js.push_str(&format!(
                                    "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{{} {}(e.detail); }}); }}\n",
                                    nid, js_event, pd, wrapper_name
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
                attrs.push_str(&format!(" data-hero-transition=\"{}\"", escape_html(&trans_str)));
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
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.style.{} = {}.value; }});\n",
                                    nid, js_prop, prefixed
                                ));
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
                                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.style.{} = {}; }});\n",
                                    nid, js_prop, js_parts.join(" + ")
                                ));
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
                                        "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.style.{} = {}({}); }});\n",
                                        nid, js_prop, prefixed, arg_strs.join(", ")
                                    ));
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
                let pd = if js_event == "submit" {
                    " e.preventDefault();"
                } else {
                    ""
                };
                match handler.kind.as_str() {
                    "fn_ref" => {
                        if let Some(ref fn_name) = handler.r#ref {
                            let prefixed = format!("{}_{}", self.current_component, fn_name);
                            self.js.push_str(&format!(
                                "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{{} {}(e.detail); }}); }}\n",
                                nid, js_event, pd, prefixed
                            ));
                        }
                    }
                    "store_action" => {
                        if let Some(ref action_path) = handler.r#ref {
                            let var_name = action_path.replace('.', "_");
                            self.js.push_str(&format!(
                                "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{{} {}(); }}); }}\n",
                                nid, js_event, pd, var_name
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
                        self.js.push_str(&format!(
                            "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {{{} {}(e.detail); }}); }}\n",
                            nid, js_event, pd, wrapper_name
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

        // Self-closing tags
        let void_tags = [
            "br", "hr", "img", "input", "meta", "link", "area", "base", "col", "embed", "source",
            "track", "wbr",
        ];
        if void_tags.contains(&html_tag.as_str()) {
            return format!("{}<{}{} />\n", pad, html_tag, attrs);
        }

        out.push_str(&format!("{}<{}{}>\n", pad, html_tag, attrs));

        // Children
        for child in &el.children {
            let child_html = self.view_to_html(child, indent + 1);
            out.push_str(&child_html);
        }

        out.push_str(&format!("{}</{}>\n", pad, html_tag));

        // Emit any interpolation _effect() calls
        for effect in &interp_effects {
            self.js.push_str(effect);
        }

        // Class map dynamic bindings: for each dynamic class, emit an effect
        // that toggles it reactively via classList.toggle().
        if !el.class_map_bindings.is_empty() {
            for (class_name, expr) in &el.class_map_bindings {
                let js_expr = self.expr_to_js(expr);
                let js_access = if self.signal_names.contains(&js_expr) {
                    format!("{}.value", js_expr)
                } else {
                    js_expr
                };
                self.js.push_str(&format!(
                    "__omni_effect(() => {{ const el = __omni_el(\"{}\"); if (el) el.classList.toggle(\"{}\", !!{}); }});\n",
                    nid, class_name, js_access
                ));
            }
        }

        // Motion triggers — emit animation setup for @enter, @hover, @press, @focus, @leave
        if !el.motion_triggers.is_empty() {
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
        if let Some(paused) = el.paused_override {
            self.mount_js.push_str(&format!(
                "// @paused override\n\
                 (function() {{\n\
                 \x20 __omni_animations.forEach((player, key) => {{\n\
                 \x20\x20 if (key.startsWith(\"{nid}_\")) {{\n\
                 \x20\x20\x20 if ({paused}) {{ player.pause(); }} else {{ player.resume(); }}\n\
                 \x20\x20 }}\n\
                 \x20 }});\n\
                 }})();\n",
                nid = nid,
                paused = if paused { "true" } else { "false" }
            ));
        }

        // states: — multi-state animation map (reserved or custom signal-driven)
        if let Some(ref states_ir) = el.states {
            self.emit_states_js(&nid, states_ir);
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

    // ══════════════════════════════════════════════════════
    // Motion / Animation Codegen
    // ══════════════════════════════════════════════════════

    /// Emit a JS function that creates an AnimationPlayer for a named @motion block.
    fn emit_motion_js(&mut self, motion: &MotionIntent) {
        let config = self.motion_body_to_config(&motion.body);
        self.js.push_str(&format!(
            "const __omni_motion_{name} = function(el, overrides) {{\n\
             \x20  return new AnimationPlayer(el, Object.assign({{{config}}}, overrides || {{}}));\n\
             }};\n",
            name = motion.name,
            config = config
        ));
    }

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
                let dir = direction.as_deref().unwrap_or("out");
                let name = match dir {
                    "in" => "backIn",
                    "out" => "backOut",
                    "inOut" => "backInOut",
                    _ => "backOut",
                };
                format!("\"{}\"", name)
            }
            EasingIR::Elastic {
                amplitude: _,
                period: _,
                direction,
            } => {
                let dir = direction.as_deref().unwrap_or("out");
                let name = match dir {
                    "in" => "elasticIn",
                    "out" => "elasticOut",
                    "inOut" => "elasticInOut",
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
            };
            parts.push(format!("\"interrupt\": \"{}\"", int_str));
        }
        parts
    }

    /// Emit JS for states: { ... } multi-state animation on an element.
    /// Two modes:
    ///   - Reserved names (idle, hover, pressed): auto-bind mouse/touch events
    ///   - Custom names (loading, success, error): driven by `on: {signal}` via __omni_effect
    fn emit_states_js(&mut self, nid: &str, states_ir: &StatesIR) {
        let state_names: Vec<&str> = states_ir.states.keys().map(|s| s.as_str()).collect();
        let is_reserved = state_names
            .iter()
            .all(|n| matches!(*n, "idle" | "hover" | "pressed"));

        if is_reserved {
            // Emit mouse event bindings for idle/hover/pressed state transitions
            self.mount_js.push_str(&format!(
                "// states: reserved (idle/hover/pressed)\n\
                 (function() {{\n\
                 \x20 const el = __omni_el(\"{nid}\");\n\
                 \x20 if (!el) return;\n\
                 \x20 let _state = \"idle\";\n\
                 \x20 function _transition(state) {{\n\
                 \x20\x20 if (state === _state) return;\n\
                 \x20\x20 const targets = __omni_states_{nid}[state];\n\
                 \x20\x20 if (!targets) return;\n\
                 \x20\x20 const from = {{}};\n\
                 \x20\x20 for (const k of Object.keys(targets.to)) from[k] = getComputedStyle(el).getPropertyValue(k);\n\
                 \x20\x20 el.animate([from, targets.to], {{\n\
                 \x20\x20\x20 duration: targets.over || 150,\n\
                 \x20\x20\x20 easing: targets.ease || \"ease-out\",\n\
                 \x20\x20\x20 fill: \"forwards\"\n\
                 \x20\x20 }});\n\
                 \x20\x20 _state = state;\n\
                 \x20 }}\n\
                 \x20 el.addEventListener(\"mouseenter\", () => _transition(\"hover\"));\n\
                 \x20 el.addEventListener(\"mouseleave\", () => _transition(\"idle\"));\n\
                 \x20 el.addEventListener(\"mousedown\", () => _transition(\"pressed\"));\n\
                 \x20 el.addEventListener(\"mouseup\", () => _transition(\"hover\"));\n\
                 }})();\n",
                nid = nid
            ));
        } else if let Some(ref signal_expr) = states_ir.signal {
            // Custom states driven by signal value
            let signal_js = self.expr_to_js(signal_expr);
            let signal_access = if self.signal_names.contains(&signal_js) {
                format!("{}.value", signal_js)
            } else {
                signal_js.clone()
            };
            self.mount_js.push_str(&format!(
                "// states: custom (signal-driven)\n\
                 __omni_effect(() => {{\n\
                 \x20 const el = __omni_el(\"{nid}\");\n\
                 \x20 if (!el) return;\n\
                 \x20 const stateVal = String({sig});\n\
                 \x20 const targets = __omni_states_{nid}[stateVal];\n\
                 \x20 if (!targets) return;\n\
                 \x20 const from = {{}};\n\
                 \x20 for (const k of Object.keys(targets.to)) from[k] = getComputedStyle(el).getPropertyValue(k);\n\
                 \x20 el.animate([from, targets.to], {{\n\
                 \x20\x20 duration: targets.over || 300,\n\
                 \x20\x20 easing: targets.ease || \"ease-out\",\n\
                 \x20\x20 fill: \"forwards\"\n\
                 \x20 }});\n\
                 }});\n",
                nid = nid,
                sig = signal_access
            ));
        }

        // Emit the state targets lookup table
        self.js
            .push_str(&format!("var __omni_states_{nid} = {{\n", nid = nid));
        for (state_name, tween) in &states_ir.states {
            let over_ms = tween
                .over
                .as_ref()
                .map_or("150".to_string(), |o| self.expr_to_js(o));
            let ease_str = tween
                .ease
                .as_ref()
                .map_or("\"ease-out\"".to_string(), |e| self.easing_to_js(e));
            let props: Vec<String> = tween
                .properties
                .iter()
                .map(|(k, v)| format!("\"{}\": \"{}\"", k, self.expr_to_js(v)))
                .collect();
            self.js.push_str(&format!(
                "  \"{name}\": {{ to: {{ {props} }}, over: {over}, ease: {ease} }},\n",
                name = state_name,
                props = props.join(", "),
                over = over_ms,
                ease = ease_str
            ));
        }
        self.js.push_str("};\n");
    }

    /// Emit motion trigger JS for element triggers (@enter, @hover, etc.).
    /// Runs in the deferred/mount JS context where DOM elements exist.
    fn emit_motion_triggers_for_element(&mut self, nid: &str, el: &ElementIR) {
        for trigger in &el.motion_triggers {
            let trigger_str = match trigger.kind {
                TriggerKind::Enter => "enter",
                TriggerKind::Leave => "leave",
                TriggerKind::Hover => "hover",
                TriggerKind::Unhover => "unhover",
                TriggerKind::Press => "press",
                TriggerKind::Focus => "focus",
                TriggerKind::Animate => "animate",
                TriggerKind::InView => "inView",
                TriggerKind::Scroll => "scroll",
                TriggerKind::DragHorizontal => "dragHorizontal",
                TriggerKind::DragVertical => "dragVertical",
                TriggerKind::Timeline => "timeline",
            };

            // If there's an inline motion definition, emit it as a player config
            // If there's a motion_ref, reference the named motion function
            let config_override = if let Some(ref inline) = trigger.inline_motion {
                self.motion_body_to_config(inline)
            } else {
                String::new()
            };

            if let Some(ref motion_name) = trigger.motion_ref {
                match trigger.kind {
                    TriggerKind::Enter => {
                        // @enter: create animation and play immediately (in mount JS)
                        self.mount_js.push_str(&format!(
                            "// @enter trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 player.play();\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Hover => {
                        // @hover: play on mouseenter, reverse on mouseleave
                        self.js.push_str(&format!(
                            "// @hover trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"mouseenter\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseleave\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Press => {
                        // @press: play on mousedown, reverse on mouseup
                        self.js.push_str(&format!(
                            "// @press trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"mousedown\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseup\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Focus => {
                        // @focus: play on focus, reverse on blur
                        self.js.push_str(&format!(
                            "// @focus trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"focus\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"blur\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                    TriggerKind::Leave => {
                        // @leave: register cleanup on unmount
                        self.mount_js.push_str(&format!(
                            "// @leave trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 __omni_createAnimation(\"{nid}\", config);\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str
                        ));
                    }
                    TriggerKind::InView => {
                        // @in-view: use IntersectionObserver to play animation when element enters viewport
                        let repeat_val = match &el.in_view_repeat {
                            Some(expr) => {
                                let js = self.expr_to_js(expr);
                                if js == "true" {
                                    true
                                } else {
                                    false
                                }
                            }
                            None => false,
                        };
                        // Default: play once (disconnect after fire). Only repeat if @in-view-repeat={true}
                        let should_disconnect = if repeat_val { "false" } else { "true" };
                        let threshold_val = match &el.in_view_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @in-view trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 const threshold = {threshold};\n\
                             \x20 const observer = new IntersectionObserver((entries) => {{\n\
                             \x20\x20 entries.forEach(entry => {{\n\
                             \x20\x20\x20 if (entry.isIntersecting) {{\n\
                             \x20\x20\x20\x20 player.play();\n\
                             \x20\x20\x20\x20 if ({disconnect}) {{\n\
                             \x20\x20\x20\x20\x20 observer.disconnect();\n\
                             \x20\x20\x20\x20 }}\n\
                             \x20\x20\x20 }}\n\
                             \x20\x20 }});\n\
                             \x20 }}, {{ threshold: threshold }});\n\
                             \x20 observer.observe(el);\n\
                             \x20 __omni_inview_observers = __omni_inview_observers || [];\n\
                             \x20 __omni_inview_observers.push(observer);\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name,
                            disconnect = should_disconnect,
                            threshold = threshold_val,
                        ));
                    }
                    TriggerKind::Scroll => {
                        // @scroll: drive animation progress from scroll position
                        let scroll_start_str = el.scroll_start.as_deref().unwrap_or("top 80%");
                        let scroll_end_str = el.scroll_end.as_deref().unwrap_or("bottom 20%");
                        let scrub_val = match &el.scroll_scrub {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @scroll trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
                             \x20 const startCfg = \"{scroll_start}\";\n\
                             \x20 const endCfg = \"{scroll_end}\";\n\
                             \x20 const scrubDelay = {scrub};\n\
                             \x20 function computeProgress() {{\n\
                             \x20\x20 const rect = el.getBoundingClientRect();\n\
                             \x20\x20 const vh = window.innerHeight;\n\
                             \x20\x20 const startPos = __omni_parseScrollTrigger(startCfg, el, vh);\n\
                             \x20\x20 const endPos = __omni_parseScrollTrigger(endCfg, el, vh);\n\
                             \x20\x20 const raw = (window.scrollY - startPos) / (endPos - startPos);\n\
                             \x20\x20 return Math.max(0, Math.min(1, raw));\n\
                             \x20 }}\n\
                             \x20 let currentProgress = 0;\n\
                             \x20 window.addEventListener('scroll', () => {{\n\
                             \x20\x20 const target = computeProgress();\n\
                             \x20\x20 if (scrubDelay > 0) {{\n\
                             \x20\x20\x20 currentProgress += (target - currentProgress) * Math.min(1, 16.67 / (scrubDelay * 1000));\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 currentProgress = target;\n\
                             \x20\x20 }}\n\
                             \x20\x20 player.seek(currentProgress);\n\
                             \x20 }}, {{ passive: true }});\n\
                             \x20 player.seek(computeProgress());\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name,
                            scroll_start = scroll_start_str,
                            scroll_end = scroll_end_str,
                            scrub = scrub_val,
                        ));
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        let axis_label = if is_horizontal { "x" } else { "y" };
                        let pointer_axis = if is_horizontal { "clientX" } else { "clientY" };
                        let edge_min = if is_horizontal { "left" } else { "top" };
                        let edge_max = if is_horizontal { "right" } else { "bottom" };
                        let rect_size = if is_horizontal { "width" } else { "height" };
                        let constraints_str = el.drag_constraints.as_deref().unwrap_or("");
                        let elastic_val = match &el.drag_elastic {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let threshold_val = match &el.drag_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let velocity_max_val = match &el.drag_velocity_max {
                            Some(expr) => self.expr_to_js(expr),
                            None => "2000".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @drag-{axis} trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
                             \x20 let startPos = 0, currentDelta = 0;\n\
                             \x20 let isDragging = false;\n\
                             \x20 let lastMoveTime = 0, lastMovePos = 0;\n\
                             \x20 let velocity = 0;\n\
                             \x20 const constraints = \"{constraints}\";\n\
                             \x20 const elastic = {elastic};\n\
                             \x20 const threshold = {threshold};\n\
                             \x20 const velocityMax = {velocity_max};\n\
                             \x20 function parseConstraint(cfg) {{\n\
                             \x20\x20 if (!cfg) return {{ min: -Infinity, max: Infinity }};\n\
                             \x20\x20 if (cfg === 'parent') {{\n\
                             \x20\x20\x20 const parent = el.parentElement;\n\
                             \x20\x20\x20 if (!parent) return {{ min: -Infinity, max: Infinity }};\n\
                             \x20\x20\x20 const elRect = el.getBoundingClientRect();\n\
                             \x20\x20\x20 const parentRect = parent.getBoundingClientRect();\n\
                             \x20\x20\x20 return {{ min: 0, max: parentRect.{size} - elRect.{size} }};\n\
                             \x20\x20 }}\n\
                             \x20\x20 if (cfg.startsWith('ref:')) {{\n\
                             \x20\x20\x20 const refId = cfg.slice(4);\n\
                             \x20\x20\x20 const refEl = __omni_el(refId);\n\
                             \x20\x20\x20 if (!refEl) return {{ min: -Infinity, max: Infinity }};\n\
                             \x20\x20\x20 const elRect = el.getBoundingClientRect();\n\
                             \x20\x20\x20 const refRect = refEl.getBoundingClientRect();\n\
                             \x20\x20\x20 return {{ min: 0, max: refRect.{size} - elRect.{size} }};\n\
                             \x20\x20 }}\n\
                             \x20\x20 const parts = cfg.split(' ');\n\
                             \x20\x20 let min = -Infinity, max = Infinity;\n\
                             \x20\x20 parts.forEach(p => {{\n\
                             \x20\x20\x20 const [key, val] = p.split(':');\n\
                             \x20\x20\x20 if (key === '{edge_min}') min = parseFloat(val) || 0;\n\
                             \x20\x20\x20 if (key === '{edge_max}') max = parseFloat(val) || Infinity;\n\
                             \x20\x20 }});\n\
                             \x20\x20 return {{ min, max }};\n\
                             \x20 }}\n\
                             \x20 const constraint = parseConstraint(constraints);\n\
                             \x20 const maxExtent = Math.abs(constraint.max - constraint.min) || 1;\n\
                             \x20 el.addEventListener('pointerdown', (e) => {{\n\
                             \x20\x20 isDragging = true;\n\
                             \x20\x20 startPos = e.{ptr_axis};\n\
                             \x20\x20 currentDelta = 0;\n\
                             \x20\x20 el.setPointerCapture(e.pointerId);\n\
                             \x20 }});\n\
                             \x20 el.addEventListener('pointermove', (e) => {{\n\
                             \x20\x20 if (!isDragging) return;\n\
                             \x20\x20 const now = performance.now();\n\
                             \x20\x20 const raw = e.{ptr_axis} - startPos;\n\
                             \x20\x20 if (lastMoveTime > 0) {{\n\
                             \x20\x20\x20 const dt = now - lastMoveTime;\n\
                             \x20\x20\x20 const dp = Math.abs(raw - lastMovePos);\n\
                             \x20\x20\x20 if (dt > 0) velocity = Math.min(velocityMax, dp / dt * 1000);\n\
                             \x20\x20 }}\n\
                             \x20\x20 lastMoveTime = now;\n\
                             \x20\x20 lastMovePos = raw;\n\
                             \x20\x20 let constrained = raw;\n\
                             \x20\x20 if (raw < constraint.min) {{\n\
                             \x20\x20\x20 const overshoot = constraint.min - raw;\n\
                             \x20\x20\x20 constrained = constraint.min - overshoot * (1 - elastic);\n\
                             \x20\x20 }} else if (raw > constraint.max) {{\n\
                             \x20\x20\x20 const overshoot = raw - constraint.max;\n\
                             \x20\x20\x20 constrained = constraint.max + overshoot * (1 - elastic);\n\
                             \x20\x20 }}\n\
                             \x20\x20 currentDelta = raw;\n\
                             \x20\x20 const progress = Math.abs(constrained) / maxExtent;\n\
                             \x20\x20 player.seek(Math.min(1, progress));\n\
                             \x20 }});\n\
                             \x20 el.addEventListener('pointerup', (e) => {{\n\
                             \x20\x20 isDragging = false;\n\
                             \x20\x20 el.releasePointerCapture(e.pointerId);\n\
                             \x20\x20 const progress = Math.abs(currentDelta) / maxExtent;\n\
                             \x20\x20 if (progress >= threshold && maxExtent > 0) {{\n\
                             \x20\x20\x20 player.play();\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 player.reverse();\n\
                             \x20\x20 }}\n\
                             \x20 }});\n\
                             }})();\n",
                            nid = nid,
                            axis = axis_label,
                            trigger = trigger_str,
                            motion = motion_name,
                            constraints = constraints_str,
                            elastic = elastic_val,
                            threshold = threshold_val,
                            ptr_axis = pointer_axis,
                            edge_min = edge_min,
                            edge_max = edge_max,
                            size = rect_size,
                            velocity_max = velocity_max_val,
                        ));
                    }
                    _ => {
                        // Other triggers: create animation, don't auto-play
                        self.js.push_str(&format!(
                            "// @{trigger} trigger\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\" }};\n\
                             \x20\x20 const player = __omni_motion_{motion}(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            motion = motion_name
                        ));
                    }
                }
            } else if !config_override.is_empty() {
                // Inline motion definition — full config embedded
                match trigger.kind {
                    TriggerKind::Enter => {
                        self.mount_js.push_str(&format!(
                            "// @enter (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20\x20 const player = new AnimationPlayer(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 player.play();\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override
                        ));
                    }
                    TriggerKind::Hover => {
                        self.js.push_str(&format!(
                            "// @hover (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20\x20 const player = new AnimationPlayer(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20\x20 el.addEventListener(\"mouseenter\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.play(); }});\n\
                             \x20\x20 el.addEventListener(\"mouseleave\", function() {{ __omni_getAnimation(\"{nid}\", \"{trigger}\")?.reverse(); }});\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override
                        ));
                    }
                    TriggerKind::InView => {
                        let once_val = match &el.in_view_once {
                            Some(expr) => self.expr_to_js(expr),
                            None => "true".to_string(),
                        };
                        let threshold_val = match &el.in_view_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @in-view (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20 const player = new AnimationPlayer(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 const threshold = {threshold};\n\
                             \x20 const observer = new IntersectionObserver((entries) => {{\n\
                             \x20\x20 entries.forEach(entry => {{\n\
                             \x20\x20\x20 if (entry.isIntersecting) {{\n\
                             \x20\x20\x20\x20 player.play();\n\
                             \x20\x20\x20\x20 if ({once}) {{\n\
                             \x20\x20\x20\x20\x20 observer.disconnect();\n\
                             \x20\x20\x20\x20 }}\n\
                             \x20\x20\x20 }}\n\
                             \x20\x20 }});\n\
                             \x20 }}, {{ threshold: threshold }});\n\
                             \x20 observer.observe(el);\n\
                             \x20 __omni_inview_observers = __omni_inview_observers || [];\n\
                             \x20 __omni_inview_observers.push(observer);\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override,
                            once = once_val,
                            threshold = threshold_val,
                        ));
                    }
                    TriggerKind::Scroll => {
                        let scroll_start_str = el.scroll_start.as_deref().unwrap_or("top 80%");
                        let scroll_end_str = el.scroll_end.as_deref().unwrap_or("bottom 20%");
                        let scrub_val = match &el.scroll_scrub {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @scroll (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20 const player = new AnimationPlayer(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
                             \x20 const startCfg = \"{scroll_start}\";\n\
                             \x20 const endCfg = \"{scroll_end}\";\n\
                             \x20 const scrubDelay = {scrub};\n\
                             \x20 function computeProgress() {{\n\
                             \x20\x20 const rect = el.getBoundingClientRect();\n\
                             \x20\x20 const vh = window.innerHeight;\n\
                             \x20\x20 const startPos = __omni_parseScrollTrigger(startCfg, el, vh);\n\
                             \x20\x20 const endPos = __omni_parseScrollTrigger(endCfg, el, vh);\n\
                             \x20\x20 const raw = (window.scrollY - startPos) / (endPos - startPos);\n\
                             \x20\x20 return Math.max(0, Math.min(1, raw));\n\
                             \x20 }}\n\
                             \x20 let currentProgress = 0;\n\
                             \x20 window.addEventListener('scroll', () => {{\n\
                             \x20\x20 const target = computeProgress();\n\
                             \x20\x20 if (scrubDelay > 0) {{\n\
                             \x20\x20\x20 currentProgress += (target - currentProgress) * Math.min(1, 16.67 / (scrubDelay * 1000));\n\
                             \x20\x20\x20 player.seek(currentProgress);\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 currentProgress = target;\n\
                             \x20\x20\x20 player.seek(currentProgress);\n\
                             \x20\x20 }}\n\
                             \x20 }}, {{ passive: true }});\n\
                             \x20 player.seek(computeProgress());\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override,
                            scroll_start = scroll_start_str,
                            scroll_end = scroll_end_str,
                            scrub = scrub_val,
                        ));
                    }
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical => {
                        let is_horizontal = matches!(trigger.kind, TriggerKind::DragHorizontal);
                        let pointer_axis = if is_horizontal { "clientX" } else { "clientY" };
                        let edge_min = if is_horizontal { "left" } else { "top" };
                        let edge_max = if is_horizontal { "right" } else { "bottom" };
                        let rect_size = if is_horizontal { "width" } else { "height" };
                        let axis_label = if is_horizontal { "x" } else { "y" };
                        let constraints_str = el.drag_constraints.as_deref().unwrap_or("");
                        let elastic_val = match &el.drag_elastic {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let threshold_val = match &el.drag_threshold {
                            Some(expr) => self.expr_to_js(expr),
                            None => "0".to_string(),
                        };
                        let velocity_max_val = match &el.drag_velocity_max {
                            Some(expr) => self.expr_to_js(expr),
                            None => "2000".to_string(),
                        };
                        self.js.push_str(&format!(
                            "// @drag-{axis} (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (!el) return;\n\
                             \x20 const config = {{ trigger: \"{trigger}\", {cfg} }};\n\
                             \x20 const player = new AnimationPlayer(el, config);\n\
                             \x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 player.pause();\n\
                             \x20 let startPos = 0, currentDelta = 0;\n\
                             \x20 let isDragging = false;\n\
                             \x20 let lastMoveTime = 0, lastMovePos = 0;\n\
                             \x20 let velocity = 0;\n\
                             \x20 const constraints = \"{constraints}\";\n\
                             \x20 const elastic = {elastic};\n\
                             \x20 const threshold = {threshold};\n\
                             \x20 const velocityMax = {velocity_max};\n\
                             \x20 function parseConstraint(cfg) {{\n\
                             \x20\x20 if (!cfg) return {{ min: -Infinity, max: Infinity }};\n\
                             \x20\x20 if (cfg === 'parent') {{\n\
                             \x20\x20\x20 const parent = el.parentElement;\n\
                             \x20\x20\x20 if (!parent) return {{ min: -Infinity, max: Infinity }};\n\
                             \x20\x20\x20 const elRect = el.getBoundingClientRect();\n\
                             \x20\x20\x20 const parentRect = parent.getBoundingClientRect();\n\
                             \x20\x20\x20 return {{ min: 0, max: parentRect.{size} - elRect.{size} }};\n\
                             \x20\x20 }}\n\
                             \x20\x20 if (cfg.startsWith('ref:')) {{\n\
                             \x20\x20\x20 const refId = cfg.slice(4);\n\
                             \x20\x20\x20 const refEl = __omni_el(refId);\n\
                             \x20\x20\x20 if (!refEl) return {{ min: -Infinity, max: Infinity }};\n\
                             \x20\x20\x20 const elRect = el.getBoundingClientRect();\n\
                             \x20\x20\x20 const refRect = refEl.getBoundingClientRect();\n\
                             \x20\x20\x20 return {{ min: 0, max: refRect.{size} - elRect.{size} }};\n\
                             \x20\x20 }}\n\
                             \x20\x20 const parts = cfg.split(' ');\n\
                             \x20\x20 let min = -Infinity, max = Infinity;\n\
                             \x20\x20 parts.forEach(p => {{\n\
                             \x20\x20\x20 const [key, val] = p.split(':');\n\
                             \x20\x20\x20 if (key === '{edge_min}') min = parseFloat(val) || 0;\n\
                             \x20\x20\x20 if (key === '{edge_max}') max = parseFloat(val) || Infinity;\n\
                             \x20\x20 }});\n\
                             \x20\x20 return {{ min, max }};\n\
                             \x20 }}\n\
                             \x20 const constraint = parseConstraint(constraints);\n\
                             \x20 const maxExtent = Math.abs(constraint.max - constraint.min) || 1;\n\
                             \x20 el.addEventListener('pointerdown', (e) => {{\n\
                             \x20\x20 isDragging = true;\n\
                             \x20\x20 startPos = e.{ptr_axis};\n\
                             \x20\x20 currentDelta = 0;\n\
                             \x20\x20 el.setPointerCapture(e.pointerId);\n\
                             \x20 }});\n\
                             \x20 el.addEventListener('pointermove', (e) => {{\n\
                             \x20\x20 if (!isDragging) return;\n\
                             \x20\x20 const now = performance.now();\n\
                             \x20\x20 const raw = e.{ptr_axis} - startPos;\n\
                             \x20\x20 if (lastMoveTime > 0) {{\n\
                             \x20\x20\x20 const dt = now - lastMoveTime;\n\
                             \x20\x20\x20 const dp = Math.abs(raw - lastMovePos);\n\
                             \x20\x20\x20 if (dt > 0) velocity = Math.min(velocityMax, dp / dt * 1000);\n\
                             \x20\x20 }}\n\
                             \x20\x20 lastMoveTime = now;\n\
                             \x20\x20 lastMovePos = raw;\n\
                             \x20\x20 let constrained = raw;\n\
                             \x20\x20 if (raw < constraint.min) {{\n\
                             \x20\x20\x20 const overshoot = constraint.min - raw;\n\
                             \x20\x20\x20 constrained = constraint.min - overshoot * (1 - elastic);\n\
                             \x20\x20 }} else if (raw > constraint.max) {{\n\
                             \x20\x20\x20 const overshoot = raw - constraint.max;\n\
                             \x20\x20\x20 constrained = constraint.max + overshoot * (1 - elastic);\n\
                             \x20\x20 }}\n\
                             \x20\x20 currentDelta = raw;\n\
                             \x20\x20 const progress = Math.abs(constrained) / maxExtent;\n\
                             \x20\x20 player.seek(Math.min(1, progress));\n\
                             \x20 }});\n\
                             \x20 el.addEventListener('pointerup', (e) => {{\n\
                             \x20\x20 isDragging = false;\n\
                             \x20\x20 el.releasePointerCapture(e.pointerId);\n\
                             \x20\x20 const progress = Math.abs(currentDelta) / maxExtent;\n\
                             \x20\x20 if (progress >= threshold && maxExtent > 0) {{\n\
                             \x20\x20\x20 player.play();\n\
                             \x20\x20 }} else {{\n\
                             \x20\x20\x20 player.reverse();\n\
                             \x20\x20 }}\n\
                             \x20 }});\n\
                             }})();\n",
                            nid = nid,
                            cfg = config_override,
                            axis = axis_label,
                            trigger = trigger_str,
                            constraints = constraints_str,
                            elastic = elastic_val,
                            threshold = threshold_val,
                            ptr_axis = pointer_axis,
                            edge_min = edge_min,
                            edge_max = edge_max,
                            size = rect_size,
                            velocity_max = velocity_max_val,
                        ));
                    }
                    _ => {
                        self.js.push_str(&format!(
                            "// @{trigger} (inline)\n\
                             (function() {{\n\
                             \x20 const el = __omni_el(\"{nid}\");\n\
                             \x20 if (el) {{\n\
                             \x20\x20 const config = {{ trigger: \"{trigger}\", {config} }};\n\
                             \x20\x20 const player = new AnimationPlayer(el, config);\n\
                             \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", player);\n\
                             \x20 }}\n\
                             }})();\n",
                            nid = nid,
                            trigger = trigger_str,
                            config = config_override
                        ));
                    }
                }
            }
        }
    }

    /// Emit a timeline definition JS (orchestrates animations by element ID).
    fn emit_timeline_js(&mut self, timeline: &TimelineIntent) {
        // Timeline items use element IDs to reference elements across the component tree.
        // Each item schedules an animation with setTimeout at the computed delay offset.
        // This is emitted as part of the deferred JS to ensure DOM is ready.
        self.mount_js
            .push_str(&format!("/// ── @timeline {} ──\n", timeline.name));

        let mut cursor_delay = 0.0;
        let mut labels: Vec<(String, f64)> = Vec::new();

        for (i, item) in timeline.items.iter().enumerate() {
            let item_label = format!("tl_{}_item_{}", timeline.name, i);

            match &item.kind {
                TimelineItemKindIR::Direction {
                    dir,
                    properties,
                    over,
                    ease,
                    delay,
                    color_space: _,
                    start,
                    ref path,
                    ref path_rotate,
                } => {
                    // Build direction properties (from/to/fromTo)
                    let (from_props, to_props) = match dir {
                        DirectionIR::To => {
                            let to: Vec<String> = properties
                                .iter()
                                .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                                .collect();
                            (String::new(), to.join(", "))
                        }
                        DirectionIR::FromTo => {
                            // Expect an even number of entries: alternating from/to
                            let mut from: Vec<String> = Vec::new();
                            let mut to: Vec<String> = Vec::new();
                            let mut is_to = false;
                            for (k, v) in properties.iter() {
                                if is_to {
                                    to.push(format!("\"{}\": {}", k, self.expr_to_js(v)));
                                } else {
                                    from.push(format!("\"{}\": {}", k, self.expr_to_js(v)));
                                }
                                is_to = !is_to;
                            }
                            (from.join(", "), to.join(", "))
                        }
                    };

                    // Override cursor_delay if this item has explicit delay
                    let item_delay = delay
                        .as_ref()
                        .and_then(|d| {
                            if let ExprIR::LiteralExpr { value } = d {
                                value.as_f64()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(cursor_delay);

                    // Over duration
                    let over_ms = if let ExprIR::LiteralExpr { value } = over {
                        value.as_f64().unwrap_or(300.0)
                    } else {
                        300.0
                    };

                    // Reference element by ID attribute
                    let element_ref = start.as_deref().unwrap_or_else(|| {
                        // Use generic element query — will be refined in practice
                        "[data-nid]"
                    });

                    // Path animation support
                    let path_config = path.as_ref().map(|p| {
                        let mut path_parts = vec![format!("\"path\": \"{}\"", p.replace('"', "\\\""))];
                        if let Some(pr) = path_rotate {
                            path_parts.push(format!("\"pathRotate\": {}", pr));
                        }
                        path_parts.join(",\n\x20\x20\x20\x20")
                    }).unwrap_or_default();

                    self.mount_js.push_str(&format!(
                        "setTimeout(function() {{\n\
                         \x20 const el = document.querySelector(\"{sel}\");\n\
                         \x20 if (el) {{\n\
                         \x20\x20 const config = {{\n\
                         \x20\x20\x20 {from}\n\
                         \x20\x20\x20 {to}\n\
                         \x20\x20\x20 \"over\": {over},\n\
                         \x20\x20\x20 \"easing\": {easing}{path_comma}{path}\n\
                         \x20\x20 }};\n\
                         \x20\x20 const player = new AnimationPlayer(el, config);\n\
                         \x20\x20 __omni_animations.set(\"{label}\", player);\n\
                         \x20\x20 player.play();\n\
                         \x20 }}\n\
                         }}, {delay});\n",
                        sel = element_ref,
                        from = if from_props.is_empty() {
                            String::new()
                        } else {
                            format!("\"from\": {{{}}},\n", from_props)
                        },
                        to = if to_props.is_empty() {
                            String::new()
                        } else {
                            format!("\"to\": {{{}}},\n", to_props)
                        },
                        over = over_ms,
                        easing = ease
                            .as_ref()
                            .map(|e| self.easing_to_js(e))
                            .unwrap_or_else(|| "\"easeOut\"".into()),
                        path_comma = if path_config.is_empty() { "" } else { ",\n\x20\x20\x20\x20" },
                        path = path_config,
                        delay = item_delay,
                        label = item_label
                    ));

                    // Advance cursor by over + delay for next chained item
                    cursor_delay = item_delay + over_ms;
                }
                TimelineItemKindIR::Set(props) => {
                    // Set: immediate property change, no animation
                    let props_str: Vec<String> = props
                        .iter()
                        .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                        .collect();
                    self.mount_js.push_str(&format!(
                        "setTimeout(function() {{\n\
                         \x20 document.querySelector(\"[data-nid]\").style.cssText = Object.entries({{{props}}}).map(([k,v]) => k + ':' + v).join(';');\n\
                         }}, {delay});\n",
                        props = props_str.join(", "),
                        delay = cursor_delay
                    ));
                }
                TimelineItemKindIR::MotionRef(motion_name) => {
                    // Reference a named @motion on a set of elements
                    self.mount_js.push_str(&format!(
                        "setTimeout(function() {{\n\
                         \x20 document.querySelectorAll(\"[data-nid]\").forEach(function(el) {{\n\
                         \x20\x20 const player = __omni_motion_{motion}(el, {{}});\n\
                         \x20\x20 __omni_animations.set(el.getAttribute('data-nid') + '_{motion}', player);\n\
                         \x20\x20 player.play();\n\
                         \x20 }});\n\
                         }}, {delay});\n",
                        motion = motion_name,
                        delay = cursor_delay
                    ));
                }
                TimelineItemKindIR::Stagger(motion_expr, timing, _direction) => {
                    let motion_name = match &**motion_expr {
                        // &** dereferences &Box<MotionExprIR> → &MotionExprIR
                        MotionExprIR::Ref(name) => name.clone(),
                        MotionExprIR::Inline(_) => String::new(),
                    };
                    let stagger_delay = if let ExprIR::LiteralExpr { value } = &timing.each {
                        value.as_f64().unwrap_or(50.0)
                    } else {
                        50.0
                    };
                    if !motion_name.is_empty() {
                        self.mount_js.push_str(&format!(
                            "setTimeout(function() {{\n\
                             \x20 document.querySelectorAll(\"[data-nid]\").forEach(function(el, idx) {{\n\
                             \x20\x20 setTimeout(function() {{\n\
                             \x20\x20\x20 const player = __omni_motion_{motion}(el, {{}});\n\
                             \x20\x20\x20 __omni_animations.set(el.getAttribute('data-nid') + '_{motion}', player);\n\
                             \x20\x20\x20 player.play();\n\
                             \x20\x20 }}, idx * {stagger});\n\
                             \x20 }});\n\
                             }}, {delay});\n",
                            motion = motion_name,
                            stagger = stagger_delay,
                            delay = cursor_delay
                        ));
                    }
                }
                TimelineItemKindIR::Label(ref name, ref on_enter) => {
                    // Label: record position for labels dictionary
                    labels.push((name.clone(), cursor_delay));
                    // Execute onEnter callback at the label's position if provided
                    if let Some(ref expr) = on_enter {
                        let callback_js = self.expr_to_js(expr);
                        self.mount_js.push_str(&format!(
                            "setTimeout(function() {{ {} }}, {});\n",
                            callback_js, cursor_delay
                        ));
                    }
                }
            }

            // Update cursor based on connector type
            match &item.connector {
                ConnectorIR::None => {
                    // Reset cursor — items are independent
                    cursor_delay = 0.0;
                }
                ConnectorIR::Then => {
                    // Sequential: cursor accumulates
                    // Already accumulated above
                }
                ConnectorIR::With => {
                    // Parallel: don't accumulate over; cursor stays at current
                    // (already handled — item_delay is computed independently)
                }
            }
        }

        // Emit labels dictionary for timeline
        if !labels.is_empty() {
            let labels_str: Vec<String> = labels
                .iter()
                .map(|(name, pos)| format!("\"{}\": {}", name, pos))
                .collect();
            self.mount_js
                .push_str(&format!("__omni_timeline_labels.set(\"{}\", {{{}}});\n",
                    timeline.name, labels_str.join(", ")));
        }
    }

    // ══════════════════════════════════════════════════════
    // Expression → JavaScript
    // ══════════════════════════════════════════════════════

    fn expr_to_js(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => serde_value_to_js(value),
            ExprIR::IdentifierExpr { name } => {
                // Check rename mapping first: component-local signals, then global fn/events
                let resolved = self
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.global_renames.get(name.as_str()))
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                // Signal references → emit .value for reading
                if self.signal_names.contains(resolved.as_str()) {
                    format!("{}.value", resolved)
                } else if self.signal_names.contains(name.as_str()) {
                    format!("{}.value", name)
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
                // If object is a store name, use flat naming: TodoStore.items → TodoStore_items
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
                // Map built-in Omni functions to JS equivalents
                if name == "println" || name == "print" {
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                    return format!("console.log({})", arg_strs.join(", "));
                }
                // Check stdlib intrinsics
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
                    // Field read access — use .value if it's a signal/getter
                    if self.signal_names.contains(&var_name) {
                        format!("{}.value", var_name)
                    } else {
                        var_name.clone()
                    }
                } else {
                    // Action call with arguments
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
                // Get raw store name before name_renames resolution (for inline stores)
                let raw_store_name = if let ExprIR::IdentifierExpr { name } = object.as_ref() {
                    name.clone()
                } else {
                    String::new()
                };
                let base = self.expr_to_js(object);
                // If base is a store name, use flat naming: TodoStore.add(text) → TodoStore_add(text)
                if self.store_names.contains(&raw_store_name) || self.store_names.contains(&base) {
                    // Use renamed base for inline stores (CartStore → ComponentName)
                    // Check both raw and resolved names in name_renames
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
                // If base is a signal name, use .value for access
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
                        // IIFE: push to array and return the array (not the length)
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
                // Block expression: preceding statements (side effects) are
                // preserved in IR for future codegen handling.
                // For now, emit the tail expression as the value.
                // TODO: emit preceding statements as IIFE when they have side effects.
                if !statements.is_empty() {
                    // Warn: side-effecting statements in block expression
                    // will be preserved in IR but not yet emitted in JS
                }
                self.expr_to_js(tail)
            }
            ExprIR::IndexExpr { object, index } => {
                let obj = self.expr_to_js(object);
                let idx = self.expr_to_js(index);
                // If object is a signal, use .value for the array access
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
                                // Emit let-bindings for each captured field so the body
                                // can reference them directly (e.g. `error` → `_v.error`)
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
                // Wrap in IIFE so match can be used inside expressions/templates.
                // The `return` captures the IIFE result back to the caller.
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

    /// Format a block's trailing expression as a JS value (for match arm bodies).
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

    /// Format a JSON value as a JS literal.
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

    fn emit_style(&mut self, style: &StyleRuleIR) {
        self.css.push_str(&format!(
            "[data-component=\"{}\"] .{} {{\n",
            self.current_component, style.class_name
        ));
        for (prop, value) in &style.properties {
            let css_val = self.expr_to_css(value);
            let css_prop = to_css_property(prop);
            self.css
                .push_str(&format!("  {}: {};\n", css_prop, css_val));
        }
        self.css.push_str("}\n\n");
    }

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
fn serde_value_to_js(value: &serde_json::Value) -> String {
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
            let items: Vec<String> = arr.iter().map(serde_value_to_js).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let entries: Vec<String> = obj
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, serde_value_to_js(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
    }
}

// ══════════════════════════════════════════════════════════
// Mapping Helpers
// ══════════════════════════════════════════════════════════

/// Map omni component tags to HTML tags.
fn map_tag(tag: &str) -> String {
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
fn map_event(name: &str) -> String {
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
        n if n.starts_with("on") => n[2..].to_lowercase(),
        n => n.to_string(),
    }
}

/// Map omni binary operators to JS operators.
fn map_binary_op(op: &str) -> &str {
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
fn to_kebab(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert camelCase CSS property to kebab-case.
fn to_css_property(prop: &str) -> String {
    let mut result = String::new();
    for c in prop.chars() {
        if c.is_uppercase() {
            result.push('-');
            result.push(c.to_lowercase().next().unwrap_or(c));
        } else {
            result.push(c);
        }
    }
    result
}

/// Escape HTML special characters.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ══════════════════════════════════════════════════════════
// Codegen Dispatch Wrapper
// ══════════════════════════════════════════════════════════



use super::{CompileOutput, CompiledFile};

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
    let output = generate(ir, false);
    CompileOutput::success(vec![
        CompiledFile {
            path: "index.js".into(),
            content: output.js,
        },
        CompiledFile {
            path: "index.html".into(),
            content: output.html,
        },
        CompiledFile {
            path: "index.css".into(),
            content: output.css,
        },
    ])
}

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
