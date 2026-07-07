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

use crate::codegen::writer::CompiledFile;
use crate::ir::{ComponentIR, StoreIR};

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

    /// Compute a content hash for a [`ComponentIR`] via its Debug output.
    ///
    /// Uses `format!("{:?}", …)` so that every field that participates in the
    /// codegen output is reflected in the hash.  This is safe (deterministic)
    /// because all IR types use derived [`Debug`].
    pub(crate) fn hash_component(comp: &ComponentIR) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{:?}", comp).hash(&mut hasher);
        hasher.finish()
    }

    /// Compute a content hash for a [`StoreIR`].
    pub(crate) fn hash_store(store: &StoreIR) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{:?}", store).hash(&mut hasher);
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

impl Default for CodegenCache {
    fn default() -> Self {
        Self::new()
    }
}
