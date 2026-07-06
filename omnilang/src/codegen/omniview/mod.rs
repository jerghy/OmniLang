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

mod component_html;
mod editor;
mod expr;
mod html;
mod list;
mod list_render;
mod motion;
mod motion_config;
mod motion_triggers;
mod runtime;
mod stmt;
mod store;
mod timeline;
mod utils;
mod view;
pub(crate) use utils::emit_event_object;
pub use utils::{compile_files, wrap_debug_html};

use crate::ir::*;
use crate::sourcemap::SourceMapEntry;
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
        warnings: Vec::new(),
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
    /// Diagnostic warnings collected during codegen (e.g., missing animation params).
    warnings: Vec<String>,
}

impl OmniViewGenerator {
    // ══════════════════════════════════════════════════════
    // Headers
    // ══════════════════════════════════════════════════════

    // ══════════════════════════════════════════════════════
    // Store Emission
    // ══════════════════════════════════════════════════════

    // Emit a store's signals, getters, and actions as global JS variables.
    // Naming convention: `{StoreName}_{fieldName}` for all store fields.

    // ══════════════════════════════════════════════════════
    // View → HTML Generation
    // ══════════════════════════════════════════════════════
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
