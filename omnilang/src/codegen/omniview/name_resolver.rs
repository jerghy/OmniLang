//! Name resolution state for the OmniView codegen.
//!
//! Groups all name-related registries (signal names, store names, component map,
//! renames) that were scattered across `OmniViewGenerator` and `ComponentScope`.
//! Provides a `take()`/`restore()` snapshot mechanism.

use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

use crate::ir::ComponentIR;

/// All name-related state for the OmniView codegen.
///
/// Holds signal, store, component, and motion name registries that are global
/// to the generator but logically grouped for maintainability. Provides a
/// snapshot mechanism for per-component save/restore (see `ComponentScope`).
#[derive(Default)]
pub(super) struct NameResolver {
    /// Per-component signal names (prefixed with component name, e.g. "App_count")
    pub signal_names: FxHashSet<String>,
    /// Store signal/getter names with prefix (e.g. "TodoStore_items") — need `.value` access
    pub store_signal_names: FxHashSet<String>,
    /// Per-component renames (unprefixed → prefixed name) for signals, props, events
    pub name_renames: FxHashMap<String, String>,
    /// Global function/event renames — never cleared.
    /// All component functions/events are registered here so template calls
    /// like `{getModStatus()}` resolve to the correct prefixed name.
    pub global_renames: FxHashMap<String, String>,
    /// Cache of all component IRs for view inlining (keyed by component name)
    pub component_map: FxHashMap<String, Arc<ComponentIR>>,
    /// All store field names with prefix (e.g. "TodoStore_items", "TodoStore_add") — defined globally
    pub store_field_names: FxHashSet<String>,
    /// Store names (without prefix) for recognizing store references like "TodoStore"
    pub store_names: FxHashSet<String>,
    /// Set of known motion names — used to detect `@click={motionName}` patterns
    pub motion_names: FxHashSet<String>,
    /// `@for` template: maps child component state names → `(original_name, default_js)`
    pub for_state_names: FxHashMap<String, (String, String)>,
    /// Prop names whose declared type is a function (`fn(String) -> String`).
    /// Call syntax on them (`{transform("hello")}`) must emit `X.value(args)`
    /// — the stored value IS the function, unlike computeds where the value is
    /// the computed result.
    pub fn_prop_names: FxHashSet<String>,
}

/// Snapshot of all `NameResolver` fields, returned by `NameResolver::take()`.
#[derive(Default)]
pub(super) struct NameResolverSnapshot {
    signal_names: FxHashSet<String>,
    store_signal_names: FxHashSet<String>,
    name_renames: FxHashMap<String, String>,
    global_renames: FxHashMap<String, String>,
    component_map: FxHashMap<String, Arc<ComponentIR>>,
    store_field_names: FxHashSet<String>,
    store_names: FxHashSet<String>,
    motion_names: FxHashSet<String>,
    for_state_names: FxHashMap<String, (String, String)>,
    fn_prop_names: FxHashSet<String>,
}

impl NameResolver {
    /// Take ownership of all name state, leaving default/empty values behind.
    pub(super) fn take(&mut self) -> NameResolverSnapshot {
        NameResolverSnapshot {
            signal_names: std::mem::take(&mut self.signal_names),
            store_signal_names: std::mem::take(&mut self.store_signal_names),
            name_renames: std::mem::take(&mut self.name_renames),
            global_renames: std::mem::take(&mut self.global_renames),
            component_map: std::mem::take(&mut self.component_map),
            store_field_names: std::mem::take(&mut self.store_field_names),
            store_names: std::mem::take(&mut self.store_names),
            motion_names: std::mem::take(&mut self.motion_names),
            for_state_names: std::mem::take(&mut self.for_state_names),
            fn_prop_names: std::mem::take(&mut self.fn_prop_names),
        }
    }

    /// Restore name state from a previously-taken snapshot.
    pub(super) fn restore(&mut self, snapshot: NameResolverSnapshot) {
        self.signal_names = snapshot.signal_names;
        self.store_signal_names = snapshot.store_signal_names;
        self.name_renames = snapshot.name_renames;
        self.global_renames = snapshot.global_renames;
        self.component_map = snapshot.component_map;
        self.store_field_names = snapshot.store_field_names;
        self.store_names = snapshot.store_names;
        self.motion_names = snapshot.motion_names;
        self.for_state_names = snapshot.for_state_names;
        self.fn_prop_names = snapshot.fn_prop_names;
    }
}
