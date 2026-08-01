//! Standard library intrinsic registry.
//!
//! Maps `import { fn } from "std/<module>"` to per-target native implementations.
//! Functions not available on a target produce compile-time errors.

use std::sync::OnceLock;

use crate::codegen::Target;

mod array;
mod async_mod;
mod console;
mod datetime;
mod encode;
mod http;
mod json;
mod math;
mod regex;
mod storage;
mod string;

#[cfg(test)]
mod tests;

/// A registered standard library function.
#[derive(Debug, Clone)]
pub struct StdlibFn {
    pub module: &'static str,
    pub name: &'static str,
    pub available_targets: &'static [Target],
    /// Emit target-native code for this function call.
    /// Receives the formatted argument strings and target.
    pub codegen: fn(args: &[String], target: Target) -> String,
    /// Whether the call must be wrapped in `await`.
    pub is_async: bool,
}

pub fn registry() -> &'static [StdlibFn] {
    static REGISTRY: OnceLock<Vec<StdlibFn>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut r = Vec::new();
        r.extend(http::register());
        r.extend(json::register());
        r.extend(storage::register());
        r.extend(math::register());
        r.extend(string::register());
        r.extend(array::register());
        r.extend(console::register());
        r.extend(async_mod::register());
        r.extend(datetime::register());
        r.extend(encode::register());
        r.extend(regex::register());
        r
    })
}

/// Look up a stdlib function by module and name.
#[must_use]
pub fn lookup(module: &str, name: &str) -> Option<&'static StdlibFn> {
    registry()
        .iter()
        .find(|f| f.module == module && f.name == name)
}

/// Check if a stdlib module is available on a target.
#[must_use]
pub fn module_available(module: &str, target: Target) -> bool {
    registry()
        .iter()
        .filter(|f| f.module == module)
        .all(|f| f.available_targets.contains(&target))
}

/// List available targets for a module (for error messages).
#[must_use]
pub fn available_targets_for(module: &str) -> Vec<Target> {
    let mut targets: Vec<Target> = registry()
        .iter()
        .filter(|f| f.module == module)
        .flat_map(|f| f.available_targets.iter())
        .copied()
        .collect();
    targets.sort_by_key(|t| format!("{:?}", t));
    targets.dedup();
    targets
}

pub(super) const ALL_TARGETS: [Target; 9] = [
    Target::JavaScript,
    Target::React,
    Target::Vue,
    Target::OmniView,
    Target::Python,
    Target::Rust,
    Target::Dart,
    Target::Flutter,
    Target::Dioxus,
];

pub(super) const STORAGE_TARGETS: [Target; 5] = [
    Target::JavaScript,
    Target::React,
    Target::Vue,
    Target::OmniView,
    Target::Flutter,
];
