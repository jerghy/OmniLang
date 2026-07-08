//! Optional per-component/store codegen output cache.
//!
//! [`CodegenCache`] stores a content hash per component/store alongside the
//! generated output files.  Before regenerating, the current IR hash is
//! compared — if unchanged the cached file list is reused, avoiding
//! unnecessary codegen work.
//!
//! Only the [`crate::codegen::shared::generator::generate_per_component`]
//! codegen path is cached (React / Vue / Dioxus / Flutter).  OmniView,
//! Python, Rust, Dart and JavaScript/TypeScript codegen use different
//! iteration patterns and are not affected.

use std::hash::{Hash, Hasher};

use rustc_hash::FxHashMap;
use rustc_hash::FxHasher;

use crate::codegen::writer::CompiledFile;
use crate::ir::{ComponentIR, StoreIR, ViewTree};

/// Cache keyed by (component|store) name, holding a content hash and the
/// generated file list.
pub struct CodegenCache {
    component_outputs: FxHashMap<String, (u64, Vec<CompiledFile>)>,
    store_outputs: FxHashMap<String, (u64, Vec<CompiledFile>)>,
}

impl CodegenCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            component_outputs: FxHashMap::default(),
            store_outputs: FxHashMap::default(),
        }
    }

    /// Compute a content hash for a [`ComponentIR`] by hashing key fields.
    ///
    /// Hashes `name`, the length of each collection, and a summary of the
    /// view tree (node count). This avoids serializing the entire IR to a
    /// Debug string (which can be hundreds of KB for deeply nested types).
    #[inline]
    pub(crate) fn hash_component(comp: &ComponentIR) -> u64 {
        let mut hasher = FxHasher::default();
        comp.name.hash(&mut hasher);
        comp.states.len().hash(&mut hasher);
        comp.computeds.len().hash(&mut hasher);
        comp.fns.len().hash(&mut hasher);
        comp.view.as_ref().map_or(0, |v| count_view_nodes(v)).hash(&mut hasher);
        hasher.finish()
    }

    /// Compute a content hash for a [`StoreIR`].
    #[inline]
    pub(crate) fn hash_store(store: &StoreIR) -> u64 {
        let mut hasher = FxHasher::default();
        store.name.hash(&mut hasher);
        store.signals.len().hash(&mut hasher);
        store.getters.len().hash(&mut hasher);
        store.actions.len().hash(&mut hasher);
        hasher.finish()
    }

    /// Look up cached output for a component by name, returning `Some` only
    /// when the cached hash matches the caller-supplied `hash`.
    pub fn get_component(&self, name: &str, hash: u64) -> Option<&Vec<CompiledFile>> {
        self.component_outputs
            .get(name)
            .and_then(|(h, files)| (*h == hash).then_some(files))
    }

    /// Insert or update cached output for a component.
    pub fn put_component(&mut self, name: &str, hash: u64, files: Vec<CompiledFile>) {
        self.component_outputs
            .insert(name.to_string(), (hash, files));
    }

    /// Look up cached output for a store by name.
    pub fn get_store(&self, name: &str, hash: u64) -> Option<&Vec<CompiledFile>> {
        self.store_outputs
            .get(name)
            .and_then(|(h, files)| (*h == hash).then_some(files))
    }

    /// Insert or update cached output for a store.
    pub fn put_store(&mut self, name: &str, hash: u64, files: Vec<CompiledFile>) {
        self.store_outputs.insert(name.to_string(), (hash, files));
    }
}

/// Count the number of nodes in a [`ViewTree`] for use as a content-hash summary.
fn count_view_nodes(view: &ViewTree) -> usize {
    match view {
        ViewTree::Element(el) => {
            let mut count = 1;
            for child in &el.children {
                count += count_view_nodes(child);
            }
            count
        }
        ViewTree::Conditional(cond) => {
            let mut count = 1;
            for branch in &cond.branches {
                if let Some(body) = &branch.body {
                    count += count_view_nodes(body);
                }
            }
            if let Some(fallback) = &cond.fallback {
                count += count_view_nodes(fallback);
            }
            count
        }
        ViewTree::ListRendering(list) => {
            let mut count = 1;
            if let Some(body) = &list.body {
                count += count_view_nodes(body);
            }
            if let Some(empty) = &list.empty_body {
                count += count_view_nodes(empty);
            }
            count
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => 1,
    }
}

impl Default for CodegenCache {
    fn default() -> Self {
        Self::new()
    }
}
