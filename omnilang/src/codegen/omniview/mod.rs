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
mod list;
pub mod typescript;

mod motion;
mod motion_config;
mod motion_triggers;
mod runtime;
mod stmt;
mod store;
mod timeline;
mod utils;
mod view;
pub use utils::{build_file_output, wrap_debug_html};
mod debug;

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
    /// Mapping from WASM nid (e.g. "el-div-001") to design element ID (UUID).
    /// Built during the same walk as element_classes — keyed by element id prop.
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    pub nid_map: FxHashMap<String, String>,
    /// TypeScript type declarations (.d.ts) for all components.
    /// Contains export interfaces for props, event handlers, and component types.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub types: String,
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
    /// Mapping from WASM nid (e.g. "el-div-001") to design element ID (UUID).
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    pub nid_map: FxHashMap<String, String>,
    /// Compilation warnings (unknown variant, missing design, etc.)
    pub warnings: Vec<String>,
    /// Parse / semantic errors.
    pub errors: Vec<Diagnostic>,
}

/// Codegen adapter — makes OmniView participate in the standard `Codegen` trait
/// system like every other backend, instead of using a special-cased `compile_files()`
/// path in `codegen::compile()`.
///
/// Carries OmniView-specific configuration (`editor_mode`, `css_only`, `debug_mode`)
/// that other backends don't need. `OmniViewOutput` remains available for WASM/FFI
/// paths that need the full structured metadata (element_classes, element_tree, etc.).
#[derive(Debug, Clone, Copy, Default)]
pub struct OmniViewCodegen {
    pub editor_mode: bool,
    pub css_only: bool,
    pub debug_mode: bool,
}

impl crate::codegen::Codegen for OmniViewCodegen {
    fn generate(
        &self,
        ir: &IRProgram,
        _source_files: &FxHashMap<String, String>,
    ) -> crate::codegen::CompileOutput {
        let output = generate(ir, self.editor_mode, self.css_only, self.debug_mode);
        let mut files = Vec::with_capacity(4);
        let html = utils::html_document_wrapper(&output.html, false, !self.editor_mode);
        files.push(crate::codegen::CompiledFile {
            path: "index.js".into(),
            content: output.js,
        });
        files.push(crate::codegen::CompiledFile {
            path: "index.html".into(),
            content: html,
        });
        files.push(crate::codegen::CompiledFile {
            path: "body.html".into(),
            content: output.html,
        });
        files.push(crate::codegen::CompiledFile {
            path: "index.css".into(),
            content: output.css,
        });
        crate::codegen::CompileOutput {
            files,
            errors: vec![],
            source_map: output.source_map,
            warnings: output.warnings,
        }
    }
}

/// Generate OmniView output from an IR program.
pub fn generate(
    ir: &IRProgram,
    editor_mode: bool,
    css_only: bool,
    debug_mode: bool,
) -> OmniViewOutput {
    debug_assert!(
        !debug_mode || editor_mode,
        "debug_mode requires editor_mode=true"
    );
    let mut gen = OmniViewGenerator {
        js: String::new(),
        html: String::new(),
        css: String::new(),
        source_map: Vec::new(),
        node_counter: 0,
        signal_names: FxHashSet::default(),
        signal_defaults: FxHashMap::default(),
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
        design_class_names: FxHashSet::default(),
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
        nid_map: FxHashMap::default(),
        debug_mode,
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
                    gen.motion_names.insert(timeline.name.clone());
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
    } else if all_comps.iter().any(|(n, _)| *n == "Main") {
        "Main"
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
    // self.design_names to decide whether to emit the data-design attribute,
    // and self.design_class_names to validate class: references.
    for module in ir.modules.values() {
        for design in &module.designs {
            gen.design_names.push(design.name.clone());
            gen.design_map.insert(design.name.clone(), design.clone());
            for class in &design.classes {
                gen.design_class_names.insert(class.name.clone());
            }
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
    // In editor mode, use unscoped selectors — the canvas has no data-design attr.
    for module in ir.modules.values() {
        for design in &module.designs {
            let css_result = if editor_mode && css_only {
                design::emit_design_css_editor(design, &design.name)
            } else {
                design::emit_design_css(design, &design.name)
            };
            if let Ok(css) = css_result {
                gen.css
                    .push_str(&format!("\n/* ── Design: {} ── */\n", design.name));
                gen.css.push_str(&css);
            }
        }
    }

    if !css_only {
        // Emit theme runtime JS if any design block declares @themes
        gen.emit_theme_runtime();
        gen.emit_dom_and_bindings();
        // Inject editor bridge code (postMessage, nid_map, highlight, hot-replace)
        gen.emit_editor_bridge();
        // Inject debug bridge code (Tauri event communication for standalone debug window)
        gen.emit_debug_bridge();
        gen.js.push_str("\n})();\n");
    }

    // Transfer accumulated element_tree from generator
    gen.element_tree = std::mem::take(&mut gen.element_tree);

    let all_comp_vec: Vec<crate::ir::ComponentIR> =
        ir.all_components().into_values().cloned().collect();
    let types = typescript::generate_typescript_declarations(&all_comp_vec);

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
        nid_map: gen.nid_map,
        types,
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
            if !comp.hero_defs.is_empty() {
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
    omnilang_core::codegen::shared::view_tree::view_tree_any(view, |el| {
        el.motion.is_some() || el.scroll.is_some() || el.drag.is_some() || el.hero.is_some()
    })
}

struct OmniViewGenerator {
    js: String,
    html: String,
    css: String,
    source_map: Vec<SourceMapEntry>,
    node_counter: u32,
    /// Set of signal names in the current component (for .value / .set() generation)
    signal_names: FxHashSet<String>,
    /// Default values for signals (name → JS value string), for editor canvas preview
    signal_defaults: FxHashMap<String, String>,
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
    /// (nid, motion_name, trigger_kind, component_name) pairs for replay on signal change.
    /// trigger_kind is "enter" or "timeline" — used to build correct replay code.
    /// component_name identifies which component the trigger belongs to for per-component effects.
    enter_trigger_nids: Vec<(String, String, String, String)>,
    /// @for template: maps child component state names -> (original_name, default_js)
    for_state_names: FxHashMap<String, (String, String)>,
    /// Design names collected during codegen, used for `data-design` attribute emission.
    design_names: Vec<String>,
    /// Pre-built HashSet of all design class names for O(1) lookup during element codegen.
    /// Populated during design CSS emission to avoid repeated iteration over design_map.
    design_class_names: FxHashSet<String>,
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
    /// Mapping from WASM nid to the element's design ID (UUID from id prop).
    nid_map: FxHashMap<String, String>,
    /// When true, emit debug bridge code for the standalone debug window.
    /// Uses Tauri events instead of parent.postMessage.
    debug_mode: bool,
}

/// A parsed part of an attribute template value with {word} placeholders.
#[derive(Debug, Clone)]
pub(super) enum AttrPart {
    /// Literal text segment
    Static(String),
    /// Dynamic expression from {word} placeholder
    Dynamic(String),
}

/// Saved generator state for component inlining — ensures isolation
/// between parent and child component contexts during view inlining.
/// Prevents bugs when new fields are added to OmniViewGenerator but
/// forgotten in the save/restore block (see ADR-025 D1, ADR-026 D3).
struct InliningState {
    current_component: String,
    signal_names: FxHashSet<String>,
    name_renames: FxHashMap<String, String>,
    for_text_bindings: Vec<(String, String)>,
    for_class_bindings: Vec<(String, String, String)>,
    for_attr_bindings: Vec<(String, String, Vec<AttrPart>)>,
    for_rebind_fn: Option<String>,
    in_for_template: bool,
}

impl InliningState {
    fn save(gen: &OmniViewGenerator) -> Self {
        Self {
            current_component: gen.current_component.clone(),
            signal_names: gen.signal_names.iter().cloned().collect(),
            name_renames: gen
                .name_renames
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            for_text_bindings: gen.for_text_bindings.clone(),
            for_class_bindings: gen.for_class_bindings.clone(),
            for_attr_bindings: gen.for_attr_bindings.clone(),
            for_rebind_fn: gen.for_rebind_fn.clone(),
            in_for_template: gen.in_for_template,
        }
    }

    fn restore(self, gen: &mut OmniViewGenerator) {
        gen.current_component = self.current_component;
        gen.signal_names = self.signal_names;
        gen.name_renames = self.name_renames;
        gen.for_text_bindings = self.for_text_bindings;
        gen.for_class_bindings = self.for_class_bindings;
        gen.for_attr_bindings = self.for_attr_bindings;
        gen.for_rebind_fn = self.for_rebind_fn;
        gen.in_for_template = self.in_for_template;
    }
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
            signal_names: FxHashSet::default(),
            signal_defaults: FxHashMap::default(),
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
            design_class_names: FxHashSet::default(),
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
            nid_map: FxHashMap::default(),
            debug_mode: false,
        }
    }

    /// Assert that JS emission is allowed (not after DOM emission).
    /// Only active in debug builds — zero cost in release.
    /// NOTE: does NOT check css_only — view-generation methods (try_inline_component,
    /// emit_element_events, etc.) are legitimately called in css_only mode for
    /// HTML/element-tree generation. Use assert_not_css_only() on pure-JS entry points.
    #[track_caller]
    fn assert_can_emit_js(&self) {
        debug_assert!(
            !self.dom_emitted,
            "JS emission after DOM phase — use mount_js instead"
        );
    }

    /// Assert NOT in css_only mode. Use on pure-JS methods (emit_store, emit_js_header,
    /// emit_signal_runtime) that should never run during css_only style resolution.
    #[track_caller]
    fn assert_not_css_only(&self) {
        debug_assert!(!self.css_only, "JS emission in css_only mode");
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

    /// Emit the `__omni_theme` JS runtime for design blocks that declare `@themes`.
    ///
    /// Generates a runtime object with `set()`, `toggle()`, `init()` methods,
    /// `localStorage` persistence, and `prefers-color-scheme` auto-detection.
    /// Scans `self.design_map` for theme configurations and skips if no themes
    /// are declared in any design block.
    pub(super) fn emit_theme_runtime(&mut self) {
        // Collect all unique theme names across all design blocks
        let mut all_themes: Vec<String> = Vec::new();
        let mut has_dark = false;
        for design in self.design_map.values() {
            for theme in &design.themes {
                if !all_themes.contains(theme) {
                    all_themes.push(theme.clone());
                }
                if theme == "dark" {
                    has_dark = true;
                }
            }
        }

        if all_themes.is_empty() {
            return;
        }

        let themes_json: Vec<String> = all_themes.iter().map(|t| format!("\"{}\"", t)).collect();
        let themes_array = themes_json.join(", ");

        let media_init = if has_dark {
            r#"    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    if (prefersDark && this._themes.includes("dark")) {
      this.set("dark");
      return;
    }
"#
            .to_string()
        } else {
            String::new()
        };

        self.js.push_str(&format!(
            r#"// ═══ Theme Runtime (generated from design @themes) ═══
const __omni_theme = {{
  _themes: [{}],
  _current: null,
  _storageKey: "omni-theme",

  get current() {{ return this._current; }},

  set(name) {{
    if (!this._themes.includes(name)) return;
    document.documentElement.dataset.theme = name;
    this._current = name;
    try {{ localStorage.setItem(this._storageKey, name); }} catch (_) {{}}
  }},

  toggle() {{
    const i = this._themes.indexOf(this._current);
    const next = this._themes[(i + 1) % this._themes.length];
    this.set(next);
  }},

  init() {{
    const saved = (() => {{ try {{ return localStorage.getItem(this._storageKey); }} catch (_) {{ return null; }} }})();
    if (saved && this._themes.includes(saved)) {{
      this.set(saved);
      return;
    }}
{}
    // Default: first theme
    if (!this._current) {{
      this.set(this._themes[0]);
    }}
  }}
}};
__omni_theme.init();
"#,
            themes_array, media_init
        ));
    }
}

// ══════════════════════════════════════════════════════════
// Value Serialization
// ══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_regression;
