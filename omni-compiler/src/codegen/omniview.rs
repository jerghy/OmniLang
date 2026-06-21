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

/// Output of the OmniView codegen — standalone JS/HTML/CSS.
#[derive(Debug, Clone)]
pub struct OmniViewOutput {
    /// JavaScript code (inline signal runtime + generated component)
    pub js: String,
    /// HTML markup with data-nid attributes
    pub html: String,
    /// CSS style rules
    pub css: String,
    /// Source map entries
    pub source_map: Vec<SourceMapEntry>,
}

/// Generate OmniView output from an IR program.
pub fn generate(ir: &IRProgram) -> OmniViewOutput {
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
    };

    gen.emit_js_header();
    gen.emit_signal_runtime();

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

    // Generate each component (signals, fns). Only root component emits its view.
    for (comp_name, comp) in ir.all_components() {
        gen.generate_component(comp_name, comp, ir, comp_name == root_name);
    }

    // Emit ONE innerHTML insertion with accumulated HTML + ALL deferred bindings
    gen.emit_dom_and_bindings();

    gen.finalize_js();

    OmniViewOutput {
        js: gen.js,
        html: gen.html,
        css: gen.css,
        source_map: gen.source_map,
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
        self.current_component = name.to_string();
        // Clear component-local signal renames (global function/event renames persist).
        self.name_renames.clear();
        self.signal_names.clear();
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
                        self.mount_js.push_str(&format!("// @mount {}\n", name));
                        self.mount_js.push_str(&body_js);
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
                self.mount_js.push_str(&format!(
                    "// @effect {}\n__omni_effect(() => {{\n{}}});\n",
                    name, body_js
                ));
            }
            self.signal_names = saved_signal_names;
            self.current_component = saved_component;
        }

        self.js.push('\n');
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
        let params: Vec<String> = f.params.iter().map(|p| super::sanitize_js_ident(&p.name)).collect();
        // Register renamed params so body references also use the sanitized name
        for (i, p) in f.params.iter().enumerate() {
            let sanitized = &params[i];
            if sanitized != &p.name {
                self.name_renames.insert(p.name.clone(), sanitized.clone());
            }
        }
        let async_kw = if f.is_async { "async " } else { "" };
        self.js.push_str(&format!(
            "{}function {}({}) {{\n",
            async_kw,
            name,
            params.join(", ")
        ));
        for stmt in &f.body.statements {
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
                self.js.push_str(&format!("  let {} = {};\n", safe_name, val));
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
                else_body,
            } => {
                let cond = self.expr_to_js(condition);
                self.js.push_str(&format!("  if ({}) {{\n", cond));
                for s in &then_body.statements {
                    self.emit_statement(s);
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
                        pad, nid, js_expr
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

        self.js
            .push_str(&format!("\n/// ── @if conditional {} ──\n", nid));
        self.js.push_str("__omni_effect(() => {\n");

        // Emit all NIDs as a JS array for efficient bulk toggling
        let nid_list: Vec<String> = branch_nids
            .iter()
            .chain(fallback_nid.iter())
            .map(|n| format!("\"{}\"", n))
            .collect();
        self.js
            .push_str(&format!("  const _nids = [{}];\n", nid_list.join(", ")));
        // Helper to toggle all branches at once (O(n) not O(n²))
        self.js.push_str("  const _show = (nid) => { _nids.forEach(id => { const e = __omni_el(id); if (e) e.style.display = (id === nid) ? \"block\" : \"none\"; }); };\n");

        // For each branch, build the if/else if chain
        for (i, (cond_js, branch_nid)) in conditions_js.iter().enumerate() {
            let if_word = if i == 0 { "if" } else { "else if" };
            self.js.push_str(&format!(
                "  {} ({}) {{ _show(\"{}\"); }}\n",
                if_word, cond_js, branch_nid
            ));
        }

        // Else clause (fallback or hide-all)
        if let Some(ref fb_nid) = fallback_nid {
            self.js
                .push_str(&format!("  else {{ _show(\"{}\"); }}\n", fb_nid));
        } else {
            // No @else — hide all branches when no condition matches
            self.js.push_str("  else { _nids.forEach(id => { const e = __omni_el(id); if (e) e.style.display = \"none\"; }); }\n");
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

        // Re-render loop: clear container, clone template for each item
        self.js.push_str("  // Clear existing rendered items\n");
        self.js
            .push_str("  ctr.querySelectorAll('[data-for-item]').forEach(el => el.remove());\n");

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

        // Iterate and clone
        self.js
            .push_str(&format!("  data.forEach(({}, _idx) => {{\n", item_var));
        self.js
            .push_str("    const clone = tpl.content.firstElementChild.cloneNode(true);\n");
        self.js
            .push_str("    clone.setAttribute('data-for-item', _idx);\n");
        self.js.push_str("    // Bind text interpolation markers\n");
        self.js
            .push_str("    clone.querySelectorAll('[data-bind-text]').forEach(el => {\n");
        self.js
            .push_str("      const expr = el.getAttribute('data-bind-text');\n");
        self.js.push_str("      if (expr) {\n");
        self.js.push_str("        try {\n");
        self.js.push_str(
            "          // Use Function constructor to evaluate expressions like `item.name`\n",
        );
        self.js.push_str(&format!(
            "          el.textContent = String((new Function('{}', 'return ' + expr))({}));\n",
            item_var, item_var
        ));
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
            "            const resolved = (new Function('{}', 'return `' + jsTemplate + '`'))({});\n",
            item_var, item_var
        ));
        self.js
            .push_str("            el.setAttribute(attr.name, resolved);\n");
        self.js
            .push_str("          } catch(e) { /* keep original */ }\n");
        self.js.push_str("        }\n");
        self.js.push_str("      });\n");
        self.js.push_str("    });\n");
        self.js.push_str("    ctr.appendChild(clone);\n");
        self.js.push_str("  });\n");

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
                        match handler.kind.as_str() {
                            "fn_ref" => {
                                if let Some(ref fn_name) = handler.r#ref {
                                    let prefixed =
                                        format!("{}_{}", self.current_component, fn_name);
                                    self.js.push_str(&format!(
                                        "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {}(e.detail)); }}\n",
                                        nid, js_event, prefixed
                                    ));
                                }
                            }
                            "store_action" => {
                                if let Some(ref action_path) = handler.r#ref {
                                    let var_name = action_path.replace('.', "_");
                                    self.js.push_str(&format!(
                                        "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", () => {{ {}(); }}); }}\n",
                                        nid, js_event, var_name
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
                                    "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {}(e.detail)); }}\n",
                                    nid, js_event, wrapper_name
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
                | "pattern" | "selected"
                | "controls" | "width" | "height" | "coords" | "shape" | "media" | "srcset" | "sizes"
                | "target" | "rel" | "lang" | "dir" | "datetime" | "cite" | "list"
                | "loop" | "muted" | "poster" | "preload" | "tabindex" | "role"
                | "content" | "charset" | "http-equiv" | "property" | "itemprop"
                | "crossorigin" | "integrity" | "referrerpolicy" | "loading" | "decoding"
                | "async" | "defer" | "nonce" => {
                    let val_str = self.expr_to_static_str(value);
                    // Known boolean attributes with value "true" — emit bare attribute name
                    let is_boolean_attr = matches!(
                        name.as_str(),
                        "disabled" | "checked" | "required" | "readonly" | "selected"
                        | "autofocus" | "multiple" | "controls" | "loop" | "muted"
                        | "async" | "defer"
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
                    } else if !val_str.is_empty() {
                        attrs.push_str(&format!(" {}=\"{}\"", name, escape_html(&val_str)));
                    } else {
                        attrs.push_str(&format!(" {}", name));
                    }
                }
                _ => {
                    // Emit as data attribute for unknown props
                    let val_str = self.expr_to_static_str(value);
                    if !val_str.is_empty() {
                        attrs.push_str(&format!(" data-{}=\"{}\"", name, escape_html(&val_str)));
                    }
                }
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
            match handler.kind.as_str() {
                "fn_ref" => {
                    if let Some(ref fn_name) = handler.r#ref {
                        let prefixed = format!("{}_{}", self.current_component, fn_name);
                        self.js.push_str(&format!(
                            "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {}(e.detail)); }}\n",
                            nid, js_event, prefixed
                        ));
                    }
                }
                "store_action" => {
                    if let Some(ref action_path) = handler.r#ref {
                        let var_name = action_path.replace('.', "_");
                        self.js.push_str(&format!(
                            "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", () => {{ {}(); }}); }}\n",
                            nid, js_event, var_name
                        ));
                    }
                }
                "inline_fn" => {
                    // Inline event handlers get a named wrapper with params
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
                        "{{ const el = __omni_el(\"{}\"); if (el) el.addEventListener(\"{}\", (e) => {}(e.detail)); }}\n",
                        nid, js_event, wrapper_name
                    ));
                }
                _ => {}
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
                let guard_name = format!("_guard_{}", self.guard_counter);
                self.js
                    .push_str(&format!("let {} = false;\n", guard_name));
                self.js.push_str(&format!(
                    "{{ const el = __omni_el(\"{0}\"); if (el) el.addEventListener(\"input\", (e) => {{ if (!{1}) {{ {2}.set(e.target.value); }} }}); }}\n",
                    nid, guard_name, sig_name
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
                            vals.join(", "), obj
                        )
                    }
                    "pop" => format!("(/* pop */ ({} || []).pop())", obj),
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
            ExprIR::VariantLiteral { variant, fields, .. } => {
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
                            MatchPatternKindIR::Variant { name, .. } => {
                                format!("if (_v._variant === \"{}\") {{ return {}; }}", name, body_js)
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
                    "(() => {{ const _v = {}; {} }})()",
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
                StatementIR::ReturnStmtIR { value } => {
                    value.as_ref()
                        .map(|e| self.expr_to_js(e))
                        .unwrap_or_else(|| "undefined".into())
                }
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

use std::collections::HashMap;

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
    let output = generate(ir);
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
