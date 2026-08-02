//! Shared codegen infrastructure shared across multiple backend families.
//!
//! Contains the `FormatExpr`, `FormatStmt`, `FormatType` traits and their
//! language-family implementations.

pub mod design;
pub mod drag;
pub mod event;
pub mod sanitize;
pub mod style;

use rustc_hash::FxHashMap;

use crate::ir::{ComponentIR, IRProgram};

pub(crate) use omnilang_core::codegen::shared::expr;
pub(crate) use omnilang_core::codegen::shared::html;
pub(crate) use omnilang_core::codegen::shared::inline_handler;
pub(crate) use omnilang_core::codegen::shared::stmt;
pub(crate) use omnilang_core::codegen::shared::types;
pub(crate) use omnilang_core::codegen::shared::visitor;

// ── Re-exported from omnilang-core (P-gen dedup) ──
// `emit_constructor_init` and `emit_store_constructor_init` are identical
// between the two crates — re-export directly.
pub(crate) use omnilang_core::codegen::shared::generator::{
    emit_constructor_init, emit_store_constructor_init,
};

/// Adapter: delegates directly to core's `build_component_map`, returning
/// `FxHashMap<&str, &ComponentIR>` (borrowed references tied to the IR lifetime).
pub(crate) fn build_component_map(ir: &IRProgram) -> FxHashMap<&str, &ComponentIR> {
    omnilang_core::codegen::shared::generator::build_component_map(ir)
}
