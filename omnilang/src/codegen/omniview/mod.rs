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
mod name_resolver;
mod scope;
#[cfg(feature = "codegen-dts")]
pub mod typescript;

mod motion;
mod motion_config;
mod motion_triggers;
mod router;
mod runtime;
mod stmt;
mod store;
pub(super) mod utils;
mod view;
pub use utils::{build_file_output, wrap_debug_html};
mod debug;

use crate::error::Diagnostic;
use crate::ir::*;
use crate::sourcemap::SourceMapEntry;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use scope::ComponentScope;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;
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
    /// Reverse source map — maps generated positions back to source positions.
    /// Built after the forward source map is constructed during codegen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse_source_map: Option<crate::sourcemap::ReverseSourceMap>,
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
/// `html` and `css` were removed as reserved fields (unused).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOutput {
    pub js: String,
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
            // OmniView warnings carry no source spans (element context is not
            // threaded on this backend) — honest -:0:0 entries.
            warnings: output
                .warnings
                .into_iter()
                .map(crate::codegen::CodegenWarning::new)
                .collect(),
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
        scope: ComponentScope::new(),
        name_resolver: name_resolver::NameResolver::default(),
        numeric_signal_names: collect_numeric_signal_names(ir),
        signal_defaults: FxHashMap::default(),
        deferred_js: String::new(),
        all_deferred_js: String::new(),
        dom_emitted: false,
        guard_counter: 0,
        mount_js: String::new(),
        editor_mode,
        comp_mount_js: String::new(),
        component_js: FxHashMap::default(),
        enter_trigger_nids: Vec::new(),
        motion_bodies: FxHashMap::default(),
        design_names: Vec::new(),
        design_class_names: FxHashSet::default(),
        design_map: FxHashMap::default(),
        warnings: Vec::new(),
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
        scroll_handler_emitted: false,
        ref_nid_map: Vec::new(),
        ref_nid_set: FxHashSet::default(),
        pending_modal_title_id: false,
        shimmer_css_emitted: false,
        modal_css_emitted: false,
        toast_runtime_emitted: false,
        router_active: false,
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

    // Emit all stores as global JS variables (signals, getters, actions).
    // Phase 5.2: inline stores (defined in a component body) are registered
    // as module StoreIRs and emit here like any top-level store — the
    // component path (store.rs) resolves their references to this store
    // level, keeping reads and writes consistent (the flattened component
    // copies stay emitted but unused).
    if !css_only {
        for (store_name, store) in ir.all_stores() {
            gen.emit_store(store_name, store);
        }
    }

    // Emit all module-level public functions (e.g., lib.omni utilities)
    if !css_only {
        for func in ir.all_functions() {
            let _ = writeln!(gen.js, "\n// pub fn {}", func.name);
            gen.emit_function(func);
        }
    }

    // Emit module-level motion definitions (@motion blocks defined outside any component).
    // These must be emitted before component-level motions because they're referenced by
    // component view triggers like @enter={motionName}.
    if !css_only {
        for module in ir.modules.values() {
            if !module.motions.is_empty() {
                let _ = writeln!(
                    gen.js,
                    "\n/// ── Module Motion Definitions: {} ──\n",
                    module.name
                );
                for motion in &module.motions {
                    gen.name_resolver.motion_names.insert(motion.name.clone());
                    gen.motion_bodies
                        .insert(motion.name.clone(), motion.body.clone());
                    gen.emit_motion_js(motion);
                }
            }
        }
    }

    // Build component map for view inlining
    for (comp_name, comp) in ir.all_components() {
        gen.name_resolver
            .component_map
            .insert(comp_name.to_string(), Arc::new(comp.clone()));
        // Pre-register ALL component function & event renames globally.
        // These never get cleared so template interpolations like {getModStatus()}
        // resolve correctly even when the component is inlined as a child.
        for f in &comp.fns {
            let prefixed = format!("{}_{}", comp_name, f.name);
            gen.name_resolver
                .global_renames
                .insert(f.name.clone(), prefixed);
        }
        for event in &comp.events {
            let prefixed = format!("{}_{}", comp_name, event.name);
            gen.name_resolver
                .global_renames
                .insert(event.name.clone(), prefixed);
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

    // Task 8 (batch 2): hash router gate — any @route/@layout intent turns on
    // the router emission (zero-regression: no intent → no router code at all).
    gen.router_active = !editor_mode && !css_only && router::has_routing(ir);

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
        // SAFETY: element_tree_stack must be empty before each component.
        // A non-empty stack indicates a bug in the previous component's
        // element_to_html tree_push/tree_pop pairing.
        debug_assert!(
            gen.element_tree_stack.is_empty(),
            "element_tree_stack not empty before component '{comp_name}' \
             — previous component has unbalanced tree_push/tree_pop"
        );

        gen.generate_component(comp_name, comp, ir, comp_name == root_name);
    }

    // Emit global __mergeTouches helper for TouchEvent support (only if needed)
    if !css_only && project_uses_touch_events(ir) {
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
        components.insert(name.clone(), ComponentOutput { js: js.clone() });
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
                let _ = writeln!(gen.css, "\n/* ── Design: {} ── */", design.name);
                gen.css.push_str(&css);
            }
        }
    }

    if !css_only {
        // Emit theme runtime JS if any design block declares @themes
        gen.emit_theme_runtime();

        // ── Sync the @theme-marked store signal to __omni_theme ──
        // When a design block declares @themes, __omni_theme manages
        // data-theme on <html>. The store signal marked `@theme` (v2 域 5,
        // design D1 — DesignIR.theme_signal, "StoreName.signalName") is THE
        // current theme mode and syncs bidirectionally:
        //   1. Init: if __omni_theme.current (from localStorage/prefers-color-scheme) differs
        //      from the store default, override the store to respect the saved/preferred theme.
        //   2. Reactive: any @theme signal change propagates to __omni_theme, updating data-theme.
        // Recognition is by IR only — the former `mode` magic-name matching is gone.
        let theme_signal: Option<String> =
            gen.design_map.values().find_map(|d| d.theme_signal.clone());
        if let Some((store_name, signal_name)) =
            theme_signal.as_deref().and_then(|p| p.split_once('.'))
        {
            if ir
                .all_stores()
                .get(store_name)
                .is_some_and(|s| s.signals.contains_key(signal_name))
            {
                let var_name = format!("{}_{}", store_name, signal_name);
                let _ = writeln!(
                    gen.mount_js,
                    "\
        // ── Theme mode sync: {var_name} ↔ __omni_theme (declared @theme signal) ──
        (function() {{
          // 1. Init: respect saved/preferred theme from __omni_theme.init()
          if ({var_name}.value !== __omni_theme.current) {{
            __omni_subs.get(\"{var_name}\").set(__omni_theme.current);
          }}
          // 2. Reactive: store {signal_name} changes propagate to __omni_theme
          __omni_effect(function () {{ __omni_theme.set({var_name}.value); }});
        }})();\n"
                );
            }
        }

        // ── Phase 2: Dynamic token effects ──
        // Generate __omni_effect blocks for tokens with runtime expression values.
        // These set CSS custom properties on document.documentElement reactively.
        // Pre-collect to avoid borrow checker conflict between design_map (immutable)
        // and expr_to_js (mutable borrow of generator).
        let dynamic_tokens: Vec<(String, ExprIR)> = gen
            .design_map
            .values()
            .flat_map(|d| d.tokens.iter())
            .filter_map(|(name, token)| match &token.value {
                TokenValueIR::Dynamic(expr) => Some((name.clone(), expr.clone())),
                _ => None,
            })
            .collect();
        if !dynamic_tokens.is_empty() {
            // Separate steps to satisfy the borrow checker: the closure captures
            // &mut gen for expr_to_js, so the call must finish before gen.js is accessed.
            let effects_js =
                design::emit_dynamic_token_effects(&dynamic_tokens, |expr| gen.expr_to_js(expr));
            gen.js.push_str(&effects_js);
        }

        gen.emit_dom_and_bindings();
        // Inject editor bridge code (postMessage, nid_map, highlight, hot-replace)
        gen.emit_editor_bridge();
        // Inject debug bridge code (Tauri event communication for standalone debug window)
        gen.emit_debug_bridge();
        // Task 8: hash router (route table + runtime + listener) — shares the
        // IIFE scope with __omni_subs so param/query injection can write the
        // writable route signals emitted by store.rs.
        if gen.router_active {
            router::emit_router_js(&mut gen.js, ir, root_name, &mut gen.warnings);
        }
        gen.js.push_str("\n})();\n");
    }

    // Transfer accumulated element_tree from generator
    gen.element_tree = std::mem::take(&mut gen.element_tree);

    // ADR-031 D6: Skip TypeScript declaration generation — zero production consumers.
    // The typescript.rs module (298 lines) is preserved for future use when a
    // consumer materializes (e.g., Poiesis IDE intellisense, npm package publishing).
    // let all_comp_vec: Vec<crate::ir::ComponentIR> =
    //     ir.all_components().into_values().cloned().collect();
    // let types = typescript::generate_typescript_declarations(&all_comp_vec);
    let types = String::new();

    // Build reverse source map from the forward entries
    let reverse_source_map = (!gen.source_map.is_empty())
        .then(|| crate::sourcemap::build_reverse_source_map(&gen.source_map));

    OmniViewOutput {
        js: gen.js,
        html: gen.html,
        css: gen.css,
        source_map: gen.source_map,
        reverse_source_map,
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

/// Returns true if any component in the IR program uses touch or pointer events.
/// Guards the __mergeTouches helper from being emitted unconditionally.
/// Checks both component-level declared events and element-level DOM events
/// in the view tree (where __mergeTouches is actually referenced via TouchEvent data extraction).
/// Collect all numeric (Int/Float) signal full names across components and
/// stores — `{Comp}_{sig}` / `{Store}_{field}`. Used by bind:value writeback
/// to emit `parseFloat` for numeric signals (DOM values are strings).
fn collect_numeric_signal_names(ir: &IRProgram) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for (comp_name, comp) in ir.all_components() {
        for sig in &comp.signals {
            if matches!(sig.r#type.kind, TypeKind::Int | TypeKind::Float) {
                names.insert(format!("{}_{}", comp_name, sig.name));
            }
        }
    }
    for (store_name, store) in ir.all_stores() {
        for (field, ty) in &store.signals {
            if matches!(ty.kind, TypeKind::Int | TypeKind::Float) {
                names.insert(format!("{}_{}", store_name, field));
            }
        }
    }
    names
}

fn project_uses_touch_events(ir: &IRProgram) -> bool {
    ir.modules.values().any(|m| {
        m.components.values().any(|c| {
            // Check component-level declared events (e.g., @onTouchStart in component signature)
            c.events.iter().any(|ev| {
                ev.name.contains("touch") || ev.name.contains("pointer")
            })
            // Check element-level DOM events in the view tree
            || c.view.as_ref().is_some_and(|view| {
                omnilang_core::codegen::shared::view_tree::view_tree_any(view, |el| {
                    el.events.keys().any(|name| {
                        name.contains("touch") || name.contains("pointer")
                    })
                })
            })
        })
    })
}

/// Returns true if any part of the IR program uses motion/animation features.
/// Guards the 46KB motion runtime JS from being embedded unconditionally.
fn project_uses_motion(ir: &IRProgram) -> bool {
    for module in ir.modules.values() {
        if !module.motions.is_empty() {
            return true;
        }
        for comp in module.components.values() {
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
/// (@scroll / @drag-* control dimensions live on TriggerKind (Task 5.1), so
/// `motion` alone covers them.)
fn view_tree_has_motion(view: &ViewTree) -> bool {
    omnilang_core::codegen::shared::view_tree::view_tree_any(view, |el| el.motion.is_some())
}

#[derive(Default)]
struct OmniViewGenerator {
    js: String,
    html: String,
    css: String,
    source_map: Vec<SourceMapEntry>,
    node_counter: u32,
    /// Per-component codegen state — atomically swapped during inlining.
    /// See `ComponentScope::take()` / `ComponentScope::restore()`.
    pub(super) scope: ComponentScope,
    /// Default values for signals (name → JS value string), for editor canvas preview
    signal_defaults: FxHashMap<String, String>,
    /// Name resolution state (signal names, store names, renames, component map, etc.)
    pub(super) name_resolver: name_resolver::NameResolver,
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
    /// 数值类型信号全名集合（`{Comp}_{sig}` / `{Store}_{field}`，Int/Float）。
    /// bind:value 写回时对数值信号 parseFloat——e.target.value 是字符串，
    /// f64 信号被字符串污染后 style 绑定（__omni_px typeof 判断）无法附加
    /// px，滑块拖不动尺寸。
    numeric_signal_names: FxHashSet<String>,
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
    /// Motion name → body, populated while emitting module-level @motion
    /// definitions. Used by @drag-* triggers to derive the drag extent from
    /// the motion's own keyframe range when no constraints are given
    /// (ADR-053 removed @drag-constraints; deferral #19).
    pub(super) motion_bodies: FxHashMap<String, KeyframeIR>,
    /// Whether the shared scroll handler has been emitted.
    /// Used to emit __omni_scrollElements / __omni_scrollHandler / __omni_ensureScrollHandler
    /// exactly once before any per-element scroll trigger.
    scroll_handler_emitted: bool,
    /// Accumulates (ref_name, nid) pairs during element_to_html.
    /// Cleared per component. Consumed by emit_ref_wrappers.
    pub ref_nid_map: Vec<(String, String)>,
    /// Duplicate-detection set for ref names — only checked in debug builds.
    /// The semantic validator catches duplicates before codegen; this is defense-in-depth.
    pub ref_nid_set: FxHashSet<String>,
    /// When set, the next Heading-class element emitted gets the compiler
    /// generated modal title id (`id="omni-modal-title"`) and the flag is
    /// cleared. Set while emitting a @modal shell whose children contain a
    /// heading (the dialog's `aria-labelledby` target), spec 2026-08-09 §4.1.
    pending_modal_title_id: bool,
    /// Whether the @skeleton shimmer keyframes have been emitted into the
    /// CSS buffer — emitted exactly once per document (spec 2026-08-09 §4.3).
    shimmer_css_emitted: bool,
    /// Whether the @modal shell CSS has been emitted into the CSS buffer —
    /// emitted exactly once per document (spec 2026-08-09 §4.1 自绘遮罩).
    modal_css_emitted: bool,
    /// Whether the global showToast runtime (`.omni-toast` container +
    /// `__omni_toast` queue) has been emitted — exactly once per document
    /// (spec 2026-08-09 §4.2 全局自绘容器).
    toast_runtime_emitted: bool,
    /// Whether the hash router emission is active: the program carries
    /// @route/@layout intent AND we are in full JS mode (not editor/css-only).
    /// Gates route-container emission (store.rs), root-container route attr
    /// (view/mod.rs) and the router JS injection (generate()).
    router_active: bool,
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
    // test_new() removed — use OmniViewGenerator::default() instead

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

    /// Push a (ref_name, nid) pair into `ref_nid_map`, guarding against duplicates.
    ///
    /// Panics (debug_assert) on duplicates — the semantic validator
    /// should catch duplicates before codegen, but this is defense-in-depth.
    #[track_caller]
    pub(super) fn push_ref_nid(&mut self, ref_name: String, nid: String) {
        debug_assert!(
            !self.ref_nid_set.contains(&ref_name),
            "duplicate ref binding '{}' — semantic validator should have caught this",
            ref_name
        );
        self.ref_nid_set.insert(ref_name.clone());
        self.ref_nid_map.push((ref_name, nid));
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

        let _ = write!(
            self.js,
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
        );
    }
}

// ══════════════════════════════════════════════════════════
// Value Serialization
// ══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_regression;
