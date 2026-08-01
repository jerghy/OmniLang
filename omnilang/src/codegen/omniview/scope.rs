//! Per-component codegen state — atomically saved/restored during component
//! inlining to prevent parent↔child state leakage.
//!
//! Any field that must be isolated per-component during `try_inline_component`
//! belongs here. Fields that are truly global (JS/CSS/HTML buffers, component_map,
//! design_map, warnings) stay on `OmniViewGenerator`.
//!
//! Design motivation (ADR-025 D1, ADR-026 D3): The earlier InliningState
//! pattern manually saved/restored 8 of 46 generator fields. Every new
//! per-component field added to OmniViewGenerator was a potential leak
//! vector. By placing ALL per-component state in this struct, the compiler
//! enforces completeness — adding a field here automatically includes it
//! in the save/restore cycle.

use super::AttrPart;
use crate::ir::EventDataKind;
use rustc_hash::{FxHashMap, FxHashSet};

/// All state scoped to a single component during codegen.
/// Saved before inlining a child component, restored after.
/// Note: `signal_names` and `name_renames` live on `NameResolver` but are
/// snapshotted here because they are per-component (cleared per inlining).
#[derive(Debug, Clone, Default)]
pub(super) struct ComponentScope {
    pub current_component: String,
    pub signal_names: FxHashSet<String>,
    pub name_renames: FxHashMap<String, String>,
    pub for_text_bindings: Vec<(String, String)>,
    pub for_class_bindings: Vec<(String, String, String)>,
    pub for_attr_bindings: Vec<(String, String, Vec<AttrPart>)>,
    pub for_rebind_fn: Option<String>,
    pub in_for_template: bool,
    pub emitting_root_view: bool,
    /// Event types (JS names) collected while generating the current @for
    /// template's data-event-* attributes — the delegated listener set for
    /// that @for. Populated by emit_element_events (which sees inlined child
    /// component events too, unlike AST-based collection), consumed by
    /// emit_list_dom. Only the outermost @for clears them (see list.rs).
    pub for_event_types: Vec<String>,
    /// (js_event, EventDataKind) pairs for the event-data lookup map.
    pub for_event_kinds: Vec<(String, EventDataKind)>,
    /// Label → union of used fields across handlers of that kind.
    pub used_fields_by_kind: FxHashMap<String, Option<Vec<String>>>,
    /// The current @for loop variable name (e.g. "item") while generating
    /// the loop template. When set, expression emission that is destined for
    /// top-level JS (event handler bodies, rebind fns) rewrites the variable
    /// to __omni_forItemVar, which the delegated listener populates at event
    /// time from the clicked clone's data-for-item index.
    pub for_item_var: Option<String>,
    /// Whether expressions emitted right now are destined for top-level JS
    /// (event handler bodies) where the @for loop variable is out of scope.
    pub rewrite_for_item: bool,
}

impl ComponentScope {
    /// Take ownership of per-component state from the generator,
    /// leaving default/empty values behind.
    pub(super) fn take(gen: &mut super::OmniViewGenerator) -> Self {
        ComponentScope {
            current_component: std::mem::take(&mut gen.scope.current_component),
            signal_names: std::mem::take(&mut gen.name_resolver.signal_names),
            name_renames: std::mem::take(&mut gen.name_resolver.name_renames),
            for_text_bindings: std::mem::take(&mut gen.scope.for_text_bindings),
            for_class_bindings: std::mem::take(&mut gen.scope.for_class_bindings),
            for_attr_bindings: std::mem::take(&mut gen.scope.for_attr_bindings),
            for_rebind_fn: gen.scope.for_rebind_fn.take(),
            in_for_template: gen.scope.in_for_template,
            emitting_root_view: gen.scope.emitting_root_view,
            for_event_types: std::mem::take(&mut gen.scope.for_event_types),
            for_event_kinds: std::mem::take(&mut gen.scope.for_event_kinds),
            used_fields_by_kind: std::mem::take(&mut gen.scope.used_fields_by_kind),
            for_item_var: gen.scope.for_item_var.take(),
            rewrite_for_item: gen.scope.rewrite_for_item,
        }
    }

    /// Restore per-component state back to the generator.
    pub(super) fn restore(self, gen: &mut super::OmniViewGenerator) {
        gen.scope.current_component = self.current_component;
        gen.name_resolver.signal_names = self.signal_names;
        gen.name_resolver.name_renames = self.name_renames;
        gen.scope.for_text_bindings = self.for_text_bindings;
        gen.scope.for_class_bindings = self.for_class_bindings;
        gen.scope.for_attr_bindings = self.for_attr_bindings;
        gen.scope.for_rebind_fn = self.for_rebind_fn;
        gen.scope.in_for_template = self.in_for_template;
        gen.scope.emitting_root_view = self.emitting_root_view;
        gen.scope.for_event_types = self.for_event_types;
        gen.scope.for_event_kinds = self.for_event_kinds;
        gen.scope.used_fields_by_kind = self.used_fields_by_kind;
        gen.scope.for_item_var = self.for_item_var;
        gen.scope.rewrite_for_item = self.rewrite_for_item;
    }

    /// Default scope — all fields in their zero/empty state.
    pub(super) fn new() -> Self {
        ComponentScope {
            current_component: String::new(),
            signal_names: FxHashSet::default(),
            name_renames: FxHashMap::default(),
            for_text_bindings: Vec::new(),
            for_class_bindings: Vec::new(),
            for_attr_bindings: Vec::new(),
            for_rebind_fn: None,
            in_for_template: false,
            emitting_root_view: false,
            for_event_types: Vec::new(),
            for_event_kinds: Vec::new(),
            used_fields_by_kind: FxHashMap::default(),
            for_item_var: None,
            rewrite_for_item: false,
        }
    }
}
