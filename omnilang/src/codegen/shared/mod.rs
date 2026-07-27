//! Shared codegen infrastructure shared across multiple backend families.
//!
//! Contains the `FormatExpr`, `FormatStmt`, `FormatType` traits and their
//! language-family implementations.

use rustc_hash::FxHashMap;

use crate::ir::{ComponentIR, IRProgram};

pub mod expr;
pub(crate) mod html;
pub(crate) mod inline_handler;
pub(crate) mod stmt;
pub(crate) mod types;
pub mod ui_codegen;
pub(crate) mod visitor;

// ── Re-exported from omnilang-core (P-gen dedup) ──
// `emit_constructor_init` and `emit_store_constructor_init` are identical
// between the two crates — re-export directly.
pub(crate) use omnilang_core::codegen::shared::generator::{
    emit_constructor_init, emit_store_constructor_init,
};

/// Thin adapter: core's `build_component_map` returns `FxHashMap<&str, &ComponentIR>`
/// (borrowed references), but the facade codegen backends still expect owned
/// `FxHashMap<String, ComponentIR>`.  Full migration to borrowed refs is deferred
/// (P-gen — requires lifetime annotations across 4+ backends).
pub(crate) fn build_component_map(ir: &IRProgram) -> FxHashMap<String, ComponentIR> {
    omnilang_core::codegen::shared::generator::build_component_map(ir)
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}
