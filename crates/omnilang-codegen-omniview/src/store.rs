//! Store emission: signals, getters, actions, and component generation.
use super::utils::serde_value_to_js;
use super::OmniViewGenerator;
use omnilang_core::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::fmt::Write;

/// Extract a human-readable default value string from an ExprIR for editor preview.
fn expr_default_display(expr: &ExprIR, gen: &OmniViewGenerator) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => {
            let js = serde_value_to_js(value);
            // Strip surrounding quotes from string values
            if js.starts_with('"') && js.ends_with('"') && js.len() >= 2 {
                js[1..js.len() - 1].to_string()
            } else {
                js
            }
        }
        _ => {
            let js = gen.expr_to_js(expr);
            // Truncate long expressions
            if js.len() > 40 {
                format!("{}…", &js[..39])
            } else {
                js
            }
        }
    }
}

impl OmniViewGenerator {
    pub(super) fn emit_store(&mut self, _store_name: &str, store: &StoreIR) {
        let prefix = |field: &str| format!("{}_{}", store.name, field);
        self.store_names.insert(store.name.clone());

        // Set up rename mapping: store-local names → flat JS names
        // This ensures getter/action bodies reference the correct JS variables
        // Save and restore the rename map to prevent cross-store pollution
        let saved_renames: FxHashMap<String, String> = std::mem::take(&mut self.name_renames);
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

            let _ = writeln!(
                self.js,
                "const {} = __omni_signal(\"{}\", {});",
                var_name, var_name, init_val
            );
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
            let _ = writeln!(
                self.js,
                "const {} = __omni_computed(\"{}\", () => ({}));",
                var_name, var_name, body_js
            );
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
            let _ = writeln!(self.js, "function {}({}) {{", func_name, params.join(", "));
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

    pub(super) fn generate_component(
        &mut self,
        name: &str,
        comp: &ComponentIR,
        ir: &IRProgram,
        is_root: bool,
    ) {
        let js_before = self.js.len();
        // Snapshot CSS position for per-component delta tracking (css_only mode).
        if self.css_only {
            self.current_css_start = self.css.len();
        }
        self.current_component = name.to_string();
        // Clear component-local signal renames (global function/event renames persist).
        self.name_renames.clear();
        self.signal_names.clear();
        self.comp_mount_js.clear();
        // Clear @enter replay tracking for this component
        self.enter_trigger_nids.clear();
        // Populate component-local style class names for class: validation
        self.component_style_classes.clear();
        for style_rule in &comp.styles {
            if !style_rule.class_name.is_empty() {
                self.component_style_classes
                    .insert(style_rule.class_name.clone());
            }
        }

        // Editor mode: wrap component in a factory IIFE so it can be hot-replaced
        // via __omni_replaceComponent in the editor bridge.
        if self.editor_mode {
            let _ = writeln!(
                self.js,
                "window.__omni_components[\"{}\"] = (function() {{",
                name
            );
        }
        // Register THIS component's signal/computed names with component prefix.
        for state in &comp.states {
            let prefixed = format!("{}_{}", name, state.name);
            self.name_renames
                .insert(state.name.clone(), prefixed.clone());
            self.signal_names.insert(prefixed);
            // Collect default value for editor canvas preview
            let default_str = expr_default_display(&state.initial_value, self);
            self.signal_defaults.insert(state.name.clone(), default_str);
        }
        for c in &comp.computeds {
            let prefixed = format!("{}_{}", name, c.name);
            self.name_renames.insert(c.name.clone(), prefixed.clone());
            self.signal_names.insert(prefixed);
        }
        // Add store signal/getter names so store field access uses .value
        for store_name in &comp.store_refs {
            if let Some(store) = ir.all_stores().get(store_name.as_str()) {
                // Register the store name so FieldAccessExpr on store objects
                // can resolve to {store}_{field}.value format
                self.store_names.insert(store_name.clone());
                for sig_name in store.signals.keys() {
                    let full_name = format!("{}_{}", store_name, sig_name);
                    self.signal_names.insert(full_name);
                    // Collect store signal default value for editor canvas
                    if let Some(val) = store.signal_defaults.get(sig_name) {
                        let key = format!("{}.{}", store_name, sig_name);
                        self.signal_defaults.insert(key, serde_value_to_js(val));
                    }
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

        let _ = write!(self.js, "\n/// ── Component: {} ──\n\n", name);

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
            let _ = writeln!(
                self.js,
                "const {} = __omni_signal(\"{}\", {});",
                prefixed, prefixed, init_val
            );
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
            let _ = writeln!(self.js, "function {}(...args) {{", prefixed_name);
            let _ = writeln!(
                self.js,
                "  const el = document.querySelector('[data-component=\"{}\"]');",
                name
            );
            let _ = writeln!(self.js,
                "  if (el) el.dispatchEvent(new CustomEvent('{}', {{ detail: args[0], bubbles: false }}));",
                dom_event
            );
            self.js.push_str("}\n");
        }

        // Functions — prefix with component name to avoid global name collisions.
        // name_renames already has event→prefixed mappings, so function bodies will
        // correctly call AddTodo_onAdd() instead of onAdd().
        // FIRST pass: register ALL function names so inline store actions are resolved
        // when referencing functions (e.g. CartStore.addItem resolves to InlineStoreDemo_addItem)
        // BEFORE any function body is emitted.
        for f in &comp.fns {
            let prefixed_name = format!("{}{}", fn_prefix, f.name);
            self.name_renames
                .insert(f.name.clone(), prefixed_name.clone());
        }
        // SECOND pass: emit function bodies (name_renames is now fully populated)
        for f in &comp.fns {
            let prefixed_name = format!("{}{}", fn_prefix, f.name);
            self.emit_function_named(f, &prefixed_name);
        }

        // Motion definitions — emit JS functions that create WAAPI Animation instances.
        // Each @motion block generates a const __omni_motion_{name} function.
        if !comp.motions.is_empty() {
            let _ = write!(self.js, "\n/// ── Motion Definitions: {} ──\n\n", name);
            for motion in &comp.motions {
                self.motion_names.insert(motion.name.clone());
                self.emit_motion_js(motion);
            }
        }

        // Timeline definitions — emit JS that orchestrates animations by element ID.
        if !comp.timelines.is_empty() {
            let _ = write!(self.js, "\n/// ── Timeline Definitions: {} ──\n\n", name);
            for timeline in &comp.timelines {
                self.motion_names.insert(timeline.name.clone());
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
            let saved_signal_names: FxHashSet<String> = self.signal_names.iter().cloned().collect();
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
                match kind {
                    LifecycleKind::Mount => {
                        if self.editor_mode {
                            let _ = writeln!(self.comp_mount_js, "// @mount {}", name);
                            self.comp_mount_js.push_str(&body_js);
                        } else {
                            let _ = writeln!(self.mount_js, "// @mount {}", name);
                            self.mount_js.push_str(&body_js);
                        }
                    }
                    LifecycleKind::Unmount => {
                        let _ = write!(
                            self.js,
                            "// @unmount {}\nwindow.__omni_unmount_{} = () => {{\n{}}};\n",
                            name, name, body_js
                        );
                    }
                    LifecycleKind::Effect => {
                        debug_assert!(
                            false,
                            "LifecycleKind::Effect should not reach codegen; filtered during IR lowering"
                        );
                    }
                }
            }
            self.signal_names = saved_signal_names;
            self.current_component = saved_component;
        }

        // @effect hooks — stored separately in comp.effects (not in comp.lifecycles)
        if !comp.effects.is_empty() {
            let saved_component = self.current_component.clone();
            self.current_component = name.to_string();
            let saved_signal_names: FxHashSet<String> = self.signal_names.iter().cloned().collect();
            for state in &comp.states {
                let prefixed = format!("{}_{}", name, state.name);
                self.signal_names.insert(prefixed);
            }

            // Collect body JS for all effects first, then emit to buffer.
            // This avoids a borrow conflict between &mut self.mount_js and self.emit_statement().
            let mut effect_bodies: Vec<String> = Vec::with_capacity(comp.effects.len());
            for effect in &comp.effects {
                let old_js_len = self.js.len();
                for stmt in &effect.body.statements {
                    self.emit_statement(stmt);
                }
                let body_js = self.js[old_js_len..].to_string();
                self.js.truncate(old_js_len);
                effect_bodies.push(body_js);
            }

            // Emit reactive effect via __omni_effect, which auto-tracks
            // signal dependencies and re-runs when they change.
            // Capture the return value (cleanup function) so it can be
            // called on unmount to unsubscribe from signal subscriptions.
            // Placed in mount_js so DOM elements are available.

            // Open IIFE for scope isolation — prevents variable name conflicts
            // between multiple components' effect collections in mount_js.
            if self.editor_mode {
                let _ = write!(
                    self.comp_mount_js,
                    "// @effect {}\n(function() {{\n  var __c = [];\n",
                    name
                );
                for body_js in &effect_bodies {
                    let _ = write!(
                        self.comp_mount_js,
                        "  __c.push(__omni_effect(() => {{\n{}}}));\n",
                        body_js
                    );
                }
                // Chain all captured cleanups into the unmount handler.
                // Wraps any existing window.__omni_unmount_{name} so @unmount
                // lifecycle handlers are also preserved.
                let _ = write!(
                    self.comp_mount_js,
                    "  var _p = window.__omni_unmount_{name};\n\
                     window.__omni_unmount_{name} = _p\n\
                     \x20  ? function() {{ _p(); __c.forEach(function(f){{f();}}); }}\n\
                     \x20  : function() {{ __c.forEach(function(f){{f();}}); }};\n\
                     }})();\n",
                    name = name
                );
            } else {
                let _ = write!(
                    self.mount_js,
                    "// @effect {}\n(function() {{\n  var __c = [];\n",
                    name
                );
                for body_js in &effect_bodies {
                    let _ = write!(
                        self.mount_js,
                        "  __c.push(__omni_effect(() => {{\n{}}}));\n",
                        body_js
                    );
                }
                // Chain all captured cleanups into the unmount handler.
                let _ = write!(
                    self.mount_js,
                    "  var _p = window.__omni_unmount_{name};\n\
                     window.__omni_unmount_{name} = _p\n\
                     \x20  ? function() {{ _p(); __c.forEach(function(f){{f();}}); }}\n\
                     \x20  : function() {{ __c.forEach(function(f){{f();}}); }};\n\
                     }})();\n",
                    name = name
                );
            }

            self.signal_names = saved_signal_names;
            self.current_component = saved_component;
        }

        // @enter replay effect — watches ALL component signals and replays @enter animations
        // when any signal changes. This enables "Play All Animations" buttons to work.
        if !self.enter_trigger_nids.is_empty() && is_root {
            // Collect all signal names from ALL components for dependency tracking
            let mut all_sig_reads: Vec<String> = Vec::new();
            for (cname, ccomp) in ir.all_components() {
                for state in &ccomp.states {
                    let prefixed = format!("{}_{}", cname, state.name);
                    all_sig_reads.push(format!("{}.value;", prefixed));
                }
            }
            let all_sig_reads_str = all_sig_reads.join(" ");
            // Build replay function calls for each @enter trigger
            let mut replay_fns: Vec<String> = Vec::new();
            for (nid, motion, trigger_kind) in &self.enter_trigger_nids {
                // Timeline triggers auto-play via internal setTimeouts — no explicit play() call needed.
                // @enter triggers need explicit play().
                let play_call = if trigger_kind == "timeline" {
                    String::new()
                } else {
                    " if (el.offsetParent !== null) p.play();".into()
                };
                replay_fns.push(format!(
                    "(function() {{ var el = __omni_el(\"{nid}\"); if (el) {{ var _old = __omni_animations.get(\"{nid}_{kind}\"); if (_old) _old.cancel(); var p = __omni_motion_{motion}(el, {{trigger: \"{kind}\"}}); __omni_animations.set(\"{nid}_{kind}\", p);{play} }} }})()",
                    nid = nid, motion = motion, kind = trigger_kind, play = play_call
                ));
            }
            let replay_fns_str = replay_fns.join("; ");
            let _ = write!(self.mount_js,
                "// @enter replay (watches all signals)\n__omni_effect(function __omni_enter_replay() {{\n\
                 \x20 {all_sig_reads}\n\
                 \x20 {replay_fns}\n\
                 }});\n",
                all_sig_reads = all_sig_reads_str,
                replay_fns = replay_fns_str,
            );
            self.enter_trigger_nids.clear();
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
            for prop_name in comp.prop_defaults.keys() {
                let prefixed = format!("{}_{}", name, prop_name);
                signal_entries.push(format!("{}: {}", prefixed, prefixed));
            }

            let signal_entries_str = signal_entries.join(", ");

            let _ = write!(self.js,
                "  return {{\n    signals: {{ {} }},\n    teardown: function() {{\n      var root = document.querySelector('[data-component=\"{}\"]');\n      if (root) root.remove();\n    }}\n  }};\n}})();\n",
                signal_entries_str, name
            );
        }

        self.js.push('\n');

        // Capture per-component JS delta for editor hot-replace (only in editor mode)
        if self.editor_mode {
            let component_js = self.js[js_before..].to_string();
            self.component_js.insert(name.to_string(), component_js);
        }
        // Capture per-component CSS delta for resolve_styles (css_only mode).
        if self.css_only {
            let component_css = self.css[self.current_css_start..].to_string();
            self.per_component_css
                .insert(name.to_string(), component_css);
        }
    }

    /// Emit a style property assignment for an element.
    /// Uses `el.style.setProperty()` for CSS custom properties (`--` prefix),
    /// and direct `el.style.prop = value` for standard properties.
    pub(super) fn emit_style_assignment(&mut self, prop: &str, value_expr: &str) {
        if prop.starts_with("--") {
            let _ = write!(
                self.js,
                "el.style.setProperty(\"{}\",{});",
                prop, value_expr
            );
        } else {
            let _ = write!(self.js, "el.style.{}={};", prop, value_expr);
        }
    }

    /// Finalize the JS output with closing IIFE wrapper.
    pub(super) fn finalize_js(&mut self) {
        self.js.push_str("\n})();\n");
    }

    // ══════════════════════════════════════════════════════
    // Signal Declaration
    // ══════════════════════════════════════════════════════

    pub(super) fn emit_signal_decl(&mut self, state: &StateIntent) {
        let init_val = self.expr_to_js(&state.initial_value);
        let prefixed = format!("{}_{}", self.current_component, state.name);
        let _ = writeln!(
            self.js,
            "const {} = __omni_signal(\"{}\", {});",
            prefixed, prefixed, init_val
        );
    }

    pub(super) fn emit_computed_decl(&mut self, computed: &ComputedIntent) {
        let body = self.expr_to_js(&computed.body);
        let prefixed = format!("{}_{}", self.current_component, computed.name);
        let _ = writeln!(
            self.js,
            "const {} = __omni_computed(\"{}\", () => ({}));",
            prefixed, prefixed, body
        );
    }

    // ══════════════════════════════════════════════════════
    // Function Emission
    // ══════════════════════════════════════════════════════

    pub(super) fn emit_function(&mut self, f: &FunctionIR) {
        self.emit_function_named(f, &f.name);
    }

    pub(super) fn emit_function_named(&mut self, f: &FunctionIR, name: &str) {
        // Sanitize parameter names to avoid JS reserved words (e.g. `default` → `default_`)
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| omnilang_core::codegen::sanitize_js_ident(&p.name))
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
        let _ = writeln!(
            self.js,
            "{}function {}({}) {{",
            async_kw,
            name,
            params.join(", ")
        );
        for (i, stmt) in f.body.statements.iter().enumerate() {
            let is_last = i == f.body.statements.len() - 1;
            // Implicit return: if the function has a return type and the last
            // statement is a trailing expression (not an explicit return), wrap it
            // with `return ` so the value flows back to the caller.
            if is_last && has_ret {
                if let StatementIR::ExprStmtIR { expr } = stmt {
                    let _ = writeln!(self.js, "return {};", self.expr_to_js(expr));
                    continue;
                }
            }
            self.emit_statement(stmt);
        }
        self.js.push_str("}\n\n");
    }
}
