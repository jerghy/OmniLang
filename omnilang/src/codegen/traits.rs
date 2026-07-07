//! Codegen trait -- implement for each target.
//!
//! This module contains both the original `Codegen` trait (kept for backwards
//! compatibility) and the new unified `UnifiedCodegen` trait that subsumes
//! both the old `Codegen` and `PerComponentCodegen`.
//!
//! # Migration Path
//!
//! 1. Phase 1 (this): Add `FormatBundle` + `UnifiedCodegen` alongside old trait.
//! 2. Phase 2: Migrate Camp A backends (Python, JS/TS, Rust, Dart).
//! 3. Phase 3: Migrate Camp B backends (React, Vue, Dioxus, Flutter).
//! 4. Phase 4: Remove old `Codegen` trait and `shared/generator.rs`.

use rustc_hash::FxHashMap;
use crate::ir::IRProgram;

use super::CompileOutput;

// ═══════════════════════════════════════════════════════════
// Original Codegen trait (backwards compatible — DO NOT REMOVE)
// ═══════════════════════════════════════════════════════════

/// Codegen trait — implement for each target.
///
/// **Legacy trait.** New backends should implement [`UnifiedCodegen`] instead.
/// Will be removed in Phase 4 once all backends are migrated.
pub trait Codegen: Send + Sync {
    /// Generate output files for the given IR program and source files.
    #[must_use]
    fn generate(&self, ir: &IRProgram, source_files: &FxHashMap<String, String>) -> CompileOutput;
}

// ═══════════════════════════════════════════════════════════
// New unified Codegen trait (extends FormatBundle)
// ═══════════════════════════════════════════════════════════

#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::codegen::shared::bundle::FormatBundle;
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::codegen::writer::{CodeWriter, CompiledFile};
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::ir::*;

/// Unified codegen trait — subsumes both old `Codegen` and `PerComponentCodegen`.
///
/// Provides default per-module and per-component generation strategies.
/// Backends override only the methods they need.
///
/// # Built-in Strategies
///
/// - [`generate_per_module`](UnifiedCodegen::generate_per_module): Iterates all
///   modules and emits code for each one (Camp A style). Override per-item
///   methods (`generate_function`, `generate_component`, etc.) to customize.
///
/// - [`generate_per_component`](UnifiedCodegen::generate_per_component): Iterates
///   all components and stores, delegating to `gen_component_file` /
///   `gen_store_file` (Camp B style).
///
/// Backends can call the default strategy from their `generate()` implementation
/// or provide completely custom logic.
///
/// # Associated Types
///
/// The associated types (`Expr`, `Stmt`, `Type`) come from the [`FormatBundle`]
/// supertrait and provide language-specific formatting for expressions,
/// statements, and types without needing separate formatter lookups.
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
pub(crate) trait UnifiedCodegen: FormatBundle {
    /// Generate output for the entire IR program.
    ///
    /// Backends should call one of the built-in strategies
    /// ([`generate_per_module`](UnifiedCodegen::generate_per_module) or
    /// [`generate_per_component`](UnifiedCodegen::generate_per_component)) or
    /// provide completely custom logic.
    #[must_use]
    fn generate(&self, ir: &IRProgram) -> CompileOutput;

    // ── Per-item generators (defaults do nothing) ────────────

    /// Generate module-level header (imports, declarations, etc.).
    fn generate_module_header(&self, _w: &mut CodeWriter, _module: &ModuleIR) {}

    /// Generate a single function definition.
    fn generate_function(&self, _w: &mut CodeWriter, _f: &FunctionIR) {}

    /// Generate a single component definition or registration.
    fn generate_component(&self, _w: &mut CodeWriter, _name: &str, _comp: &ComponentIR) {}

    /// Generate a single store definition or registration.
    fn generate_store(&self, _w: &mut CodeWriter, _name: &str, _store: &StoreIR) {}

    // ── Per-component (Camp B) overrides ────────────────────

    /// Generate output file(s) for a single component.
    ///
    /// Override this to produce one or more files per component.
    /// The default returns an empty vector (no output).
    fn gen_component_file(
        &self,
        _name: &str,
        _comp: &ComponentIR,
        _ir: &IRProgram,
    ) -> Vec<CompiledFile> {
        vec![]
    }

    /// Generate output file(s) for a single store.
    ///
    /// Override this to produce one or more files per store.
    /// The default returns an empty vector (no output).
    fn gen_store_file(&self, _name: &str, _store: &StoreIR) -> Vec<CompiledFile> {
        vec![]
    }

    // ── Built-in strategies ─────────────────────────────────

    /// Generate output by iterating all modules (Camp A style).
    ///
    /// For each module, calls [`generate_module_header`] once, then
    /// [`generate_function`], [`generate_component`], and [`generate_store`]
    /// for each item.  All output is written to a single file using the
    /// target's file extension.
    ///
    /// Override the individual `generate_*` methods to customize output.
    #[must_use]
    fn generate_per_module(&self, ir: &IRProgram) -> CompileOutput {
        let mut w = CodeWriter::new();
        for (_name, module) in &ir.modules {
            self.generate_module_header(&mut w, module);
            for (_func_name, func) in &module.functions {
                self.generate_function(&mut w, func);
            }
            for (comp_name, comp) in &module.components {
                self.generate_component(&mut w, comp_name, comp);
            }
            for (store_name, store) in &module.stores {
                self.generate_store(&mut w, store_name, store);
            }
        }
        CompileOutput::success(vec![CompiledFile {
            path: format!("output.{}", self.target().extension()),
            content: w.into_string(),
        }])
    }

    /// Generate output by iterating all components and stores (Camp B style).
    ///
    /// Delegates to [`gen_component_file`] and [`gen_store_file`] for each
    /// component and store, collecting all returned files into a single
    /// `CompileOutput`.
    #[must_use]
    fn generate_per_component(&self, ir: &IRProgram) -> CompileOutput {
        let mut files = Vec::new();
        for (name, comp) in ir.all_components() {
            files.extend(self.gen_component_file(name, comp, ir));
        }
        for (name, store) in ir.all_stores() {
            files.extend(self.gen_store_file(name, store));
        }
        CompileOutput::success(files)
    }
}
