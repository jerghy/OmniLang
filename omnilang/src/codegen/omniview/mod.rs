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
pub mod design;
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
pub use utils::{compile_files, wrap_debug_html};

use crate::error::Diagnostic;
use crate::ir::*;
use crate::sourcemap::SourceMapEntry;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
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
    pub components: FxHashMap<String, ComponentOutput>,
    /// Diagnostic warnings collected during codegen (e.g., unknown variant references)
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Per-element CSS class names: nid → class names applied
    #[serde(default)]
    pub element_classes: FxHashMap<String, Vec<String>>,
    /// Per-element HTML tag names: nid → tag
    #[serde(default)]
    pub element_tags: FxHashMap<String, String>,
    /// Per-component CSS deltas (component name → scoped CSS rules)
    #[serde(default)]
    pub per_component_css: FxHashMap<String, String>,
    /// Structured element tree for React-side rendering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub element_tree: Vec<ElementTreeNode>,
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

/// A single node in the rendered element tree, used by the Poiesis editor
/// to reconstruct the view hierarchy in React without parsing HTML strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementTreeNode {
    pub nid: String,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ElementTreeNode>,
}

/// Top-level result from resolve_styles().
/// Returns compiled CSS + element mappings from the full codegen pipeline
/// (not manual IR traversal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleResolveResult {
    pub success: bool,
    /// Global design-block CSS (tokens + class rules + variant rules),
    /// scoped via [data-design~="Name"] selectors.
    pub design_css: String,
    /// Per-component CSS (style {} block compilation output),
    /// keyed by component name.
    pub component_css: FxHashMap<String, String>,
    /// nid → CSS class names applied to each element during compilation.
    pub element_classes: FxHashMap<String, Vec<String>>,
    /// nid → HTML tag name.
    pub element_tags: FxHashMap<String, String>,
    /// Structured element tree for React-side rendering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub element_tree: Vec<ElementTreeNode>,
    /// Compilation warnings (unknown variant, missing design, etc.)
    pub warnings: Vec<String>,
    /// Parse / semantic errors.
    pub errors: Vec<Diagnostic>,
}

/// Generate OmniView output from an IR program.
pub fn generate(ir: &IRProgram, editor_mode: bool, css_only: bool) -> OmniViewOutput {
    let mut gen = OmniViewGenerator {
        js: String::new(),
        html: String::new(),
        css: String::new(),
        source_map: Vec::new(),
        node_counter: 0,
        current_nid_path: Vec::new(),
        signal_names: FxHashSet::default(),
        store_signal_names: FxHashSet::default(),
        store_field_names: FxHashSet::default(),
        store_names: FxHashSet::default(),
        name_renames: FxHashMap::default(),
        global_renames: FxHashMap::default(),
        component_map: FxHashMap::default(),
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
        component_js: FxHashMap::default(),
        enter_trigger_nids: Vec::new(),
        for_state_names: FxHashMap::default(),
        motion_names: FxHashSet::default(),
        design_names: Vec::new(),
        design_map: FxHashMap::default(),
        warnings: Vec::new(),
        for_text_bindings: Vec::new(),
        for_class_bindings: Vec::new(),
        for_attr_bindings: Vec::new(),
        for_rebind_fn: None,
        css_only,
        element_classes: FxHashMap::default(),
        element_tags: FxHashMap::default(),
        per_component_css: FxHashMap::default(),
        current_css_start: 0,
        component_style_classes: FxHashSet::default(),
        element_tree_stack: Vec::new(),
        element_tree: Vec::new(),
    };

    // In css_only mode, skip ALL JS emission — we only need CSS + element mappings.
    if !css_only {
        gen.emit_js_header();
        gen.emit_signal_runtime();

        // Only embed the 46KB motion runtime if the project actually uses motion features
        if project_uses_motion(ir) {
            gen.emit_animation_runtime();
        }
    }

    // Emit all stores as global JS variables (signals, getters, actions)
    if !css_only {
        for (store_name, store) in ir.all_stores() {
            gen.emit_store(store_name, store);
        }
    }

    // Emit all module-level public functions (e.g., lib.omni utilities)
    if !css_only {
        for func in ir.all_functions() {
            gen.js.push_str(&format!("\n// pub fn {}\n", func.name));
            gen.emit_function(func);
        }
    }

    // Emit module-level motion definitions (@motion blocks defined outside any component).
    // These must be emitted before component-level motions because they're referenced by
    // component view triggers like @enter={motionName}.
    if !css_only {
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
    if gen.editor_mode && !css_only {
        gen.js.push_str(
            "window.__omni_signals = {};\nwindow.__omni_components = window.__omni_components || {};\n"
        );
    }

    // ── Collect design names before component generation ──
    // Must happen BEFORE generate_component() because emit_view() reads
    // self.design_names to decide whether to emit the data-design attribute.
    for module in ir.modules.values() {
        for design in &module.designs {
            gen.design_names.push(design.name.clone());
            gen.design_map.insert(design.name.clone(), design.clone());
        }
    }

    // Generate each component (signals, fns). Only root component emits its view.
    for (comp_name, comp) in ir.all_components() {
        gen.generate_component(comp_name, comp, ir, comp_name == root_name);
    }

    // Emit global __mergeTouches helper for TouchEvent support
    if !css_only {
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
    }

    // Build per-component output map for editor hot-replace
    let mut components: FxHashMap<String, ComponentOutput> = FxHashMap::default();
    for (name, js) in &gen.component_js {
        let css = gen.per_component_css.get(name).cloned().unwrap_or_default();
        components.insert(
            name.clone(),
            ComponentOutput {
                js: js.clone(),
                html: String::new(),
                css,
            },
        );
    }

    // ── Emit scoped design CSS (tokens, classes, variants) ──
    // Design names were collected before component generation so emit_view()
    // sees them when writing the data-design attribute on the root <div>.
    for module in ir.modules.values() {
        for design in &module.designs {
            if let Ok(css) = design::emit_design_css(design, &design.name) {
                gen.css
                    .push_str(&format!("\n/* ── Design: {} ── */\n", design.name));
                gen.css.push_str(&css);
            }
        }
    }

    if !css_only {
        gen.emit_dom_and_bindings();
        // Inject editor bridge code (postMessage, nid_map, highlight, hot-replace)
        gen.emit_editor_bridge();
        gen.finalize_js();
    }

    // Transfer accumulated element_tree from generator
    gen.element_tree = std::mem::take(&mut gen.element_tree);

    OmniViewOutput {
        js: gen.js,
        html: gen.html,
        css: gen.css,
        source_map: gen.source_map,
        components,
        warnings: gen.warnings,
        element_classes: gen.element_classes,
        element_tags: gen.element_tags,
        per_component_css: gen.per_component_css,
        element_tree: gen.element_tree,
    }
}

/// Returns true if any part of the IR program uses motion/animation features.
/// Guards the 46KB motion runtime JS from being embedded unconditionally.
fn project_uses_motion(ir: &IRProgram) -> bool {
    for module in ir.modules.values() {
        if !module.motions.is_empty() || !module.timelines.is_empty() {
            return true;
        }
        for comp in module.components.values() {
            if !comp.motions.is_empty() || !comp.timelines.is_empty() || !comp.hero_defs.is_empty()
            {
                return true;
            }
            if let Some(ref view) = comp.view {
                if view_tree_has_motion(view) {
                    return true;
                }
            }
        }
    }
    false
}

/// Recursively scan a view tree for any motion/animation-related fields.
fn view_tree_has_motion(view: &ViewTree) -> bool {
    match view {
        ViewTree::Element(el) => {
            if el.motion.is_some() || el.scroll.is_some() || el.drag.is_some() || el.hero.is_some()
            {
                return true;
            }
            el.children.iter().any(view_tree_has_motion)
        }
        ViewTree::Conditional(cond) => {
            if cond.transition_mode.is_some() {
                return true;
            }
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(view_tree_has_motion))
                || cond.fallback.as_ref().is_some_and(view_tree_has_motion)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(view_tree_has_motion)
                || list.empty_body.as_ref().is_some_and(view_tree_has_motion)
        }
        _ => false,
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
    signal_names: FxHashSet<String>,
    /// Store signal/getter names with prefix (e.g. "TodoStore_items") — need .value access
    store_signal_names: FxHashSet<String>,
    /// All store field names with prefix (e.g. "TodoStore_items", "TodoStore_add") — defined globally
    store_field_names: FxHashSet<String>,
    /// Store names (without prefix) for recognizing store references like "TodoStore"
    store_names: FxHashSet<String>,
    /// Map from store-local / component-local signal names to flat JS names.
    /// Cleared per component — only contains the CURRENT component's signals.
    name_renames: FxHashMap<String, String>,
    /// Global function/event renames — never cleared.
    /// All component functions/events are registered here so template calls
    /// like {getModStatus()} resolve to the correct prefixed name.
    global_renames: FxHashMap<String, String>,
    /// Cache of all component IRs for view inlining (keyed by component name)
    component_map: FxHashMap<String, ComponentIR>,
    /// Set of known motion names — used to detect @click={motionName} patterns
    motion_names: FxHashSet<String>,
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
    component_js: FxHashMap<String, String>,
    /// (nid, motion_name, trigger_kind) pairs for replay on signal change.
    /// trigger_kind is "enter" or "timeline" — used to build correct replay code.
    enter_trigger_nids: Vec<(String, String, String)>,
    /// @for template: maps child component state names -> (original_name, default_js)
    for_state_names: FxHashMap<String, (String, String)>,
    /// Design names collected during codegen, used for `data-design` attribute emission.
    design_names: Vec<String>,
    /// Compiled DesignIR map keyed by design name, used for variant expansion
    /// and design-class resolution at codegen time.
    design_map: FxHashMap<String, crate::ir::design::DesignIR>,
    /// Diagnostic warnings collected during codegen (e.g., missing animation params).
    warnings: Vec<String>,
    /// Pre-compiled text bindings for @for templates: (nid, js_expression)
    for_text_bindings: Vec<(String, String)>,
    /// Pre-compiled class bindings for @for templates: (nid, class_name, js_expression)
    for_class_bindings: Vec<(String, String, String)>,
    /// Pre-compiled attribute bindings for @for templates: (nid, attr_name, parts)
    for_attr_bindings: Vec<(String, String, Vec<AttrPart>)>,
    /// Optional name of the rebind function for this @for loop
    for_rebind_fn: Option<String>,
    /// When true, skip JS emission entirely — only collect CSS + element mappings.
    /// Used by resolve_styles() for the Poiesis editor canvas.
    css_only: bool,
    /// Per-element CSS class names collected during element_to_html(): nid → [class_names]
    element_classes: FxHashMap<String, Vec<String>>,
    /// Per-element HTML tag names collected during element_to_html(): nid → tag
    element_tags: FxHashMap<String, String>,
    /// Per-component CSS deltas accumulated during generate_component(): name → css
    per_component_css: FxHashMap<String, String>,
    /// Snapshot of self.css.len() at the start of a component, for delta tracking.
    current_css_start: usize,
    /// Component-local style class names for validating class: references.
    /// Populated during generate_component from the component's StyleRuleIR entries.
    component_style_classes: FxHashSet<String>,
    /// Stack of ElementTreeNode builders — each level collects children of the current parent.
    element_tree_stack: Vec<ElementTreeNode>,
    /// Root-level tree nodes accumulated during view_to_html.
    element_tree: Vec<ElementTreeNode>,
}

/// A parsed part of an attribute template value with {word} placeholders.
#[derive(Debug, Clone)]
pub(super) enum AttrPart {
    /// Literal text segment
    Static(String),
    /// Dynamic expression from {word} placeholder
    Dynamic(String),
}

impl OmniViewGenerator {
    /// Create a minimal generator instance for testing.
    #[cfg(test)]
    pub(super) fn test_new() -> Self {
        OmniViewGenerator {
            js: String::new(),
            html: String::new(),
            css: String::new(),
            source_map: Vec::new(),
            node_counter: 0,
            current_nid_path: Vec::new(),
            signal_names: FxHashSet::default(),
            store_signal_names: FxHashSet::default(),
            store_field_names: FxHashSet::default(),
            store_names: FxHashSet::default(),
            name_renames: FxHashMap::default(),
            global_renames: FxHashMap::default(),
            component_map: FxHashMap::default(),
            emitting_root_view: false,
            current_component: String::new(),
            in_for_template: false,
            deferred_js: String::new(),
            all_deferred_js: String::new(),
            dom_emitted: false,
            guard_counter: 0,
            mount_js: String::new(),
            editor_mode: false,
            comp_mount_js: String::new(),
            component_js: FxHashMap::default(),
            enter_trigger_nids: Vec::new(),
            for_state_names: FxHashMap::default(),
            motion_names: FxHashSet::default(),
            design_names: Vec::new(),
            design_map: FxHashMap::default(),
            warnings: Vec::new(),
            for_text_bindings: Vec::new(),
            for_class_bindings: Vec::new(),
            for_attr_bindings: Vec::new(),
            for_rebind_fn: None,
            css_only: false,
            element_classes: FxHashMap::default(),
            element_tags: FxHashMap::default(),
            per_component_css: FxHashMap::default(),
            current_css_start: 0,
            component_style_classes: FxHashSet::default(),
            element_tree_stack: Vec::new(),
            element_tree: Vec::new(),
        }
    }

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
mod tests;
