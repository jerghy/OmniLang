//! Shared codegen infrastructure shared across multiple backend families.
//!
//! Contains the `FormatExpr`, `FormatStmt`, `FormatType` traits and their
//! language-family implementations.

use rustc_hash::FxHashMap;

use crate::ir::{ComponentIR, IRProgram};

pub(crate) use omnilang_core::codegen::shared::expr;
pub(crate) use omnilang_core::codegen::shared::html;
pub(crate) use omnilang_core::codegen::shared::inline_handler;
pub(crate) use omnilang_core::codegen::shared::stmt;
pub(crate) use omnilang_core::codegen::shared::types;
pub(crate) use omnilang_core::codegen::shared::visitor;
pub use omnilang_core::codegen::shared::ui_codegen;

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
